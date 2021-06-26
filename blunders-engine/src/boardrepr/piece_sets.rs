//! Piece-Centric representation of a chess board.

use std::fmt::{self, Display};
use std::ops::{Index, IndexMut, Range};

use crate::bitboard::Bitboard;
use crate::boardrepr::Mailbox;
use crate::coretypes::{Color, Piece, PieceKind, Square};
use crate::coretypes::{Color::*, PieceKind::*};

// These offset impls are used to index their corresponding place in PieceSets.
// PieceSets contains an array with one index for each kind of piece.
// Optionally, Color and PieceKind discriminants could directly be these values
// however they will stay decoupled for now to decrease dependency on enum ordering
// until order is stabilized.

impl Color {
    /// Get the position of the start of the block for a color.
    /// There are 6 piece_kinds per color, so one should start at 0, and the other at 6.
    #[inline(always)]
    const fn offset_block(&self) -> usize {
        match self {
            White => 0,
            Black => 6,
        }
    }
}
impl PieceKind {
    /// Get the offset of a piece_kind within a block.
    /// Values must cover all numbers of [0, 1, 2, 3, 4, 5].
    #[inline(always)]
    const fn offset_pk(&self) -> usize {
        match self {
            King => 0,
            Pawn => 1,
            Knight => 2,
            Queen => 3,
            Rook => 4,
            Bishop => 5,
        }
    }
}
impl Piece {
    /// Get the completely qualified index for a piece.
    #[inline(always)]
    const fn offset(&self) -> usize {
        self.color.offset_block() + self.piece_kind.offset_pk()
    }
}

/// A Piece-Centric representation of pieces on a chessboard.
/// A Bitboard is used to encode the squares of each chess piece.
/// PieceSets indexes by piece to get squares, as opposed to Mailbox which
/// indexes by square to get a piece.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PieceSets {
    pieces: [Bitboard; Self::SIZE],
}

impl PieceSets {
    const SIZE: usize = 12; // 1 White, 1 Black BB for each piece type.

    /// Returns PieceSets with all Bitboards set to empty.
    pub fn new() -> Self {
        PieceSets {
            pieces: [Bitboard::EMPTY; Self::SIZE],
        }
    }

    /// Returns PieceSets arranged in starting chess position.
    pub fn start_position() -> Self {
        let mb: Mailbox = Mailbox::start_position();
        Self::from(&mb)
    }

    /// Return a bitboard representing the set of squares occupied by any piece.
    /// Note: Compiler can auto-vectorize, however looking at assembly on godbolt
    /// may be limited to avx128. Does not seem to use avx512 on supported cpus.
    pub fn occupied(&self) -> Bitboard {
        self.pieces.iter().fold(Bitboard::EMPTY, |acc, bb| acc | bb)
    }

    /// Return a bitboard representing the set of squares occupied by piece of color.
    pub fn color_occupied(&self, color: &Color) -> Bitboard {
        self[color].iter().fold(Bitboard::EMPTY, |acc, bb| acc | bb)
    }

    /// Returns pretty-printed chess board representation of Self.
    /// Uses Mailbox pretty.
    pub fn pretty(&self) -> String {
        Mailbox::from(self).pretty()
    }

    /// Returns true if all sets in self are disjoint (mutually exclusive).
    /// In other words, there is no more than 1 piece per square. If a square is in one set, it is in no other.
    /// PieceSets should be disjoint at all times.
    pub fn is_disjoint(&self) -> bool {
        let occupied_sum = self.occupied().count_squares();
        let individual_sum = self
            .pieces
            .iter()
            .fold(0, |acc, bb| acc + bb.count_squares());

        occupied_sum == individual_sum
    }

    /// Returns true if Self is valid.
    /// A valid PieceSets has the following properties:
    /// * Has a single king per side.
    /// * Each bitboard is disjoint (mutually exclusive) meaning a square cannot have more than one piece.
    pub fn is_valid(&self) -> bool {
        // Illegal if no White King.
        if self[(White, King)].count_squares() != 1 {
            return false;
        }
        // Illegal if no Black King.
        if self[(Black, King)].count_squares() != 1 {
            return false;
        }
        // Illegal if more than one piece per any square.
        if !self.is_disjoint() {
            return false;
        }
        // Illegal if any white pawn on first rank or black pawn on eighth rank.
        let w_pawns_first_rank = self[(White, Pawn)] & Bitboard::RANK_1;
        let b_pawns_eighth_rank = self[(Black, Pawn)] & Bitboard::RANK_8;
        if !w_pawns_first_rank.is_empty() || !b_pawns_eighth_rank.is_empty() {
            return false;
        }

        true
    }
}

impl Index<&Piece> for PieceSets {
    type Output = Bitboard;
    fn index(&self, piece: &Piece) -> &Self::Output {
        &self.pieces[piece.offset()]
    }
}

impl IndexMut<&Piece> for PieceSets {
    fn index_mut(&mut self, piece: &Piece) -> &mut Self::Output {
        &mut self.pieces[piece.offset()]
    }
}

impl Index<(Color, PieceKind)> for PieceSets {
    type Output = Bitboard;
    fn index(&self, (color, piece_kind): (Color, PieceKind)) -> &Self::Output {
        &self.pieces[color.offset_block() + piece_kind.offset_pk()]
    }
}

