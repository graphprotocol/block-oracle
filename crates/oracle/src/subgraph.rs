use crate::models::Caip2ChainId;
use anyhow::ensure;
use async_trait::async_trait;
use graphql_client::{GraphQLQuery, Response};
use itertools::Itertools;
use reqwest::Url;
use std::sync::Arc;
use tracing::{debug, error};

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
    type State = GlobalState;
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
            Ok(data
                .global_state
                .map(|gs| gs.try_into().map_err(SubgraphQueryError::BadData))
                .transpose()?)
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

async fn query(url: Url) -> reqwest::Result<Response<graphql::subgraph_state::ResponseData>> {
    // TODO: authentication token.
    let client = reqwest::Client::builder()
        .user_agent("block-oracle")
        .build()
        .unwrap();
    let request_body = graphql::SubgraphState::build_query(graphql::subgraph_state::Variables);
    let request = client.post(url).json(&request_body);
    let response = request.send().await?;

    Ok(response.json().await?)
}

/// Coordinates the retrieval of subgraph data and the transition of its own internal [`State`].
pub struct SubgraphStateTracker<A>
where
    A: SubgraphApi,
{
    last_state: Option<A::State>,
    error: Option<Arc<anyhow::Error>>,
    subgraph_api: A,
}

impl<A> SubgraphStateTracker<A>
where
    A: SubgraphApi,
    A::State: Clone,
    A::Error: Into<anyhow::Error>,
{
    pub fn new(api: A) -> Self {
        Self {
            last_state: None,
            error: None,
            subgraph_api: api,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.error.is_none() && self.last_state.is_some()
    }

    pub fn is_uninitialized(&self) -> bool {
        self.last_state.is_none()
    }

    pub fn is_failed(&self) -> bool {
        self.error.is_some() && self.last_state.is_some()
    }

    pub fn data(&self) -> Option<&A::State> {
        self.last_state.as_ref()
    }

    pub fn error(&self) -> Option<Arc<anyhow::Error>> {
        self.error.clone()
    }

    /// Handles the retrieval of new subgraph state and the transition of its internal [`State`]
    pub async fn refresh(&mut self) {
        debug!("Fetching latest subgraph state");

        match self.subgraph_api.get_subgraph_state().await {
            Ok(s) => {
                self.last_state = s;
                self.error = None;
            }
            Err(err) => {
                if self.is_failed() {
                    error!("Failed to retrieve state from a previously failed subgraph");
                }
                self.error = Some(Arc::new(err.into()));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub networks: Vec<Network>,
    pub encoding_version: i64,
    pub latest_epoch_number: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Network {
    pub id: Caip2ChainId,
    pub latest_block_number: u64,
    pub acceleration: i64,
    pub delta: i64,
    pub updated_at_epoch_number: u64,
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

        let id = value
            .id
            .as_str()
            .parse()
            .map_err(|s| anyhow::anyhow!("Invalid network name: {}", s))?;
        let block_number_info = value.block_numbers.pop().unwrap();

        Ok(Network {
            id,
            latest_block_number: block_number_info.block_number.parse()?,
            acceleration: block_number_info.acceleration.parse()?,
            delta: block_number_info.delta.parse()?,
            updated_at_epoch_number: block_number_info.epoch.epoch_number.parse()?,
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
    use std::sync::Mutex;

    #[derive(Clone)]
    struct FakeInnerState {
        counter: u8,
    }

    impl FakeInnerState {
        fn bump(&mut self) {
            self.counter += 1;
        }
    }

    struct FakeApi {
        state: Arc<Mutex<FakeInnerState>>,
        error_switch: bool,
        data_switch: bool,
        error_description: &'static str,
    }

    impl FakeApi {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(FakeInnerState { counter: 0 })),
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
        type State = FakeInnerState;
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
        assert!(state_tracker.data().is_none());
        assert!(state_tracker.error().is_none());
        assert!(!state_tracker.is_valid());
        assert!(state_tracker.is_uninitialized());

        // Initialization can fail, and the state will still be uninitialized.
        state_tracker.subgraph_api.toggle_errors(true);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_none());
        assert!(state_tracker.error().is_some());
        assert!(!state_tracker.is_valid());
        assert!(state_tracker.is_uninitialized());

        // Even if the API is responsive, it might still send us no data and we will stay in the
        // Uninitialized state. All previous errors will be removed.
        state_tracker.subgraph_api.toggle_errors(false);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_none());
        assert!(state_tracker.error().is_none());
        assert!(!state_tracker.is_valid());
        assert!(state_tracker.is_uninitialized());

        // Once the subgraph has valid data, the state tracker can yield it.
        state_tracker.subgraph_api.toggle_data(true);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.error().is_none());
        assert!(state_tracker.is_valid());
        assert_eq!(state_tracker.data().unwrap().counter, 1);

        // On failure, we retain the last valid data, but state is considered invalid.
        state_tracker.subgraph_api.toggle_errors(true);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.error().is_some());
        assert!(!state_tracker.is_valid());
        assert!(state_tracker.is_failed());
        assert_eq!(state_tracker.data().unwrap().counter, 1);
        assert_eq!(state_tracker.error().unwrap().to_string(), "oops");

        // We can fail again, keeping the same data as before.
        // Errors might be different from previous failed states.
        state_tracker.subgraph_api.set_error("oh no");
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(!state_tracker.is_valid());
        assert!(state_tracker.is_failed());
        assert_eq!(state_tracker.data().unwrap().counter, 1);
        assert_eq!(state_tracker.error().unwrap().to_string(), "oh no");

        // We then recover from failure, becoming valid again and presenting new data.
        state_tracker.subgraph_api.toggle_errors(false);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.is_valid());
        assert_eq!(state_tracker.data().unwrap().counter, 2);

        // We can successfull valid states.
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.is_valid());
        assert_eq!(state_tracker.data().unwrap().counter, 3);
    }
}
