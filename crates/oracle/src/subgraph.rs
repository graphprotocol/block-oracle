use super::metrics::METRICS;
use crate::models::Caip2ChainId;
use crate::runner::error_handling::{MainLoopFlow, OracleControlFlow};
use anyhow::ensure;
use graphql_client::{GraphQLQuery, Response};
use itertools::Itertools;
use reqwest::Url;
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug, thiserror::Error)]
pub enum SubgraphQueryError {
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error("The subgraph is in a failed state")]
    IndexingError,
    #[error("Bad or invalid entity data found in the subgraph: {}", .0.to_string())]
    BadData(anyhow::Error),
    #[error("Unknown error: {0}")]
    Other(anyhow::Error),
}

impl MainLoopFlow for SubgraphQueryError {
    fn instruction(&self) -> OracleControlFlow {
        match self {
            SubgraphQueryError::Transport(_) => {
                // There's no guarantee that the `reqwest::Error` disappears if we wait a full
                // minute, it's just a simple heuristic that might work when dealing with
                // straightforward connectivity issues.
                OracleControlFlow::Continue(Some(Duration::from_secs(60)))
            }
            // Other errors require external intervention, so we poll less frequently.
            _ => OracleControlFlow::Continue(Some(Duration::from_secs(600))),
        }
    }
}

