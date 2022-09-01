use std::ops::ControlFlow;

/// Tells us whether an application error is recoverable or not. If recoverable,
/// the cooldown should last for the given amount of polling cycles before
/// resuming. Rather than directly specifying a
/// [`Duration`](std::time::Duration), this approach allows for scaling all wait
/// times by a configuration value.
pub type OracleControlFlow = ControlFlow<(), u32>;

/// Sends instructions to control the Oracle main loop flow.
///
/// When continuing, the implementor can opt to define a different duration for
/// the sleep cycle.
pub trait MainLoopFlow {
    fn instruction(&self) -> OracleControlFlow;
}
