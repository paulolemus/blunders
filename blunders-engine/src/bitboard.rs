//! bitboard.rs
//! A general purpose way to efficiently encode data.
//! Uses:
//! - Positional data of
//!
//! Data Order:
//! Little-Endian Rank-File mapping
//! A1 = least significant bit = 0x1
//! H8 = most significant bit = 0x8000000000000000
//!
//! Compass Rose:
//!
//! NoWe       North       NoEa
//!      +7     +8      +9
//! West -1      0      +1 East
//!      -9     -8      -7
//! SoWe       South       SoEa
//!

pub const BLACK_SQUARES: u64 = 0xAA55AA55AA55AA55;
pub const WHITE_SQUARES: u64 = !BLACK_SQUARES;

//pub struct Bitboard {
//    b: u64,
//}
