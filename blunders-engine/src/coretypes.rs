//! coretypes.rs
//! Types:
//! Color, PieceKind, Piece, Castling, File, Rank, Square, MoveCount

use std::convert::TryFrom;
use std::fmt::{self, Display, Write};
use std::mem::transmute; // unsafe
use std::ops::{BitOr, Not};
use std::str::FromStr;

///
/// Constants
///
pub const NUM_FILES: usize = 8; // A, B, C, D, E, F, G, H
pub const NUM_RANKS: usize = 8; // 1, 2, 3, 4, 5, 6, 7, 8

///
/// Data and Structures
///

/// Counter for half-move clock and full-moves.
pub type MoveCount = u32;

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
    King = 5,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Piece {
    color: Color,
    piece_kind: PieceKind,
}

/// Observe Castling rights for a position.
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[rustfmt::skip]
#[repr(u8)]
pub enum File {
    A, B, C, D, E, F, G, H = 7u8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[rustfmt::skip]
#[repr(u8)]
pub enum Rank {
    R1, R2, R3, R4, R5, R6, R7, R8 = 7u8,
}

/// Square
/// Every possible square on a chess board.
/// The order of enums is important, as `Square::A1 as u8` corresponds to
/// that Square's bit position in a bitboard.
/// WARNING: The exact ordering of enums is important for their discriminants.
///          Changing the discriminant of any variant is breaking.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[rustfmt::skip]
#[repr(u8)]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8 = 63u8,
}

/// Move
/// Long Algebraic form of moving a single chess piece.
/// A chess "half move", or "ply".
//pub struct Move {
//    piece_kind: PieceKind,
//    from: Square,
//    to: Square,
//    promotion: Option<PieceKind>,
//}

///
/// Traits
///

/// Indexable returns a number between 0-63 inclusive representing square
/// on a chess board in rank-file order.
/// WARNING: Returning values outside of 0-63 might cause panic.
pub trait Indexable {
    fn idx(&self) -> usize;
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

impl Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_char(self.into())
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

impl From<Piece> for char {
    fn from(piece: Piece) -> Self {
        piece.to_char()
    }
}
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
        f.write_char(self.into())
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

/// Castling ::= '-' | ['K'] ['Q'] ['k'] ['q']
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

impl File {
    pub const fn to_char(&self) -> char {
        match self {
            Self::A => 'a',
            Self::B => 'b',
            Self::C => 'c',
            Self::D => 'd',
            Self::E => 'e',
            Self::F => 'f',
            Self::G => 'g',
            Self::H => 'h',
        }
    }
}

impl Rank {
    pub const fn to_char(&self) -> char {
        match self {
            Self::R1 => '1',
            Self::R2 => '2',
            Self::R3 => '3',
            Self::R4 => '4',
            Self::R5 => '5',
            Self::R6 => '6',
            Self::R7 => '7',
            Self::R8 => '8',
        }
    }
}

impl TryFrom<char> for File {
    type Error = &'static str;
    fn try_from(ch: char) -> Result<Self, Self::Error> {
        match ch {
            'a' => Ok(Self::A),
            'b' => Ok(Self::B),
            'c' => Ok(Self::C),
            'd' => Ok(Self::D),
            'e' => Ok(Self::E),
            'f' => Ok(Self::F),
            'g' => Ok(Self::G),
            'h' => Ok(Self::H),
            _ => Err("file char not of abcdefgh"),
        }
    }
}

impl TryFrom<char> for Rank {
    type Error = &'static str;
    fn try_from(ch: char) -> Result<Self, Self::Error> {
        match ch {
            '1' => Ok(Self::R1),
            '2' => Ok(Self::R2),
            '3' => Ok(Self::R3),
            '4' => Ok(Self::R4),
            '5' => Ok(Self::R5),
            '6' => Ok(Self::R6),
            '7' => Ok(Self::R7),
            '8' => Ok(Self::R8),
            _ => Err("rank char not of 12345678"),
        }
    }
}

impl Display for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_char(self.to_char())
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_char(self.to_char())
    }
}

impl Indexable for (File, Rank) {
    fn idx(&self) -> usize {
        let &(file, rank) = self;
        NUM_FILES * rank as usize + file as usize
    }
}

pub struct SquareIterator {
    square_discriminant: u8,
}

impl SquareIterator {
    fn new() -> Self {
        Self {
            square_discriminant: 0,
        }
    }
}

impl Iterator for SquareIterator {
    type Item = Square;
    fn next(&mut self) -> Option<Self::Item> {
        let maybe_item = Square::from_u8(self.square_discriminant);
        if self.square_discriminant <= Square::H8 as u8 {
            self.square_discriminant += 1;
        }
        return maybe_item;
    }
}

