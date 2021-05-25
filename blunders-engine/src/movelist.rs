//! MoveList types used in Blunders engine.
//!
//! The underlying type of MoveList may change at any time during
//! pre-1.0 development, so a MoveList type alias makes changes easy.

use crate::arrayvec::ArrayVec;
use crate::coretypes::{Move, MoveInfo};

/// Maximum possible number of moves for any position.
const MAX_MOVES: usize = 218;

// Two primary types used in engine.
pub type MoveList = ArrayVec<Move, MAX_MOVES>;
pub type MoveInfoList = ArrayVec<MoveInfo, MAX_MOVES>;
