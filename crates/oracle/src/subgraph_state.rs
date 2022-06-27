//! Subgraph State Transitions
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error};

/// Exposes the current [`SubgraphState`] internal error.
#[derive(Debug, thiserror::Error)]
pub enum SubgraphStateError {
    #[error("Failed to retrieve latest subgraph state")]
    Failed(#[source] Arc<anyhow::Error>),
    #[error("Subgraph failed to initialize")]
    Uninitialized(#[source] Arc<anyhow::Error>),
}

impl crate::MainLoopFlow for SubgraphStateError {
    fn instruction(&self) -> crate::OracleControlFlow {
        use std::ops::ControlFlow::*;
        use SubgraphStateError::*;
        match self {
            outer_error @ Failed(error) => {
                error!(%error, "{outer_error}");
                Continue(None)
            }
            outer_error @ Uninitialized(error) => {
                error!(%error, "{outer_error}");
                Continue(None)
            }
        }
    }
}

/// Represents Subgraph states.
pub struct State<S, E> {
    last_state: Option<S>,
    error: Option<Arc<E>>,
}

/// Retrieves the latest state from a subgraph.
#[async_trait]
pub trait SubgraphApi {
    type State: Send;

    async fn get_subgraph_state(&self) -> anyhow::Result<Option<Self::State>>;
}

/// Coordinates the retrieval of subgraph data and the transition of its own internal [`State`].
pub struct SubgraphStateTracker<A>
where
    A: SubgraphApi,
    A::State: Clone,
{
    inner: State<A::State, anyhow::Error>,
    subgraph_api: A,
}

impl<A> SubgraphStateTracker<A>
where
    A: SubgraphApi,
    A::State: Clone,
{
    pub fn new(api: A) -> Self {
        let initial = State {
            last_state: None,
            error: None,
        };
        Self {
            inner: initial,
            subgraph_api: api,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.inner.error.is_none() && self.inner.last_state.is_some()
    }

    pub fn is_uninitialized(&self) -> bool {
        self.inner.last_state.is_none()
    }

    pub fn is_failed(&self) -> bool {
        self.inner.error.is_some() && self.inner.last_state.is_some()
    }

    pub fn data(&self) -> Option<&A::State> {
        self.inner.last_state.as_ref()
    }

    pub fn error(&self) -> Option<Arc<anyhow::Error>> {
        self.inner.error.clone()
    }

    /// Handles the retrieval of new subgraph state and the transition of its internal [`State`]
    pub async fn refresh(&mut self) {
        debug!("Fetching latest subgraph state");

        match self.subgraph_api.get_subgraph_state().await {
            Ok(s) => {
                self.inner.last_state = s;
                self.inner.error = None;
            }
            Err(err) => {
                if self.is_failed() {
                    error!("Failed to retrieve state from a previously failed subgraph");
                }
                self.inner.error = Some(Arc::new(err));
            }
        }
    }

    pub(crate) fn error_for_state(&self) -> Result<(), SubgraphStateError> {
        match (&self.inner.last_state, &self.inner.error) {
            (_, None) => Ok(()),
            (None, Some(e)) => Err(SubgraphStateError::Uninitialized(e.clone())),
            (Some(_), Some(e)) => Err(SubgraphStateError::Failed(e.clone())),
        }
    }
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
