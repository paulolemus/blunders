//! mailbox.rs
//! A mailbox is a board-oriented representation of a chess board.
//! Mailbox is an array of size Files x Ranks where each index may
//! contain a chess piece or be empty.

use crate::coretypes::{Color, Piece, PieceKind, NUM_FILES, NUM_RANKS};
use crate::fen::Fen;
use std::fmt;
use std::ops;

/// Classic 8x8 square board representation of Chess board.
/// Index starts at A1.
/// A1 = idx 0
/// B1 = idx 1
/// A2 = idx 8
/// H7 = idx 63
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Mailbox {
    board: [Option<Piece>; Mailbox::SIZE],
}

impl Mailbox {
    const FILES: usize = NUM_FILES;
    const RANKS: usize = NUM_RANKS;
    const SIZE: usize = NUM_FILES * NUM_RANKS;

    /// Same as Default::default().
    pub fn new() -> Self {
        Self::default()
    }

    /// Create Empty Mailbox with all values set to None.
    pub fn with_none() -> Self {
        Mailbox {
            board: [None; Mailbox::SIZE],
        }
    }

    pub fn board(&self) -> &[Option<Piece>; Self::SIZE] {
        &self.board
    }
}

impl ops::Index<usize> for Mailbox {
    type Output = Option<Piece>;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.board[idx]
    }
}

impl ops::IndexMut<usize> for Mailbox {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.board[idx]
    }
}

/// default value is that of a standard starting chess position.
impl Default for Mailbox {
    fn default() -> Self {
        const PAWN: usize = 40;
        const REST: usize = 56;
        let mut mb = Mailbox::with_none();

        // Pawns
        for idx in 8..16usize {
            mb[idx] = Some(Piece::new(Color::White, PieceKind::Pawn));
            mb[idx + PAWN] = Some(Piece::new(Color::Black, PieceKind::Pawn));
        }

        // Rooks
        for &idx in &[0, 7usize] {
            mb[idx] = Some(Piece::new(Color::White, PieceKind::Rook));
            mb[idx + REST] = Some(Piece::new(Color::Black, PieceKind::Rook));
        }

        // Knights
        for &idx in &[1, 6usize] {
            mb[idx] = Some(Piece::new(Color::White, PieceKind::Knight));
            mb[idx + REST] = Some(Piece::new(Color::Black, PieceKind::Knight));
        }

        // Bishops
        for &idx in &[2, 5usize] {
            mb[idx] = Some(Piece::new(Color::White, PieceKind::Bishop));
            mb[idx + REST] = Some(Piece::new(Color::Black, PieceKind::Bishop));
        }

        mb[3] = Some(Piece::new(Color::White, PieceKind::Queen));
        mb[3 + REST] = Some(Piece::new(Color::Black, PieceKind::Queen));
        mb[4] = Some(Piece::new(Color::White, PieceKind::King));
        mb[4 + REST] = Some(Piece::new(Color::Black, PieceKind::King));

        mb
    }
}

impl fmt::Display for Mailbox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::with_capacity(Self::SIZE + Self::RANKS);

        for rank in (0..Self::RANKS).rev() {
            for file in 0..Self::FILES {
                s.push(match self[rank * Self::RANKS + file] {
                    Some(ref piece) => char::from(piece),
                    None => ' ',
                });
            }
            s.push('\n');
        }
        s.pop();
        write!(f, "{}", s)
    }
}

//impl From<&Fen> for Mailbox {
//fn from(fen: &Fen) -> Self {

//}
//}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_default_mailbox() {
        const DEFAULT_STRING: &str =
            "rnbqkbnr\npppppppp\n        \n        \n        \n        \nPPPPPPPP\nRNBQKBNR";
        let mb = Mailbox::default();
        assert_eq!(mb.to_string(), DEFAULT_STRING);
        println!("{}", mb);
    }

    #[test]
    fn from_default_fen() {
        //let fen = Fen::default();
        //let mb = Mailbox::from(&fen);
        //assert_eq!(mb, Mailbox::default());
    }
}
