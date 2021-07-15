//! Search functions.

mod alpha_beta;
mod ids;
mod minimax;
mod negamax;

pub use alpha_beta::*;
pub use ids::*;
pub use minimax::*;
pub use negamax::*;

use std::time::Duration;

use crate::coretypes::{Color, Move};
use crate::evaluation::Cp;
use crate::movelist::Line;
use crate::Position;

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

/// Blunders Engine primary position search function. WIP.
pub fn search(position: Position, ply: u32) -> (Cp, Move) {
    assert_ne!(ply, 0);
    let result = ids(position, ply);
    (result.score, result.best_move)
}

impl Color {
    pub(super) const fn sign(&self) -> Cp {
        match self {
            Color::White => Cp(1),
            Color::Black => Cp(-1),
        }
    }
}
