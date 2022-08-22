use crate::{metrics::METRICS, models::Caip2ChainId, MainLoopFlow, OracleControlFlow};
use anyhow::ensure;
use async_trait::async_trait;
use graphql_client::{GraphQLQuery, Response};
use itertools::Itertools;
use reqwest::Url;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

pub struct SubgraphQuery {
    url: Url,
}

impl SubgraphQuery {
    pub fn new(url: Url) -> Self {
        Self { url }
    }
}

/// Retrieves the latest state from a subgraph.
#[async_trait]
pub trait SubgraphApi {
    type State;
    type Error;

    async fn get_subgraph_state(&self) -> Result<Option<Self::State>, Self::Error>;
}

#[async_trait]
impl SubgraphApi for SubgraphQuery {
    type State = (u64, GlobalState);
    type Error = SubgraphQueryError;

    async fn get_subgraph_state(&self) -> Result<Option<Self::State>, Self::Error> {
        let response_body = query(self.url.clone()).await?;
        match response_body.errors.as_deref() {
            Some([]) | None => {}
            Some(errors) => {
                // We only deal with the first error and ignore the rest.
                let e = &errors[0];
                if e.message == "indexing_error" {
                    return Err(SubgraphQueryError::IndexingError);
                } else {
                    return Err(SubgraphQueryError::Other(anyhow::anyhow!("{}", e.message)));
                }
            }
        }
        if let Some(data) = response_body.data {
            let (gs, meta) = match (data.global_state, data.meta) {
                (Some(gs), Some(meta)) => (gs, meta),
                _ => return Ok(None),
            };
            Ok(Some((
                meta.block.number as u64,
                gs.try_into().map_err(SubgraphQueryError::BadData)?,
            )))
        } else {
            Err(SubgraphQueryError::Other(anyhow::anyhow!(
                "No response data"
            )))
        }
    }
}

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

async fn query(url: Url) -> reqwest::Result<Response<graphql::subgraph_state::ResponseData>> {
    // TODO: authentication token.
    let client = reqwest::Client::builder()
        .user_agent("block-oracle")
        .build()
        .unwrap();
    let request_body = graphql::SubgraphState::build_query(graphql::subgraph_state::Variables);
    let request = client.post(url).json(&request_body);
    let response = request.send().await?.error_for_status()?;
    response.json().await
}

/// Coordinates the retrieval of subgraph data and the transition of its own internal state.
pub struct SubgraphStateTracker<A>
where
    A: SubgraphApi,
{
    last_result: Result<Option<A::State>, Arc<A::Error>>,
    subgraph_api: A,
}