pub async fn query_subgraph(
    url: &Url,
    bearer_token: &str,
) -> Result<SubgraphState, SubgraphQueryError> {
    info!("Fetching latest subgraph state");

    let client = reqwest::Client::builder()
        .user_agent("block-oracle")
        .build()
        .unwrap();
    let request_body = graphql::SubgraphState::build_query(graphql::subgraph_state::Variables);
    let request = client
        .post(url.clone())
        .json(&request_body)
        .bearer_auth(bearer_token);
    let response = request.send().await?.error_for_status()?;
    let response_body: Response<graphql::subgraph_state::ResponseData> = response.json().await?;

    match response_body.errors.as_deref() {
        Some([]) | None => {
            METRICS.set_subgraph_indexing_errors(false);
        }
        Some(errors) => {
            // We only deal with the first error and ignore the rest.
            let e = &errors[0];
            if e.message == "indexing_error" {
                METRICS.set_subgraph_indexing_errors(true);
                return Err(SubgraphQueryError::IndexingError);
            } else {
                return Err(SubgraphQueryError::Other(anyhow::anyhow!("{}", e.message)));
            }
        }
    }

    let data = if let Some(data) = response_body.data {
        data
    } else {
        return Err(SubgraphQueryError::Other(anyhow::anyhow!(
            "No response data"
        )));
    };

    let last_indexed_block_number = data.meta.block.number as u64;
    let global_state = if let Some(gs) = data.global_state {
        Some(gs.try_into().map_err(SubgraphQueryError::BadData)?)
    } else {
        None
    };

    Ok(SubgraphState {
        last_indexed_block_number,
        global_state,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubgraphState {
    pub last_indexed_block_number: u64,
    pub global_state: Option<GlobalState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalState {
    pub networks: Vec<Network>,
    pub encoding_version: i64,
    pub latest_epoch_number: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Network {
    pub id: Caip2ChainId,
    pub latest_block_number: u64,
    pub acceleration: i64,
    pub delta: i64,
    pub updated_at_epoch_number: u64,
    pub array_index: u64,
}

impl From<Network> for epoch_encoding::Network {
    fn from(val: Network) -> Self {
        epoch_encoding::Network {
            block_number: val.latest_block_number,
            block_delta: val.delta,
            array_index: val.array_index,
        }
    }
}

impl TryFrom<graphql::subgraph_state::SubgraphStateGlobalStateNetworks> for Network {
    type Error = anyhow::Error;

    fn try_from(
        mut value: graphql::subgraph_state::SubgraphStateGlobalStateNetworks,
    ) -> Result<Self, Self::Error> {
        ensure!(
            value.block_numbers.len() == 1,
            "Network with ID {} has invalid block numbers",
            value.id
        );

        let id: Caip2ChainId = value
            .id
            .as_str()
            .parse()
            .map_err(|s| anyhow::anyhow!("Invalid network name: {}", s))?;

        let block_number_info = value.block_numbers.pop().unwrap();
        let latest_block_number: u64 = block_number_info.block_number.parse()?;
        let acceleration: i64 = block_number_info.acceleration.parse()?;
        let delta: i64 = block_number_info.delta.parse()?;
        let updated_at_epoch_number: u64 = block_number_info.epoch_number.parse()?;
        let array_index = value
            .array_index
            .ok_or_else(|| anyhow::anyhow!("Expected a valid array_index for Network"))?
            as u64;

        METRICS.set_latest_block_number(id.as_str(), "subgraph", latest_block_number as i64);

        Ok(Network {
            id,
            latest_block_number,
            acceleration,
            delta,
            updated_at_epoch_number,
            array_index,
        })
    }
}

impl TryFrom<graphql::subgraph_state::SubgraphStateGlobalState> for GlobalState {
    type Error = anyhow::Error;

    fn try_from(
        value: graphql::subgraph_state::SubgraphStateGlobalState,
    ) -> Result<Self, Self::Error> {
        let mut networks = vec![];

        for (expected_i, value) in value.networks.into_iter().enumerate() {
            ensure!(
                value.array_index == Some(expected_i as i64),
                "Network with ID {} has a bad index",
                value.id
            );

            networks.push(Network::try_from(value)?);
        }

        ensure!(
            networks.iter().map(|s| &s.id).all_unique(),
            "Found duplicated network IDs"
        );

        Ok(Self {
            latest_epoch_number: value
                .latest_valid_epoch
                .map(|x| x.epoch_number.parse())
                .transpose()?,
            encoding_version: value.encoding_version,
            networks,
        })
    }
}

mod graphql {
    use super::*;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/graphql/schema.graphql",
        query_path = "src/graphql/query.graphql",
        deprecated = "warn"
    )]
    pub struct SubgraphState;
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::server::conn::Http;
    use hyper::{Body, Response};
    use serde_json::json;
    use serde_json::Value as Json;
    use tokio::net::TcpListener;

    struct FakeServer {
        value: serde_json::Value,
    }

    impl FakeServer {
        fn new(value: serde_json::Value) -> Self {
            Self { value }
        }

        async fn serve(self) -> Url {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();

            tokio::spawn(async move {
                let service = hyper::service::service_fn({
                    |_req| {
                        let response = self.value.clone();
                        async move {
                            Ok::<_, hyper::Error>(Response::new(Body::from(response.to_string())))
                        }
                    }
                });

                loop {
                    let (stream, _) = listener.accept().await.unwrap();

                    Http::new().serve_connection(stream, service).await.unwrap();
                }
            });

            let mut url = Url::parse("http://127.0.0.1").unwrap();
            url.set_port(Some(port)).unwrap();
            url
        }
    }

    async fn parse_response(json: Json) -> Result<SubgraphState, SubgraphQueryError> {
        let server = FakeServer::new(json);
        let url = &server.serve().await;
        let bearer_token = "foobar";
        query_subgraph(url, bearer_token).await
    }

    #[tokio::test]
    async fn no_latest_valid_epoch() {
        let state = parse_response(json!({
            "data": {
                "globalState": {
                    "activeNetworkCount": 0,
                    "networks": [],
                    "encodingVersion": 0,
                },
                "_meta": {
                    "block": {
                        "number": 7333988
                    }
                }
            }
        }))
        .await
        .unwrap();
        assert!(state.global_state.as_ref().unwrap().networks.is_empty());
        assert_eq!(
            state.global_state.as_ref().unwrap().latest_epoch_number,
            None
        );
    }

    #[tokio::test]
    async fn no_networks() {
        let state = parse_response(json!({
            "data": {
                "globalState": {
                    "activeNetworkCount": 0,
                    "networks": [],
                    "encodingVersion": 0,
                    "latestValidEpoch": {
                        "epochNumber": "150"
                    }
                },
                "_meta": {
                    "block": {
                        "number": 7333988
                    }
                }
            }
        }))
        .await
        .unwrap();
        assert!(state.global_state.as_ref().unwrap().networks.is_empty());
        assert_eq!(
            state.global_state.as_ref().unwrap().latest_epoch_number,
            Some(150)
        );
    }

    #[tokio::test]
    async fn many_networks() {
        let state = parse_response(
            serde_json::from_str(include_str!(
                "resources/test-response-subgraph-with-data.json",
            ))
            .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(state.last_indexed_block_number, 7333988);
        let gs = state.global_state.unwrap();
        assert_eq!(gs.encoding_version, 0);
        assert_eq!(gs.latest_epoch_number, Some(150));
        assert_eq!(gs.networks.len(), 27);
    }

    #[tokio::test]
    async fn uninitialized() {
        let state = parse_response(json!({
            "data": {
                "_meta": {
                    "block": {
                        "number": 2
                    }
                }
            }
        }))
        .await
        .unwrap();
        assert_eq!(state.last_indexed_block_number, 2);
        assert!(state.global_state.is_none());
    }

    #[tokio::test]
    async fn indexing_error() {
        let error = parse_response(json!({
            "data": {
                "_meta": {
                    "block": {
                        "number": 2
                    }
                }
            },
            "errors": [
                {
                    "message": "indexing_error"
                }
            ]
        }))
        .await
        .err()
        .unwrap();
        assert!(matches!(error, SubgraphQueryError::IndexingError));
    }
}
