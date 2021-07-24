//! Search functions.

mod alpha_beta;
mod ids;
mod minimax;
mod negamax;
mod quiescence;

pub use alpha_beta::*;
pub use ids::*;
pub use minimax::*;
pub use negamax::*;
pub use quiescence::*;

use std::fmt::{self, Display};
use std::time::Duration;

use crate::coretypes::{Color, Move};
use crate::eval::Cp;
use crate::movelist::Line;
use crate::transposition::TranspositionTable;
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

impl SearchResult {
    /// Get average nodes per second of search.
    pub fn nps(&self) -> f64 {
        (self.nodes as f64 / self.elapsed.as_secs_f64()).round()
    }
}

impl Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut displayed = String::new();
        displayed.push_str("SearchResult {\n");
        displayed.push_str(&format!("    best_move: {}\n", self.best_move));
        displayed.push_str(&format!("    score    : {}\n", self.score));
        displayed.push_str(&format!("    pv_line  : {}\n", self.pv_line));
        displayed.push_str(&format!("    nodes    : {}\n", self.nodes));
        displayed.push_str(&format!(
            "    elapsed  : {}.{}s\n",
            self.elapsed.as_secs(),
            self.elapsed.subsec_millis()
        ));
        displayed.push_str("}\n");

        write!(f, "{}", displayed)
    }
}

/// Blunders Engine primary position search function. WIP.
pub fn search(position: Position, ply: u32) -> SearchResult {
    assert_ne!(ply, 0);
    ids(position, ply)
}

/// Blunders Engine primary position search function. WIP.
pub fn search_with_tt(position: Position, ply: u32, tt: &mut TranspositionTable) -> SearchResult {
    assert_ne!(ply, 0);
    ids_with_tt(position, ply, tt)
}

impl Color {
    pub const fn sign(&self) -> Cp {
        match self {
            Color::White => Cp(1),
            Color::Black => Cp(-1),
        }
    }
}