impl Square {
    /// Square enum variants cover all u8 values from 0-63 inclusive.
    /// WARNING: Uses `unsafe`.
    /// TODO: Change to const safe code covering all cases using match in macro.
    pub fn from_u8(value: u8) -> Option<Square> {
        // If value is in valid range, transmute, otherwise return None.
        (value <= Square::H8 as u8).then(|| unsafe { transmute::<u8, Square>(value) })
    }
    pub fn from_idx<I: Indexable>(indexable: I) -> Option<Square> {
        Self::from_u8(indexable.idx() as u8)
    }
    pub fn iter() -> SquareIterator {
        SquareIterator::new()
    }

    pub const fn file(&self) -> File {
        use Square::*;
        match self {
            A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8 => File::A,
            B1 | B2 | B3 | B4 | B5 | B6 | B7 | B8 => File::B,
            C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 => File::C,
            D1 | D2 | D3 | D4 | D5 | D6 | D7 | D8 => File::D,
            E1 | E2 | E3 | E4 | E5 | E6 | E7 | E8 => File::E,
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 => File::F,
            G1 | G2 | G3 | G4 | G5 | G6 | G7 | G8 => File::G,
            H1 | H2 | H3 | H4 | H5 | H6 | H7 | H8 => File::H,
        }
    }

    pub const fn rank(&self) -> Rank {
        use Square::*;
        match self {
            A1 | B1 | C1 | D1 | E1 | F1 | G1 | H1 => Rank::R1,
            A2 | B2 | C2 | D2 | E2 | F2 | G2 | H2 => Rank::R2,
            A3 | B3 | C3 | D3 | E3 | F3 | G3 | H3 => Rank::R3,
            A4 | B4 | C4 | D4 | E4 | F4 | G4 | H4 => Rank::R4,
            A5 | B5 | C5 | D5 | E5 | F5 | G5 | H5 => Rank::R5,
            A6 | B6 | C6 | D6 | E6 | F6 | G6 | H6 => Rank::R6,
            A7 | B7 | C7 | D7 | E7 | F7 | G7 | H7 => Rank::R7,
            A8 | B8 | C8 | D8 | E8 | F8 | G8 | H8 => Rank::R8,
        }
    }

    /// Returns 0-based file (0,1,2,3,4,5,6,7), not 1-based chess file.
    pub const fn file_u8(&self) -> u8 {
        *self as u8 % NUM_RANKS as u8
    }

    /// Returns 0-based rank (0,1,2,3,4,5,6,7), not 1-based chess rank.
    pub const fn rank_u8(&self) -> u8 {
        *self as u8 / NUM_FILES as u8
    }
}

/// There are better ways to do this, however as I am new to Rust I figure I should
/// stay away from using unsafe blocks.
/// TODO: Find a safe way to shorten this.
impl From<(File, Rank)> for Square {
    fn from((file, rank): (File, Rank)) -> Square {
        match file {
            File::A => match rank {
                Rank::R1 => Self::A1,
                Rank::R2 => Self::A2,
                Rank::R3 => Self::A3,
                Rank::R4 => Self::A4,
                Rank::R5 => Self::A5,
                Rank::R6 => Self::A6,
                Rank::R7 => Self::A7,
                Rank::R8 => Self::A8,
            },
            File::B => match rank {
                Rank::R1 => Self::B1,
                Rank::R2 => Self::B2,
                Rank::R3 => Self::B3,
                Rank::R4 => Self::B4,
                Rank::R5 => Self::B5,
                Rank::R6 => Self::B6,
                Rank::R7 => Self::B7,
                Rank::R8 => Self::B8,
            },
            File::C => match rank {
                Rank::R1 => Self::C1,
                Rank::R2 => Self::C2,
                Rank::R3 => Self::C3,
                Rank::R4 => Self::C4,
                Rank::R5 => Self::C5,
                Rank::R6 => Self::C6,
                Rank::R7 => Self::C7,
                Rank::R8 => Self::C8,
            },
            File::D => match rank {
                Rank::R1 => Self::D1,
                Rank::R2 => Self::D2,
                Rank::R3 => Self::D3,
                Rank::R4 => Self::D4,
                Rank::R5 => Self::D5,
                Rank::R6 => Self::D6,
                Rank::R7 => Self::D7,
                Rank::R8 => Self::D8,
            },
            File::E => match rank {
                Rank::R1 => Self::E1,
                Rank::R2 => Self::E2,
                Rank::R3 => Self::E3,
                Rank::R4 => Self::E4,
                Rank::R5 => Self::E5,
                Rank::R6 => Self::E6,
                Rank::R7 => Self::E7,
                Rank::R8 => Self::E8,
            },
            File::F => match rank {
                Rank::R1 => Self::F1,
                Rank::R2 => Self::F2,
                Rank::R3 => Self::F3,
                Rank::R4 => Self::F4,
                Rank::R5 => Self::F5,
                Rank::R6 => Self::F6,
                Rank::R7 => Self::F7,
                Rank::R8 => Self::F8,
            },
            File::G => match rank {
                Rank::R1 => Self::G1,
                Rank::R2 => Self::G2,
                Rank::R3 => Self::G3,
                Rank::R4 => Self::G4,
                Rank::R5 => Self::G5,
                Rank::R6 => Self::G6,
                Rank::R7 => Self::G7,
                Rank::R8 => Self::G8,
            },
            File::H => match rank {
                Rank::R1 => Self::H1,
                Rank::R2 => Self::H2,
                Rank::R3 => Self::H3,
                Rank::R4 => Self::H4,
                Rank::R5 => Self::H5,
                Rank::R6 => Self::H6,
                Rank::R7 => Self::H7,
                Rank::R8 => Self::H8,
            },
        }
    }
}

