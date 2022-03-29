//! A general purpose way to efficiently encode data,
//! where each bit index of a 64-bit unsigned integer represents a chessboard square.
//!
//! Data Order:
//! * Little-Endian Rank-File mapping (LSR)
//! * A1 = least significant bit = 0b0 = 0
//! * B1 = 0b1 = 1
//! * C1 = 0b10 = 2
//! * A2 = 0b1000 = 8
//! * H8 = most significant bit = 0x8000000000000000
//!
//! Compass Rose Bit Shifting:
//! ```text
//! NoWe       North       NoEa
//!      +7     +8      +9
//! West -1      0      +1 East
//!      -9     -8      -7
//! SoWe       South       SoEa
//! ```
//!
//! Examples of data that may be represented with Bitboards:
//! * W/B King position
//! * W/B Queen positions
//! * W/B Rook positions
//! * W/B Bishop positions
//! * W/B Knight positions
//! * W/B Pawn positions
//! * Pawn Attack Pattern per square
//! * Knight Attack Pattern per square
//! * King Attack Pattern per square
//! * Sliding Attack Pattern per square
//! * Pass Pawns

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Not};

use crate::coretypes::{
    File, Rank, Square, Square::*, SquareIndexable, NUM_FILES, NUM_RANKS, NUM_SQUARES,
};

/// Alias for inner type of Bitboard. Useful for const evaluation.
pub type BitboardKind = u64;

/// Bitboard is a wrapper around a u64 integer, where each bit represents some or none
/// on its corresponding chess board square. It is used to encode a set of some arbitrary
/// homogenous data for an entire chess board.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct Bitboard(pub(crate) BitboardKind);

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
pub(crate) use bb_from_shifts;

/// Bitboard Constants
impl Bitboard {
    pub const EMPTY: Bitboard = Self(0x0);
    pub const BLACK_SQUARES: Bitboard = Self(0xAA55AA55AA55AA55);
    pub const WHITE_SQUARES: Bitboard = Self(!Self::BLACK_SQUARES.0);
    // Ranks
    pub const RANK_1: Bitboard = bb_from_shifts!(A1, B1, C1, D1, E1, F1, G1, H1);
    pub const RANK_2: Bitboard = bb_from_shifts!(A2, B2, C2, D2, E2, F2, G2, H2);
    pub const RANK_3: Bitboard = bb_from_shifts!(A3, B3, C3, D3, E3, F3, G3, H3);
    pub const RANK_4: Bitboard = bb_from_shifts!(A4, B4, C4, D4, E4, F4, G4, H4);
    pub const RANK_5: Bitboard = bb_from_shifts!(A5, B5, C5, D5, E5, F5, G5, H5);
    pub const RANK_6: Bitboard = bb_from_shifts!(A6, B6, C6, D6, E6, F6, G6, H6);
    pub const RANK_7: Bitboard = bb_from_shifts!(A7, B7, C7, D7, E7, F7, G7, H7);
    pub const RANK_8: Bitboard = bb_from_shifts!(A8, B8, C8, D8, E8, F8, G8, H8);
    // Files
    pub const FILE_A: Bitboard = bb_from_shifts!(A1, A2, A3, A4, A5, A6, A7, A8);
    pub const FILE_B: Bitboard = bb_from_shifts!(B1, B2, B3, B4, B5, B6, B7, B8);
    pub const FILE_C: Bitboard = bb_from_shifts!(C1, C2, C3, C4, C5, C6, C7, C8);
    pub const FILE_D: Bitboard = bb_from_shifts!(D1, D2, D3, D4, D5, D6, D7, D8);
    pub const FILE_E: Bitboard = bb_from_shifts!(E1, E2, E3, E4, E5, E6, E7, E8);
    pub const FILE_F: Bitboard = bb_from_shifts!(F1, F2, F3, F4, F5, F6, F7, F8);
    pub const FILE_G: Bitboard = bb_from_shifts!(G1, G2, G3, G4, G5, G6, G7, G8);
    pub const FILE_H: Bitboard = bb_from_shifts!(H1, H2, H3, H4, H5, H6, H7, H8);
    // Squares between king and kingside rook. Useful for checking castling.
    pub const KINGSIDE_BETWEEN: Bitboard = bb_from_shifts!(F1, G1, F8, G8);
    pub const QUEENSIDE_BETWEEN: Bitboard = bb_from_shifts!(B1, C1, D1, B8, C8, D8);
    // Squares that king passes through during castling.
    pub const KINGSIDE_PASS: Bitboard = bb_from_shifts!(E1, F1, G1, E8, F8, G8);
    pub const QUEENSIDE_PASS: Bitboard = bb_from_shifts!(C1, D1, E1, C8, D8, E8);
}

