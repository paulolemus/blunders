//! Negamax implementation of Minimax with Alpha-Beta pruning.

use std::time::Instant;

use crate::arrayvec;
use crate::coretypes::{Move, MoveInfo, Square::*, MAX_DEPTH, MAX_MOVES};
use crate::eval::{terminal, Cp};
use crate::movelist::{Line, MoveList};
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
    assert!(ply < MAX_DEPTH as u32);

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
    let ordered_legal_moves = order_all_moves(position, legal_moves, hash, tt);
    debug_assert_eq!(num_moves, ordered_legal_moves.len());

    // Placeholder best_move, is guaranteed to be overwritten as there is at
    // lest one legal move, and the score of that move is better than worst
    // possible score.
    let mut best_move = Move::new(A1, H7, None);
    let mut local_pv = Line::new();
    let mut best_score = Cp::MIN;

    // For each child of current position, recursively find maxing move.
    for legal_move in ordered_legal_moves.into_iter().rev() {
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
            arrayvec::append(pv_line, local_pv.clone());

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Label {
    Initialize,
    Search,
    Retrieve,
}

// Stack variables
#[derive(Debug, Clone)]
struct Frame {
    pub label: Label,
    pub local_pv: Line,
    pub ordered_moves: MoveList,
    pub alpha: Cp,
    pub beta: Cp,
    pub best_score: Cp,
    pub best_move: Move,
    pub hash: HashKind,
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            label: Label::Initialize,
            local_pv: Line::new(),
            ordered_moves: MoveList::new(),
            alpha: Cp::MIN,
            beta: Cp::MAX,
            best_score: Cp::MIN,
            best_move: Move::new(A1, H7, None),
            hash: 0,
        }
    }
}

#[inline(always)]
fn parent(frame_idx: usize) -> usize {
    frame_idx - 1
}

#[inline(always)]
fn child(frame_idx: usize) -> usize {
    frame_idx + 1
}

#[inline(always)]
fn curr_ply(frame_idx: usize) -> u32 {
    debug_assert!(frame_idx > 0);
    (frame_idx - 1) as u32
}

