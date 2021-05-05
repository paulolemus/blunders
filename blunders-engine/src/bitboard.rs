//! A general purpose way to efficiently encode data,
//! where each bit index of a 64-bit unsigned integer represents a chessboard square.
//!
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

use crate::coretypes::{Square, Square::*, SquareIndexable};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Bitboard(pub(crate) u64);

// Generate a bitboard made from bit-or-ing the bit-shifted representation of
// each identifier passed.
// Macro needed because `from` trait not const.
// example: bb_from_shifts!(A1, A2) ->
//          Bitboard(0u64 | (1u64 << A1 as u8) | (1u64 << A2 as u8))
macro_rules! bb_from_shifts {
    ($($shiftable:ident),+) => {
        Bitboard(0u64 $( | (1u64 << $shiftable as u8))*)
    };
}

/// Bitboard Constants
impl Bitboard {
    pub const EMPTY: Bitboard = Self(0x0);
    pub const BLACK_SQUARES: Bitboard = Self(0xAA55AA55AA55AA55);
    pub const WHITE_SQUARES: Bitboard = Self(!Self::BLACK_SQUARES.0);
    pub const RANK_1: Bitboard = bb_from_shifts!(A1, B1, C1, D1, E1, F1, G1, H1);
    pub const RANK_2: Bitboard = bb_from_shifts!(A2, B2, C2, D2, E2, F2, G2, H2);
    pub const RANK_7: Bitboard = bb_from_shifts!(A7, B7, C7, D7, E7, F7, G7, H7);
    pub const RANK_8: Bitboard = bb_from_shifts!(A8, B8, C8, D8, E8, F8, G8, H8);
    pub const FILE_A: Bitboard = bb_from_shifts!(A1, A2, A3, A4, A5, A6, A7, A8);
    pub const FILE_H: Bitboard = bb_from_shifts!(H1, H2, H3, H4, H5, H6, H7, H8);
}

/// Bitboard is a wrapper for a u64.
/// Each bit represents the presence of something in that bit position.
impl Bitboard {
    pub const fn bits(&self) -> &u64 {
        &self.0
    }

    /// Returns number of squares present.
    /// Equivalent to number of bits in binary representation that are '1'.
    pub const fn count_squares(&self) -> u32 {
        self.0.count_ones()
    }

    /// Returns true if index is populated.
    pub fn has_square<I: SquareIndexable>(&self, idx: I) -> bool {
        self.0 & idx.shift() != 0
    }
    /// Sets bit index to 1.
    pub fn set_square<I: SquareIndexable>(&mut self, idx: I) {
        self.0 |= idx.shift();
    }
    /// Sets bit index to 0.
    pub fn clear_square<I: SquareIndexable>(&mut self, idx: I) {
        self.0 &= !idx.shift();
    }
    /// Toggles bit index. 0 -> 1, 1 -> 0.
    pub fn toggle_square<I: SquareIndexable>(&mut self, idx: I) {
        self.0 ^= idx.shift();
    }

    /// Returns new Bitboard with all squares shifted 1 square north (ex: D4 -> D5).
    pub const fn to_north(&self) -> Self {
        Self(self.0 << 8)
    }
    /// Returns new Bitboard with all squares shifted 1 square south (ex: D4 -> D3).
    pub const fn to_south(&self) -> Self {
        Self(self.0 >> 8)
    }
    /// Returns new Bitboard with all squares shifted 1 square east (ex: D4 -> E4).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_A.
    pub const fn to_east(&self) -> Self {
        const NOT_FILE_A: u64 = !Bitboard::FILE_A.0;
        Self((self.0 << 1) & NOT_FILE_A)
    }
    /// Returns new Bitboard with all squares shifted 1 square west (ex: D4 -> C4).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_H.
    pub const fn to_west(&self) -> Self {
        const NOT_FILE_H: u64 = !Bitboard::FILE_H.0;
        Self((self.0 >> 1) & NOT_FILE_H)
    }

