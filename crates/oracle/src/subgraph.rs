use super::metrics::METRICS;
use crate::models::Caip2ChainId;
use crate::runner::error_handling::{MainLoopFlow, OracleControlFlow};
use anyhow::ensure;
use graphql_client::{GraphQLQuery, Response};
use itertools::Itertools;
use reqwest::Url;
use tracing::{error, info, warn};

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
                OracleControlFlow::Continue(4)
            }
            // Other errors require external intervention, so we poll less frequently.
            _ => OracleControlFlow::Continue(40),
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
    let global_state = data
        .global_state
        .map(|gs| gs.try_into())
        .transpose()
        .map_err(SubgraphQueryError::BadData)?;
    let last_payload: Option<Payload> = data
        .payloads.first()
        .map(|p| p.try_into())
        .transpose()
        .map_err(SubgraphQueryError::BadData)?;

    // Check if the last payload indexed by the subgraph is valid.
    if let Some(payload) = &last_payload {
        METRICS.set_subgraph_last_payload_health(payload.valid, payload.created_at);
    } else {
        warn!("Epoch Subgraph had no previous payload");
    };

    Ok(SubgraphState {
        last_indexed_block_number,
        global_state,
        last_payload,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubgraphState {
    pub last_indexed_block_number: u64,
    pub global_state: Option<GlobalState>,
    pub last_payload: Option<Payload>,
}

impl SubgraphState {
    pub fn latest_epoch_number(&self) -> Option<u64> {
        self.global_state
            .as_ref()
            .and_then(|gs| gs.latest_epoch_number)
    }

    pub fn has_registered_networks(&self) -> bool {
        self.global_state
            .as_ref()
            .map(|gs| gs.networks.len())
            .unwrap_or(0)
            > 0
    }
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
    pub array_index: u64,
    pub latest_block_update: Option<BlockUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockUpdate {
    pub block_number: u64,
    pub acceleration: i64,
    pub delta: i64,
    pub updated_at_epoch_number: u64,
}

impl From<Network> for epoch_encoding::Network {
    fn from(val: Network) -> Self {
        let (block_number, block_delta) = if let Some(block_update) = val.latest_block_update {
            (block_update.block_number, block_update.delta)
        } else {
            (0, 0)
        };

        epoch_encoding::Network {
            block_number,
            block_delta,
            array_index: val.array_index,
        }
    }
}

impl TryFrom<graphql::subgraph_state::SubgraphStateGlobalStateNetworks> for Network {
    type Error = anyhow::Error;

    fn try_from(
        mut value: graphql::subgraph_state::SubgraphStateGlobalStateNetworks,
    ) -> Result<Self, Self::Error> {
        let id: Caip2ChainId = value
            .id
            .as_str()
            .parse()
            .map_err(|s| anyhow::anyhow!("Invalid network name: {}", s))?;

        let array_index = value
            .array_index
            .ok_or_else(|| anyhow::anyhow!("Expected a valid array_index for Network"))?
            as u64;

        ensure!(
            value.block_numbers.len() <= 1,
            "Network with ID {} has multiple block numbers. Expected either zero or one.",
            value.id
        );

        let latest_block_update = if let Some(block_data) = value.block_numbers.pop() {
            let block_update = BlockUpdate {
                block_number: block_data.block_number.parse()?,
                acceleration: block_data.acceleration.parse()?,
                delta: block_data.delta.parse()?,
                updated_at_epoch_number: { block_data.epoch_number.parse()? },
            };
            METRICS.set_latest_block_number(
                id.as_str(),
                "subgraph",
                block_update.block_number as i64,
            );
            Some(block_update)
        } else {
            info!("Network {} is uninitialized", id.as_str());
            None
        };

        Ok(Network {
            id,
            array_index,
            latest_block_update,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    valid: bool,
    created_at: i64,
}

impl TryFrom<&graphql::subgraph_state::SubgraphStatePayloads> for Payload {
    type Error = anyhow::Error;

    fn try_from(
        value: &graphql::subgraph_state::SubgraphStatePayloads,
    ) -> Result<Self, Self::Error> {
        Ok(Payload {
            valid: value.valid,
            created_at: value.created_at.parse()?,
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
                },
                "payloads": [
                    {
                        "valid": true,
                        "createdAt": "7503546"
                    }
                ]
            }
        }))
        .await
        .unwrap();
        assert!(state.global_state.as_ref().unwrap().networks.is_empty());
        assert_eq!(
            state.global_state.as_ref().unwrap().latest_epoch_number,
            None
        );
        assert_eq!(
            state.last_payload,
            Some(Payload {
                valid: true,
                created_at: 7503546
            })
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
                },
                "payloads": []

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
                },
                "payloads":[]
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
                },
                "payloads": []
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
