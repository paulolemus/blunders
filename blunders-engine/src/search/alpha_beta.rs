//! Minimax wth Alpha-Beta pruning implementation.

use std::cmp;

use crate::coretypes::Color::*;
use crate::coretypes::{Move, Square};
use crate::evaluation::{static_evaluate, Cp};
use crate::Position;

/// Base alpha_beta call. This function assumes that the current player in the passed position
/// is the engine.
/// It returns the best move and score for the position in the search tree.
pub fn alpha_beta(position: Position, ply: u32) -> (Cp, Move) {
    debug_assert_ne!(ply, 0);
    alpha_beta_root(position, ply)
}

/// alpha_beta_root is almost the same as alpha_beta impl, except it links a Cp score to its node.
/// It can only operate on positions that are not terminal positions.
///
/// Alpha Beta cannot prune any of its children directly because:
/// 1. Alpha and Beta are inherited as -Inf and +Inf.
/// 2. Only one of Alpha and Beta can be updated from a nodes children.
/// Thus, for the root position either Alpha or Beta will stay infinitely bounded,
/// so no pruning can occur.
fn alpha_beta_root(mut position: Position, ply: u32) -> (Cp, Move) {
    let legal_moves = position.get_legal_moves();
    debug_assert_ne!(ply, 0);
    debug_assert!(legal_moves.len() > 0);

    if position.player == White {
        let mut best_move = Move::new(Square::D2, Square::D4, None);
        let mut alpha = Cp::MIN;
        let beta = Cp::MAX;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<{ Black as u8 }>(&mut position, ply - 1, alpha, beta);
            position.undo_move(move_info);

            if move_cp > alpha {
                alpha = move_cp;
                best_move = legal_move;
            }
        }
        (alpha, best_move)
    } else {
        let mut best_move = Move::new(Square::D2, Square::D4, None);
        let alpha = Cp::MIN;
        let mut beta = Cp::MAX;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<{ White as u8 }>(&mut position, ply - 1, alpha, beta);
            position.undo_move(move_info);

            if move_cp < beta {
                beta = move_cp;
                best_move = legal_move;
            }
        }
        (beta, best_move)
    }
}

fn alpha_beta_impl<const COLOR: u8>(position: &mut Position, ply: u32, alpha: Cp, beta: Cp) -> Cp {
    let legal_moves = position.get_legal_moves();
    let num_moves = legal_moves.len();

    // Stop at terminal node: Checkmate/Stalemate/last depth.
    if ply == 0 || legal_moves.len() == 0 {
        return static_evaluate(&position, num_moves);
    }

    if COLOR == White as u8 {
        let mut best_cp = Cp::MIN;
        let mut alpha = alpha;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<{ Black as u8 }>(position, ply - 1, alpha, beta);
            position.undo_move(move_info);

            best_cp = cmp::max(best_cp, move_cp);
            alpha = cmp::max(alpha, best_cp);
            if alpha >= beta {
                // Beta cutoff
                return best_cp;
            }
        }
        best_cp
    } else {
        let mut best_cp = Cp::MAX;
        let mut beta = beta;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<{ White as u8 }>(position, ply - 1, alpha, beta);
            position.undo_move(move_info);

            best_cp = cmp::min(best_cp, move_cp);
            beta = cmp::min(beta, best_cp);
            if alpha >= beta {
                // Alpha cutoff
                return best_cp;
            }
        }
        best_cp
    }
}
