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
use crate::Position;

/// notes:
/// Quiescence search returns a score relative to active player.
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
pub fn quiescence(position: &Position, _alpha: Cp, _beta: Cp) -> Cp {
    // TODO!
    return evaluate(position);
}
