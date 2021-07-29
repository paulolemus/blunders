//! Iterative Deepening Search.

use std::time::Instant;

use crate::coretypes::{Move, Square::*};
use crate::eval::Cp;
use crate::movelist::Line;
use crate::search;
use crate::search::SearchResult;
use crate::transposition::{NodeKind, TranspositionInfo, TranspositionTable};
use crate::Position;

/// Run Iterative Deepening search on a root position to depth "ply".
/// It returns the best move and score for the position in the search tree.
pub fn ids(position: Position, ply: u32) -> SearchResult {
    let mut tt = TranspositionTable::new();
    ids_with_tt(position, ply, &mut tt)
}

/// Run Iterative Deepening search to depth "ply" using a persistent transposition table.
pub fn ids_with_tt(position: Position, ply: u32, tt: &mut TranspositionTable) -> SearchResult {
    assert_ne!(ply, 0);

    let hash = tt.generate_hash(&position);
    let instant = Instant::now();

    let mut nodes = 0;

    // Invalid default values, will be overwritten after each loop.
    let mut search_result = SearchResult {
        best_move: Move::new(A1, H7, None),
        score: Cp(0),
        pv_line: Line::new(),
        nodes,
        elapsed: instant.elapsed(),
    };

    // Run a search for each ply from 1 to target ply.
    // After each search, ensure that the principal variation from the previous
    // iteration is in the tt.
    for ids_ply in 1..=ply {
        search_result = search::iterative_negamax(position, ids_ply, tt);
        nodes += search_result.nodes;

        // The length of the pv_line should be the same as the depth searched to
        // if a game-ending line was not found.
        // TODO: figure out how to check this correctly.
        // assert_eq!(search_result.pv_line.len(), ids_ply as usize);

        // All nodes in the PV have the same score, because that score propagated up
        // from a terminal node. TranspositionInfo for all PV nodes can be fully recreated.
        let mut position = position.clone();
        let mut hash = hash.clone();
        let mut move_ply = ids_ply.clone();
        let mut relative_pv_score = search_result.score * position.player.sign();
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
            assert!(ply > 0); // Ply cannot ever be used as zero.
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
