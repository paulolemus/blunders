//! fen.rs
//! Forsyth-Edwards Notation, a standard notation for describing a chess position.
//! https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation
//! https://www.chessprogramming.org/Forsyth-Edwards_Notation
//!
//! Example:
//! Starting Chess FEN: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1

use std::convert::TryFrom;
use std::fmt::{self, Display};
use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::coretypes::{Castling, Color, File, MoveCount, Piece, Rank, Square};
use crate::mailbox::Mailbox;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParseFenError {
    IllFormed,
    Placement,
    SideToMove,
    Castling,
    EnPassant,
    HalfMoveClock,
    FullMoveNumber,
}

/// Allows converting data that can be represented as a FEN sub-string
/// to and from &str.
trait FenComponent: Sized {
    type Error;
    fn try_from_fen_str(s: &str) -> Result<Self, Self::Error>;
    fn to_fen_str(&self) -> String;
}

/// Placement FenComponent.
impl FenComponent for Mailbox {
    type Error = ParseFenError;
    fn try_from_fen_str(s: &str) -> Result<Self, Self::Error> {
        // Placement is 8 ranks separated by '/'.
        // Each rank need to sum up to 8 pieces.
        const NUMS: RangeInclusive<char> = '1'..='8';
        const PIECES: [char; 12] = ['R', 'N', 'B', 'Q', 'K', 'P', 'r', 'n', 'b', 'q', 'k', 'p'];
        const ERR: ParseFenError = ParseFenError::Placement;

        let mut num_ranks = 0u32;
        let mut squares = Square::iter();
        let mut board = Mailbox::with_none();

        // Iterate FEN string in normal Rank-File order.
        for rank_str in s.split('/').rev() {
            let mut sum_rank = 0;
            num_ranks += 1;

            for ch in rank_str.chars() {
                if NUMS.contains(&ch) {
                    let num = ch.to_digit(10).ok_or(ERR)?;
                    squares.nth(num as usize - 1);
                    sum_rank += num;
                } else if PIECES.contains(&ch) {
                    let piece = Piece::try_from(ch).map_err(|_| ERR)?;
                    let square = squares.next().ok_or(ERR)?;
                    board[square] = Some(piece);
                    sum_rank += 1;
                } else {
                    return Err(ERR);
                }
            }
            if sum_rank != 8 {
                return Err(ERR);
            }
        }

        (num_ranks == 8)
            .then(|| board)
            .ok_or(ParseFenError::Placement)
    }

    fn to_fen_str(&self) -> String {
        // For each Rank, count consecutive empty squares.
        // Before pushing some char, add empty count if not 0 then set to 0.
        use File::*;
        use Rank::*;
        let mut fen_str = String::new();

        for &rank in &[R8, R7, R6, R5, R4, R3, R2, R1] {
            let mut empty_counter = 0u8;

            for &file in &[A, B, C, D, E, F, G, H] {
                match self[(file, rank)] {
                    Some(piece) => {
                        if empty_counter != 0 {
                            fen_str.push_str(&empty_counter.to_string());
                            empty_counter = 0;
                        }
                        fen_str.push(piece.into())
                    }
                    None => empty_counter += 1,
                };
            }

            if empty_counter != 0 {
                fen_str.push_str(&empty_counter.to_string());
            }
            fen_str.push('/');
        }
        fen_str.pop(); // Extra '/'.
        fen_str
    }
}

/// Side-To-Move FenComponent.
impl FenComponent for Color {
    type Error = ParseFenError;
    /// Side to move is either character 'w' | 'b'
    fn try_from_fen_str(s: &str) -> Result<Self, Self::Error> {
        let ch = s.chars().next().ok_or(ParseFenError::SideToMove)?;
        Color::try_from(ch).map_err(|_| ParseFenError::SideToMove)
    }
    fn to_fen_str(&self) -> String {
        self.to_string()
    }
}

