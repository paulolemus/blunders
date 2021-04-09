use std::convert::TryFrom;
use std::fmt::{self, Display};
use std::ops::{BitOr, Not};
use std::str::FromStr;

///
/// Constants
///
pub const NUM_FILES: usize = 8; // A, B, C, D, E, F, G, H
pub const NUM_RANKS: usize = 8; // 1, 2, 3, 4, 5, 6, 7, 8

///
/// Structures
///

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PieceKind {
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
    piece_kind: PieceKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Castling(u8);

/// Castling Enum constants.
impl Castling {
    pub const W_KING: Castling = Castling(0b00000001);
    pub const W_QUEEN: Castling = Castling(0b00000010);
    pub const B_KING: Castling = Castling(0b00000100);
    pub const B_QUEEN: Castling = Castling(0b00001000);
    pub const W_SIDE: Castling = Castling(Self::W_KING.0 | Self::W_QUEEN.0);
    pub const B_SIDE: Castling = Castling(Self::B_KING.0 | Self::B_QUEEN.0);
    pub const ALL: Castling = Castling(Self::W_SIDE.0 | Self::B_SIDE.0);
    pub const NONE: Castling = Castling(0u8);
}

///
/// Implementations
///

impl Color {
    /// FEN compliant conversion.
    pub const fn to_char(&self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }
}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl From<&Color> for char {
    fn from(color: &Color) -> Self {
        color.to_char()
    }
}

impl TryFrom<char> for Color {
    type Error = &'static str;
    fn try_from(ch: char) -> Result<Self, Self::Error> {
        match ch {
            'w' => Ok(Color::White),
            'b' => Ok(Color::Black),
            _ => Err("char is not w|b"),
        }
    }
}

impl PieceKind {
    /// FEN compliant conversion, defaults as white pieces.
    pub const fn to_char(&self) -> char {
        match self {
            PieceKind::Pawn => 'P',
            PieceKind::Rook => 'R',
            PieceKind::Knight => 'N',
            PieceKind::Bishop => 'B',
            PieceKind::Queen => 'Q',
            PieceKind::King => 'K',
        }
    }
}

impl Piece {
    pub fn new(color: Color, piece_kind: PieceKind) -> Self {
        Piece { color, piece_kind }
    }
    /// Immutable Getters.
    pub fn color(&self) -> &Color {
        &self.color
    }
    pub fn piece_kind(&self) -> &PieceKind {
        &self.piece_kind
    }

    pub fn to_char(&self) -> char {
        match self.color {
            Color::White => self.piece_kind.to_char(),
            Color::Black => self.piece_kind.to_char().to_ascii_lowercase(),
        }
    }
}

// Convert &Piece into a char.
impl From<&Piece> for char {
    fn from(piece: &Piece) -> Self {
        piece.to_char()
    }
}

// Try to convert char into a Piece.
impl TryFrom<char> for Piece {
    type Error = &'static str;
    fn try_from(value: char) -> Result<Self, Self::Error> {
        let color = match value.is_ascii_uppercase() {
            true => Color::White,
            false => Color::Black,
        };
        let piece_kind = match value.to_ascii_uppercase() {
            'P' => PieceKind::Pawn,
            'R' => PieceKind::Rook,
            'N' => PieceKind::Knight,
            'B' => PieceKind::Bishop,
            'Q' => PieceKind::Queen,
            'K' => PieceKind::King,
            _ => return Err("char is not in PRNBQKprnbqk"),
        };
        Ok(Piece { color, piece_kind })
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", char::from(self))
    }
}

impl Castling {
    /// Returns true if Castling mask has all of provided bits.
    pub fn has(&self, rights: Castling) -> bool {
        assert!(Self::is_mask_valid(rights));
        self.0 & rights.0 == rights.0
    }

    /// Returns true if there are no castling rights.
    pub fn is_none(&self) -> bool {
        self.0 == 0u8
    }

    /// Set given bits to '1' on Castling mask.
    pub fn add(&mut self, rights: Castling) {
        assert!(Self::is_mask_valid(rights));
        self.0 |= rights.0;
    }

    /// Set given bits to '0' on Castling mask.
    pub fn remove(&mut self, rights: Castling) {
        assert!(Self::is_mask_valid(rights));
        self.0 &= !rights.0;
    }

    fn is_mask_valid(rights: Castling) -> bool {
        rights.0 <= Self::ALL.0
    }
}

impl Default for Castling {
    fn default() -> Self {
        Self::ALL
    }
}

impl BitOr for Castling {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Display for Castling {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut castling_str = String::with_capacity(4);

        if self.is_none() {
            castling_str.push('-');
        } else {
            if self.has(Self::W_KING) {
                castling_str.push('K');
            }
            if self.has(Self::W_QUEEN) {
                castling_str.push('Q');
            }
            if self.has(Self::B_KING) {
                castling_str.push('k');
            }
            if self.has(Self::B_QUEEN) {
                castling_str.push('q');
            }
        }
        f.write_str(&castling_str)
    }
}

///Castling ::= '-' | ['K'] ['Q'] ['k'] ['q']
impl FromStr for Castling {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars().take(4);
        let mut castling_rights = Castling::NONE;

        // First character is either '-' or in KQkq.
        match chars.next().ok_or("No characters")? {
            '-' => return Ok(castling_rights),
            'K' => castling_rights.add(Self::W_KING),
            'Q' => castling_rights.add(Self::W_QUEEN),
            'k' => castling_rights.add(Self::B_KING),
            'q' => castling_rights.add(Self::B_QUEEN),
            _ => return Err("First char not of -KQkq"),
        };

        // castling_rights is now valid, add rest of rights or return early.
        for ch in chars {
            match ch {
                'K' => castling_rights.add(Self::W_KING),
                'Q' => castling_rights.add(Self::W_QUEEN),
                'k' => castling_rights.add(Self::B_KING),
                'q' => castling_rights.add(Self::B_QUEEN),
                _ => return Ok(castling_rights),
            };
        }
        Ok(castling_rights)
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

    #[test]
    fn castling_logical_ops() {
        let mut cr = Castling::default();
        assert!(cr.has(Castling::ALL));
        assert!(cr.has(Castling::W_KING));
        assert!(cr.has(Castling::W_QUEEN));
        assert!(cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(cr.has(Castling::W_SIDE));
        assert!(cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.remove(Castling::W_KING);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(cr.has(Castling::W_QUEEN));
        assert!(cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.remove(Castling::W_QUEEN);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(!cr.has(Castling::W_QUEEN));
        assert!(cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.remove(Castling::B_KING);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(!cr.has(Castling::W_QUEEN));
        assert!(!cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(!cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.remove(Castling::B_QUEEN);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(!cr.has(Castling::W_QUEEN));
        assert!(!cr.has(Castling::B_KING));
        assert!(!cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(!cr.has(Castling::B_SIDE));
        assert!(cr.is_none());
    }

    #[test]
    fn castling_to_from_string() {
        let valid_none = "-";
        let valid_w_king = "K";
        let valid_kings = "Kk";
        let valid_all = "KQkq";
        let valid_queens = "Qq";
        let invalid_empty = "";
        let invalid_char = "x";

        let none = Castling::from_str(valid_none);
        let w_king = Castling::from_str(valid_w_king);
        let kings = Castling::from_str(valid_kings);
        let all = Castling::from_str(valid_all);
        let queens = Castling::from_str(valid_queens);
        let empty = Castling::from_str(invalid_empty);
        let ch = Castling::from_str(invalid_char);

        assert_eq!(none.unwrap(), Castling::NONE);
        assert_eq!(w_king.unwrap(), Castling::W_KING);
        assert_eq!(kings.unwrap(), Castling::W_KING | Castling::B_KING);
        assert_eq!(all.unwrap(), Castling::ALL);
        assert_eq!(queens.unwrap(), Castling::W_QUEEN | Castling::B_QUEEN);
        assert!(empty.is_err());
        assert!(ch.is_err());
    }
}
