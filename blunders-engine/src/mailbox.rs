use crate::coretypes::{Color, Piece, PieceType, NUM_FILES, NUM_RANKS};
use std::default::Default;
use std::ops::{Index, IndexMut};

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
    const SIZE: usize = NUM_FILES * NUM_RANKS;

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

/// default value is that of a standard starting chess position.
impl Default for Mailbox {
    fn default() -> Self {
        const PAWN: usize = 40;
        const REST: usize = 56;
        let mut mb = Mailbox::with_none();

        // Pawns
        for idx in 8..16usize {
            mb[idx] = Some(Piece::new(Color::White, PieceType::Pawn));
            mb[idx + PAWN] = Some(Piece::new(Color::Black, PieceType::Pawn));
        }

        // Rooks
        for &idx in &[0, 7usize] {
            mb[idx] = Some(Piece::new(Color::White, PieceType::Rook));
            mb[idx + REST] = Some(Piece::new(Color::Black, PieceType::Rook));
        }

        // Knights
        for &idx in &[1, 6usize] {
            mb[idx] = Some(Piece::new(Color::White, PieceType::Knight));
            mb[idx + REST] = Some(Piece::new(Color::Black, PieceType::Knight));
        }

        // Bishops
        for &idx in &[2, 5usize] {
            mb[idx] = Some(Piece::new(Color::White, PieceType::Bishop));
            mb[idx + REST] = Some(Piece::new(Color::Black, PieceType::Bishop));
        }

        mb[3] = Some(Piece::new(Color::White, PieceType::Queen));
        mb[3 + REST] = Some(Piece::new(Color::Black, PieceType::Queen));
        mb[4] = Some(Piece::new(Color::White, PieceType::King));
        mb[4 + REST] = Some(Piece::new(Color::Black, PieceType::King));

        mb
    }
}