impl IndexMut<(Color, PieceKind)> for PieceSets {
    fn index_mut(&mut self, (color, piece_kind): (Color, PieceKind)) -> &mut Self::Output {
        &mut self.pieces[color.offset_block() + piece_kind.offset_pk()]
    }
}

impl Index<&(Color, PieceKind)> for PieceSets {
    type Output = Bitboard;
    fn index(&self, (color, piece_kind): &(Color, PieceKind)) -> &Self::Output {
        &self.pieces[color.offset_block() + piece_kind.offset_pk()]
    }
}

impl IndexMut<&(Color, PieceKind)> for PieceSets {
    fn index_mut(&mut self, (color, piece_kind): &(Color, PieceKind)) -> &mut Self::Output {
        &mut self.pieces[color.offset_block() + piece_kind.offset_pk()]
    }
}

/// Get a slice of all pieces of same color.
/// ```rust
/// # use blunders_engine::{coretypes::Color, boardrepr::PieceSets, bitboard::Bitboard};
/// let ps = PieceSets::start_position();
/// let w_slice = &ps[&Color::White];
/// let b_slice = &ps[&Color::Black];
/// assert_eq!(b_slice.len(), w_slice.len());
/// assert_eq!(b_slice.len(), 6);
/// ```
impl Index<&Color> for PieceSets {
    type Output = [Bitboard];
    fn index(&self, color: &Color) -> &Self::Output {
        const RANGES: (Range<usize>, Range<usize>) = color_ranges();
        match color {
            White => &self.pieces[RANGES.0],
            Black => &self.pieces[RANGES.1],
        }
    }
}

impl IndexMut<&Color> for PieceSets {
    fn index_mut(&mut self, color: &Color) -> &mut Self::Output {
        const RANGES: (Range<usize>, Range<usize>) = color_ranges();
        match color {
            White => &mut self.pieces[RANGES.0],
            Black => &mut self.pieces[RANGES.1],
        }
    }
}

impl Index<Color> for PieceSets {
    type Output = [Bitboard];
    fn index(&self, color: Color) -> &Self::Output {
        const RANGES: (Range<usize>, Range<usize>) = color_ranges();
        match color {
            White => &self.pieces[RANGES.0],
            Black => &self.pieces[RANGES.1],
        }
    }
}

impl IndexMut<Color> for PieceSets {
    fn index_mut(&mut self, color: Color) -> &mut Self::Output {
        const RANGES: (Range<usize>, Range<usize>) = color_ranges();
        match color {
            White => &mut self.pieces[RANGES.0],
            Black => &mut self.pieces[RANGES.1],
        }
    }
}

/// Used in 4 Index<Color> traits above to get correct ranges to represent each color's block.
const fn color_ranges() -> (Range<usize>, Range<usize>) {
    const W_RANGE: Range<usize> = match White.offset_block() < Black.offset_block() {
        true => White.offset_block()..Black.offset_block(),
        false => White.offset_block()..PieceSets::SIZE,
    };
    const B_RANGE: Range<usize> = match White.offset_block() < Black.offset_block() {
        true => Black.offset_block()..PieceSets::SIZE,
        false => Black.offset_block()..White.offset_block(),
    };
    (W_RANGE, B_RANGE)
}

impl From<&Mailbox> for PieceSets {
    fn from(mb: &Mailbox) -> Self {
        let mut pieces = Self::new();

        for square in Square::iter() {
            if let Some(ref piece) = mb[square] {
                pieces[piece].set_square(square);
            }
        }
        pieces
    }
}

/// Defaults to standard chess piece starting positions.
impl Default for PieceSets {
    fn default() -> Self {
        Self::start_position()
    }
}

impl Display for PieceSets {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Square::*;
    #[test]
    fn piece_indexing() {
        let pieces = PieceSets::start_position();
        let w_king = &pieces[&Piece::new(White, King)];
        assert_eq!(w_king.count_squares(), 1);
        assert!(w_king.has_square(E1));
    }

    #[test]
    fn color_indexing() {
        let pieces = PieceSets::start_position();
        let white_pieces = &pieces[White];
        let w_occupancy = white_pieces
            .iter()
            .fold(Bitboard::EMPTY, |acc, piece| acc | piece);
        assert_eq!(w_occupancy.count_squares(), 16);
        for square in [A1, B1, C1, D1, E1, F1, G1, H1] {
            assert!(w_occupancy.has_square(&square));
        }
        for square in [A2, B2, C2, D2, E2, F2, G2, H2] {
            assert!(w_occupancy.has_square(&square));
        }

        let black_pieces = &pieces[Black];
        let b_occupancy = black_pieces
            .iter()
            .fold(Bitboard::EMPTY, |acc, piece| acc | piece);
        assert_eq!(b_occupancy.count_squares(), 16);
        for square in [A7, B7, C7, D7, E7, F7, G7, H7] {
            assert!(b_occupancy.has_square(&square));
        }
        for square in [A8, B8, C8, D8, E8, F8, G8, H8] {
            assert!(b_occupancy.has_square(&square));
        }
    }

    #[test]
    fn check_is_valid() {
        let mut set = PieceSets::start_position();
        assert!(set.is_valid());

        set[(White, Pawn)].set_square(H8);
        assert!(!set.is_valid());
    }
}