/// Square::= <fileLetter><rankNumber>
impl FromStr for Square {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let file = File::try_from(chars.next().ok_or("No fileChar")?)?;
        let rank = Rank::try_from(chars.next().ok_or("No rankChar")?)?;
        Ok(Square::from((file, rank)))
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

impl Indexable for Square {
    fn idx(&self) -> usize {
        *self as usize
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

    #[test]
    fn square_to_from_string() {
        let valid_a1 = "a1"; // valid strings.
        let valid_a2 = "a2";
        let valid_a3 = "a3";
        let valid_a4 = "a4";
        let valid_a5 = "a5";
        let valid_a6 = "a6";
        let valid_a7 = "a7";
        let valid_a8 = "a8";
        let valid_b7 = "b7";
        let valid_h8 = "h8";
        let invalid_1 = "A1"; // invalid strings.
        let invalid_2 = "X3";
        let invalid_3 = "a$";
        let invalid_4 = "g";
        let a1 = Square::from_str(valid_a1).unwrap(); // valid squares.
        let a2 = Square::from_str(valid_a2).unwrap();
        let a3 = Square::from_str(valid_a3).unwrap();
        let a4 = Square::from_str(valid_a4).unwrap();
        let a5 = Square::from_str(valid_a5).unwrap();
        let a6 = Square::from_str(valid_a6).unwrap();
        let a7 = Square::from_str(valid_a7).unwrap();
        let a8 = Square::from_str(valid_a8).unwrap();
        let b7 = Square::from_str(valid_b7).unwrap();
        let h8: Square = valid_h8.parse().unwrap();
        assert_eq!(a1, Square::A1); // compare parsed Square with expected.
        assert_eq!(a2, Square::A2);
        assert_eq!(a3, Square::A3);
        assert_eq!(a4, Square::A4);
        assert_eq!(a5, Square::A5);
        assert_eq!(a6, Square::A6);
        assert_eq!(a7, Square::A7);
        assert_eq!(a8, Square::A8);
        assert_eq!(b7, Square::B7);
        assert_eq!(h8, Square::H8);
        assert!(Square::from_str(invalid_1).is_err()); // Errors are errors.
        assert!(Square::from_str(invalid_2).is_err());
        assert!(Square::from_str(invalid_3).is_err());
        assert!(Square::from_str(invalid_4).is_err());
        assert_eq!(a1.to_string(), valid_a1); // Square as string equals parsed.
        assert_eq!(a2.to_string(), valid_a2);
        assert_eq!(a3.to_string(), valid_a3);
        assert_eq!(a4.to_string(), valid_a4);
        assert_eq!(a5.to_string(), valid_a5);
        assert_eq!(a6.to_string(), valid_a6);
        assert_eq!(a7.to_string(), valid_a7);
        assert_eq!(a8.to_string(), valid_a8);
        assert_eq!(b7.to_string(), valid_b7);
        assert_eq!(h8.to_string(), valid_h8);
    }

    #[test]
    fn square_to_from_file_rank() {
        {
            let a1 = Square::from((File::A, Rank::R1));
            assert_eq!(a1.file(), File::A);
            assert_eq!(a1.rank(), Rank::R1);
            assert_eq!(a1.file_u8(), File::A as u8);
            assert_eq!(a1.rank_u8(), Rank::R1 as u8);
        }
        {
            let a7 = Square::from((File::A, Rank::R7));
            assert_eq!(a7.file(), File::A);
            assert_eq!(a7.rank(), Rank::R7);
            assert_eq!(a7.file_u8(), File::A as u8);
            assert_eq!(a7.rank_u8(), Rank::R7 as u8);
        }
        {
            let h8 = Square::from((File::H, Rank::R8));
            assert_eq!(h8.file(), File::H);
            assert_eq!(h8.rank(), Rank::R8);
            assert_eq!(h8.file_u8(), File::H as u8);
            assert_eq!(h8.rank_u8(), Rank::R8 as u8);
        }
        {
            let e4 = Square::from((File::E, Rank::R4));
            assert_eq!(e4.file(), File::E);
            assert_eq!(e4.rank(), Rank::R4);
            assert_eq!(e4.file_u8(), File::E as u8);
            assert_eq!(e4.rank_u8(), Rank::R4 as u8);
        }
    }
}