/// Iterative Negamax implementation
///
/// Why change from recursive to iterative?
/// * Need to be able to STOP search at any given time. Attempting this from a recursive call is difficult
///       because the return value would need to be changed or checked for a special condition.
///       It is EASY to return immediately from an iterative call than from a recursive call.
/// * Makes it easier to tell how far a node is from root.
/// * Can stop without risk of corrupting transposition table.
///
/// Recursive: all refer to same mut position and
//fn iterative_negamax(
//    mut position: Position,
//    ply: u32,
//    tt: &mut TranspositionTable,
//) -> SearchResult {
//    // Must have a valid ply to search to, and root position must not be terminal.
//    assert_ne!(ply, 0);
//    assert!(ply < MAX_DEPTH as u32);
//    assert_ne!(position.get_legal_moves().len(), 0);
//
//    // Meta Search variables
//    // Time search
//    let instant = Instant::now();
//    // Keep color of root player
//    let _root_player = *position.player();
//    // Count nodes visited
//    let mut nodes: u64 = 0;
//
//    // Stack holds local data used per ply.
//    // Size is +1 because the 0th index holds the most recently updated data
//    // for the root position. This is useful because it allows for returning
//    // without completing an entire depth, as the best pv so far is saved.
//    // 0th Idx: Global, best-so-far data
//    // 1st Idx: Root position data
//    let mut stack: ArrayVec<Frame, { MAX_DEPTH + 1 }> = ArrayVec::new();
//    // Fill stack with default values
//    while !stack.is_full() {
//        stack.push(Default::default());
//    }
//
//    // Stack of move info history.
//    let mut move_history: ArrayVec<MoveInfo, MAX_DEPTH> = ArrayVec::new();
//
//    // Frame indexer, begins at 1 (root) as 0 is for global pv.
//    let mut frame_idx: usize = 1;
//
//    // Set initial valid root parameters.
//    {
//        let root_frame: &mut Frame = &mut stack[frame_idx];
//        root_frame.label = Label::Initialize;
//        root_frame.hash = tt.generate_hash(&position);
//    }
//
//    // MAIN ITERATIVE LOOP //
//    while frame_idx > 0 {
//        let label: Label = stack[frame_idx].label;
//
//        // INITIALIZE MODE
//        // A new node has been created.
//        // If it is terminal, a leaf, or has been evaluated in the past,
//        // it immediately returns its evaluation up the stack to its parent.
//        // Otherwise, it has children nodes to search and sets itself into Search mode.
//        //
//        // Initialize -> Return eval to parent | set self to search mode.
//        if Label::Initialize == label {
//            let legal_moves = position.get_legal_moves();
//            let num_moves = legal_moves.len();
//            nodes += 1;
//
//            // Parent frame and hash are only used if this node early returns.
//            let hash = stack[frame_idx].hash;
//            let parent_idx = parent(frame_idx);
//            let parent_frame = &mut stack[parent_idx];
//
//            // This position has no best move.
//            // Store its evaluation and tell parent to retrieve value.
//            if num_moves == 0 {
//                parent_frame.local_pv.clear();
//                parent_frame.label = Label::Retrieve;
//
//                stack[frame_idx].best_score = terminal(&position);
//
//                frame_idx = parent_idx;
//
//            // Check if this position exists in tt.
//            } else if let Some(tt_info) = tt.get(hash) {
//                let remaining_ply = ply - curr_ply(frame_idx);
//                if tt_info.ply >= remaining_ply && legal_moves.contains(&tt_info.key_move) {
//                    // Found a usable Transposition hit. Its value can be used immediately
//                    // since this node has already been searched completely.
//                    parent_frame.local_pv.clear();
//                    parent_frame.local_pv.push(tt_info.key_move);
//                    parent_frame.label = Label::Retrieve;
//
//                    let relative_score = tt_info.score * position.player().sign();
//                    stack[frame_idx].best_score = relative_score;
//
//                    frame_idx = parent_idx;
//                }
//
//            // Max depth (leaf node) reached. Statically evaluate position and return value.
//            } else if curr_ply(frame_idx) == ply {
//                parent_frame.local_pv.clear();
//                parent_frame.label = Label::Retrieve;
//
//                stack[frame_idx].best_score = quiescence(&position, Cp::MIN, Cp::MAX);
//
//                frame_idx = parent_idx;
//
//            // Otherwise this node has children to continue to search.
//            // Order all of this node's legal moves, and set it to search mode.
//            } else {
//                let this_frame = &mut stack[frame_idx];
//                let ordered_moves = order_all_moves(&position, legal_moves, this_frame.hash, tt);
//                this_frame.ordered_moves = ordered_moves.into_iter();
//                this_frame.label = Label::Search;
//            }
//
//        // SEARCH MODE
//        // If a node ever enters search mode, it is guaranteed to have had a legal move to search.
//        // Each search either pushes a child node onto the stack during which it waits
//        // to be set to RETRIEVE, or it sees that it has evaluated all of its children and returns
//        // its own score to its parent.
//        } else if Label::Search == label {
//            // This position has a child position to search.
//            // Increment global variables for the child and initialize its frame.
//            if let Some(legal_move) = stack[frame_idx].ordered_moves.next() {
//                let this_frame = &stack[frame_idx];
//                let hash = this_frame.hash;
//                let alpha = this_frame.alpha;
//                let beta = this_frame.beta;
//
//                let move_info = position.do_move(legal_move);
//                let child_hash = tt.update_from_hash(hash, &position, &move_info);
//                move_history.push(move_info);
//
//                let child_frame = &mut stack[child(frame_idx)];
//                child_frame.label = Label::Initialize;
//                child_frame.hash = child_hash;
//                child_frame.alpha = -beta;
//                child_frame.beta = -alpha;
//                child_frame.best_score = Cp::MIN;
//
//                frame_idx = child(frame_idx);
//
//            // Every move for this node has been evaluated, so its complete score is returned.
//            // This node's hashtable index may be occupied, so it is added on the condition that
//            // its node-kind is less important than what exists in tt.
//            } else {
//                let this_frame = &stack[frame_idx];
//                let abs_node_score = this_frame.best_score * position.player().sign();
//                let remaining_ply = ply - curr_ply(frame_idx);
//                let tt_info = TranspositionInfo::new(
//                    this_frame.hash,
//                    NodeKind::Other,
//                    this_frame.best_move,
//                    remaining_ply,
//                    abs_node_score,
//                );
//                tt.replace_by(tt_info, |replacing, slotted| {
//                    replacing.node_kind >= slotted.node_kind
//                });
//
//                stack[parent(frame_idx)].label = Label::Retrieve;
//                frame_idx = parent(frame_idx);
//            }
//
//        // RETRIEVE MODE
//        // Only a child of the current node sets this value to RETRIEVE.
//        // This node is allowed to take the return value and process it.
//        //
//        } else if Label::Retrieve == label {
//            // Need to negate child's best score so its relative to this node.
//            let child_score = -stack[child(frame_idx)].best_score;
//            let this_frame = &mut stack[frame_idx];
//
//            let move_info = move_history.pop().unwrap();
//            position.undo_move(move_info);
//
//            // Update our best_* trackers if this move is best seen so far.
//            if child_score > this_frame.best_score {
//                this_frame.best_score = child_score;
//                this_frame.best_move = move_info.move_;
//            }
//
//            // Cut-off has occurred, no further children of this position need to be searched.
//            // This branch will not be taken further up the tree as there is a better move.
//            // Push this cut-node into the tt, with an absolute score, instead of relative.
//            if this_frame.best_score >= this_frame.beta {
//                let abs_best_score = this_frame.best_score * position.player().sign();
//                let remaining_ply = ply - curr_ply(frame_idx);
//                let tt_info = TranspositionInfo::new(
//                    this_frame.hash,
//                    NodeKind::Cut,
//                    this_frame.best_move,
//                    remaining_ply,
//                    abs_best_score,
//                );
//                tt.replace(tt_info);
//
//                // Early return.
//                stack[parent(frame_idx)].label = Label::Retrieve;
//                frame_idx = parent(frame_idx);
//
//            // Continue to search this node.
//            } else {
//                this_frame.label = Label::Search;
//
//                // New local PV has been found. Update alpha and store new Line.
//                // Update this node in tt as a PV node.
//                if this_frame.best_score > this_frame.alpha {
//                    this_frame.alpha = this_frame.best_score;
//                    let local_pv = this_frame.local_pv.clone();
//
//                    let abs_best_score = this_frame.best_score * position.player().sign();
//                    let remaining_ply = ply - curr_ply(frame_idx);
//                    let tt_info = TranspositionInfo::new(
//                        this_frame.hash,
//                        NodeKind::Pv,
//                        move_info.move_,
//                        remaining_ply,
//                        abs_best_score,
//                    );
//                    tt.replace(tt_info);
//
//                    let parent_frame = &mut stack[parent(frame_idx)];
//                    parent_frame.local_pv.clear();
//                    parent_frame.local_pv.push(move_info.move_);
//                    parent_frame.local_pv.append(local_pv);
//                }
//            }
//        }
//    }
//
//    let result = SearchResult {
//        best_move: Move::new(A1, H7, None),
//        score: Cp(0),
//        pv_line: Line::new(),
//        nodes,
//        elapsed: instant.elapsed(),
//    };
//    result
//}

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
