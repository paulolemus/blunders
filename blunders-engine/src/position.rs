//! position.rs
//! Holds Position struct, which is the most important data
//! structure for the engine.
//! It holds a Chess position, and methods used for assessing
//! itself.

use crate::bitboard::Bitboard;
use crate::coretypes::{Castling, Color, MoveCount, Square};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    pieces: [Bitboard; Self::BB_ARRAY_SIZE],
    side_to_move: Color,
    castling: Castling,
    en_passant: Option<Square>,
    halfmoves: MoveCount,
    fullmoves: MoveCount,
}

impl Position {
    const BB_ARRAY_SIZE: usize = 12; // 1 White, 1 Black BB for each piece type.
    const WHITE_OFFSET: usize = 0; // First 6 idx of pieces are white pieces.
    const BLACK_OFFSET: usize = 6; // Last 6 idx of pieces are black pieces.
}

//impl From<Fen> for Position {
//    fn from(fen: Fen) -> Self {
//        let pieces =
//        Self {
//            pieces,
//            side_to_move: *fen.side_to_move(),
//            castling: *fen.castling(),
//            en_passant: *fen.en_passant(),
//            halfmoves: *fen.halfmove_clock(),
//            fullmoves: *fen.fullmove_number(),
//        }
//    }
//}

impl Color {
    const fn offset(&self) -> usize {
        match self {
            Self::White => Position::WHITE_OFFSET,
            Self::Black => Position::BLACK_OFFSET,
        }
    }
}
