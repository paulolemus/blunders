//! Iterative Deepening Search.

use std::cmp;

use crate::coretypes::Color::*;
use crate::coretypes::{Move, Square};
use crate::evaluation::{static_evaluate, Cp};
use crate::movelist::MoveList;
use crate::search;
use crate::Position;

/// Searches game tree to depth "ply" using iterative deepening.
/// It returns the best move and score for the position in the search tree.
pub fn ids(position: Position, ply: u32) -> (Cp, Move) {
    debug_assert_ne!(ply, 0);
    ids_root(position, ply)
}

/// TODO
fn ids_root(mut position: Position, ply: u32) -> (Cp, Move) {
    let legal_moves = position.get_legal_moves();
    debug_assert_ne!(ply, 0);
    debug_assert!(legal_moves.len() > 0);

    // Steps:
    // For each depth from 0 to ply:
    // 1. Run an alpha beta search. Save the principal variation line.
    // 2. For next iteration, use principal variation to search PV first.

    // pv_line is the principal variation of most recently completed search.
    // we know for any search to a given depth, it will yield a pv_line
    // that is of len `depth`.
    // For any node, if it becomes the best move, then it can save itself in slot "depth".
    // Recursively, the actual pv_line builds itself from end to start.
    let mut pv_line = MoveList::new();
    (Cp(0), Move::new(Square::A1, Square::A2, None))
}
