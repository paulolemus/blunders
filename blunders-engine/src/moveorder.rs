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

use arrayvec::ArrayVec;

use crate::coretypes::{Cp, Move, MoveInfo, MAX_MOVES};
use crate::movelist::MoveInfoList;

// General considerations for move ordering and searching:
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
    is_tt_move: bool,      // Move listed as best move for root position in tt.
    promotion: Option<Cp>, // Cp value of promoting piece, or none.
    mvv_lva: (bool, Cp),   // is capture, followed by mvv-lva.
                           // All other nodes remain with lowest but equal priority.
}

/// OrderStrategy defaults to all false.
impl Default for OrderStrategy {
    fn default() -> Self {
        OrderStrategy {
            is_tt_move: false,
            promotion: None,
            mvv_lva: (false, Cp(0)),
        }
    }
}

impl From<(MoveInfo, Option<Move>)> for OrderStrategy {
    fn from((move_info, key_move): (MoveInfo, Option<Move>)) -> Self {
        // Give high priority to move if root position listed it in tt.
        let is_tt_move = key_move == Some(move_info.move_());

        // Set promotion CP.
        let promotion = move_info.promotion.map(|pk| pk.centipawns());

        // Sort by most-valuable-victim -> least-valuable-aggressor.
        // A decent heuristic that prioritizes capturing enemy most valuable pieces first.
        // Also prioritizes positive capture above all.
        let mvv_lva = if let Some(victim) = move_info.captured() {
            let attacker = move_info.piece_kind.centipawns();
            let victim = victim.centipawns();
            (true, victim - attacker)
        } else {
            (false, Cp(0))
        };

        Self {
            is_tt_move,
            promotion,
            mvv_lva,
        }
    }
}

/// Order all moves in a container completely, in order of worst move to best move.
/// Best moves are near the end to allow for iterating from best to worst move by using
/// `while let Some(move_) = move_list.pop() ...` or `for move_ in move_list.into_iter().rev() ...`
///
/// # Arguments
///
/// * `legal_moves`: List of MoveInfos for all legal moves of current position.
/// * `maybe_key_move`: Transposition Table move for current position.
pub fn order_all_moves(legal_moves: MoveInfoList, maybe_key_move: Option<Move>) -> MoveInfoList {
    let mut ordering_vec: ArrayVec<(MoveInfo, OrderStrategy), MAX_MOVES> = legal_moves
        .into_iter()
        .map(|move_info| (move_info, OrderStrategy::from((move_info, maybe_key_move))))
        .collect();

    // Sort all moves using their OrderStrategy as a key.
    ordering_vec.sort_unstable_by_key(|pair| pair.1);

    // Convert ordering_vec back into a MoveInfoList with sorted moves.
    ordering_vec.into_iter().map(|pair| pair.0).collect()
}

/// Pick and return the best move from a move list without allocation.
/// When run to completion, this acts as a selection sort.
pub fn pick_best_move(legal_moves: &mut MoveInfoList, key_move: Option<Move>) -> Option<MoveInfo> {
    legal_moves
        .iter()
        .enumerate()
        .max_by(|left, right| {
            let left = OrderStrategy::from((*left.1, key_move));
            let right = OrderStrategy::from((*right.1, key_move));

            left.cmp(&right)
        })
        .map(|(index, _)| index)
        .map(|index| legal_moves.swap_remove(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::{Move, PieceKind, Square::*};
    use crate::fen::Fen;
    use crate::transposition::NodeKind;
    use crate::Position;

    #[test]
    fn order_all_moves_one_capture() {
        let pos = Position::parse_fen("rnb1k1nr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RN2KBNR b - - 3 11")
            .unwrap();
        let capture = Move::new(E5, D4, None);
        let num_moves = 24; // Checked manually.
        let legal_moves = pos
            .get_legal_moves()
            .into_iter()
            .map(|move_| pos.move_info(move_))
            .collect();
        let mut ordered_legal_moves = order_all_moves(legal_moves, None);

        assert_eq!(ordered_legal_moves.len(), num_moves);
        assert_eq!(ordered_legal_moves.pop().unwrap().move_(), capture);
    }

    #[test]
    fn node_kind_ordering() {
        assert!(NodeKind::Pv > NodeKind::Cut);
        assert!(NodeKind::Cut > NodeKind::All);
    }

    #[test]
    fn order_strategy_cmp() {
        let os = OrderStrategy::default();
        let mut gt_os = OrderStrategy::default();
        gt_os.is_tt_move = true;

        let mut lt_os = OrderStrategy::default();
        lt_os.promotion = Some(PieceKind::Queen.centipawns());

        assert!(gt_os > os);
        assert!(gt_os > lt_os);
    }
}
