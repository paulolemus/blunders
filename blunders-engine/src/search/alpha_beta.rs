//! Minimax wth Alpha-Beta pruning implementation.

use std::cmp;
use std::time::Instant;

use crate::coretypes::Color::*;
use crate::coretypes::{Move, Square};
use crate::eval::{evaluate_abs, terminal_abs, Cp};
use crate::movelist::Line;
use crate::search::SearchResult;
use crate::Position;

/// Base alpha_beta call. This function assumes that the current player in the passed position
/// is the engine.
/// It returns the best move and score for the position in the search tree.
pub fn alpha_beta(position: Position, ply: u32) -> SearchResult {
    debug_assert_ne!(ply, 0);

    let instant = Instant::now();
    let mut nodes = 0;
    let (score, best_move) = alpha_beta_root(position, ply, &mut nodes, Cp::MIN, Cp::MAX);
    let mut pv_line = Line::new();
    pv_line.push(best_move);

    SearchResult {
        best_move,
        score,
        pv_line,
        nodes,
        elapsed: instant.elapsed(),
    }
}

const WHITE: u8 = White as u8;
const BLACK: u8 = Black as u8;

/// Properties of Alpha-Beta pruning.
/// * The maxing player can only update alpha from its children.
/// * The minning player can only update beta from its children.
/// * Alpha and Beta can only be inherited from their ancestors, and are otherwise Alpha=-Inf, Beta=Inf.
/// * Alpha is usually less than Beta. When they are equal or cross, a cut off occurs.

/// alpha_beta_root is almost the same as alpha_beta impl, except it links a Cp score to its node.
/// It can only operate on positions that are not terminal positions.
///
/// Alpha Beta cannot prune any of its depth 1 children directly because:
/// 1. Alpha and Beta are inherited as -Inf and +Inf.
/// 2. Only one of Alpha and Beta can be updated from a nodes children.
/// Thus, for the root position either Alpha or Beta will stay infinitely bounded,
/// so no pruning can occur.
pub(crate) fn alpha_beta_root(
    mut position: Position,
    ply: u32,
    nodes: &mut u64,
    mut alpha: Cp,
    mut beta: Cp,
) -> (Cp, Move) {
    *nodes += 1;
    let legal_moves = position.get_legal_moves();
    debug_assert_ne!(ply, 0);
    debug_assert!(legal_moves.len() > 0);

    let mut best_move = Move::new(Square::D2, Square::D4, None);

    if position.player == White {
        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<BLACK>(&mut position, ply - 1, nodes, alpha, beta);
            position.undo_move(move_info);

            if move_cp > alpha {
                alpha = move_cp;
                best_move = legal_move;
            }
        }
        (alpha, best_move)
    } else {
        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<WHITE>(&mut position, ply - 1, nodes, alpha, beta);
            position.undo_move(move_info);

            if move_cp < beta {
                beta = move_cp;
                best_move = legal_move;
            }
        }
        (beta, best_move)
    }
}

fn alpha_beta_impl<const COLOR: u8>(
    position: &mut Position,
    ply: u32,
    nodes: &mut u64,
    alpha: Cp,
    beta: Cp,
) -> Cp {
    *nodes += 1;
    let legal_moves = position.get_legal_moves();
    let num_moves = legal_moves.len();

    // Stop at terminal node: Checkmate/Stalemate/last depth.
    if num_moves == 0 {
        return terminal_abs(position);
    } else if ply == 0 {
        return evaluate_abs(position);
    }

    if COLOR == White as u8 {
        let mut best_cp = Cp::MIN;
        let mut alpha = alpha;

        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            let move_cp = alpha_beta_impl::<BLACK>(position, ply - 1, nodes, alpha, beta);
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
            let move_cp = alpha_beta_impl::<WHITE>(position, ply - 1, nodes, alpha, beta);
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
