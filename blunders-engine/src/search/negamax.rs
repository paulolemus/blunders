//! Negamax implementation of Minimax with Alpha-Beta pruning.

use std::time::Instant;

use crate::arrayvec::{self, ArrayVec};
use crate::coretypes::{Castling, Move, MoveInfo, MoveKind, PieceKind, Square::*, MAX_DEPTH};
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
    let tt_info = TranspositionInfo::new(hash, NodeKind::All, best_move, ply, abs_move_score);
    tt.replace_by(tt_info, |replacing, slotted| {
        replacing.node_kind >= slotted.node_kind
    });

    best_score
}

/// Label represents what stage of processing a node is in.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Label {
    Initialize,
    Search,
    Retrieve,
}

/// Frame contains all the variables needed during the evaluation of a node.
/// It is somewhat like the call frame for recursive negamax.
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
    pub move_info: MoveInfo,
}
/// A frame defaults with junk data, however this is acceptable
/// because nodes set appropriate data before using.
impl Default for Frame {
    fn default() -> Self {
        let illegal_move = Move::new(A1, H7, None);
        Self {
            label: Label::Initialize,
            local_pv: Line::new(),
            ordered_moves: MoveList::new(),
            alpha: Cp::MIN,
            beta: Cp::MAX,
            best_score: Cp::MIN,
            best_move: illegal_move,
            hash: 0,
            move_info: MoveInfo {
                move_: illegal_move,
                piece_kind: PieceKind::Pawn,
                move_kind: MoveKind::Quiet,
                castling: Castling::NONE,
                en_passant: None,
                halfmoves: 1,
            },
        }
    }
}

/// Extract a "Window" from a frame stack, where a window is a reference to
/// the parent, current, and child frames of the given frame index.
/// Frame index must not be 0.
#[inline(always)]
fn split_window_frames(frames: &mut [Frame], idx: usize) -> (&mut Frame, &mut Frame, &mut Frame) {
    debug_assert!(idx > 0, "cannot get parent frame of index 0");
    // split_at_mut includes the index in the second slice.
    let (parent_slice, rest) = frames.split_at_mut(idx);
    let (curr_slice, rest) = rest.split_at_mut(1);

    let parent_frame = parent_slice.last_mut().unwrap();
    let current_frame = &mut curr_slice[0];
    let child_frame = &mut rest[0];

    (parent_frame, current_frame, child_frame)
}

/// Given a frame index, returns the index of the frame's parent.
#[inline(always)]
fn parent_idx(frame_idx: usize) -> usize {
    frame_idx - 1
}

/// Given a frame index, returns the index of the frame's child.
#[inline(always)]
fn child_idx(frame_idx: usize) -> usize {
    frame_idx + 1
}

/// Convert a frame index to a ply.
#[inline(always)]
fn curr_ply(frame_idx: usize) -> u32 {
    debug_assert!(frame_idx > 0);
    (frame_idx - 1) as u32
}

