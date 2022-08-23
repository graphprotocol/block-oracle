use std::{ops::ControlFlow, time::Duration};

pub type OracleControlFlow = ControlFlow<(), Option<Duration>>;

/// Sends instructions to control the Oracle main loop flow.
///
/// When continuing, the implementor can opt to define a different duration for the sleep cycle.
pub trait MainLoopFlow {
    fn instruction(&self) -> OracleControlFlow;
}
