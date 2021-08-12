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
use std::sync::{atomic::AtomicBool, mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::arrayvec::display;
use crate::coretypes::{Color, Cp, Move};
use crate::movelist::Line;
use crate::timeman::Mode;
use crate::transposition::TranspositionTable;
use crate::Position;

/// General information gathered from searching a position.
/// members:
/// `best_move`: Best move to make for a position discovered.
/// `score`: The centipawn evaluation of making the best move, with absolute Cp (+White, -Black).
/// `pv_line`: The principal variation, or the line of play following the best move.
/// `player`: Active player of searched root position.
/// `depth`: Ply that was searched to. Currently this can be either partially or fully searched.
/// `nodes`: The number of nodes visited in the search.
/// `elapsed`: Time taken to complete a search.
/// `stopped`: Indicates if search was stopped part way.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: Cp,
    pub pv_line: Line,
    pub player: Color,
    pub depth: u32, // Same as Ply
    pub nodes: u64,
    pub elapsed: Duration,
    pub stopped: bool,
}

impl SearchResult {
    /// Get average nodes per second of search.
    pub fn nps(&self) -> f64 {
        (self.nodes as f64 / self.elapsed.as_secs_f64()).round()
    }

    /// Converts the score of the search into one that is relative to search's root player.
    pub fn relative_score(&self) -> Cp {
        self.score * self.player.sign()
    }

    /// Converts the score of the search into one that is absolute, with White as + and Black as -.
    pub fn absolute_score(&self) -> Cp {
        self.score
    }

    /// Returns the color who is leading in the search of the root position, or None if drawn.
    pub fn leading(&self) -> Option<Color> {
        match self.absolute_score().signum() {
            1 => Some(Color::White),
            -1 => Some(Color::Black),
            _ => None,
        }
    }
}

impl Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut displayed = String::new();
        displayed.push_str("SearchResult {\n");
        displayed.push_str(&format!("    best_move: {}\n", self.best_move));
        displayed.push_str(&format!("    abs_score: {}\n", self.absolute_score()));
        displayed.push_str(&format!("    pv_line  : {}\n", display(&self.pv_line)));
        displayed.push_str(&format!("    player   : {}\n", self.player));
        displayed.push_str(&format!("    depth    : {}\n", self.depth));
        displayed.push_str(&format!("    nodes    : {}\n", self.nodes));
        displayed.push_str(&format!(
            "    elapsed  : {}.{}s\n",
            self.elapsed.as_secs(),
            self.elapsed.subsec_millis()
        ));
        displayed.push_str(&format!("    stopped  : {}\n", self.stopped));
        displayed.push_str("}\n");

        write!(f, "{}", displayed)
    }
}

/// Blunders Engine primary position search function. WIP.
pub fn search(position: Position, ply: u32, tt: &mut TranspositionTable) -> SearchResult {
    assert_ne!(ply, 0);
    let mode = Mode::depth(ply, None);
    ids(position, mode, tt, Arc::new(AtomicBool::new(false)), true)
}

/// Blunders Engine non-blocking search function. This runs the search on a separate thread.
/// When the search has been completed, it returns the value by sending it over the given Sender.
/// Args:
///
/// * position: Root position to search
/// * ply: Ply to search to
/// * tt: Shared Transposition table. This may or may not lock the table for the duration of the search
/// * sender: Channel to send search result over
pub fn search_nonblocking<T: 'static + Send + From<SearchResult>>(
    position: Position,
    mode: Mode,
    tt: Arc<Mutex<TranspositionTable>>,
    stopper: Arc<AtomicBool>,
    sender: mpsc::Sender<T>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let search_result = {
            let mut locked_tt = tt.lock().unwrap();
            ids(position, mode, &mut locked_tt, stopper, true)
        };
        sender.send(search_result.into()).unwrap();
    })
}
