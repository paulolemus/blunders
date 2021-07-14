//! Primary Search algorithm for engine.

use crate::coretypes::Move;
use crate::evaluation::Cp;
use crate::search;
use crate::Position;

/// Blunders Engine primary position search function. WIP.
pub fn search(position: Position, ply: u32) -> (Cp, Move) {
    debug_assert_ne!(ply, 0);
    let result = search::negamax(position, ply);
    (result.score, result.best_move)
}
