//! Iterative Deepening Search.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::arrayvec::display;
use crate::coretypes::{Cp, Move, Square::*, MAX_DEPTH};
use crate::movelist::Line;
use crate::search;
use crate::search::SearchResult;
use crate::timeman::Mode;
use crate::transposition::{NodeKind, TranspositionInfo, TranspositionTable};
use crate::Position;

/// Run Iterative Deepening search on a root position to depth "ply" using
/// a persistent transposition table.
/// It returns the best move and score for the position in the search tree.
/// TODO: Bug fix, returns invalid result in case where stopper was set too quickly.
pub fn ids(
    position: Position,
    mode: Mode,
    tt: &mut TranspositionTable,
    stopper: Arc<AtomicBool>,
) -> SearchResult {
    let hash = tt.generate_hash(&position);
    let instant = Instant::now();
    let root_player = position.player;

    let mut nodes = 0;

    // Invalid default values, will be overwritten after each loop.
    let mut search_result = SearchResult {
        player: position.player,
        depth: 0,
        best_move: Move::new(A1, H7, None),
        score: Cp(0),
        pv_line: Line::new(),
        nodes,
        elapsed: instant.elapsed(),
        stopped: true,
    };

    // Run a search for each ply from 1 to target ply.
    // After each search, ensure that the principal variation from the previous
    // iteration is in the tt.
    for ids_ply in 1..=MAX_DEPTH as u32 {
        // Check if we need to stop before the current iteration.
        if mode.stop(root_player, ids_ply) {
            break;
        }

        let stopper = Arc::clone(&stopper);
        let maybe_result = search::iterative_negamax(position, ids_ply, mode, tt, stopper);

        // Use the most recent valid search_result,
        // and return early if search_result is flagged as stopped.
        if let Some(result) = maybe_result {
            nodes += result.nodes;
            search_result = result;

            if search_result.stopped {
                break;
            } else {
                // Print UCI info for this completed search result.
                println!(
                    "info depth {} score cp {} time {} nodes {} nps {} pv {}",
                    search_result.depth,
                    search_result.relative_score(),
                    search_result.elapsed.as_millis(),
                    search_result.nodes,
                    search_result.nps(),
                    display(&search_result.pv_line),
                );
            }
        } else {
            break;
        }

        // Check if this completed search result contains a checkmate, to return early.
        if search_result.score.is_mate() && !search_result.stopped {
            break;
        }

        // Before the next iteration, attempt to fill missing moves in the PV by referencing
        // the transposition table.
        // The length of the pv_line should be the same as the depth searched to
        // if a game-ending line was not found.
        // TODO: figure out how to check this correctly.
        // assert_eq!(search_result.pv_line.len(), ids_ply as usize);

        // All nodes in the PV have the same score, because that score propagated up
        // from a terminal node. TranspositionInfo for all PV nodes can be fully recreated.
        let mut position = position.clone();
        let mut hash = hash.clone();
        let mut move_ply = ids_ply.clone();
        let mut relative_pv_score = search_result.relative_score();
        let pv_line = search_result.pv_line.clone();

        // For each move in PV, TranspositionInfo is recreated from the current position,
        // before applying the best move. Then the hash, position, ply, and score,
        // are updated for the next loop.
        // The TranspositionInfo for each pv are inserted unconditionally.
        // TODO:
        // Check for possible bugs where the pv is incorrect.
        // This might be fixed by checking if a position exists in the tt already,
        // but has different values from what is recreated.
        for pv_move in pv_line {
            let pv_info =
                TranspositionInfo::new(hash, NodeKind::Pv, pv_move, move_ply, relative_pv_score);
            tt.replace(pv_info);

            let move_info = position.do_move(pv_move);
            tt.update_hash(&mut hash, &position, &move_info);
            move_ply -= 1;
            relative_pv_score = -relative_pv_score;
        }
    }

    // Update values with those tracked in top level.
    search_result.nodes = nodes;
    search_result.elapsed = instant.elapsed();

    search_result
}
