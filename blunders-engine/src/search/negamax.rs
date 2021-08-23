//! Negamax implementation of Minimax with Alpha-Beta pruning.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::arrayvec::{self, ArrayVec};
use crate::coretypes::{Cp, Move, MoveInfo, MoveKind, PieceKind, PlyKind, MAX_DEPTH};
use crate::eval::{draw, terminal};
use crate::movelist::{Line, MoveInfoList};
use crate::moveorder::order_all_moves;
use crate::position::{Cache, Position};
use crate::search::{quiescence, History, SearchResult};
use crate::timeman::Mode;
use crate::transposition::{Entry, NodeKind, TranspositionTable};
use crate::zobrist::HashKind;

/// Negamax implementation of Minimax with alpha-beta pruning.
/// Negamax searches to a given depth and returns the best move found.
/// Internally, Negamax treats the active player as the maxing player,
/// however the final centipawn score of the position returned is
/// absolute with White as maxing and Black as minning.
pub fn negamax(mut position: Position, ply: PlyKind, tt: &TranspositionTable) -> SearchResult {
    assert!(0 < ply && ply < MAX_DEPTH);

    let root_player = *position.player();
    let hash = tt.generate_hash(&position);
    let instant = Instant::now();

    let mut pv = Line::new();
    let mut nodes = 0;

    let best_score = negamax_impl(
        &mut position,
        tt,
        hash,
        &mut pv,
        &mut nodes,
        ply,
        Cp::MIN,
        Cp::MAX,
    );

    SearchResult {
        player: root_player,
        depth: ply,
        best_move: *pv.get(0).unwrap(),
        score: best_score * root_player.sign(),
        pv,
        nodes,
        elapsed: instant.elapsed(),
        ..Default::default()
    }
}

/// The player whose turn it is to move for a position is always treated as the maxing player.
/// negamax_impl returns the max possible score of the current maxing player.
/// Therefore, when interpreting the score of a child node, the score needs to be negated.
///
/// negamax_impl stores the principal variation of the current move into the pv parameter.
///
/// Parameters:
///
/// position: current position to search.
/// tt: Transposition Table used for recalling search history.
/// hash: Incrementally updatable hash of provided position.
/// pv: Line of moves in principal variation.
/// nodes: Counter for number of nodes visited in search.
/// ply: remaining depth to search to.
/// alpha: Best (greatest) guaranteed value for current player.
/// beta: Best (lowest) guaranteed value for opposite player.
fn negamax_impl(
    position: &mut Position,
    tt: &TranspositionTable,
    hash: HashKind,
    pv: &mut Line,
    nodes: &mut u64,
    ply: PlyKind,
    mut alpha: Cp,
    beta: Cp,
) -> Cp {
    *nodes += 1;
    let mut q_nodes = 0; // TODO: Consolidate metrics.

    let legal_moves = position.get_legal_moves();
    let num_moves = legal_moves.len();

    // Save tt lookup from nested if.
    let mut hash_move = None;

    // Search can return when any of the following are encountered:
    // * Checkmate / Stalemate (terminal node)
    // * Tt move evaluated at equal or greater depth than searching depth
    // * depth 0 reached (leaf node)
    //
    // An eval is returned with respect to the current player.
    // (+Cp good, -Cp bad)
    // Terminal and leaf nodes have no following moves so pv of parent is cleared.
    if num_moves == 0 {
        pv.clear();
        return terminal(&position);
    }
    // Check if current move exists in tt. If so, we might be able to return that value
    // right away if has a greater or equal depth than we are considering.
    // Check that the tt key_move is a legal move, as extra (but not complete)
    // protection against Key collisions.
    // TODO: Verify that this is bug free. It is possible this may cut the Pv line,
    //       or that returning early is incorrect.
    else if let Some(tt_entry) = tt.get(hash) {
        if tt_entry.ply >= ply && legal_moves.contains(&tt_entry.key_move) {
            pv.clear();
            pv.push(tt_entry.key_move);
            return tt_entry.score;
        }
        hash_move = Some(tt_entry.key_move);

    // Run a Quiescence Search for non-terminal leaf nodes to find a more stable
    // evaluation than a static evaluation.
    // The parent of this node receives an empty pv,
    // because this leaf node has no best move, and is not in history.
    } else if ply == 0 {
        pv.clear();
        let q_ply = 10;
        return quiescence(position, alpha, beta, q_ply, &mut q_nodes);
    }

    // Move Ordering
    // Sort legal moves with estimated best move first.
    let legal_moves = legal_moves
        .into_iter()
        .map(|move_| position.move_info(move_))
        .collect();
    let ordered_legal_moves = order_all_moves(legal_moves, hash_move);
    debug_assert_eq!(num_moves, ordered_legal_moves.len());

    // Placeholder best_move, is guaranteed to be overwritten as there is at
    // lest one legal move, and the score of that move is better than worst
    // possible score.
    let cache = position.cache();
    let mut best_move = Move::illegal();
    let mut local_pv = Line::new();
    let mut best_score = Cp::MIN;

    // For each child of current position, recursively find maxing move.
    for legal_move_info in ordered_legal_moves.into_iter().rev() {
        // Get value of a move relative to active player.
        position.do_move_info(legal_move_info);
        let move_hash = tt.update_from_hash(hash, &position, legal_move_info, cache);
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
        position.undo_move(legal_move_info, cache);

        // Update best_* trackers if this move is best of all seen so far.
        if move_score > best_score {
            best_score = move_score;
            best_move = legal_move_info.move_();
        }

        // Cut-off has occurred, no further children of this position need to be searched.
        // This branch will not be taken further up the tree as there is a better move.
        // Push this cut-node into the tt, with a score relative to this node's active player.
        if move_score >= beta {
            let cut_move = legal_move_info.move_();
            let tt_entry = Entry::new(hash, NodeKind::Cut, cut_move, ply, move_score);
            tt.replace(tt_entry);
            return move_score;
        }

        // A new local PV line has been found. Update alpha and store new Line.
        // Update this node in tt as a PV node.
        if best_score > alpha {
            alpha = best_score;
            pv.clear();
            pv.push(best_move);
            arrayvec::append(pv, local_pv.clone());

            let tt_entry = Entry::new(hash, NodeKind::Pv, best_move, ply, best_score);
            tt.replace(tt_entry);
        }
    }

    // Every move for this node has been evaluated. It is possible that this node
    // was added to the tt beforehand, so we can add it on the condition that
    // It's node-kind is less important than what exists in tt.
    let tt_entry = Entry::new(hash, NodeKind::All, best_move, ply, best_score);
    tt.replace_by(tt_entry, |replacing, slotted| {
        replacing.node_kind >= slotted.node_kind
    });

    best_score
}

