//! Piece-Centric representation of a chess board.

use std::fmt::{self, Display};
use std::ops::{Index, IndexMut};

use crate::bitboard::Bitboard;
use crate::boardrepr::Mailbox;
use crate::coretypes::{Color, Piece, PieceKind, Square};

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
}

impl Index<&Piece> for PieceSets {
    type Output = Bitboard;
    fn index(&self, piece: &Piece) -> &Self::Output {
        &self.pieces[piece.color as usize + piece.piece_kind as usize]
    }
}

impl IndexMut<&Piece> for PieceSets {
    fn index_mut(&mut self, piece: &Piece) -> &mut Self::Output {
        &mut self.pieces[piece.color as usize + piece.piece_kind as usize]
    }
}

impl Index<(Color, PieceKind)> for PieceSets {
    type Output = Bitboard;
    fn index(&self, (color, piece_kind): (Color, PieceKind)) -> &Self::Output {
        &self.pieces[color as usize + piece_kind as usize]
    }
}

impl Index<&(Color, PieceKind)> for PieceSets {
    type Output = Bitboard;
    fn index(&self, (color, piece_kind): &(Color, PieceKind)) -> &Self::Output {
        &self.pieces[(*color) as usize + (*piece_kind) as usize]
    }
}

impl IndexMut<&(Color, PieceKind)> for PieceSets {
    fn index_mut(&mut self, (color, piece_kind): &(Color, PieceKind)) -> &mut Self::Output {
        &mut self.pieces[(*color) as usize + (*piece_kind) as usize]
    }
}

/// Get a slice of all pieces of same color.
/// ```rust
/// # use blunders_engine::{coretypes::Color, boardrepr::PieceSets, bitboard::Bitboard};
/// # assert!((Color::White as usize) < Color::Black as usize);
/// let ps = PieceSets::start_position();
/// let w_slice = &ps[&Color::White];
/// let b_slice = &ps[&Color::Black];
/// assert_eq!(b_slice.len(), w_slice.len());
/// assert_eq!(b_slice.len(), 6);
/// ```
impl Index<&Color> for PieceSets {
    type Output = [Bitboard];
    fn index(&self, color: &Color) -> &Self::Output {
        match color {
            Color::White => &self.pieces[Color::White as usize..Color::Black as usize],
            Color::Black => &self.pieces[Color::Black as usize..Self::SIZE],
        }
    }
}

impl IndexMut<&Color> for PieceSets {
    fn index_mut(&mut self, color: &Color) -> &mut Self::Output {
        match color {
            Color::White => &mut self.pieces[Color::White as usize..Color::Black as usize],
            Color::Black => &mut self.pieces[Color::Black as usize..Self::SIZE],
        }
    }
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
    use crate::coretypes::PieceKind;
    use Square::*;
    #[test]
    fn piece_indexing() {
        let pieces = PieceSets::start_position();
        let w_king = &pieces[&Piece::new(Color::White, PieceKind::King)];
        assert_eq!(w_king.count_squares(), 1);
        assert!(w_king.has_square(E1));
    }

    #[test]
    fn color_indexing() {
        let pieces = PieceSets::start_position();
        let white_pieces = &pieces[&Color::White];
        let w_occupancy = white_pieces
            .iter()
            .fold(Bitboard::EMPTY, |acc, piece| acc | piece);
        assert_eq!(w_occupancy.count_squares(), 16);
        for &square in &[A1, B1, C1, D1, E1, F1, G1, H1] {
            assert!(w_occupancy.has_square(square));
        }
        for &square in &[A2, B2, C2, D2, E2, F2, G2, H2] {
            assert!(w_occupancy.has_square(square));
        }

        let black_pieces = &pieces[&Color::Black];
        let b_occupancy = black_pieces
            .iter()
            .fold(Bitboard::EMPTY, |acc, piece| acc | piece);
        assert_eq!(b_occupancy.count_squares(), 16);
        for &square in &[A7, B7, C7, D7, E7, F7, G7, H7] {
            assert!(b_occupancy.has_square(square));
        }
        for &square in &[A8, B8, C8, D8, E8, F8, G8, H8] {
            assert!(b_occupancy.has_square(square));
        }
    }
}
