//! fen.rs
//! Forsyth-Edwards Notation, a standard notation for describing a chess position.
//! https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation
//! https://www.chessprogramming.org/Forsyth-Edwards_Notation
//!
//! Example:
//! Starting Chess FEN: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1

use std::fmt::{self, Display};
use std::ops::RangeInclusive;
use std::str::FromStr;

type MoveInt = u64;

/// An intermediary structure used for converting
/// to and from String, and to and from A Position object.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Fen {
    placement: String,
    side_to_move: String,
    castling: Option<String>,
    en_passant: Option<String>,
    halfmove_clock: MoveInt,
    fullmove_number: MoveInt,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ParseFenError {
    IllFormed,
    Placement,
    SideToMove,
    Castling,
    EnPassant,
    HalfMoveClock,
    FullMoveNumber,
}

impl Fen {
    /// Return an iterator that yields Option<Piece>.
    //fn placement(&self) {
    //for rank in self.placement.split('/').rev() {
    //for ch in rank.chars() {}
    //}
    //}

    /// Placement is 8 ranks separated by '/'.
    /// Each rank need to sum up to 8 pieces.
    fn parse_placement(s: &str) -> Result<String, ParseFenError> {
        const NUMS: RangeInclusive<char> = '1'..='8';
        const PIECES: [char; 12] = ['R', 'N', 'B', 'Q', 'K', 'P', 'r', 'n', 'b', 'q', 'k', 'p'];
        const TO_VALUE: fn(char) -> u32 = |ch: char| -> u32 {
            if NUMS.contains(&ch) {
                ch.to_digit(10).unwrap()
            } else if PIECES.contains(&ch) {
                1
            } else {
                9 // Assures failure as is past 8.
            }
        };
        let mut num_ranks = 0u32;

        for rank in s.split('/') {
            num_ranks += 1;
            let sum_rank: u32 = rank.chars().map(TO_VALUE).sum();
            if sum_rank != 8 {
                return Err(ParseFenError::Placement);
            }
        }

        if num_ranks == 8 {
            Ok(s.to_string())
        } else {
            Err(ParseFenError::Placement)
        }
    }

    /// Side to move is either character 'w' | 'b'
    fn parse_side_to_move(s: &str) -> Result<String, ParseFenError> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 1 {
            Err(ParseFenError::SideToMove)
        } else {
            match chars[0] {
                'w' | 'b' => Ok(chars[0].to_string()),
                _ => Err(ParseFenError::SideToMove),
            }
        }
    }

    /// Castling is either '-' or [K][Q][k][q]
    fn parse_castling(s: &str) -> Result<Option<String>, ParseFenError> {
        let chars: Vec<char> = s.chars().collect();

        // Check for -
        if chars.len() < 1 || chars.len() > 4 {
            return Err(ParseFenError::Castling);
        } else if chars.len() == 1 && chars[0] == '-' {
            return Ok(None);
        }

        let mut castling: [Option<char>; 4] = [None, None, None, None]; // KQkq
        for ch in &chars {
            match ch {
                'K' => castling[0] = Some('K'),
                'Q' => castling[1] = Some('Q'),
                'k' => castling[2] = Some('k'),
                'q' => castling[3] = Some('q'),
                _ => return Err(ParseFenError::Castling),
            }
        }

        Ok(Some(castling.iter().flatten().collect()))
    }

    /// En Passant is either - or a square coordinate ex: "a4".
    fn parse_en_passant(s: &str) -> Result<Option<String>, ParseFenError> {
        const LETTERS: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
        const RANKS: [char; 2] = ['3', '6'];

        let chars: Vec<char> = s.chars().collect();

        if chars.len() == 1 {
            // Check for -
            match chars[0] {
                '-' => Ok(None),
                _ => Err(ParseFenError::EnPassant),
            }
        } else if chars.len() == 2 && LETTERS.contains(&chars[0]) && RANKS.contains(&chars[1]) {
            // Check for coordinate
            Ok(Some(format!("{}{}", chars[0], chars[1])))
        } else {
            Err(ParseFenError::EnPassant)
        }
    }

    /// HalfMove Clock is any non-negative number.
    fn parse_halfmove_clock(s: &str) -> Result<MoveInt, ParseFenError> {
        match s.parse::<MoveInt>() {
            Ok(halfmove) => Ok(halfmove),
            Err(_) => Err(ParseFenError::HalfMoveClock),
        }
    }

    /// FullMove Number starts at 1, and can increment infinitely.
    pub fn parse_fullmove_number(s: &str) -> Result<MoveInt, ParseFenError> {
        let fullmove: MoveInt = s.parse().unwrap_or(0);
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
            placement: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR".into(),
            side_to_move: 'w'.into(),
            castling: Some("KQkq".into()),
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

        // Check piece placement substring.
        let placement: String = Fen::parse_placement(fen_parts[0])?;
        // Check color character.
        let side_to_move: String = Fen::parse_side_to_move(fen_parts[1])?;
        // Check castling rights substring.
        let castling: Option<String> = Fen::parse_castling(fen_parts[2])?;
        // Check En Passant substring.
        let en_passant: Option<String> = Fen::parse_en_passant(fen_parts[3])?;
        // Check halfmove integer.
        let halfmove_clock = Fen::parse_halfmove_clock(fen_parts[4])?;
        // Check fullmove integer.
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
            self.placement,
            self.side_to_move,
            self.castling.as_ref().unwrap_or(&'-'.into()),
            self.en_passant.as_ref().unwrap_or(&'-'.into()),
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

        assert_eq!(Fen::parse_placement(VALID1).unwrap(), VALID1);
        assert_eq!(Fen::parse_placement(VALID2).unwrap(), VALID2);
        assert_eq!(Fen::parse_placement(VALID3).unwrap(), VALID3);
        assert_eq!(Fen::parse_placement(VALID4).unwrap(), VALID4);
        assert!(Fen::parse_placement(INVALID1).is_err());
        assert!(Fen::parse_placement(INVALID2).is_err());
        assert!(Fen::parse_placement(INVALID3).is_err());
        assert!(Fen::parse_placement(INVALID4).is_err());
        assert!(Fen::parse_placement(INVALID5).is_err());
        assert!(Fen::parse_placement(INVALID6).is_err());
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
        const INVALID3: &str = "QQQQQ";
        const INVALID4: &str = " KQkq";

        assert_eq!(Fen::parse_castling(VALID1).unwrap(), None);
        assert_eq!(Fen::parse_castling(VALID2).unwrap().unwrap(), VALID2);
        assert_eq!(Fen::parse_castling(VALID3).unwrap().unwrap(), VALID3);
        assert_eq!(Fen::parse_castling(VALID4).unwrap().unwrap(), VALID4);
        assert_eq!(Fen::parse_castling(VALID5).unwrap().unwrap(), VALID5);
        assert_eq!(Fen::parse_castling(VALID6).unwrap().unwrap(), VALID6);
        assert!(Fen::parse_castling(INVALID1).is_err());
        assert!(Fen::parse_castling(INVALID2).is_err());
        assert!(Fen::parse_castling(INVALID3).is_err());
        assert!(Fen::parse_castling(INVALID4).is_err());
    }
}
