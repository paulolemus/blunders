//! Engine struct acts as a simplified API for the various parts of the Blunders engine.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::error::{self, ErrorKind};
use crate::position::{Game, Position};
use crate::search::{self, SearchResult};
use crate::timeman::Mode;
use crate::TranspositionTable;

/// EngineBuilder allows for parameters of an Engine to be set and built once,
/// avoiding repeating costly initialization steps of making then changing an Engine.
///
/// Default values:
///
/// * `game`: Starting chess position
/// * `transpositions_mb`: 1 megabytes
/// * `num_threads`: 1,
/// * `debug`: true
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EngineBuilder {
    game: Game,
    transpositions_mb: usize,
    num_threads: usize,
    debug: bool,
}

impl EngineBuilder {
    /// Create a new default EngineBuilder.
    pub fn new() -> Self {
        Self {
            game: Game::start_position(),
            transpositions_mb: 1,
            num_threads: 1,
            debug: true,
        }
    }

    /// Create and return a new Engine.
    pub fn build(&self) -> Engine {
        let tt = Arc::new(TranspositionTable::with_mb(self.transpositions_mb));
        let stopper = Arc::new(AtomicBool::new(false));

        Engine {
            game: self.game.clone(),
            tt,
            stopper,
            debug: self.debug,
            search_handle: None,
        }
    }

    /// Set the Engine's initial game state.
    pub fn game(mut self, game: Game) -> Self {
        self.game = game;
        self
    }

    /// Set the engine's initial search thread pool size.
    pub fn threads(mut self, num_threads: usize) -> Self {
        self.num_threads = num_threads;
        self
    }

    /// Set the engine's initial transposition table size in megabytes.
    pub fn transpositions_mb(mut self, transpositions_mb: usize) -> Self {
        self.transpositions_mb = transpositions_mb;
        self
    }

    /// Set whether the engine begins in debug mode.
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

/// Engine wraps up all parameters required for running any kind of search.
/// It is stateful because to properly evaluate a chess position the history of
/// moves for the current game need to be tracked.
///
/// If a new game is going to be started, the engine needs to be told so.
pub struct Engine {
    // Search fields
    game: Game,
    tt: Arc<TranspositionTable>,
    stopper: Arc<AtomicBool>,
    debug: bool,

    // Meta fields
    search_handle: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            game: Game::from(Position::start_position()),
            tt: Arc::new(TranspositionTable::new()),
            stopper: Arc::new(AtomicBool::new(false)),
            debug: true,
            search_handle: None,
        }
    }

    /// Returns reference to current game of engine.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Returns reference to current debug flag of engine.
    pub fn debug(&self) -> &bool {
        &self.debug
    }

    /// Returns reference to engine's transposition table.
    pub fn transposition_table(&self) -> &TranspositionTable {
        &self.tt
    }

    /// Set the game or position for evaluation.
    pub fn set_game<T: Into<Game>>(&mut self, game: T) {
        self.game = game.into();
    }

    /// Update the engine's debug parameter.
    pub fn set_debug(&mut self, new_debug: bool) {
        self.debug = new_debug;
    }

    /// Informs engine that next search will be from a new game.
    /// Returns Ok if engine succeeded in changing state for a new game, Err otherwise.
    pub fn new_game(&mut self) -> error::Result<()> {
        self.try_clear_transpositions()
    }

    /// Attempt to set a new size for the transposition table in Megabytes.
    /// Table is set only if there is exactly one reference to the table (not used in search).
    /// Returns Ok(new capacity) on success or Err if no change was made.
    pub fn try_set_transpositions_mb(&mut self, new_mb: usize) -> error::Result<usize> {
        Arc::get_mut(&mut self.tt)
            .map(|inner_tt| inner_tt.set_mb(new_mb))
            .ok_or(ErrorKind::EngineTranspositionTableInUse.into())
    }

    /// Attempt to clear the transposition table. Table is cleared only if there
    /// are no other Arcs to the table.
    /// Returns Ok on success or Err if the table was not cleared.
    pub fn try_clear_transpositions(&mut self) -> error::Result<()> {
        Arc::get_mut(&mut self.tt)
            .map(|inner_tt| inner_tt.clear())
            .ok_or(ErrorKind::EngineTranspositionTableInUse.into())
    }

    /// Run a blocking search.
    pub fn search_sync(&mut self, mode: Mode) -> SearchResult {
        // Block until a search is ready to run.
        self.stop();
        self.wait();
        self.unstop();

        let (sender, receiver) = mpsc::channel();
        self.search(mode, sender).unwrap();
        self.wait();
        receiver.recv().unwrap()
    }

    /// Run a non-blocking search.
    /// The engine only runs one search at a time, so if it is not ready, it fails to begin.
    /// If the engine is available for searching, it ensures its stopper is unset.
    pub fn search<T>(&mut self, mode: Mode, sender: Sender<T>) -> error::Result<()>
    where
        T: From<SearchResult> + Send + 'static,
    {
        if self.search_handle.is_none() {
            self.unstop();

            let handle = search::search_nonblocking(
                self.game.clone(),
                mode,
                Arc::clone(&self.tt),
                Arc::clone(&self.stopper),
                sender,
            );
            self.search_handle = Some(handle);

            Ok(())
        } else {
            Err((ErrorKind::EngineAlreadySearching, "failed to begin search").into())
        }
    }

    pub fn ponder(&self) {
        todo!()
    }

    /// Informs the active search to stop searching as soon as possible.
    pub fn stop(&self) {
        self.stopper.store(true, Ordering::Relaxed);
    }

    /// Resets stopper flag.
    pub fn unstop(&self) {
        self.stopper.store(false, Ordering::Relaxed);
    }

    /// Engine blocks thread until search is completed.
    pub fn wait(&mut self) {
        let handle_opt = self.search_handle.take();

        if let Some(handle) = handle_opt {
            handle.join().unwrap();
        }
    }

    /// Returns true if the engine is ready to start a search.
    /// Only one search may run at a time, so if a search is in progress, engine is not ready.
    pub fn ready(&self) -> bool {
        self.search_handle.is_none()
    }

    /// Consumes and shuts down the Engine. Signals any threads to stop searching
    /// and waits for internal resources to close first.
    /// The engine will normally close up properly when dropped,
    /// however this function provides a way to do it explicitly
    /// directly from the API.
    pub fn shutdown(self) {}
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
        self.wait();
    }
}