/// Castling FenComponent.
impl FenComponent for Castling {
    type Error = ParseFenError;
    /// Castling is either '-' or [K][Q][k][q]
    fn try_from_fen_str(s: &str) -> Result<Self, Self::Error> {
        Castling::from_str(s).map_err(|_| ParseFenError::Castling)
    }
    fn to_fen_str(&self) -> String {
        self.to_string()
    }
}

/// En-Passant FenComponent.
impl FenComponent for Option<Square> {
    type Error = ParseFenError;
    /// En Passant is either - or a square coordinate ex: "a4".
    fn try_from_fen_str(s: &str) -> Result<Self, Self::Error> {
        const RANKS: [char; 2] = ['3', '6'];
        let mut chars = s.chars();
        let first = chars.next().ok_or(ParseFenError::EnPassant)?;

        if first == '-' {
            Ok(None)
        } else {
            let second = chars.next().ok_or(ParseFenError::EnPassant)?;
            Ok(Some(
                RANKS
                    .contains(&second)
                    .then(|| Square::from_str(s))
                    .ok_or(ParseFenError::EnPassant)?
                    .map_err(|_| ParseFenError::EnPassant)?,
            ))
        }
    }
    fn to_fen_str(&self) -> String {
        match self {
            Some(square) => square.to_string(),
            None => "-".to_string(),
        }
    }
}

/// An intermediary structure used for converting
/// to and from String, and to and from A Position object.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Fen {
    placement: Mailbox,
    side_to_move: Color,
    castling: Castling,
    en_passant: Option<Square>,
    halfmove_clock: MoveCount,
    fullmove_number: MoveCount,
}

impl Fen {
    /// Immutable Getters
    pub fn placement(&self) -> &Mailbox {
        &self.placement
    }
    pub fn side_to_move(&self) -> &Color {
        &self.side_to_move
    }
    pub fn castling(&self) -> &Castling {
        &self.castling
    }
    pub fn en_passant(&self) -> &Option<Square> {
        &self.en_passant
    }
    pub fn halfmove_clock(&self) -> &MoveCount {
        &self.halfmove_clock
    }
    pub fn fullmove_number(&self) -> &MoveCount {
        &self.fullmove_number
    }

    /// HalfMove Clock is any non-negative number.
    fn parse_halfmove_clock(s: &str) -> Result<MoveCount, ParseFenError> {
        s.parse::<MoveCount>()
            .map_err(|_| ParseFenError::HalfMoveClock)
    }

    /// FullMove Number starts at 1, and can increment infinitely.
    pub fn parse_fullmove_number(s: &str) -> Result<MoveCount, ParseFenError> {
        let fullmove: MoveCount = s.parse().unwrap_or(0);
        if fullmove != 0 {
            Ok(fullmove)
        } else {
            Err(ParseFenError::FullMoveNumber)
        }
    }
}

impl Default for Fen {
    /// Fen for starting chess position.
    fn default() -> Self {
        Fen {
            placement: Mailbox::default(),
            side_to_move: Color::White,
            castling: Castling::default(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        }
    }
}

impl FromStr for Fen {
    type Err = ParseFenError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Ensure 6 whitespace separated components.
        if s.split_whitespace().count() != 6 {
            return Err(ParseFenError::IllFormed);
        }
        let fen_parts: Vec<&str> = s.split_whitespace().collect();

        // Fen Order: Placement/Side-To-Move/Castling/En-Passant/Halfmove/Fullmove
        let placement: Mailbox = FenComponent::try_from_fen_str(fen_parts[0])?;
        let side_to_move: Color = FenComponent::try_from_fen_str(fen_parts[1])?;
        let castling: Castling = FenComponent::try_from_fen_str(fen_parts[2])?;
        let en_passant: Option<Square> = FenComponent::try_from_fen_str(fen_parts[3])?;
        let halfmove_clock = Fen::parse_halfmove_clock(fen_parts[4])?;
        let fullmove_number = Fen::parse_fullmove_number(fen_parts[5])?;

        Ok(Fen {
            placement,
            side_to_move,
            castling,
            en_passant,
            halfmove_clock,
            fullmove_number,
        })
    }
}