/// Iterative Negamax implementation with alpha-beta pruning.
///
/// Why change from recursive to iterative?
/// * Need to be able to STOP searching at any time.
/// This is hard to do from a recursive search without changing/checking return value.
/// * Makes it easier to tell how far a node is from root.
/// * Easy to stop without risk of corrupting transposition table entries.
pub fn iterative_negamax(
    mut position: Position,
    ply: u32,
    tt: &mut TranspositionTable,
) -> SearchResult {
    // Guard: must have a valid searchable ply, and root position must not be terminal.
    assert_ne!(ply, 0);
    assert!(ply < MAX_DEPTH as u32);
    assert_ne!(position.get_legal_moves().len(), 0);

    // Meta Search variables
    let instant = Instant::now(); // Timer for search
    let root_player = *position.player(); // Keep copy of root player for assertions

    // Metrics
    let mut nodes: u64 = 0; // Number of nodes created

    // Stack holds frame data, where each ply gets one frame.
    // Size is +1 because the 0th index holds the PV so far for root position.
    // 0th Idx: Root PV (root passes PV to parent).
    // 1st Idx: Root data frame.
    const BASE_IDX: usize = 0;
    const ROOT_IDX: usize = 1;
    let mut stack: ArrayVec<Frame, { MAX_DEPTH + 1 }> = ArrayVec::new();
    // Fill stack with default values to navigate, opposed to pushing and popping.
    while !stack.is_full() {
        stack.push(Default::default());
    }
    // Set initial valid root parameters.
    {
        let root_frame: &mut Frame = &mut stack[ROOT_IDX];
        root_frame.label = Label::Initialize;
        root_frame.hash = tt.generate_hash(&position);
    }

    // Frame indexer, begins at 1 (root) as 0 is for global pv.
    // Incrementing -> recurse to child
    // Decrementing -> return to parent
    let mut frame_idx: usize = ROOT_IDX;

    // MAIN ITERATIVE LOOP
    while frame_idx > 0 {
        // Take a mut sliding window view into the stack.
        let (parent, us, child) = split_window_frames(&mut stack, frame_idx);

        // How many ply left to target depth.
        let remaining_ply = ply - curr_ply(frame_idx);

        let label: Label = us.label;

        // INITIALIZE MODE
        // A new node has been created.
        // If it is terminal, a leaf, or has been evaluated in the past,
        // it immediately returns its evaluation up the stack to its parent.
        // Otherwise, it has children nodes to search and sets itself into Search mode.
        //
        // Flow: Return eval to parent || set self to search mode
        if Label::Initialize == label {
            let legal_moves = position.get_legal_moves();
            let num_moves = legal_moves.len();
            nodes += 1;

            // This position has no best move.
            // Store its evaluation and tell parent to retrieve value.
            if num_moves == 0 {
                parent.label = Label::Retrieve;
                parent.local_pv.clear();
                us.best_score = terminal(&position);

                frame_idx = parent_idx(frame_idx);
                continue;

            // Check if this position exists in tt and has been searched to/beyond our ply.
            // If so the score is usable, store this value and return to parent.
            } else if let Some(tt_info) = tt.get(us.hash) {
                if tt_info.ply >= remaining_ply && legal_moves.contains(&tt_info.key_move) {
                    parent.label = Label::Retrieve;
                    parent.local_pv.clear();
                    parent.local_pv.push(tt_info.key_move);

                    let relative_score = tt_info.score * position.player().sign();
                    us.best_score = relative_score;

                    frame_idx = parent_idx(frame_idx);
                    continue;
                }

            // Max depth (leaf node) reached. Statically evaluate position and return value.
            } else if remaining_ply == 0 {
                parent.label = Label::Retrieve;
                parent.local_pv.clear();

                us.best_score = quiescence(&position, Cp::MIN, Cp::MAX);

                frame_idx = parent_idx(frame_idx);
                continue;
            }

            // This node has not returned early, so it has moves to search.
            // Order all of this node's legal moves, and set it to search mode.
            us.ordered_moves = order_all_moves(&position, legal_moves, us.hash, tt);
            us.label = Label::Search;

        // SEARCH MODE
        // If a node ever enters search mode, it is guaranteed to have had a legal move to search.
        // Each search either pushes a child node onto the stack during which it waits
        // to be set to RETRIEVE, or it sees that it has evaluated all of its children and returns
        // its own score to its parent.
        //
        // Flow: (Moves to search) ? recurse to child : return eval to parent
        } else if Label::Search == label {
            // This position has a child position to search, initialize its frame.
            if let Some(legal_move) = us.ordered_moves.pop() {
                us.move_info = position.do_move(legal_move);
                let child_hash = tt.update_from_hash(us.hash, &position, &us.move_info);

                child.label = Label::Initialize;
                child.hash = child_hash;
                child.alpha = -us.beta;
                child.beta = -us.alpha;
                child.best_score = Cp::MIN;

                frame_idx = child_idx(frame_idx);

            // Every move for this node has been evaluated, so its complete score is returned.
            } else {
                // ALL-NODE hash strategy: TODO
                // Currently adding only if it's node-kind is less important than what's in tt.
                let abs_score = us.best_score * position.player().sign();
                let tt_info = TranspositionInfo::new(
                    us.hash,
                    NodeKind::All,
                    us.best_move,
                    remaining_ply,
                    abs_score,
                );
                tt.replace_by(tt_info, |replacing, slotted| {
                    replacing.node_kind >= slotted.node_kind
                });

                parent.label = Label::Retrieve;
                frame_idx = parent_idx(frame_idx);
            }

        // RETRIEVE MODE
        // Only a child of the current node sets this value to RETRIEVE.
        // This node is allowed to take the return value and process it.
        //
        // Flow: (beta cutoff) ? Return best-score to parent : continue searching this node
        } else if Label::Retrieve == label {
            // Negate child's best score so it's relative to this node.
            let move_score = -child.best_score;

            position.undo_move(us.move_info);

            // Update our best_* trackers if this move is best seen so far.
            if move_score > us.best_score {
                us.best_score = move_score;
                us.best_move = us.move_info.move_;
            }

            // Cut-off has occurred, no further children of this position need to be searched.
            // This branch will not be taken further up the tree as there is a better move.
            // Push this cut-node into the tt, with an absolute score, instead of relative.
            if us.best_score >= us.beta {
                let abs_best_score = us.best_score * position.player().sign();
                let tt_info = TranspositionInfo::new(
                    us.hash,
                    NodeKind::Cut,
                    us.best_move,
                    remaining_ply,
                    abs_best_score,
                );
                tt.replace(tt_info);

                // Early return.
                parent.label = Label::Retrieve;
                frame_idx = parent_idx(frame_idx);
                continue;
            }

            // New local PV has been found. Update alpha and store new Line.
            // Update this node in tt as a PV node.
            // TODO: This might not be sound, since we are storing a value
            // where node is only partially checked.
            if us.best_score > us.alpha {
                us.alpha = us.best_score;

                // Give parent updated PV by appending child PV to our best move.
                parent.local_pv.clear();
                parent.local_pv.push(us.best_move);
                arrayvec::append(&mut parent.local_pv, us.local_pv.clone());

                //let abs_best_score = us.best_score * position.player().sign();
                //let tt_info = TranspositionInfo::new(
                //    us.hash,
                //    NodeKind::Pv,
                //    us.best_move,
                //    remaining_ply,
                //    abs_best_score,
                //);
                //tt.replace(tt_info);
            }

            // Default action is to attempt to continue searching this node.
            us.label = Label::Search;
        }
    }

    // Check that the position has returned to its original state.
    assert_eq!(*position.player(), root_player);

    // Extract values from stack.
    let result = SearchResult {
        best_move: stack[ROOT_IDX].best_move,
        score: stack[ROOT_IDX].best_score * root_player.sign(),
        pv_line: stack[BASE_IDX].local_pv.clone(),
        nodes,
        elapsed: instant.elapsed(),
    };

    result
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
