//! The fundamental and simple types of `blunders_engine`.

use std::convert::TryFrom;
use std::fmt::{self, Display, Write};
use std::mem::replace;
use std::mem::transmute; // unsafe
use std::ops::{BitOr, Not};
use std::str::FromStr;

///////////////
// Constants //
///////////////
pub const NUM_FILES: usize = 8; // A, B, C, D, E, F, G, H
pub const NUM_RANKS: usize = 8; // 1, 2, 3, 4, 5, 6, 7, 8
pub const NUM_SQUARES: usize = NUM_FILES * NUM_RANKS;
const WHITE_ARRAY_OFFSET: u8 = 0;
const BLACK_ARRAY_OFFSET: u8 = 6;

/////////////////////////
// Data and Structures //
/////////////////////////

/// Counter for half-move clock and full-moves.
pub type MoveCount = u32;

/// Color can represent the color of a piece, or a player.
/// Color's set discriminant is used in position.rs to index without branching.
/// First 6 are for each piece_kind as white, last 6 are for each piece_kind as black.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Color {
    White = WHITE_ARRAY_OFFSET,
    Black = BLACK_ARRAY_OFFSET,
}

/// Enum variant order and discriminant are important.
/// Must be contiguous and start from 0.
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
    pub(crate) color: Color,
    pub(crate) piece_kind: PieceKind,
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

/// Enum variant order and discriminant must be contiguous, start from 0, 
/// and be in ascending order ABCDEFGH.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[rustfmt::skip]
#[repr(u8)]
pub enum File {
    A, B, C, D, E, F, G, H = 7u8,
}

/// Enum variant order and discriminant must be contiguous, start from 0, 
/// and be in ascending order 12345678.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
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
/// Equivalent to a chess "half move", or "ply".
/// TODO: Store data to allow for unmake_move on position.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Move {
    pub(crate) from: Square,
    pub(crate) to: Square,
    pub(crate) promotion: Option<PieceKind>,
}

////////////
// Traits //
////////////

/// SquareIndexable
/// A chessboard has 64 squares on it. SquareIndexable can be implemented
/// for types whose values can map directly to a chess Square's index.
pub trait SquareIndexable {
    /// idx must be implemented.
    /// idx(&self) must return a number between 0-63 inclusive, representing
    /// a square on a chess board in little-endian, rank-file order.
    /// Warning: Values outside of 0-63 may panic or cause undefined behavior.
    fn idx(&self) -> usize;

    /// shift returns a number that represents the bit-index equivalent of a
    /// chess Square on a u64.
    fn shift(&self) -> u64 {
        1u64 << self.idx()
    }
}

// Blanket impl on references of types that are SquareIndexable.
impl<I: SquareIndexable> SquareIndexable for &I {
    fn idx(&self) -> usize {
        I::idx(*self)
    }
}

//////////////////////
/// Implementations //
//////////////////////

