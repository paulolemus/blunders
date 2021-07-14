//! Move Ordering
//!
//! Functions used for ordering a list of moves from best to worst,
//! or for picking the best move out of a list of moves.
//!
//! Move ordering is important for alpha-beta pruning performance.
//! If the best or good moves are searched early on in an alpha-beta search,
//! pruning occurs more frequently.
//!
//! Two strategies are used for when to move order.
//! 1. Sort an entire list of moves before processing.
//! 2. Pick and remove the best move from the move list each time a move is needed.
//!
//! There are several strategies for move ordering which may be used.
//! 1. Sort first by principal variation moves, then by hash moves, then by Captures (SEE)

use crate::arrayvec::ArrayVec;
use crate::coretypes::MAX_MOVES;
use crate::coretypes::{Move, MoveKind};
use crate::movelist::MoveList;
use crate::transposition::{NodeKind, TranspositionTable};
use crate::zobrist::HashKind;
use crate::Position;

// Questions for Search:
// What if we enter any node into tt? Only Cut/PV nodes?
// What if tt is never cleared?
// What if we hit a Key collision?
// How do we prevent the PV from being cut when using a tt value?
// Do we store score for a node with relative or absolute scoring?

// General considerations for move ordering and searching:
// Node type is useful during move ordering.
// For tt look ups during a search, a node only needs to search itself, not it's children.
// a/b can only be inherited, so getting a tt value from a child node within
// a call is the same as getting it from a recursive call.

// When looking up a value, we only return right away if it meets some conditions.
// 1. The tt hit depth >= current search depth. Otherwise value is not valid.
// 2. If depth if great enough, then we only return immediately if

/// Simple move ordering strategy. The following information is extracted from a move,
/// and used for sorting. The values go from most-to-least important based on
/// top-to-bottom declaration of fields.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct OrderStrategy {
    is_pv: bool,    // pv moves are most important, and are searched first.
    is_cut: bool,   // Node type is cut, or likely lead to pruning.
    is_in_tt: bool, // any move that is in the tt should follow pv nodes.
    is_capture: bool, // capture moves follow all tt moves.
                    // All other nodes remain with lowest but equal priority.
}
impl OrderStrategy {
    /// Returns new OrderStrategy with all values set to false.
    pub(crate) fn new() -> Self {
        Default::default()
    }
}

/// OrderStrategy defaults to all false.
impl Default for OrderStrategy {
    fn default() -> Self {
        OrderStrategy {
            is_pv: false,
            is_cut: false,
            is_in_tt: false,
            is_capture: false,
        }
    }
}

/// Order all moves in a container completely, with best moves at front.
pub(crate) fn order_all_moves(
    mut position: Position,
    legal_moves: MoveList,
    hash: HashKind,
    tt: &TranspositionTable,
) -> MoveList {
    let mut ordering_vec = ArrayVec::<(Move, OrderStrategy), MAX_MOVES>::new();

    // For each move, gather data needed to order, and push into a new ArrayVec.
    for legal_move in legal_moves {
        let move_info = position.do_move(legal_move);
        let move_hash = tt.update_from_hash(hash, &position, &move_info);
        position.undo_move(move_info);

        let mut order_strategy = OrderStrategy::new();

        if let MoveKind::Capture(_) = move_info.move_kind() {
            order_strategy.is_capture = true;
        }

        if let Some(tt_info) = tt.get(move_hash) {
            order_strategy.is_in_tt = true;

            match tt_info.node_kind {
                NodeKind::Pv => order_strategy.is_pv = true,
                NodeKind::Cut => order_strategy.is_cut = true,
                _ => {}
            };
        }

        ordering_vec.push((legal_move, order_strategy));
    }

    // Sort all moves by their OrderStrategy with cmp in reverse,
    // so greater comparisons come first.
    ordering_vec.sort_unstable_by(|left, right| right.1.cmp(&left.1));

    // Extract Moves from ordering_vec.
    let mut ordered_move_list = MoveList::new();
    ordering_vec
        .into_iter()
        .for_each(|pair| ordered_move_list.push(pair.0));

    ordered_move_list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::Square::*;
    use crate::fen::Fen;

    #[test]
    fn order_all_moves_one_capture() {
        let pos = Position::parse_fen("rnb1k1nr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RN2KBNR b - - 3 11")
            .unwrap();
        let capture = Move::new(E5, D4, None);
        let num_moves = 24; // Checked manually.
        let tt = TranspositionTable::new();
        let hash = tt.generate_hash(&pos);
        let ordered_legal_moves = order_all_moves(pos, pos.get_legal_moves(), hash, &tt);

        assert_eq!(ordered_legal_moves.len(), num_moves);
        assert_eq!(*ordered_legal_moves.get(0).unwrap(), capture);
    }
}
