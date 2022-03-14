//! A [mailbox](https://www.chessprogramming.org/Mailbox) is a square-centric
//! representation of a chess board.
//!
//! A Mailbox is an array of size Files x Ranks where each index may contain a
//! chess piece or be empty.

use std::fmt::{self, Display};
use std::ops::{Index, IndexMut};

use crate::boardrepr::PieceSets;
use crate::coretypes::{
    Color, Piece, PieceKind, Square, SquareIndexable, NUM_FILES, NUM_RANKS, NUM_SQUARES,
};

/// Classic 8x8 square board representation of Chess board.
/// Mailbox is Square-Centric, meaning it indexes by square to get a piece,
/// as opposed to a PieceSets which indexes by piece to get squares.
/// Index starts at A1.
/// A1 = idx 0
/// B1 = idx 1
/// A2 = idx 8
/// H7 = idx 63
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Mailbox {
    board: [Option<Piece>; Self::SIZE],
}

impl Mailbox {
    pub const FILES: usize = NUM_FILES;
    pub const RANKS: usize = NUM_RANKS;
    pub const SIZE: usize = NUM_SQUARES;

    /// Creates an empty Mailbox, where all squares are None.
    pub fn new() -> Self {
        Mailbox {
            board: [None; Mailbox::SIZE],
        }
    }

    /// Create Mailbox with pieces arranged in starting chess position.
    pub fn start_position() -> Self {
        use Color::*;
        use PieceKind::*;
        use Square::*;
        let mut mb = Self::new();

        mb[A1] = Some(Piece::new(White, Rook));
        mb[B1] = Some(Piece::new(White, Knight));
        mb[C1] = Some(Piece::new(White, Bishop));
        mb[D1] = Some(Piece::new(White, Queen));
        mb[E1] = Some(Piece::new(White, King));
        mb[F1] = Some(Piece::new(White, Bishop));
        mb[G1] = Some(Piece::new(White, Knight));
        mb[H1] = Some(Piece::new(White, Rook));
        for square in [A2, B2, C2, D2, E2, F2, G2, H2] {
            mb[square] = Some(Piece::new(White, Pawn));
        }
        mb[A8] = Some(Piece::new(Black, Rook));
        mb[B8] = Some(Piece::new(Black, Knight));
        mb[C8] = Some(Piece::new(Black, Bishop));
        mb[D8] = Some(Piece::new(Black, Queen));
        mb[E8] = Some(Piece::new(Black, King));
        mb[F8] = Some(Piece::new(Black, Bishop));
        mb[G8] = Some(Piece::new(Black, Knight));
        mb[H8] = Some(Piece::new(Black, Rook));
        for square in [A7, B7, C7, D7, E7, F7, G7, H7] {
            mb[square] = Some(Piece::new(Black, Pawn));
        }

        mb
    }

    pub fn board(&self) -> &[Option<Piece>; Self::SIZE] {
        &self.board
    }

    /// Returns pretty-printed chess board representation of Self.
    /// The chess board has borders and file/rank indicators.
    pub fn pretty(&self) -> String {
        const RANK_SEP: &'static str = "+---+---+---+---+---+---+---+---+\n";
        let mut pretty = String::with_capacity(626); // Measured in test.

        pretty.push_str(RANK_SEP);
        for rank in (0..Self::RANKS).rev() {
            pretty.push_str("| ");

            for file in 0..Self::FILES {
                pretty.push(match self[rank * Self::RANKS + file] {
                    Some(piece) => char::from(piece),
                    None => ' ',
                });
                pretty.push_str(" | ");
            }
            pretty.push_str(&(rank + 1).to_string());
            pretty.push('\n');
            pretty.push_str(RANK_SEP);
        }
        pretty.push_str("  a   b   c   d   e   f   g   h\n");

        pretty
    }
}

/// Be careful with accessing with usize, as usize value out of bounds will panic.
impl Index<usize> for Mailbox {
    type Output = Option<Piece>;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.board[idx]
    }
}

impl IndexMut<usize> for Mailbox {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.board[idx]
    }
}

impl<I: SquareIndexable> Index<I> for Mailbox {
    type Output = Option<Piece>;
    fn index(&self, idx: I) -> &Self::Output {
        &self.board[idx.idx()]
    }
}

impl<I: SquareIndexable> IndexMut<I> for Mailbox {
    fn index_mut(&mut self, idx: I) -> &mut Self::Output {
        &mut self.board[idx.idx()]
    }
}

impl From<&PieceSets> for Mailbox {
    fn from(pieces: &PieceSets) -> Mailbox {
        let mut mb = Mailbox::new();

        for color in Color::iter() {
            for piece_kind in PieceKind::iter() {
                let piece = Piece::new(color, piece_kind);
                pieces[&piece]
                    .into_iter()
                    .for_each(|square| mb[square] = Some(piece));
            }
        }
        mb
    }
}

/// Default value is that of a standard starting chess position.
impl Default for Mailbox {
    fn default() -> Self {
        Mailbox::start_position()
    }
}

impl Display for Mailbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_start_position_mailbox() {
        let mb = Mailbox::start_position();

        println!("{}", mb.pretty().len());
        println!("{mb}");
    }
}
