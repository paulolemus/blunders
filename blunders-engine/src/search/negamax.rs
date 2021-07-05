//! Negamax implementation of Minimax with Alpha-Beta pruning.

use std::cmp;
use std::time::Instant;

use crate::coretypes::Color;
use crate::evaluation::{static_evaluate, Cp};
use crate::movelist::Line;
use crate::search::SearchResult;
use crate::Position;

impl Color {
    const fn sign(&self) -> Cp {
        match self {
            Color::White => Cp(1),
            Color::Black => Cp(-1),
        }
    }
}

/// Negamax implementation of Minimax with alpha-beta pruning.
/// Negamax searches to a given depth and returns the best move found.
/// Internally, Negamax treats the active player as the maxing player,
/// however the final centipawn score of the position returned is
/// absolute with White as maxing and Black as minning.
pub fn negamax(mut position: Position, ply: u32) -> SearchResult {
    debug_assert_ne!(ply, 0);

    let active_player = *position.player();
    let instant = Instant::now();
    let mut pv_line = Line::new();
    let mut nodes = 0;

    let best_cp = negamax_impl(
        &mut position,
        &mut pv_line,
        &mut nodes,
        ply,
        Cp::MIN,
        Cp::MAX,
    );

    SearchResult {
        best_move: *pv_line.get(0).unwrap(),
        cp: best_cp * active_player.sign(),
        pv_line,
        nodes,
        elapsed: instant.elapsed(),
    }
}

/// The player whose turn it is to move for a position is always treated as the maxing player.
/// negamax_impl returns the max possible score of the current maxing player.
/// Therefore, when interpreting the score of a child node, the score needs to be negated.
///
/// negamax_impl stores the principal variation of the current move into the pv_line parameter.
///
/// Parameters:
///
/// position: current position to search.
/// pv: Line of moves in principal variation.
/// nodes: Counter for number of nodes visited in search.
/// ply: remaining depth to search to.
/// alpha: Best (greatest) guaranteed value for current player.
/// beta: Best (lowest) guaranteed value for opposite player.
fn negamax_impl(
    position: &mut Position,
    pv: &mut Line,
    nodes: &mut u64,
    ply: u32,
    mut alpha: Cp,
    beta: Cp,
) -> Cp {
    *nodes += 1;
    let legal_moves = position.get_legal_moves();
    let num_moves = legal_moves.len();

    // Stop search at terminal nodes, Checkmates/Stalemates/last depth.
    // Return evaluation with respect to current player.
    // `static_evaluate` treats white as maxing player and black and minning player,
    // so value is converted to treat active player as maxing player.
    if ply == 0 || num_moves == 0 {
        // The parent of this node receives an empty pv_line,
        // because a terminal node has no best move.
        pv.clear();
        return static_evaluate(&position, num_moves) * position.player.sign();
    }

    let mut local_pv = Line::new();
    let mut best_cp = Cp::MIN;

    // For each child of current position, recursively find maxing move.
    for legal_move in legal_moves {
        // Get value of a move relative to active player.
        let move_info = position.do_move(legal_move);
        let move_cp = -negamax_impl(position, &mut local_pv, nodes, ply - 1, -beta, -alpha);
        best_cp = cmp::max(best_cp, move_cp);
        position.undo_move(move_info);

        // Cut-off has occurred, no further children of this position need to be searched.
        // This branch will not be taken further up the tree as there is a better move.
        if move_cp >= beta {
            return move_cp;
        }

        // A new local PV line has been found. Update alpha and store new Line.
        if best_cp > alpha {
            alpha = best_cp;
            pv.clear();
            pv.push(legal_move);
            pv.append(local_pv);
        }
    }
    best_cp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::{Move, Square::*};
    use crate::fen::Fen;

    #[test]
    fn mate_pv() {
        let position =
            Position::parse_fen("r4rk1/1b3ppp/pp2p3/2p5/P1B1NR1Q/3P3P/2q3P1/7K w - - 0 24")
                .unwrap();

        let result = negamax(position, 6);
        assert_eq!(result.cp.leading(), Some(Color::White));
        assert_eq!(result.best_move, Move::new(E4, F6, None));
        println!("{:?}", result.pv_line);
    }

    #[test]
    fn color_sign() {
        let cp = Cp(40);
        let w_signed = cp * Color::White.sign();
        let b_signed = cp * Color::Black.sign();
        assert_eq!(w_signed, Cp(40));
        assert_eq!(b_signed, Cp(-40));
    }
}
