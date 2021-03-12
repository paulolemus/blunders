use std::ops::Not;

pub const NUM_FILES: usize = 8;
pub const NUM_RANKS: usize = 8;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PieceType {
    Pawn,
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Piece {
    color: Color,
    piece: PieceType,
}

pub struct Square {}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl Piece {
    pub fn new(color: Color, piece: PieceType) -> Self {
        Piece { color, piece }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_not_color() {
        assert_eq!(!Color::White, Color::Black);
        assert_eq!(!Color::Black, Color::White);
    }
}
