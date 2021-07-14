//! Search functions.

mod alpha_beta;
mod minimax;
mod negamax;
mod search;

pub use alpha_beta::alpha_beta;
pub use minimax::minimax;
pub use negamax::negamax;
pub use search::search;

use std::time::Duration;

use crate::coretypes::Move;
use crate::evaluation::Cp;
use crate::movelist::Line;

/// General information gathered from searching a position.
/// members:
/// `best_move`: Best move to make for a position discovered.
/// `score`: The centipawn evaluation of making the best move.
/// `pv_line`: The principal variation, or the line of play following the best move.
/// `nodes`: The number of nodes visited from the search.
/// `elapsed` Time taken to complete a search.
#[derive(Debug, Copy, Clone)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: Cp,
    pub pv_line: Line,
    pub nodes: u64,
    pub elapsed: Duration,
}