impl<A> SubgraphStateTracker<A>
where
    A: SubgraphApi,
    A::State: Clone + PartialEq,
{
    pub fn new(api: A) -> Self {
        Self {
            last_result: Ok(None),
            subgraph_api: api,
        }
    }

    pub fn result(&self) -> Result<Option<&A::State>, Arc<A::Error>> {
        match self.last_result {
            Ok(Some(ref s)) => Ok(Some(s)),
            Ok(None) => Ok(None),
            Err(ref e) => Err(e.clone()),
        }
    }

    /// Handles the retrieval of new subgraph state and the transition of its internal [`State`]
    pub async fn refresh(&mut self) {
        info!("Fetching latest subgraph state");

        let result = self
            .subgraph_api
            .get_subgraph_state()
            .await
            .map_err(Arc::new);

        if result.is_err() {
            error!("The subgraph is failed.");
        } else if result.as_ref().ok() != self.last_result.as_ref().ok() {
            warn!("The subgraph's state has changed since the last time we checked. This is expected.");
        }

        self.last_result = result;
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
    use anyhow::anyhow;
    use hyper::server::conn::Http;
    use hyper::{Body, Response};
    use std::sync::Mutex;
    use tokio::net::TcpListener;

    #[derive(Clone, PartialEq)]
    struct CounterState {
        counter: u8,
    }

    impl CounterState {
        fn bump(&mut self) {
            self.counter += 1;
        }
    }

    struct FakeApi {
        state: Arc<Mutex<CounterState>>,
        error_switch: bool,
        data_switch: bool,
        error_description: &'static str,
    }

    impl FakeApi {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(CounterState { counter: 0 })),
                error_switch: true,
                data_switch: false,
                error_description: "oops",
            }
        }

        fn bump_state_counter(&self) {
            self.state.lock().unwrap().bump();
        }

        /// Passing 'true` will cause the fake api to send data in the next operation
        fn toggle_data(&mut self, switch: bool) {
            self.data_switch = switch;
        }

        /// Passing 'true` will cause the fake api to fail on the next operation
        fn toggle_errors(&mut self, switch: bool) {
            self.error_switch = switch;
        }

        fn set_error(&mut self, text: &'static str) {
            self.error_description = text
        }
    }

    #[async_trait]
    impl SubgraphApi for FakeApi {
        type State = CounterState;
        type Error = anyhow::Error;

        async fn get_subgraph_state(&self) -> anyhow::Result<Option<Self::State>> {
            match (self.error_switch, self.data_switch) {
                (false, true) => {
                    self.bump_state_counter();
                    Ok(Some(self.state.lock().unwrap().clone()))
                }
                (false, false) => Ok(None),
                (true, _) => Err(anyhow!(self.error_description)),
            }
        }
    }

    #[tokio::test]
    async fn valid_state_transitions() {
        let api = FakeApi::new();
        let mut state_tracker = SubgraphStateTracker::new(api);

        // An initial state should be uninitialized, with no errors
        assert!(matches!(state_tracker.result(), Ok(None)));

        // Failed initialization.
        state_tracker.subgraph_api.toggle_errors(true);
        state_tracker.refresh().await;
        assert!(matches!(state_tracker.result(), Err(_)));

        // Remove all errors, state is okay again.
        state_tracker.subgraph_api.toggle_errors(false);
        state_tracker.refresh().await;
        assert!(matches!(state_tracker.result(), Ok(None)));

        // Once the subgraph has valid data, the state tracker can yield it.
        state_tracker.subgraph_api.toggle_data(true);
        state_tracker.refresh().await;
        assert!(matches!(state_tracker.result(), Ok(Some(_))));
        assert_eq!(state_tracker.result().unwrap().unwrap().counter, 1);

        // Sudden failure.
        state_tracker.subgraph_api.toggle_errors(true);
        state_tracker.refresh().await;
        assert_eq!(state_tracker.result().err().unwrap().to_string(), "oops");

        // Subsequent failures can have different error messages and that's okay.
        state_tracker.subgraph_api.set_error("oh no");
        state_tracker.refresh().await;
        assert_eq!(state_tracker.result().err().unwrap().to_string(), "oh no");

        // We then recover from failure, becoming valid again and presenting new data.
        state_tracker.subgraph_api.toggle_errors(false);
        state_tracker.refresh().await;
        assert_eq!(state_tracker.result().unwrap().unwrap().counter, 2);

        // Valid once again.
        state_tracker.refresh().await;
        assert_eq!(state_tracker.result().unwrap().unwrap().counter, 3);
    }

    async fn http_server_serving_static_file(contents: &'static str) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |_req| async move {
                Ok::<_, hyper::Error>(Response::new(Body::from(contents)))
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

    #[tokio::test]
    async fn successfully_decode_subgraph_data() {
        let url = http_server_serving_static_file(include_str!(
            "resources/test-response-subgraph-with-data.json",
        ))
        .await;

        let mut subgraph_state = SubgraphStateTracker::new(SubgraphQuery::new(url));
        subgraph_state.refresh().await;

        let data = subgraph_state.result().unwrap().unwrap();

        assert_eq!(data.0, 7333988);
        assert_eq!(data.1.encoding_version, 0);
        assert_eq!(data.1.latest_epoch_number, Some(150));
        assert_eq!(data.1.networks.len(), 27);
        // ...
    }
}
