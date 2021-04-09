//! position.rs
//! Holds Position struct, which is the most important data
//! structure for the engine.
//! It holds a Chess position, and methods used for assessing
//! itself.

use crate::coretypes::Color;
use crate::mailbox::Mailbox;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    board: Mailbox,
    side_to_move: Color,
}
