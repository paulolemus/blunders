//! Engine struct acts as a simplified API for the various parts of the Blunders engine.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::Position;

pub struct Engine {
    _debug: bool,
    _root_position: Position,
    _stopper: Arc<AtomicBool>,
}