impl Display for Fen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {}",
            self.placement.to_fen_str(),
            self.side_to_move.to_fen_str(),
            self.castling.to_fen_str(),
            self.en_passant.to_fen_str(),
            self.halfmove_clock,
            self.fullmove_number
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_fen_string() {
        //! Assert that the starting position FEN string parses into Fen object.
        //! Assert that starting FEN string, parsed Fen object, and default Fen object
        //! are equivalent.
        const FEN_STR: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let fen: Fen = FEN_STR.parse().unwrap();
        let default_fen: Fen = Fen::default();

        assert_eq!(fen, default_fen);
        assert_eq!(fen.to_string(), FEN_STR);
        assert_eq!(default_fen.to_string(), FEN_STR);
        println!("{}", fen.to_string());
    }

    #[test]
    fn parse_placement_fen_substrings() {
        //! Assert Fen::parse_placement(&str) works properly.
        const VALID1: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        const VALID2: &str = "rn1qkb1r/ppp2ppp/4pn2/3p4/3P2bP/2N1PN2/PPP2PP1/R1BQKB1R";
        const VALID3: &str = "r1Q2rk1/p3qppp/np1bpn2/3p4/1PpP2bP/2N1PN2/PBP2PPR/R3KB2";
        const VALID4: &str = "2r2rk1/p4p2/nR4Pp/3p4/3P2P1/P1p5/2P1KP1R/4b3";

        const INVALID1: &str = "";
        const INVALID2: &str = "hello world";
        const INVALID3: &str = "nbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        const INVALID4: &str = "nbqkbnr/ pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        const INVALID5: &str = " rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        const INVALID6: &str = "rnbqkbnr/pppppppp/27/8/8/8/PPPPPPPP/RNBQKBNR";

        assert_eq!(
            Mailbox::try_from_fen_str(VALID1).unwrap().to_fen_str(),
            VALID1
        );
        assert_eq!(
            Mailbox::try_from_fen_str(VALID2).unwrap().to_fen_str(),
            VALID2
        );
        assert_eq!(
            Mailbox::try_from_fen_str(VALID3).unwrap().to_fen_str(),
            VALID3
        );
        assert_eq!(
            Mailbox::try_from_fen_str(VALID4).unwrap().to_fen_str(),
            VALID4
        );
        assert!(Mailbox::try_from_fen_str(INVALID1).is_err());
        assert!(Mailbox::try_from_fen_str(INVALID2).is_err());
        assert!(Mailbox::try_from_fen_str(INVALID3).is_err());
        assert!(Mailbox::try_from_fen_str(INVALID4).is_err());
        assert!(Mailbox::try_from_fen_str(INVALID5).is_err());
        assert!(Mailbox::try_from_fen_str(INVALID6).is_err());
    }

    #[test]
    fn parse_castling_fen_substring() {
        const VALID1: &str = "-";
        const VALID2: &str = "Q";
        const VALID3: &str = "K";
        const VALID4: &str = "q";
        const VALID5: &str = "k";
        const VALID6: &str = "KQkq";

        const INVALID1: &str = "";
        const INVALID2: &str = "a";
        const INVALID3: &str = " KQkq";

        assert_eq!(
            Castling::try_from_fen_str(VALID1).unwrap().to_fen_str(),
            VALID1
        );
        assert_eq!(
            Castling::try_from_fen_str(VALID2).unwrap().to_fen_str(),
            VALID2
        );
        assert_eq!(
            Castling::try_from_fen_str(VALID3).unwrap().to_fen_str(),
            VALID3
        );
        assert_eq!(
            Castling::try_from_fen_str(VALID4).unwrap().to_fen_str(),
            VALID4
        );
        assert_eq!(
            Castling::try_from_fen_str(VALID5).unwrap().to_fen_str(),
            VALID5
        );
        assert_eq!(
            Castling::try_from_fen_str(VALID6).unwrap().to_fen_str(),
            VALID6
        );
        assert!(Castling::try_from_fen_str(INVALID1).is_err());
        assert!(Castling::try_from_fen_str(INVALID2).is_err());
        assert!(Castling::try_from_fen_str(INVALID3).is_err());
    }
}
