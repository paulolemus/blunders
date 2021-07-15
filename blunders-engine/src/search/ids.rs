//! Iterative Deepening Search.

use std::time::Instant;

use crate::coretypes::{Move, Square::*};
use crate::evaluation::Cp;
use crate::movelist::Line;
use crate::search;
use crate::search::SearchResult;
use crate::transposition::TranspositionTable;
use crate::Position;

/// Searches game tree to depth "ply" using iterative deepening.
/// It returns the best move and score for the position in the search tree.
pub fn ids(position: Position, ply: u32) -> SearchResult {
    let mut tt = TranspositionTable::new();
    ids_with_tt(position, ply, &mut tt)
}

pub fn ids_with_tt(position: Position, ply: u32, tt: &mut TranspositionTable) -> SearchResult {
    assert_ne!(ply, 0);

    let active_player = position.player;
    let hash = tt.generate_hash(&position);
    let instant = Instant::now();

    let mut pv_line = Line::new();
    let mut nodes = 0;

    // Invalid default values, will be overwritten after each loop.
    let mut search_result = SearchResult {
        best_move: Move::new(A1, H7, None),
        score: Cp(0),
        pv_line,
        nodes,
        elapsed: instant.elapsed(),
    };

    // Run a search for each ply from 1 to target ply.
    // After each search, ensure that the principal variation from the previous
    // iteration is in the tt.
    for ids_ply in 1..=ply {
        search_result = search::negamax_with_tt(position, ids_ply, tt);
        nodes += search_result.nodes;

        // Ideally, the length of the PV is the ply of the search.
        assert_eq!(search_result.pv_line.len(), ids_ply as usize);
    }

    // Update values with those tracked in top level.
    search_result.score = search_result.score * active_player.sign();
    search_result.nodes = nodes;
    search_result.elapsed = instant.elapsed();

    search_result
}