impl Bitboard {
    /// Returns true if there are no squares in self, false otherwise.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns number of elements (squares) in bitboard.
    /// Equivalent to number of bits in binary representation that are '1'.
    /// The limits of the return value is 0 <= len <= 64.
    /// When compiled for targets with BMI1 popcnt instruction, should resolve to a single instruction.
    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns true if index is populated.
    #[inline(always)]
    pub fn has_square<I: SquareIndexable>(&self, idx: I) -> bool {
        self.0 & idx.shift() != 0
    }
    /// Sets bit index to 1.
    #[inline(always)]
    pub fn set_square<I: SquareIndexable>(&mut self, idx: I) {
        self.0 |= idx.shift();
    }
    /// Sets bit index to 0.
    #[inline(always)]
    pub fn clear_square<I: SquareIndexable>(&mut self, idx: I) {
        self.0 &= !idx.shift();
    }
    /// Toggles bit index. 0 -> 1, 1 -> 0.
    #[inline(always)]
    pub fn toggle_square<I: SquareIndexable>(&mut self, idx: I) {
        self.0 ^= idx.shift();
    }

    /// Clears squares including and above target square.
    /// TODO:
    /// There is a BMI2 instruction BZHI to zero high hits starting at position,
    /// however the instruction is not used with &mut self, only self.
    /// Figure out how to get get compiler to use BZHI.
    #[inline(always)]
    pub fn clear_square_and_above<I: SquareIndexable>(&mut self, idx: I) {
        self.0 &= idx.shift() - 1;
    }

    /// Clears squares including and below target square.
    /// TODO:
    /// Find a BMI instruction, if applicable. Maybe BLSMSK.
    pub fn clear_square_and_below<I: SquareIndexable>(&mut self, idx: I) {
        self.0 &= !(idx.shift() ^ (idx.shift() - 1));
    }

    /// Clears the lowest square from self. If there are no squares, does nothing.
    /// When compiled for targets that support BMI1 BLSR (reset lowest set bit),
    /// should resolve to a single instruction and a move.
    #[inline(always)]
    pub fn clear_lowest_square(&mut self) {
        // self.0 &= self.0 - 1 is same as below, but wrapping_sub doesn't panic for 0.
        self.0 &= self.0.wrapping_sub(1);
    }

    /// Returns the lowest square that exists in bitboard, or None if bitboard has no squares.
    #[inline(always)]
    pub fn get_lowest_square(&self) -> Option<Square> {
        Square::try_from(self.0.trailing_zeros() as u8).ok()
    }

    /// Remove all squares in other from self.
    #[inline(always)]
    pub fn remove(&mut self, other: Bitboard) {
        *self &= !other
    }

    /// Returns true if other is a subset of self.
    /// If all squares of other are in self, then other is a subset of self.
    #[inline(always)]
    pub const fn contains(&self, other: Bitboard) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns true if self has any squares that are in other.
    /// In other words, if there is any overlap, return true.
    #[inline(always)]
    pub const fn has_any(&self, other: Bitboard) -> bool {
        self.0 & other.0 != Self::EMPTY.0
    }

