use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, warn};

/// Gracefully handles interrupts and returns `true` from [`CtrlcHandler::poll_ctrlc`] if CTRL+C
/// was detected.
pub struct CtrlcHandler {
    ctrlc_received: Arc<AtomicBool>,
}

impl CtrlcHandler {
    const ORDERING: Ordering = Ordering::Relaxed;

    pub fn init() -> Self {
        let ctrlc = Arc::new(AtomicBool::new(false));
        let ctrlc_clone = ctrlc.clone();
        ctrlc::set_handler(move || {
            let pressed_already = ctrlc_clone.load(Self::ORDERING);
            if pressed_already {
                error!(
                    "CTRL+C was pressed a second time. Exiting immediately."
                );
                std::process::exit(0);
            } else {
                warn!(
                    "CTRL+C detected. Stopping... please wait. Press CTRL+C again to exit immediately."
                );
                ctrlc_clone.store(true, Self::ORDERING);
            }
        })
        .expect("Error setting the CTRL+C handler.");
        Self {
            ctrlc_received: ctrlc,
        }
    }

    pub fn poll_ctrlc(&self) -> bool {
        self.ctrlc_received.load(Self::ORDERING)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_twice_is_okay() {
        CtrlcHandler::init();
        CtrlcHandler::init();
    }

    #[test]
    fn poll_ctrlc_returns_false_if_not_pressed() {
        let ctrlc_handler = CtrlcHandler::init();
        assert!(!ctrlc_handler.poll_ctrlc());
    }
}
