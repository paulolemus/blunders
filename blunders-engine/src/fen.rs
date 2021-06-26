//! Forsyth-Edwards Notation, a standard notation for describing a chess position.
//!
//! [Wikipedia FEN](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation)\
//! [Chess Programming FEN](https://www.chessprogramming.org/Forsyth-Edwards_Notation)\
//!
//! Example:\
//! Starting Chess FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

use std::convert::TryFrom;
use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::boardrepr::{Mailbox, PieceSets};
use crate::coretypes::{Castling, Color, File, MoveCount, Piece, Rank, Square};
use crate::position::Position;

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

/// Implement Fen for any types which can be fully parsed from a FEN string.
pub trait Fen: Sized {
    /// Attempt to parse a Fen string into implementing type.
    fn parse_fen(s: &str) -> Result<Self, ParseFenError>;

    /// Returns string representation of implementing type in Fen format.
    fn to_fen(&self) -> String;

    /// HalfMove Clock is any non-negative number.
    fn parse_halfmove_clock(s: &str) -> Result<MoveCount, ParseFenError> {
        s.parse::<MoveCount>()
            .map_err(|_| ParseFenError::HalfMoveClock)
    }

    /// FullMove Number starts at 1, and can increment infinitely.
    fn parse_fullmove_number(s: &str) -> Result<MoveCount, ParseFenError> {
        let fullmove: MoveCount = s.parse().unwrap_or(0);
        if fullmove != 0 {
            Ok(fullmove)
        } else {
            Err(ParseFenError::FullMoveNumber)
        }
    }
}

impl Fen for Position {
    /// Attempt to parse a Fen string into implementing type.
    fn parse_fen(s: &str) -> Result<Self, ParseFenError> {
        // Ensure 6 whitespace separated components.
        if s.split_whitespace().count() != 6 {
            return Err(ParseFenError::IllFormed);
        }
        let fen_parts: Vec<&str> = s.split_whitespace().collect();

        // Fen Order: Placement/Side-To-Move/Castling/En-Passant/Halfmove/Fullmove
        let pieces: PieceSets = FenComponent::try_from_fen_str(fen_parts[0])?;
        let player: Color = FenComponent::try_from_fen_str(fen_parts[1])?;
        let castling: Castling = FenComponent::try_from_fen_str(fen_parts[2])?;
        let en_passant: Option<Square> = FenComponent::try_from_fen_str(fen_parts[3])?;
        let halfmoves: MoveCount = Self::parse_halfmove_clock(fen_parts[4])?;
        let fullmoves: MoveCount = Self::parse_fullmove_number(fen_parts[5])?;

        Ok(Self {
            pieces,
            player,
            castling,
            en_passant,
            halfmoves,
            fullmoves,
        })
    }

    /// Returns string representation of implementing type in Fen format.
    fn to_fen(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.pieces().to_fen_str(),
            self.player().to_fen_str(),
            self.castling().to_fen_str(),
            self.en_passant().to_fen_str(),
            self.halfmoves(),
            self.fullmoves()
        )
    }
}

/// Allows converting data that can be represented as a FEN sub-string
/// to and from &str.
pub trait FenComponent: Sized {
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
        let mut board = Mailbox::new();

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

        for rank in [R8, R7, R6, R5, R4, R3, R2, R1] {
            let mut empty_counter = 0u8;

            for file in [A, B, C, D, E, F, G, H] {
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

/// Placement FenComponent.
impl FenComponent for PieceSets {
    type Error = ParseFenError;
    fn try_from_fen_str(s: &str) -> Result<Self, Self::Error> {
        Mailbox::try_from_fen_str(s).map(|mailbox| Self::from(&mailbox))
    }
    fn to_fen_str(&self) -> String {
        Mailbox::from(self).to_fen_str()
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
    /// Castling is either '-' or `[K][Q][k][q]`
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_fen_string() {
        //! Assert that the starting position FEN string parses into Fen object.
        //! Assert that starting FEN string, parsed Fen object, and default Fen object
        //! are equivalent.
        const FEN_STR: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let pos = Position::parse_fen(FEN_STR).unwrap();
        let start_pos = Position::start_position();

        assert_eq!(pos, start_pos);
        assert_eq!(pos.to_fen(), FEN_STR);
        assert_eq!(start_pos.to_fen(), FEN_STR);
        println!("{}", start_pos.to_fen());
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