/// Label represents what stage of processing a node is in.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
    pub legal_moves: MoveInfoList,
    pub alpha: Cp,
    pub beta: Cp,
    pub best_score: Cp,
    pub best_move: Move,
    pub hash: HashKind,
    pub move_info: MoveInfo,
    pub cache: Cache,
}
/// A frame defaults with junk data, however this is acceptable
/// because nodes set appropriate data before using.
impl Default for Frame {
    fn default() -> Self {
        let illegal_move = Move::illegal();
        Self {
            label: Label::Initialize,
            local_pv: Line::new(),
            legal_moves: MoveInfoList::new(),
            alpha: Cp::MIN,
            beta: Cp::MAX,
            best_score: Cp::MIN,
            best_move: Move::illegal(),
            hash: 0,
            move_info: MoveInfo {
                from: illegal_move.from,
                to: illegal_move.to,
                promotion: illegal_move.promotion,
                piece_kind: PieceKind::Pawn,
                move_kind: MoveKind::Quiet,
            },
            cache: Cache::illegal(),
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
fn curr_ply(frame_idx: usize) -> PlyKind {
    debug_assert!(frame_idx > 0);
    (frame_idx - 1) as PlyKind
}

/// Iterative fail-soft Negamax implementation with alpha-beta pruning and transposition table lookup.
///
/// In fail-soft, the return value of a call can exceed its given bounds alpha and beta (score < alpha, score > beta).
///
/// Why change from recursive to iterative?
/// * Need to be able to STOP searching at any time.
/// This is hard to do from a recursive search without changing/checking return value.
/// * Makes it easier to tell how far a node is from root.
/// * Easy to stop without risk of corrupting transposition table entries.
pub fn iterative_negamax(
    mut position: Position,
    ply: PlyKind,
    mode: Mode,
    mut history: History,
    tt: &TranspositionTable,
    stopper: Arc<AtomicBool>,
) -> Option<SearchResult> {
    // Guard: must have a valid searchable ply, and root position must not be terminal.
    assert!(0 < ply && ply <= MAX_DEPTH);
    assert_ne!(position.get_legal_moves().len(), 0);

    // Meta Search variables
    let root_position = position.clone(); // For assertions
    let root_hash = tt.generate_hash(&position); // Keep copy of root hash for assertions
    let root_history = history.clone();
    // Early Stop variables
    let nodes_per_stop_check = 2000; // Number of nodes between updates to stopped flag
    let mut stopped = false; // Indicates if search was stopped
    let mut stop_check_counter = nodes_per_stop_check; // When this hits 0, update stopped and reset

    // A score assigned to draws to lean engine away from drawing (Cp 0) when slightly behind.
    let contempt = Cp(50);

    // Metrics
    let instant = Instant::now(); // Timer for search.
    let mut q_elapsed = Duration::ZERO; // Time spent in quiescence.
    let mut nodes: u64 = 0; // Number of nodes in main search.
    let mut q_nodes: u64 = 0; // Number of nodes created in quiescence.
    let mut alpha_incs = 0; // Number of times alpha gets improved, anywhere.
    let mut beta_cuts = 0; // Number of times a beta cut-off occurs.
    let mut tt_hits = 0; // Number of times a position was found in tt.
    let mut tt_cuts = 0; // Number of times a tt hit was returned immediately.

    // Stack holds frame data, where each ply gets one frame.
    // Size is +1 because the 0th index holds the PV so far for root position.
    const BASE_IDX: usize = 0; // Root passes PV to this parent frame
    const ROOT_IDX: usize = 1; // Root position data frame
    let mut stack: ArrayVec<Frame, { (MAX_DEPTH + 1) as usize }> = ArrayVec::new();
    // Fill stack with default values to navigate, opposed to pushing and popping.
    while !stack.is_full() {
        stack.push(Default::default());
    }
    // Set initial valid root parameters.
    stack[ROOT_IDX].label = Label::Initialize;
    stack[ROOT_IDX].hash = root_hash;
    stack[ROOT_IDX].cache = root_position.cache();

    // Frame indexer, begins at 1 (root) as 0 is for global pv.
    // Incrementing -> recurse to child, Decrementing -> return to parent.
    let mut frame_idx: usize = ROOT_IDX;

    // MAIN ITERATIVE LOOP
    while frame_idx > 0 {
        // Take a mut sliding window view into the stack.
        let (parent, us, child) = split_window_frames(&mut stack, frame_idx);
        // How many ply left to target depth.
        let remaining_ply = ply - curr_ply(frame_idx);
        let label: Label = us.label;

        // Stop Check: Before processing, check if search has been told to stop.
        // It is safe to stop at anytime outside of the processing modes below.
        if label == Label::Initialize && stop_check_counter <= 0 {
            stop_check_counter = nodes_per_stop_check;
            stopped |= stopper.load(Ordering::Acquire);
            stopped |= mode.stop(root_position.player, ply);
        }

        // If stopped flag is ever set, breaking ends search early.
        if stopped {
            break;
        }

        // INITIALIZE MODE
        // A new node has been created.
        // If it is terminal, a leaf, or has been evaluated in the past,
        // it immediately returns its evaluation up the stack to its parent.
        // Otherwise, it has children nodes to search and sets itself into Search mode.
        //
        // Flow: Return eval to parent || set self to search mode
        if Label::Initialize == label {
            stop_check_counter -= 1;
            nodes += 1;

            let legal_moves = position.get_legal_moves();
            let num_moves = legal_moves.len();

            // Save TT lookup to avoid re-locking.
            let mut hash_move = None;

            // This position has no best move.
            // Store its evaluation and tell parent to retrieve value.
            if num_moves == 0 {
                parent.label = Label::Retrieve;
                parent.local_pv.clear();
                us.best_score = terminal(&position);

                frame_idx = parent_idx(frame_idx);
                continue;
            }
            // Check for draw by repetition or fifty-move rule.
            // After terminal because terminal can't be repeated, mate presides over 50-move rule.
            // Before tt lookup because a repeated position has a different score than when previously visited.
            // TODO:
            // Change to twofold_repetition but avoid error where root is in history.
            else if position.fifty_move_rule(num_moves)
                || history.is_threefold_repetition(us.hash)
            {
                parent.label = Label::Retrieve;
                parent.local_pv.clear();
                us.best_score = draw(root_position.player == position.player, contempt);

                frame_idx = parent_idx(frame_idx);
                continue;
            }
            // Check if this position exists in tt and has been searched to/beyond our ply.
            // If so the score is usable, store this value and return to parent.
            else if let Some(tt_entry) = tt.get(us.hash) {
                tt_hits += 1;
                if tt_entry.ply >= remaining_ply && legal_moves.contains(&tt_entry.key_move) {
                    tt_cuts += 1;
                    parent.label = Label::Retrieve;
                    parent.local_pv.clear();
                    parent.local_pv.push(tt_entry.key_move);

                    us.best_score = tt_entry.score;
                    us.best_move = tt_entry.key_move;

                    frame_idx = parent_idx(frame_idx);
                    continue;
                }
                hash_move = Some(tt_entry.key_move);
            }
            // Max depth (leaf node) reached. Statically evaluate position and return value.
            else if remaining_ply == 0 {
                parent.label = Label::Retrieve;
                parent.local_pv.clear();

                let q_ply = 10;
                let q_instant = Instant::now();
                us.best_score = quiescence(&mut position, us.alpha, us.beta, q_ply, &mut q_nodes);
                q_elapsed += q_instant.elapsed();

                frame_idx = parent_idx(frame_idx);
                continue;
            }

            // This node has not returned early, so it has moves to search.
            // Order all of this node's legal moves, and set it to search mode.
            // Optional: Either Sort all moves first, or pick best each time.
            let legal_moves: MoveInfoList = legal_moves
                .into_iter()
                .map(|move_| position.move_info(move_))
                .collect();

            us.legal_moves = order_all_moves(legal_moves, hash_move);
            us.cache = position.cache();
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
            if let Some(legal_move) = us.legal_moves.pop() {
                us.move_info = legal_move;
                position.do_move_info(legal_move);
                history.push(us.hash, us.move_info.is_unrepeatable());

                let child_hash = tt.update_from_hash(us.hash, &position, us.move_info, us.cache);
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
                let tt_entry = Entry::new(
                    us.hash,
                    NodeKind::All,
                    us.best_move,
                    remaining_ply,
                    us.best_score,
                );
                tt.replace_by(tt_entry, |replacing, slotted| {
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
            position.undo_move(us.move_info, us.cache);
            history.pop();

            // Negate child's best score so it's relative to this node.
            let move_score = -child.best_score;

            // Update our best_* trackers if this move is best seen so far.
            if move_score > us.best_score {
                us.best_score = move_score;
                us.best_move = us.move_info.move_();
            }

            // Cut-off has occurred, no further children of this position need to be searched.
            // This branch will not be taken further up the tree as there is a better move.
            // Push this cut-node into the tt, with an absolute score, instead of relative.
            if us.best_score >= us.beta {
                beta_cuts += 1;
                let tt_entry = Entry::new(
                    us.hash,
                    NodeKind::Cut,
                    us.best_move,
                    remaining_ply,
                    us.best_score,
                );
                tt.replace(tt_entry);

                // Early return.
                parent.label = Label::Retrieve;
                frame_idx = parent_idx(frame_idx);
                continue;
            }

            // New local PV has been found. Update alpha and store new Line.
            // Update this node in tt as a PV node.
            if us.best_score > us.alpha {
                alpha_incs += 1;
                us.alpha = us.best_score;

                // Give parent updated PV by appending child PV to our best move.
                parent.local_pv.clear();
                parent.local_pv.push(us.best_move);
                arrayvec::append(&mut parent.local_pv, us.local_pv.clone());
            }

            // Default action is to attempt to continue searching this node.
            us.label = Label::Search;
        }
    }

    if !stopped {
        // Position has been returned to root position. Hashes should be equal.
        debug_assert_eq!(root_hash, tt.generate_hash(&position));
        debug_assert_eq!(root_hash, stack[ROOT_IDX].hash);
        // History modified from search should return to equal that before searching.
        debug_assert_eq!(root_history, history);
    }

    // The search may not run to completion. If at any point the Root node's PV gets updated,
    // the base will have a non-zero length PV as the default is zero length.
    // This PV can be returned as a best guess. If this is coming from iterative deepening
    // this partial-search PV is guaranteed to be at least more accurate than what came from
    // a lesser depth, as long as the previous depth PV was searched first.
    if stack[BASE_IDX].local_pv.len() == 0 {
        None
    } else {
        let best_move = stack[ROOT_IDX].best_move;
        assert_ne!(best_move, Move::illegal());

        Some(SearchResult {
            player: root_position.player,
            depth: ply,
            best_move,
            score: stack[ROOT_IDX].best_score * root_position.player.sign(),
            pv: stack[BASE_IDX].local_pv.clone(),
            nodes: nodes + q_nodes,
            q_nodes,
            elapsed: instant.elapsed(),
            q_elapsed,
            stopped,
            alpha_increases: alpha_incs,
            beta_cutoffs: beta_cuts,
            tt_hits,
            tt_cuts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::{Color, Move, Square::*};
    use crate::fen::Fen;

    #[test]
    #[ignore]
    fn mate_pv() {
        let position =
            Position::parse_fen("r4rk1/1b3ppp/pp2p3/2p5/P1B1NR1Q/3P3P/2q3P1/7K w - - 0 24")
                .unwrap();

        let mut tt = TranspositionTable::new();
        let result = negamax(position, 6, &mut tt);
        assert_eq!(result.leading(), Some(Color::White));
        assert_eq!(result.best_move, Move::new(E4, F6, None));
        println!("{:?}", result.pv);
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
