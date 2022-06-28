use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
                std::process::exit(0);
            } else {
                println!(
                "\nCTRL-C detected. Stopping... please wait. Press CTRL-C to exit immediately.\n"
            );
                ctrlc_clone.store(true, Self::ORDERING);
            }
        })
        .expect("Error setting CTRL-C handler.");
        Self {
            ctrlc_received: ctrlc,
        }
    }

    pub fn poll_ctrlc(&self) -> bool {
        self.ctrlc_received.load(Self::ORDERING)
    }
}
