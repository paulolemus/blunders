// use std::convert::TryFrom;
// use std::str::FromStr;

use crate::pieces::Color;

#[derive(Debug)]
pub struct Fen {
    placement: String,
    side_to_move: Color,
    castling: String,
    en_passant: String,
    halfmove_clock: u32,
    fullmove_count: u32,
}
