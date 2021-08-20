//! Quiescence Search
//!
//! When a position is being searched, nodes at the final depth (leaf nodes)
//! can be either terminal or non-terminal.
//! Terminal nodes get an absolute score. Non-terminal nodes are scored
//! according to a static evaluation function that provides a best guess at to
//! that node's value.
//!
//! Statically evaluating non-terminal leaf nodes leads to the horizon effect.
//! An engine may see a leaf node where Queen x Pawn as a winning position,
//! while right over the horizon exists Pawn x Queen.
//!
//! To reduce this horizon effect, a quiescence search is used in place of
//! a direct static evaluation of a leaf node.
//! Quiescence search searches a small sub-tree of the leaf node to evaluate
//! quiet position, so the evaluation of the original leaf node is more stable.

use crate::coretypes::Cp;
use crate::eval::evaluate;
use crate::movelist::MoveInfoList;
use crate::moveorder::pick_best_move;
use crate::Position;
use std::cmp::max;

/// notes:
/// Quiescence search returns a score relative to active player.
/// It can be given any max depth to limit its search.
/// A depth of 0 is the same as the stand pat evaluation.
/// Quiescence is guaranteed to have a short runtime because it only evaluates captures,
/// and there are a limited number of captures to be had for any position.
///
/// Quiescence is implemented as a fail-soft negamax.
///
/// example: leaf(Queen x Pawn) -> +100
///          next(Pawn x Queen) -> -800
///          actual -> -800
/// A search would normally return a static evaluation.
/// This can be an over or underestimate.
///
/// Quiescence needs pruning. Can aggressive pruning cause inaccurate scores?
///
///
/// Initial Call to Quiescence:
/// Negamax:
///     if node is leaf and non-terminal, return quiescence(position, alpha, beta)
pub fn quiescence(
    position: &mut Position,
    mut alpha: Cp,
    beta: Cp,
    ply: u8,
    nodes: &mut u64,
) -> Cp {
    *nodes += 1;
    let mut best_score = evaluate(position);

    // Depth limited search.
    if ply == 0 {
        return best_score;
    }

    // Standing Beta cutoff.
    if best_score >= beta {
        return best_score;
    }
    if best_score > alpha {
        alpha = best_score;
    }

    let cache = position.cache();
    let mut legal_captures: MoveInfoList = position
        .get_legal_moves()
        .into_iter()
        .map(|move_| position.move_info(move_))
        .filter(|move_info| move_info.is_capture())
        .collect();

    while let Some(capture) = pick_best_move(&mut legal_captures, None) {
        position.do_move_info(capture);
        let score = -quiescence(position, -beta, -alpha, ply - 1, nodes);
        position.undo_move(capture, cache);

        best_score = max(best_score, score);

        // Beta cutoff in loop.
        if best_score >= beta {
            return best_score;
        }
        if best_score > alpha {
            alpha = best_score;
        }
    }

    return best_score;
}