    /// Returns new Bitboard with all squares shifted 1 square north (ex: D4 -> D5).
    #[inline(always)]
    pub const fn to_north(&self) -> Self {
        Self(self.0 << 8)
    }
    /// Returns new Bitboard with all squares shifted 1 square south (ex: D4 -> D3).
    #[inline(always)]
    pub const fn to_south(&self) -> Self {
        Self(self.0 >> 8)
    }
    /// Returns new Bitboard with all squares shifted 1 square east (ex: D4 -> E4).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_A.
    #[inline(always)]
    pub const fn to_east(&self) -> Self {
        Self((self.0 << 1) & !Self::FILE_A.0)
    }
    /// Returns new Bitboard with all squares shifted 1 square west (ex: D4 -> C4).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_H.
    #[inline(always)]
    pub const fn to_west(&self) -> Self {
        Self((self.0 >> 1) & !Self::FILE_H.0)
    }

    /// Returns new Bitboard with all squares shifted 1 square north east (ex: D4 -> E5).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_A.
    #[inline(always)]
    pub const fn to_north_east(&self) -> Self {
        Self((self.0 << 9) & !Self::FILE_A.0)
    }
    /// Returns new Bitboard with all squares shifted 1 square north west (ex: D4 -> C5).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_H.
    #[inline(always)]
    pub const fn to_north_west(&self) -> Self {
        Self((self.0 << 7) & !Self::FILE_H.0)
    }
    /// Returns new Bitboard with all squares shifted 1 square south east (ex: D4 -> E3).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_A.
    #[inline(always)]
    pub const fn to_south_east(&self) -> Self {
        Self((self.0 >> 7) & !Self::FILE_A.0)
    }
    /// Returns new Bitboard with all squares shifted 1 square south west (ex: D4 -> C3).
    /// To prevent wrapping of bit to other rank, bits are removed on FILE_H.
    #[inline(always)]
    pub const fn to_south_west(&self) -> Self {
        Self((self.0 >> 9) & !Self::FILE_H.0)
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
        let mut bits = *self;
        let num_ones = self.len();
        let mut vec = Vec::with_capacity(num_ones);

        for _ in 0..num_ones {
            let square_value = bits.0.trailing_zeros() as u8;
            bits.clear_lowest_square();
            let square = Square::try_from(square_value);
            debug_assert!(square_value < 64u8 && square.is_ok());
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

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl BitAnd for Bitboard {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0
    }
}

impl BitXor for Bitboard {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
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

impl From<File> for Bitboard {
    fn from(file: File) -> Self {
        use File::*;
        match file {
            A => Self::FILE_A,
            B => Self::FILE_B,
            C => Self::FILE_C,
            D => Self::FILE_D,
            E => Self::FILE_E,
            F => Self::FILE_F,
            G => Self::FILE_G,
            H => Self::FILE_H,
        }
    }
}

impl From<Rank> for Bitboard {
    fn from(rank: Rank) -> Self {
        use Rank::*;
        match rank {
            R1 => Self::RANK_1,
            R2 => Self::RANK_2,
            R3 => Self::RANK_3,
            R4 => Self::RANK_4,
            R5 => Self::RANK_5,
            R6 => Self::RANK_6,
            R7 => Self::RANK_7,
            R8 => Self::RANK_8,
        }
    }
}

/// Iterator type that yields each square in a bitboard through efficient generation.
pub struct BitboardSquareIterator {
    bb: Bitboard,
}

impl Iterator for BitboardSquareIterator {
    type Item = Square;
    fn next(&mut self) -> Option<Self::Item> {
        let maybe_square = self.bb.get_lowest_square();
        self.bb.clear_lowest_square();
        maybe_square
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.bb.len();
        (size, Some(size))
    }
}
impl ExactSizeIterator for BitboardSquareIterator {}

/// Allow the squares of a Bitboard to be iterated directly and cheaply.
impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardSquareIterator;
    fn into_iter(self) -> Self::IntoIter {
        BitboardSquareIterator { bb: self }
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Square::*;
        const RANK_SQUARES: [[Square; NUM_FILES]; NUM_RANKS] = [
            [A8, B8, C8, D8, E8, F8, G8, H8],
            [A7, B7, C7, D7, E7, F7, G7, H7],
            [A6, B6, C6, D6, E6, F6, G6, H6],
            [A5, B5, C5, D5, E5, F5, G5, H5],
            [A4, B4, C4, D4, E4, F4, G4, H4],
            [A3, B3, C3, D3, E3, F3, G3, H3],
            [A2, B2, C2, D2, E2, F2, G2, H2],
            [A1, B1, C1, D1, E1, F1, G1, H1],
        ];
        let mut buf = String::with_capacity(NUM_SQUARES + NUM_RANKS);
        for rank in RANK_SQUARES {
            for square in rank {
                buf.push(if self.has_square(square) { '1' } else { '.' });
            }
            buf.push('\n');
        }

        f.write_str(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_square_indexable() {
        for square in [A1, A2, A4, A8, D3, F6, G7, H1, H8] {
            let bb = Bitboard::from(square);
            assert!(bb.has_square(square));
            assert_eq!(bb.len(), 1);
        }
    }

    #[test]
    fn from_square_indexable_slice() {
        let slice1 = vec![A1, A2, A3];
        let bb = Bitboard::from(slice1.as_slice());
        assert_eq!(bb.len(), 3);
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

        assert_eq!(a2.len(), 1);
        assert_eq!(b1.len(), 1);
        assert_eq!(empty1.len(), 0);
        assert_eq!(empty2.len(), 0);
        assert!(a2.has_square(Square::A2));
        assert!(b1.has_square(Square::B1));
        assert_eq!(empty1, Bitboard::EMPTY);
        assert_eq!(empty2, Bitboard::EMPTY);

        let empty_boards = [
            a1.to_south().to_east().to_east(),
            a1.to_south().to_south().to_east(),
            a1.to_south().to_south().to_west(),
            a1.to_south().to_west().to_west(),
            a1.to_north().to_west().to_west(),
            a1.to_north().to_north().to_west(),
        ];
        for empty_board in empty_boards {
            assert_eq!(empty_board.len(), 0);
            assert_eq!(empty_board, Bitboard::EMPTY);
            assert!(empty_board.is_empty());
        }
    }
    #[test]
    fn to_east_west_wrapping() {
        // Test that a sideways move does not wrap to another rank.
        {
            let right_side_squares = [H1, H2, H3, H4, H5, H6, H7, H8];
            for right_square in right_side_squares {
                let bb = Bitboard::from(right_square);
                assert_eq!(bb.to_east(), Bitboard::EMPTY);
                assert_eq!(bb.to_north_east(), Bitboard::EMPTY);
                assert_eq!(bb.to_south_east(), Bitboard::EMPTY);
            }
        }

        let left_side_squares = [A1, A2, A3, A4, A5, A6, A7, A8];
        for left_square in left_side_squares {
            let bb = Bitboard::from(left_square);
            assert_eq!(bb.to_west(), Bitboard::EMPTY);
            assert_eq!(bb.to_north_west(), Bitboard::EMPTY);
            assert_eq!(bb.to_south_west(), Bitboard::EMPTY);
        }
    }

    #[test]
    fn bb_from_shifts() {
        let rank_1: u64 = 0x00000000000000FF;
        let rank_8: u64 = 0xFF00000000000000;
        assert_eq!(Bitboard::RANK_1.0, rank_1);
        assert_eq!(Bitboard::RANK_8.0, rank_8);
    }

    #[test]
    fn iterate_bitboard() {
        let bb = Bitboard::FILE_A;
        let vec: Vec<Square> = bb.into_iter().collect();
        for square in [A1, A2, A3, A4, A5, A6, A7, A8] {
            assert!(vec.contains(&square));
        }

        let mut empty = Bitboard::EMPTY.into_iter();
        assert_eq!(empty.len(), 0);
        assert_eq!(empty.next(), None);

        let empty_vec: Vec<Square> = empty.into_iter().collect();
        assert_eq!(empty_vec.len(), 0);
    }

    #[test]
    fn display_bitboard() {
        let bb = Bitboard::RANK_1 | Bitboard::FILE_A | Bitboard::from(H8);
        println!("{bb}");
    }
}
