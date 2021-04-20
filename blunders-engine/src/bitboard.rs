//! bitboard.rs
//! A general purpose way to efficiently encode data.
//! Uses:
//! - Positional data of
//!
//! Data Order:
//! Little-Endian Rank-File mapping (LSR)
//! A1 = least significant bit = 0b0 = 0
//! B1 = 0b1 = 1
//! C1 = 0b10 = 2
//! A2 = 0b1000 = 8
//! H8 = most significant bit = 0x8000000000000000
//!
//! Compass Rose Bit Shifting:
//!
//! NoWe       North       NoEa
//!      +7     +8      +9
//! West -1      0      +1 East
//!      -9     -8      -7
//! SoWe       South       SoEa
//!
//! Perpetual Data Represented by Bitboards:
//! W/B King position
//! W/B Queen positions
//! W/B Rook positions
//! W/B Bishop positions
//! W/B Knight positions
//! W/B Pawn positions
//!
//! Pawn Attack Pattern per square
//! Knight Attack Pattern per square
//! King Attack Pattern per square
//! Sliding Attack Pattern per square
//!
//! Generated Data Represented by Bitboards:
//! Pass Pawns
//!

use std::ops::{BitAnd, BitOr, Not};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Bitboard(u64);

/// Constants
impl Bitboard {
    pub const BLACK_SQUARES: Bitboard = Self(0xAA55AA55AA55AA55);
    pub const WHITE_SQUARES: Bitboard = Self(!Self::BLACK_SQUARES.0);
    pub const RANK_1: Bitboard = Self(0x00000000000000FF);
    pub const RANK_8: Bitboard = Self(0xFF00000000000000);
}

impl Bitboard {
    pub const fn bits(&self) -> &u64 {
        &self.0
    }

    /// Return number of bits that are '1'.
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

impl Not for Bitboard {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl BitOr for Bitboard {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for Bitboard {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
