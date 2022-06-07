//! Subgraph State Transitions
use async_trait::async_trait;
use std::sync::Arc;
use thiserror;
use tracing::{debug, error, info};

use self::State::*;

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
///
/// Errors will not be moved out of this type, so we use [`anyhow::Error`] because we only need to
/// log and/or display them.
enum State<S> {
    /// Initial internal state when the Oracle starts up.
    ///
    /// Will carry errors if initialization fails, and is always considered invalid.
    ///
    /// Can only be transitioned to [`State::Valid`] and to itself.
    /// Can only be transitioned from itself.
    Uninitialized { error: Option<Arc<anyhow::Error>> },

    /// Valid state.
    ///
    /// Can only be transitioned to [`State::Failed`] and to itself.
    /// Can be transitioned from all other variants, including itself.
    Valid { state: S },

    /// Failed attempt at retrieving subgraph state.
    ///
    /// Will keep the latest known valid state, and it is considered invalid.
    ///
    /// Can only be transitioned between [`State::Valid`] and [`State::Failed`].
    Failed {
        previous: S,
        error: Arc<anyhow::Error>,
    },
}

/// Retrieves the latest state from a subgraph.
#[async_trait]
pub trait SubgraphApi {
    type State: Send;
    async fn get_subgraph_state(&self) -> anyhow::Result<Self::State>;
}

/// Coordinates the retrieval of subgraph data and the transition of its own internal [`State`].
pub struct SubgraphStateTracker<S, A = S>
where
    S: Clone,
    A: SubgraphApi<State = S>,
{
    inner: State<S>,
    subgraph_api: A,
}

impl<S, A> SubgraphStateTracker<S, A>
where
    S: Clone,
    A: SubgraphApi<State = S>,
{
    pub fn new(api: A) -> Self {
        let initial = State::Uninitialized { error: None };
        Self {
            inner: initial,
            subgraph_api: api,
        }
    }

    pub fn is_valid(&self) -> bool {
        matches!(self.inner, State::Valid { .. })
    }

    pub fn data(&self) -> Option<&S> {
        match &self.inner {
            Valid { state } => Some(state),
            Failed { previous, .. } => Some(previous),
            Uninitialized { .. } => None,
        }
    }

    pub fn error(&self) -> Option<Arc<anyhow::Error>> {
        match &self.inner {
            Uninitialized { error } => error.clone(),
            Failed { error, .. } => Some(error.clone()),
            Valid { .. } => None,
        }
    }

    /// Handles the retrieval of new subgraph state and the transition of its internal [`State`]
    pub async fn refresh(&mut self) {
        debug!("Fetching latest subgraph state");
        let new_state = self.subgraph_api.get_subgraph_state().await;
        self.inner = match (&self.inner, new_state) {
            (_, Ok(state)) => {
                info!("Retrieved new subgraph state");
                Valid { state }
            }
            (Uninitialized { .. }, Err(error)) => Uninitialized {
                error: Some(Arc::new(error)),
            },
            (Failed { previous, .. }, Err(error)) => {
                error!("Failed to retrieve state from a previously failed subgraph");
                Failed {
                    previous: previous.clone(),
                    error: Arc::new(error),
                }
            }
            (Valid { state }, Err(error)) => Failed {
                previous: state.clone(),
                error: Arc::new(error),
            },
        }
    }

    pub(crate) fn error_for_state(&self) -> Result<(), SubgraphStateError> {
        match &self.inner {
            Failed { error, .. } => Err(SubgraphStateError::Failed(error.clone())),
            Uninitialized { error: Some(error) } => {
                Err(SubgraphStateError::Uninitialized(error.clone()))
            }
            Valid { .. } | Uninitialized { error: None } => Ok(()),
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
        switch: bool,
        error_description: &'static str,
    }

    impl FakeApi {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(FakeInnerState { counter: 0 })),
                switch: true,
                error_description: "oops",
            }
        }

        fn bump_state_counter(&self) {
            self.state.lock().unwrap().bump();
        }

        /// Passing 'true` will cause the fake api to fail on the next operation
        fn toggle_errors(&mut self, switch: bool) {
            self.switch = !switch;
        }

        fn set_error(&mut self, text: &'static str) {
            self.error_description = text
        }
    }

    #[async_trait]
    impl SubgraphApi for FakeApi {
        type State = FakeInnerState;

        async fn get_subgraph_state(&self) -> anyhow::Result<Self::State> {
            if self.switch {
                self.bump_state_counter();
                Ok(self.state.lock().unwrap().clone())
            } else {
                Err(anyhow!(self.error_description))
            }
        }
    }

    #[tokio::test]
    async fn state_transitions() {
        let api = FakeApi::new();
        let mut state_tracker = SubgraphStateTracker::new(api);

        // An initial state should be uninitialized, with no errors
        assert!(state_tracker.data().is_none());
        assert!(state_tracker.error().is_none());
        assert!(!state_tracker.is_valid());
        assert!(matches!(
            state_tracker.inner,
            State::Uninitialized { error: None }
        ));

        // Initialization can fail, and the state will still be uninitialized.
        state_tracker.subgraph_api.toggle_errors(true);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_none());
        assert!(state_tracker.error().is_some());
        assert!(!state_tracker.is_valid());
        assert!(matches!(
            state_tracker.inner,
            State::Uninitialized { error: Some(_) }
        ));

        // Once initialized to a valid state, we have data.
        state_tracker.subgraph_api.toggle_errors(false);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.error().is_none());
        assert!(state_tracker.is_valid());
        assert!(matches!(state_tracker.inner, State::Valid { .. }));
        assert_eq!(state_tracker.data().unwrap().counter, 1);

        // On failure, we retain the last valid data, but state is considered invalid.
        state_tracker.subgraph_api.toggle_errors(true);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.error().is_some());
        assert!(!state_tracker.is_valid());
        assert!(matches!(state_tracker.inner, State::Failed { .. }));
        assert_eq!(state_tracker.data().unwrap().counter, 1);
        assert_eq!(state_tracker.error().unwrap().to_string(), "oops");

        // We can fail again, keeping the same data as before.
        // Errors might be different from previous failed states.
        state_tracker.subgraph_api.set_error("oh no");
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(!state_tracker.is_valid());
        assert!(matches!(state_tracker.inner, State::Failed { .. }));
        assert_eq!(state_tracker.data().unwrap().counter, 1);
        assert_eq!(state_tracker.error().unwrap().to_string(), "oh no");

        // We then recover from failure, becoming valid again and presenting new data.
        state_tracker.subgraph_api.toggle_errors(false);
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.is_valid());
        assert!(matches!(state_tracker.inner, State::Valid { .. }));
        assert_eq!(state_tracker.data().unwrap().counter, 2);

        // We can successfull valid states.
        state_tracker.refresh().await;
        assert!(state_tracker.data().is_some());
        assert!(state_tracker.is_valid());
        assert!(matches!(state_tracker.inner, State::Valid { .. }));
        assert_eq!(state_tracker.data().unwrap().counter, 3);
    }
}
