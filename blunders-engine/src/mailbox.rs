use crate::pieces::{Color, Piece, PieceType};
use std::default::Default;

/// Classic 8x8 square board representation of Chess board.
/// A1 = idx 0
/// B1 = idx 1
/// A2 = idx 8
/// H7 = idx 63
#[derive(Debug, Clone)]
pub struct Mailbox {
    board: [Option<Piece>; Mailbox::SIZE],
}

impl Mailbox {
    const NUM_FILES: usize = 8;
    const NUM_RANKS: usize = 8;
    const SIZE: usize = Self::NUM_FILES * Self::NUM_RANKS;

    pub fn new() -> Self {
        Self::default()
    }

    // Create Empty Mailbox with all values set to None.
    pub fn with_none() -> Self {
        Mailbox {
            board: [None; Mailbox::SIZE],
        }
    }
}

impl Default for Mailbox {
    /// default value is that of a standard starting chess position.
    fn default() -> Self {
        const PAWN: usize = 40;
        const REST: usize = 56;
        let mut mb = Mailbox::with_none();

        // Pawns
        for idx in 8..16usize {
            mb.board[idx] = Some(Piece::new(Color::White, PieceType::Pawn));
            mb.board[idx + PAWN] = Some(Piece::new(Color::Black, PieceType::Pawn));
        }

        // Rooks
        for &idx in &[0, 7usize] {
            mb.board[idx] = Some(Piece::new(Color::White, PieceType::Rook));
            mb.board[idx + REST] = Some(Piece::new(Color::Black, PieceType::Rook));
        }

        // Knights
        for &idx in &[1, 6usize] {
            mb.board[idx] = Some(Piece::new(Color::White, PieceType::Knight));
            mb.board[idx + REST] = Some(Piece::new(Color::Black, PieceType::Knight));
        }

        // Bishops
        for &idx in &[2, 5usize] {
            mb.board[idx] = Some(Piece::new(Color::White, PieceType::Bishop));
            mb.board[idx + REST] = Some(Piece::new(Color::Black, PieceType::Bishop));
        }

        mb.board[3] = Some(Piece::new(Color::White, PieceType::Queen));
        mb.board[3 + REST] = Some(Piece::new(Color::Black, PieceType::Queen));
        mb.board[4] = Some(Piece::new(Color::White, PieceType::King));
        mb.board[4 + REST] = Some(Piece::new(Color::Black, PieceType::King));

        mb
    }
}
