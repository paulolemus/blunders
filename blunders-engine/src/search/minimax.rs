//! Minimax implementation.

use std::cmp;
use std::time::Instant;

use crate::coretypes::Color::*;
use crate::coretypes::{Move, Square};
use crate::eval::{evaluate_abs, terminal_abs, Cp};
use crate::movelist::Line;
use crate::search::SearchResult;
use crate::Position;

const WHITE: u8 = White as u8;
const BLACK: u8 = Black as u8;

/// Base minimax call. This function assumes that the current player in the passed position
/// is the engine.
/// It returns the best move and score for the position in the search tree.
pub fn minimax(position: Position, ply: u32) -> SearchResult {
    assert_ne!(ply, 0);

    let instant = Instant::now();
    let mut pv_line = Line::new();
    let mut nodes = 0;

    let (score, best_move) = minimax_root(position, ply, &mut nodes);

    pv_line.push(best_move);

    SearchResult {
        best_move,
        score,
        pv_line,
        nodes,
        elapsed: instant.elapsed(),
    }
}

/// Minimax root is almost the same as minimax impl, except it links a Cp score to its node.
/// It can only operate on positions that are not terminal positions.
///
/// Minimax cannot prune any of its children directly because:
/// 1. Alpha and Beta are inherited as -Inf and +Inf.
/// 2. Only one of Alpha and Beta can be updated from a nodes children.
/// Thus, for the root position either Alpha or Beta will stay infinitely bounded,
/// so no pruning can occur.
fn minimax_root(mut position: Position, ply: u32, nodes: &mut u64) -> (Cp, Move) {
    *nodes += 1;
    let legal_moves = position.get_legal_moves();
    assert_ne!(ply, 0);
    assert!(legal_moves.len() > 0);

    let mut best_move = Move::new(Square::D2, Square::D4, None);
    let mut best_cp;

    if position.player == White {
        best_cp = Cp::MIN;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = minimax_impl::<BLACK>(&mut position, ply - 1, nodes);
            position.undo_move(move_info);

            if move_cp > best_cp {
                best_cp = move_cp;
                best_move = legal_move;
            }
        }
    } else {
        best_cp = Cp::MAX;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = minimax_impl::<WHITE>(&mut position, ply - 1, nodes);
            position.undo_move(move_info);

            if move_cp < best_cp {
                best_cp = move_cp;
                best_move = legal_move;
            }
        }
    }

    (best_cp, best_move)
}

fn minimax_impl<const COLOR: u8>(position: &mut Position, ply: u32, nodes: &mut u64) -> Cp {
    *nodes += 1;
    let legal_moves = position.get_legal_moves();
    let num_moves = legal_moves.len();

    // Stop at terminal node: Checkmate/Stalemate/last depth.
    if num_moves == 0 {
        return terminal_abs(position);
    } else if ply == 0 {
        return evaluate_abs(position);
    }

    let mut best_cp;

    if COLOR == White as u8 {
        best_cp = Cp::MIN;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = minimax_impl::<BLACK>(position, ply - 1, nodes);
            position.undo_move(move_info);
            best_cp = cmp::max(best_cp, move_cp);
        }
    } else {
        best_cp = Cp::MAX;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = minimax_impl::<WHITE>(position, ply - 1, nodes);
            position.undo_move(move_info);
            best_cp = cmp::min(best_cp, move_cp);
        }
    }

    best_cp
}
