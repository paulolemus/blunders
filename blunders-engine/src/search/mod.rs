//! Search functions.

mod alpha_beta;
mod history;
mod ids;
mod minimax;
mod negamax;
mod quiescence;

pub use alpha_beta::*;
pub use history::*;
pub use ids::*;
pub use minimax::*;
pub use negamax::*;
pub use quiescence::*;

use std::fmt::{self, Display};
use std::sync::{atomic::AtomicBool, mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crate::coretypes::{Color, Cp, Move, PlyKind};
use crate::movelist::display;
use crate::movelist::Line;
use crate::timeman::Mode;
use crate::transposition::TranspositionTable;
use crate::{Game, Position};

/// The results found from running a search on some root position.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The best move to make for a position discovered from search.
    pub best_move: Move,
    /// The centipawn score of making the best move, with absolute Cp (+White, -Black).
    pub score: Cp,
    /// The principal variation, or a sequence of the best moves that result in an evaluation of at least `score` Cp.
    pub pv: Line,
    /// The player to move for the root position that was searched.
    pub player: Color,
    /// Depth (aka ply, half move) in plies that was searched. This depth is only fully searched if `stopped` flag is false.
    pub depth: PlyKind,
    /// Total number of nodes visited in a search, including main search nodes and quiescence nodes.
    pub nodes: u64,
    /// Total number of nodes visited in a quiescence search.
    pub q_nodes: u64,
    /// Total time elapsed from the start to the end of a search.
    pub elapsed: Duration,
    /// Total time elapsed spent in quiescence search, within main search.
    pub q_elapsed: Duration,
    /// Flag that indicates this search was aborted.
    pub stopped: bool,

    /// Number of nodes where a beta-cutoff was performed.
    pub cut_nodes: u64,
    /// Number of nodes that improved local alpha value without reaching beta.
    pub pv_nodes: u64,
    /// Number of nodes that did not improve alpha or result in a cutoff.
    pub all_nodes: u64,
    /// Number of times a position was found in the transposition table.
    pub tt_hits: u64,
    /// Number of times a tt hit score could be used and returned immediately.
    pub tt_cuts: u64,
}

impl SearchResult {
    /// Add the following metrics from `other` to this Result:
    /// nodes, q_nodes, elapsed, q_elapsed, beta_cutoffs, alpha_increases, tt_hits, tt_cuts.
    pub fn add_metrics(&mut self, other: Self) {
        self.nodes += other.nodes;
        self.q_nodes += other.q_nodes;
        self.elapsed += other.elapsed;
        self.q_elapsed += other.q_elapsed;

        self.cut_nodes += other.cut_nodes;
        self.pv_nodes += other.pv_nodes;
        self.all_nodes += other.all_nodes;
        self.tt_hits += other.tt_hits;
        self.tt_cuts += other.tt_cuts;
    }

    /// Get average nodes per second of search.
    pub fn nps(&self) -> f64 {
        (self.nodes as f64 / self.elapsed.as_secs_f64()).round()
    }

    /// Get average nodes per second of search for only quiescence search.
    pub fn q_nps(&self) -> f64 {
        (self.q_nodes as f64 / self.q_elapsed.as_secs_f64()).round()
    }

    /// Returns the percentage of elapsed time of search that was in quiescence.
    ///
    /// Example: elapsed=2.0s, q_elapsed=0.5s, quiescence_ratio=0.25
    pub fn quiescence_ratio(&self) -> f64 {
        assert!(
            self.q_elapsed <= self.elapsed,
            "logical error for q_elapsed to be greater than elapsed"
        );
        self.q_elapsed.as_secs_f64() / self.elapsed.as_secs_f64()
    }

