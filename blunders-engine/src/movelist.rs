//! MoveList types used in Blunders engine.
//!
//! The underlying type of MoveList may change at any time during
//! pre-1.0 development, so a MoveList type alias makes changes easy.

use crate::arrayvec::ArrayVec;
use crate::coretypes::MAX_LINE_LEN;
use crate::coretypes::MAX_MOVES;
use crate::coretypes::{Move, MoveInfo};

/// MoveList is a container that can hold at most `MAX_MOVES`, the most number of moves per any chess position.
pub type MoveList = ArrayVec<Move, MAX_MOVES>;
/// MoveInfoList is like MoveList however it also holds metadata for its moves.
pub type MoveInfoList = ArrayVec<MoveInfo, MAX_MOVES>;
/// Line is a sequence of legal moves that can be applied to a position. Useful for retaining a principal variation
/// found from a search.
pub type Line = ArrayVec<Move, MAX_LINE_LEN>;