impl Color {
    /// FEN compliant conversion.
    pub const fn to_char(&self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }
    pub const fn iter() -> ColorIterator {
        ColorIterator::new()
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

impl Not for &Color {
    type Output = Color;
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

pub struct ColorIterator {
    maybe_color: Option<Color>,
}

impl ColorIterator {
    pub const fn new() -> Self {
        Self {
            maybe_color: Some(Color::White),
        }
    }
}

impl Iterator for ColorIterator {
    type Item = Color;
    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.maybe_color {
            Some(Color::White) => Some(Color::Black),
            Some(Color::Black) | None => None,
        };
        replace(&mut self.maybe_color, value)
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

    pub const fn iter() -> PieceKindIterator {
        PieceKindIterator::new()
    }
}

pub struct PieceKindIterator {
    maybe_piece_kind: Option<PieceKind>,
}

impl PieceKindIterator {
    pub const fn new() -> Self {
        Self {
            maybe_piece_kind: Some(PieceKind::Pawn),
        }
    }
}

impl Iterator for PieceKindIterator {
    type Item = PieceKind;
    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.maybe_piece_kind {
            Some(PieceKind::Pawn) => Some(PieceKind::Rook),
            Some(PieceKind::Rook) => Some(PieceKind::Knight),
            Some(PieceKind::Knight) => Some(PieceKind::Bishop),
            Some(PieceKind::Bishop) => Some(PieceKind::Queen),
            Some(PieceKind::Queen) => Some(PieceKind::King),
            Some(PieceKind::King) | None => None,
        };
        replace(&mut self.maybe_piece_kind, value)
    }
}

impl Piece {
    pub const fn new(color: Color, piece_kind: PieceKind) -> Self {
        Piece { color, piece_kind }
    }
    /// Immutable Getters.
    pub const fn color(&self) -> &Color {
        &self.color
    }
    pub const fn piece_kind(&self) -> &PieceKind {
        &self.piece_kind
    }

    pub const fn to_char(&self) -> char {
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
    /// Make new Castling with all rights of initial chess position.
    pub const fn start_position() -> Self {
        Self::ALL
    }

    /// Returns true if Castling mask has all of provided bits.
    pub fn has(&self, rights: Castling) -> bool {
        assert!(Self::is_mask_valid(rights));
        self.0 & rights.0 == rights.0
    }

    /// Returns true if there are no castling rights.
    pub const fn is_none(&self) -> bool {
        self.0 == 0u8
    }

    /// Set given bits to '1' on Castling mask.
    pub fn set(&mut self, rights: Castling) {
        assert!(Self::is_mask_valid(rights));
        self.0 |= rights.0;
    }

    /// Set given bits to '0' on Castling mask.
    pub fn clear(&mut self, rights: Castling) {
        assert!(Self::is_mask_valid(rights));
        self.0 &= !rights.0;
    }

    const fn is_mask_valid(rights: Castling) -> bool {
        rights.0 <= Self::ALL.0
    }
}

/// Defaults to Castling rights for starting chess position, ALL.
impl Default for Castling {
    fn default() -> Self {
        Self::start_position()
    }
}

impl BitOr for Castling {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Displays in FEN-component format.
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
            'K' => castling_rights.set(Self::W_KING),
            'Q' => castling_rights.set(Self::W_QUEEN),
            'k' => castling_rights.set(Self::B_KING),
            'q' => castling_rights.set(Self::B_QUEEN),
            _ => return Err("First char not of -KQkq"),
        };

        // castling_rights is now valid, add rest of rights or return early.
        for ch in chars {
            match ch {
                'K' => castling_rights.set(Self::W_KING),
                'Q' => castling_rights.set(Self::W_QUEEN),
                'k' => castling_rights.set(Self::B_KING),
                'q' => castling_rights.set(Self::B_QUEEN),
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

impl SquareIndexable for (File, Rank) {
    fn idx(&self) -> usize {
        let &(file, rank) = self;
        NUM_FILES * rank as usize + file as usize
    }
}

pub struct SquareIterator {
    square_discriminant: u8,
}

impl SquareIterator {
    const fn new() -> Self {
        Self {
            square_discriminant: Square::A1 as u8,
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
    pub fn from_idx<I: SquareIndexable>(indexable: I) -> Option<Square> {
        Self::from_u8(indexable.idx() as u8)
    }
    pub const fn iter() -> SquareIterator {
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
impl From<(File, Rank)> for Square {
    fn from((file, rank): (File, Rank)) -> Self {
        use {File::*, Rank::*, Square::*};

        match file {
            #[rustfmt::skip]
            A => match rank {
                R1 => A1, R2 => A2, R3 => A3, R4 => A4,
                R5 => A5, R6 => A6, R7 => A7, R8 => A8,
            },
            #[rustfmt::skip]
            B => match rank {
                R1 => B1, R2 => B2, R3 => B3, R4 => B4,
                R5 => B5, R6 => B6, R7 => B7, R8 => B8,
            },
            #[rustfmt::skip]
            C => match rank {
                R1 => C1, R2 => C2, R3 => C3, R4 => C4,
                R5 => C5, R6 => C6, R7 => C7, R8 => C8,
            },
            #[rustfmt::skip]
            D => match rank {
                R1 => D1, R2 => D2, R3 => D3, R4 => D4,
                R5 => D5, R6 => D6, R7 => D7, R8 => D8,
            },
            #[rustfmt::skip]
            E => match rank {
                R1 => E1, R2 => E2, R3 => E3, R4 => E4,
                R5 => E5, R6 => E6, R7 => E7, R8 => E8,
            },
            #[rustfmt::skip]
            F => match rank {
                R1 => F1, R2 => F2, R3 => F3, R4 => F4,
                R5 => F5, R6 => F6, R7 => F7, R8 => F8,
            },
            #[rustfmt::skip]
            G => match rank {
                R1 => G1, R2 => G2, R3 => G3, R4 => G4,
                R5 => G5, R6 => G6, R7 => G7, R8 => G8,
            },
            #[rustfmt::skip]
            H => match rank {
                R1 => H1, R2 => H2, R3 => H3, R4 => H4,
                R5 => H5, R6 => H6, R7 => H7, R8 => H8,
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

impl SquareIndexable for Square {
    fn idx(&self) -> usize {
        *self as usize
    }
}

impl Move {
    pub const fn new(from: Square, to: Square, promotion: Option<PieceKind>) -> Self {
        Self {
            from,
            to,
            promotion,
        }
    }
    // Getters
    pub const fn from(&self) -> &Square {
        &self.from
    }
    pub const fn to(&self) -> &Square {
        &self.to
    }
    pub const fn promotion(&self) -> &Option<PieceKind> {
        &self.promotion
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

        cr.clear(Castling::W_KING);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(cr.has(Castling::W_QUEEN));
        assert!(cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.clear(Castling::W_QUEEN);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(!cr.has(Castling::W_QUEEN));
        assert!(cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.clear(Castling::B_KING);
        assert!(!cr.has(Castling::ALL));
        assert!(!cr.has(Castling::W_KING));
        assert!(!cr.has(Castling::W_QUEEN));
        assert!(!cr.has(Castling::B_KING));
        assert!(cr.has(Castling::B_QUEEN));
        assert!(!cr.has(Castling::W_SIDE));
        assert!(!cr.has(Castling::B_SIDE));
        assert!(cr.is_none() == false);

        cr.clear(Castling::B_QUEEN);
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
        use File::*;
        use Rank::*;
        let a1 = Square::from((A, R1));
        assert_eq!(a1.file(), A);
        assert_eq!(a1.rank(), R1);
        assert_eq!(a1.file_u8(), A as u8);
        assert_eq!(a1.rank_u8(), R1 as u8);

        let a7 = Square::from((A, R7));
        assert_eq!(a7.file(), A);
        assert_eq!(a7.rank(), R7);
        assert_eq!(a7.file_u8(), A as u8);
        assert_eq!(a7.rank_u8(), R7 as u8);

        let h8 = Square::from((H, R8));
        assert_eq!(h8.file(), H);
        assert_eq!(h8.rank(), R8);
        assert_eq!(h8.file_u8(), H as u8);
        assert_eq!(h8.rank_u8(), R8 as u8);

        let e4 = Square::from((E, R4));
        assert_eq!(e4.file(), E);
        assert_eq!(e4.rank(), R4);
        assert_eq!(e4.file_u8(), E as u8);
        assert_eq!(e4.rank_u8(), R4 as u8);
    }
}
