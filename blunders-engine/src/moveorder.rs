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

use std::cmp::Reverse;

use crate::arrayvec::ArrayVec;
use crate::coretypes::Move;
use crate::coretypes::MAX_MOVES;
use crate::eval::Cp;
use crate::movelist::MoveList;
use crate::transposition::TranspositionTable;
use crate::zobrist::HashKind;
use crate::Position;

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
    mvv_lva: (Cp, Reverse<Cp>), // Cp of most valuable victim -> least valuable aggressor.
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
            is_tt_move: false,
            promotion: None,
            mvv_lva: (Cp(0), Reverse(Cp(0))),
        }
    }
}

/// Order all moves in a container completely, with best moves at front.
pub(crate) fn order_all_moves(
    position: &Position,
    legal_moves: MoveList,
    hash: HashKind,
    tt: &TranspositionTable,
) -> MoveList {
    let mut ordering_vec = ArrayVec::<(Move, Reverse<OrderStrategy>), MAX_MOVES>::new();
    let maybe_key_move = tt.get(hash).and_then(|tt_info| Some(tt_info.key_move));

    // For each move, gather data needed to order, and push into a new ArrayVec.
    for legal_move in legal_moves {
        let mut order_strategy = OrderStrategy::new();

        // Give high priority to move if root position listed it in tt.
        if let Some(key_move) = maybe_key_move {
            order_strategy.is_tt_move = legal_move == key_move;
        }

        // Set promotion flag.
        order_strategy.promotion = legal_move.promotion.map(|pk| pk.centipawns());

        // Check if there were any captures, by looking at "to" square.
        // Note this ignores en-passant captures.
        // Two easy methods for sorting:
        // sort by most valuable victim followed by its least valuable aggressor.
        // sort by the greatest difference in score, so 1 ply winning trades first.
        if let Some(victim) = position.pieces.on_square(legal_move.to) {
            let attacker = position.pieces.on_square(legal_move.from).unwrap();
            order_strategy.mvv_lva = (
                victim.piece_kind.centipawns(),
                Reverse(attacker.piece_kind.centipawns()),
            );
        }

        ordering_vec.push((legal_move, Reverse(order_strategy)));
    }

    // Sort all moves using their OrderStrategy as a key.
    // Since OrderStrategy has been reversed, the most valuable moves will be
    // closer to the head of the list.
    ordering_vec.sort_unstable_by(|left, right| left.1.cmp(&right.1));

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
    use crate::coretypes::{PieceKind, Square::*};
    use crate::fen::Fen;
    use crate::transposition::NodeKind;

    #[test]
    fn order_all_moves_one_capture() {
        let pos = Position::parse_fen("rnb1k1nr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RN2KBNR b - - 3 11")
            .unwrap();
        let capture = Move::new(E5, D4, None);
        let num_moves = 24; // Checked manually.
        let tt = TranspositionTable::new();
        let hash = tt.generate_hash(&pos);
        let ordered_legal_moves = order_all_moves(&pos, pos.get_legal_moves(), hash, &tt);

        assert_eq!(ordered_legal_moves.len(), num_moves);
        assert_eq!(*ordered_legal_moves.get(0).unwrap(), capture);
    }

    #[test]
    fn node_kind_ordering() {
        assert!(NodeKind::Pv > NodeKind::Cut);
        assert!(NodeKind::Cut > NodeKind::Other);
    }

    #[test]
    fn order_strategy_cmp() {
        let os = OrderStrategy::new();
        let mut gt_os = OrderStrategy::new();
        gt_os.is_tt_move = true;

        let mut lt_os = OrderStrategy::new();
        lt_os.promotion = Some(PieceKind::Queen.centipawns());

        assert!(gt_os > os);
        assert!(gt_os > lt_os);
    }
}
