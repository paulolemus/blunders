//! Negamax implementation of Minimax with Alpha-Beta pruning.

use std::time::Instant;

use crate::coretypes::{Move, Square::*};
use crate::eval::{terminal, Cp};
use crate::movelist::Line;
use crate::moveorder::order_all_moves;
use crate::search::{quiescence, SearchResult};
use crate::transposition::{NodeKind, TranspositionInfo, TranspositionTable};
use crate::zobrist::HashKind;
use crate::Position;

/// Negamax implementation of Minimax with alpha-beta pruning.
/// Negamax searches to a given depth and returns the best move found.
/// Internally, Negamax treats the active player as the maxing player,
/// however the final centipawn score of the position returned is
/// absolute with White as maxing and Black as minning.
pub fn negamax(position: Position, ply: u32) -> SearchResult {
    let mut tt = TranspositionTable::new();
    negamax_with_tt(position, ply, &mut tt)
}

/// Negamax implementation that uses provided transposition table.
pub fn negamax_with_tt(
    mut position: Position,
    ply: u32,
    tt: &mut TranspositionTable,
) -> SearchResult {
    assert_ne!(ply, 0);

    let active_player = *position.player();
    let hash = tt.generate_hash(&position);
    let instant = Instant::now();

    let mut pv_line = Line::new();
    let mut nodes = 0;

    let best_score = negamax_impl(
        &mut position,
        tt,
        hash,
        &mut pv_line,
        &mut nodes,
        ply,
        Cp::MIN,
        Cp::MAX,
    );

    SearchResult {
        best_move: *pv_line.get(0).unwrap(),
        score: best_score * active_player.sign(),
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
/// tt: Transposition Table used for recalling search history.
/// hash: Incrementally updatable hash of provided position.
/// pv_line: Line of moves in principal variation.
/// nodes: Counter for number of nodes visited in search.
/// ply: remaining depth to search to.
/// alpha: Best (greatest) guaranteed value for current player.
/// beta: Best (lowest) guaranteed value for opposite player.
fn negamax_impl(
    position: &mut Position,
    tt: &mut TranspositionTable,
    hash: HashKind,
    pv_line: &mut Line,
    nodes: &mut u64,
    ply: u32,
    mut alpha: Cp,
    beta: Cp,
) -> Cp {
    *nodes += 1;
    let legal_moves = position.get_legal_moves();
    let num_moves = legal_moves.len();

    // Search can return when any of the following are encountered:
    // * Checkmate / Stalemate (terminal node)
    // * Tt move evaluated at equal or greater depth than searching depth
    // * depth 0 reached (leaf node)
    //
    // An eval is returned with respect to the current player.
    // (+Cp good, -Cp bad)
    // Terminal and leaf nodes have no following moves so pv_line of parent is cleared.
    if num_moves == 0 {
        pv_line.clear();
        return terminal(&position);
    }
    // Check if current move exists in tt. If so, we might be able to return that value
    // right away if has a greater or equal depth than we are considering.
    // Check that the tt key_move is a legal move, as extra (but not complete)
    // protection against Key collisions.
    // TODO: Verify that this is bug free. It is possible this may cut the Pv line,
    //       or that returning early is incorrect.
    else if let Some(tt_info) = tt.get(hash) {
        if tt_info.ply >= ply && legal_moves.contains(&tt_info.key_move) {
            pv_line.clear();
            pv_line.push(tt_info.key_move);
            let relative_score = tt_info.score * position.player.sign();
            return relative_score;
        }

    // Run a Quiescence Search for non-terminal leaf nodes to find a more stable
    // evaluation than a static evaluation.
    // The parent of this node receives an empty pv_line,
    // because this leaf node has no best move, and is not in history.
    } else if ply == 0 {
        pv_line.clear();
        return quiescence(position, alpha, beta);
    }

    // Move Ordering
    // Sort legal moves with estimated best move first.
    let ordered_legal_moves = order_all_moves(*position, legal_moves, hash, tt);
    debug_assert_eq!(num_moves, ordered_legal_moves.len());

    // Placeholder best_move, is guaranteed to be overwritten as there is at
    // lest one legal move, and the score of that move is better than worst
    // possible score.
    let mut best_move = Move::new(A1, H7, None);
    let mut local_pv = Line::new();
    let mut best_score = Cp::MIN;

    // For each child of current position, recursively find maxing move.
    for legal_move in ordered_legal_moves {
        // Get value of a move relative to active player.
        let move_info = position.do_move(legal_move);
        let move_hash = tt.update_from_hash(hash, &position, &move_info);
        let move_score = -negamax_impl(
            position,
            tt,
            move_hash,
            &mut local_pv,
            nodes,
            ply - 1,
            -beta,
            -alpha,
        );
        position.undo_move(move_info);

        // Update best_* trackers if this move is best of all seen so far.
        if move_score > best_score {
            best_score = move_score;
            best_move = legal_move;
        }

        // Cut-off has occurred, no further children of this position need to be searched.
        // This branch will not be taken further up the tree as there is a better move.
        // Push this cut-node into the tt, with an absolute score, instead of relative.
        if move_score >= beta {
            let abs_move_score = move_score * position.player.sign();
            let tt_info =
                TranspositionInfo::new(hash, NodeKind::Cut, legal_move, ply, abs_move_score);
            tt.replace(tt_info);

            return move_score;
        }

        // A new local PV line has been found. Update alpha and store new Line.
        // Update this node in tt as a PV node.
        if best_score > alpha {
            alpha = best_score;
            pv_line.clear();
            pv_line.push(legal_move);
            pv_line.append(local_pv);

            let abs_move_score = best_score * position.player.sign();
            let tt_info =
                TranspositionInfo::new(hash, NodeKind::Pv, legal_move, ply, abs_move_score);
            tt.replace(tt_info);
        }
    }

    // Every move for this node has been evaluated. It is possible that this node
    // was added to the tt beforehand, so we can add it on the condition that
    // It's node-kind is less important than what exists in tt.
    let abs_move_score = best_score * position.player.sign();
    let tt_info = TranspositionInfo::new(hash, NodeKind::Other, best_move, ply, abs_move_score);
    tt.replace_by(tt_info, |replacing, slotted| {
        replacing.node_kind >= slotted.node_kind
    });

    best_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::{Color, Move};
    use crate::fen::Fen;

    #[test]
    #[ignore]
    fn mate_pv() {
        let position =
            Position::parse_fen("r4rk1/1b3ppp/pp2p3/2p5/P1B1NR1Q/3P3P/2q3P1/7K w - - 0 24")
                .unwrap();

        let result = negamax(position, 6);
        assert_eq!(result.score.leading(), Some(Color::White));
        assert_eq!(result.best_move, Move::new(E4, F6, None));
        println!("{:?}", result.pv_line);
    }

    #[test]
    fn color_sign() {
        let cp = Cp(40); // Absolute score.

        // Relative scores.
        let w_signed = cp * Color::White.sign();
        let b_signed = cp * Color::Black.sign();
        assert_eq!(w_signed, Cp(40));
        assert_eq!(b_signed, Cp(-40));
    }
}