    /// Returns a vector of all the Squares represented in the Bitboard.
    /// # Examples
    /// ```rust
    /// # use blunders_engine::bitboard::Bitboard;
    /// # use blunders_engine::coretypes::Square;
    /// let squares = vec![Square::A1, Square::D7];
    /// let mut board = Bitboard::EMPTY;
    /// squares.iter().for_each(|square| board.set_square(*square));
    /// assert_eq!(board.squares(), squares);
    /// ```
    /// # Algorithm
    /// For each '1' bit in Bitboard:
    /// * Count trailing zeros. This is equal to the Square index, and is of 0-63.
    /// * Get shift index by shifting by square index.
    /// * Use shift index to remove bit from Bitboard.
    /// * Convert square index to a Square and add to list.
    pub fn squares(&self) -> Vec<Square> {
        let mut bits: u64 = self.0;
        let num_ones = bits.count_ones() as usize;
        let mut vec = Vec::with_capacity(num_ones);

        for _ in 0..num_ones {
            let square_value = bits.trailing_zeros() as u8;
            bits ^= 1u64 << square_value;
            let square = Square::from_u8(square_value);
            debug_assert!(square_value < 64u8);
            debug_assert!(square.is_some());
            vec.push(square.unwrap());
        }
        vec
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

impl BitOr<&Bitboard> for Bitboard {
    type Output = Self;
    fn bitor(self, rhs: &Bitboard) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOr<Bitboard> for &Bitboard {
    type Output = Bitboard;
    fn bitor(self, rhs: Bitboard) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitAnd for Bitboard {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAnd<Bitboard> for &Bitboard {
    type Output = Bitboard;
    fn bitand(self, rhs: Bitboard) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl<I: SquareIndexable> From<I> for Bitboard {
    fn from(square_index: I) -> Self {
        Self(square_index.shift())
    }
}

impl<I: SquareIndexable> From<&[I]> for Bitboard {
    fn from(square_index_slice: &[I]) -> Self {
        let mut bb = Bitboard::EMPTY;
        square_index_slice
            .iter()
            .for_each(|square| bb.set_square(square));
        bb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_square_indexable() {
        let a1 = Bitboard::from(Square::A1);
        let a2 = Bitboard::from(Square::A2);
        let a4 = Bitboard::from(Square::A4);
        let a8 = Bitboard::from(Square::A8);
        let d3 = Bitboard::from(Square::D3);
        let h8 = Bitboard::from(Square::H8);
        assert!(a1.has_square(Square::A1));
        assert!(a2.has_square(Square::A2));
        assert!(a4.has_square(Square::A4));
        assert!(a8.has_square(Square::A8));
        assert!(d3.has_square(Square::D3));
        assert!(h8.has_square(Square::H8));
        assert_eq!(a1.count_squares(), 1);
        assert_eq!(a2.count_squares(), 1);
        assert_eq!(a4.count_squares(), 1);
        assert_eq!(a8.count_squares(), 1);
        assert_eq!(d3.count_squares(), 1);
        assert_eq!(h8.count_squares(), 1);
    }

    #[test]
    fn from_square_indexable_slice() {
        let slice1 = vec![A1, A2, A3];
        let bb = Bitboard::from(slice1.as_slice());
        assert_eq!(bb.count_squares(), 3);
        assert!(bb.has_square(A1));
        assert!(bb.has_square(A2));
        assert!(bb.has_square(A3));
    }

    #[test]
    fn to_north_west_south_east() {
        let a1 = Bitboard::from(Square::A1);

        let a2 = a1.to_north();
        let b1 = a1.to_east();
        let empty1 = a1.to_south();
        let empty2 = a1.to_west();

        assert_eq!(a2.count_squares(), 1);
        assert_eq!(b1.count_squares(), 1);
        assert_eq!(empty1.count_squares(), 0);
        assert_eq!(empty2.count_squares(), 0);
        assert!(a2.has_square(Square::A2));
        assert!(b1.has_square(Square::B1));
        assert!(empty1 == Bitboard::EMPTY);
        assert!(empty2 == Bitboard::EMPTY);

        let empty3 = a1.to_south().to_east().to_east();
        let empty4 = a1.to_south().to_south().to_east();
        let empty5 = a1.to_south().to_south().to_west();
        let empty6 = a1.to_south().to_west().to_west();
        let empty7 = a1.to_north().to_west().to_west();
        let empty8 = a1.to_north().to_north().to_west();
        assert_eq!(empty3.count_squares(), 0);
        assert_eq!(empty4.count_squares(), 0);
        assert_eq!(empty5.count_squares(), 0);
        assert_eq!(empty6.count_squares(), 0);
        assert_eq!(empty7.count_squares(), 0);
        assert_eq!(empty8.count_squares(), 0);
        assert!(empty3 == Bitboard::EMPTY);
        assert!(empty4 == Bitboard::EMPTY);
        assert!(empty5 == Bitboard::EMPTY);
        assert!(empty6 == Bitboard::EMPTY);
        assert!(empty7 == Bitboard::EMPTY);
        assert!(empty8 == Bitboard::EMPTY);
    }
    #[test]
    fn to_east_west_wrapping() {
        // Test that a sideways move does not wrap to another rank.
        let a4 = Bitboard::from(Square::A4);
        let h4 = Bitboard::from(Square::H4);
        assert_eq!(a4.to_west(), Bitboard::EMPTY);
        assert_eq!(h4.to_east(), Bitboard::EMPTY);
    }

    #[test]
    fn bb_from_shifts() {
        let rank_1: u64 = 0x00000000000000FF;
        let rank_8: u64 = 0xFF00000000000000;
        assert_eq!(Bitboard::RANK_1.0, rank_1);
        assert_eq!(Bitboard::RANK_8.0, rank_8);
    }
}
