//! Functionality related to multi-threading.

use std::process;
use std::thread;

/// PoisonPill is used to cause the process to abort if there are
/// any panics in any thread. This may lead to a resource leak,
/// but also allows us to better handle bugs in threads.
/// TODO: Remove after squashing bugs.
pub struct PoisonPill;

impl Drop for PoisonPill {
    fn drop(&mut self) {
        if thread::panicking() {
            process::exit(1);
        }
    }
}
