//! Iterative Deepening Search.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::arrayvec::display;
use crate::coretypes::MAX_DEPTH;
use crate::search;
use crate::search::History;
use crate::search::SearchResult;
use crate::timeman::Mode;
use crate::transposition::{Entry, NodeKind, TranspositionTable};
use crate::Position;

/// Run Iterative Deepening search on a root position to depth "ply" using
/// a persistent transposition table.
/// It returns the best move and score for the position in the search tree.
/// TODO: Bug fix, returns invalid result in case where stopper was set too quickly.
pub fn ids(
    position: Position,
    mode: Mode,
    history: History,
    tt: &TranspositionTable,
    stopper: Arc<AtomicBool>,
    debug: bool,
) -> SearchResult {
    let hash = tt.generate_hash(&position);
    let instant = Instant::now();
    let age = position.age();

    // Invalid default values, will be overwritten after each loop.
    let mut search_result = SearchResult {
        player: position.player,
        stopped: true,
        ..Default::default()
    };

    // Run a search for each ply from 1 to target ply.
    // After each search, ensure that the principal variation from the previous
    // iteration is in the tt.
    for ply in 1..=MAX_DEPTH {
        // Check if we need to stop before the current iteration.
        if mode.stop(position.player, ply) {
            break;
        }

        let stopper = Arc::clone(&stopper);
        let history = history.clone();
        let maybe_result = search::iterative_negamax(position, ply, mode, history, tt, stopper);

        // Update search_result from deeper iteration, and return early if it's flagged as stop.
        // Need to update nodes, q_nodes, and q_elapsed to get running total.
        if let Some(mut result) = maybe_result {
            result.add_metrics(search_result);
            search_result = result;

            if search_result.stopped {
                break;
            }
        } else {
            break;
        }

        if debug && !search_result.stopped {
            // Print UCI info for this completed search result.
            println!(
                "info depth {} score cp {} time {} nodes {} nps {} pv {}",
                search_result.depth,
                search_result.relative_score(),
                search_result.elapsed.as_millis(),
                search_result.nodes,
                search_result.nps(),
                display(&search_result.pv),
            );
        }

        // Check if this completed search result contains a checkmate, to return early.
        if search_result.score.is_mate() && !search_result.stopped {
            break;
        }

        // Each value in the PV has the same score, so a TT Entry is remade for each
        // position to ensure the PV is searched first in the next search of deeper ply.
        // PV may theoretically much longer than the ply of the current search, due to TT hits.
        // Only positions up to the current ply may be used.
        // TODO:
        // Check for possible bugs where the pv is incorrect.
        // This might be fixed by checking if a position exists in the tt already,
        // but has different values from what is recreated.
        let mut position = position.clone();
        let mut hash = hash.clone();
        let mut move_ply = ply.clone();
        let mut relative_pv_score = search_result.relative_score();

        for &pv_move in search_result.pv.iter().take(move_ply as usize) {
            let pv_entry = Entry::new(hash, pv_move, relative_pv_score, move_ply, NodeKind::Pv);
            tt.replace(pv_entry, age);

            let cache = position.cache();
            let move_info = position.do_move(pv_move);
            tt.update_hash(&mut hash, &position, move_info, cache);
            move_ply -= 1;
            relative_pv_score = -relative_pv_score;
        }
        // TODO: Handle part of PV that is longer than depth searched.
    }

    // Update values with those tracked in top level.
    search_result.elapsed = instant.elapsed();

    search_result
}