    /// Returns the percentage of tt hits that result in tt cuts.
    pub fn tt_cut_ratio(&self) -> f64 {
        self.tt_cuts as f64 / self.tt_hits as f64
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

/// Note that this default is technically illegal and does not represent any actual search.
impl Default for SearchResult {
    fn default() -> Self {
        Self {
            best_move: Move::illegal(),
            score: Cp(0),
            pv: Line::new(),
            player: Color::White,
            depth: 0,
            nodes: 0,
            q_nodes: 0,
            elapsed: Duration::ZERO,
            q_elapsed: Duration::ZERO,
            stopped: false,
            cut_nodes: 0,
            pv_nodes: 0,
            all_nodes: 0,
            tt_hits: 0,
            tt_cuts: 0,
        }
    }
}

impl Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut displayed = String::new();
        displayed.push_str("SearchResult {\n");
        displayed.push_str(&format!("    best_move: {}\n", self.best_move));
        displayed.push_str(&format!("    abs_score: {}\n", self.absolute_score()));
        displayed.push_str(&format!("    pv       : {}\n", display(&self.pv)));
        displayed.push_str(&format!("    player   : {}\n", self.player));
        displayed.push_str(&format!("    depth    : {}\n", self.depth));
        displayed.push_str(&format!("    nodes    : {}\n", self.nodes));
        displayed.push_str(&format!("    nps      : {}\n", self.nps()));
        displayed.push_str(&format!(
            "    elapsed  : {}.{}s\n",
            self.elapsed.as_secs(),
            self.elapsed.subsec_millis()
        ));
        displayed.push_str(&format!("    q_ratio  : {:.2}\n", self.quiescence_ratio()));
        displayed.push_str(&format!("    stopped  : {}\n", self.stopped));
        displayed.push_str(&format!("    pv_nodes : {}\n", self.pv_nodes));
        displayed.push_str(&format!("    cut_nodes: {}\n", self.cut_nodes));
        displayed.push_str(&format!("    all_nodes: {}\n", self.all_nodes));
        displayed.push_str(&format!("    tt_cuts  : {}\n", self.tt_cuts));
        displayed.push_str(&format!("    tt_hits  : {}\n", self.tt_hits));
        displayed.push_str(&format!("    tt_ratio : {:.2}\n", self.tt_cut_ratio()));
        displayed.push_str("}\n");

        write!(f, "{displayed}")
    }
}

/// Blunders Engine primary position search function. WIP.
pub fn search(
    position: Position,
    mode: Mode,
    tt: &TranspositionTable,
    start_time: Option<Instant>,
) -> SearchResult {
    let start_time = start_time.unwrap_or_else(Instant::now);
    let history = History::new(&position.into(), tt.zobrist_table());
    ids(
        position,
        mode,
        history,
        tt,
        start_time,
        Arc::new(AtomicBool::new(false)),
        true,
    )
}

/// Blunders Engine non-blocking search function. This runs the search on a separate thread.
/// When the search has been completed, it returns the value by sending it over the given Sender.
///
/// # Arguments
///
/// * `game`: State of the current active game
/// * `mode`: Mode of search determines when the search stops and how deep it searches
/// * `tt`: Shared Transposition table. This may or may not lock the table for the duration of the search
/// * `stopper`: Tell search to stop early from an external source
/// * `debug`: When true prints extra debugging information
/// * `sender`: Channel to send search result over
pub fn search_nonblocking<P, T>(
    game: P,
    mode: Mode,
    tt: Arc<TranspositionTable>,
    start_time: Option<Instant>,
    stopper: Arc<AtomicBool>,
    debug: bool,
    sender: mpsc::Sender<T>,
) -> thread::JoinHandle<()>
where
    T: 'static + Send + From<SearchResult>,
    P: Into<Game>,
{
    let start_time = start_time.unwrap_or_else(Instant::now);
    let game: Game = game.into();
    let position = game.position;
    let history = History::new(&game, tt.zobrist_table());

    thread::spawn(move || {
        let search_result = ids(position, mode, history, &tt, start_time, stopper, debug);
        sender.send(search_result.into()).unwrap();
    })
}
