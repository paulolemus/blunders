//! Static Evaluation Functions.
//!
//! An evaluation function may have two types of calls: relative or absolute.
//!
//! An absolute score treats White as a maxing player and Black as a minning player,
//! so a centipawn score of +10 is winning for White, while -10 is winning for Black.
//! A relative score treats the player to move as the maxing player, so if it is
//! Black to move, +10 is winning for Black.

use std::fmt::{self, Display};
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

use crate::bitboard::{self, Bitboard};
use crate::coretypes::{Color, PieceKind, NUM_RANKS, NUM_SQUARES};
use crate::coretypes::{Color::*, PieceKind::*};
use crate::movegen as mg;
use crate::position::Position;

/// Centipawn, a common unit of measurement in chess, where 100 Centipawn == 1 Pawn.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct Cp(pub CpKind);

// Type alias to make changing type easy if needed.
type CpKind = i32;

// Newtype pattern boilerplate
impl Cp {
    pub const MIN: Cp = Self(CpKind::MIN + 1); // + 1 to avoid overflow error on negate.
    pub const MAX: Cp = Self(CpKind::MAX);

    /// Returns the sign of Centipawn value, either 1, -1, or 0.
    pub const fn signum(&self) -> CpKind {
        self.0.signum()
    }

    /// Returns currently leading player, or None is position is equal.
    pub const fn leading(&self) -> Option<Color> {
        match self.signum() {
            1 => Some(White),
            -1 => Some(Black),
            _ => None,
        }
    }
}

impl Add for Cp {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl AddAssign for Cp {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}
impl Sub for Cp {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}
impl Mul for Cp {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl Mul<u32> for Cp {
    type Output = Cp;
    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0 * rhs as i32)
    }
}
impl Neg for Cp {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
impl Display for Cp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:+}", self.0)
    }
}

impl PieceKind {
    /// Default, independent value per piece.
    pub const fn centipawns(&self) -> Cp {
        Cp(match self {
            Pawn => 100,   // 100 Centipawn == 1 Pawn
            Knight => 305, // slightly prefer knight over 3 default pawns
            Bishop => 310, // slightly prefer bishop over 3 default pawns
            Rook => 510,
            Queen => 950,
            King => 400_000,
        })
    }
}

// Evaluation Constants
const CHECKMATE: Cp = Cp(Cp::MAX.0 / 2 - 1);
const STALEMATE: Cp = Cp(0);
const MOBILITY_CP: Cp = Cp(2);

// Relative Evaluation Functions

/// Given a terminal node, return a score representing a checkmate or a draw.
/// The return score is relative to the player to move.
pub fn terminal(position: &Position) -> Cp {
    // Checkmate position is strictly bad for player to move.
    if position.is_checkmate() {
        -CHECKMATE
    } else {
        STALEMATE
    }
}

/// Primary hand-crafted evaluate function for engine, with return relative to player to move.
/// Statically evaluates a non-terminal position.
pub fn evaluate(position: &Position) -> Cp {
    evaluate_abs(position) * position.player.sign()
}

// Absolute Evaluation Functions

/// Given a terminal node (no moves can be made), return a score representing
/// a checkmate for white/black, or a draw.
pub fn terminal_abs(position: &Position) -> Cp {
    if position.is_checkmate() {
        match position.player {
            White => -CHECKMATE,
            Black => CHECKMATE,
        }
    } else {
        STALEMATE
    }
}

/// Primary evaluate function for engine.
/// Statically evaluate a non-terminal position using a variety of heuristics.
pub fn evaluate_abs(position: &Position) -> Cp {
    let cp_material = material(position);
    let cp_pass_pawns = pass_pawns(position);
    let cp_xray_king = xray_king_attacks(position);
    let cp_mobility = mobility(position);
    let cp_king_safety = king_safety(position);

    let cp_total = cp_material + cp_pass_pawns + cp_xray_king + cp_mobility + cp_king_safety;
    cp_total
}

/// Returns relative strength difference of pieces in position.
/// Is equivalent of piece_centipawn(White) - pieces_centipawn(Black).
/// A positive value is an advantage for white, 0 is even, negative is advantage for black.
pub fn material(position: &Position) -> Cp {
    let w_piece_cp: Cp = PieceKind::iter()
        .map(|pk| pk.centipawns() * position.pieces[(White, pk)].count_squares())
        .fold(Cp::default(), |acc, value| acc + value);

    let b_piece_cp: Cp = PieceKind::iter()
        .map(|pk| pk.centipawns() * position.pieces[(Black, pk)].count_squares())
        .fold(Cp::default(), |acc, value| acc + value);

    w_piece_cp - b_piece_cp
}

pub fn king_safety(position: &Position) -> Cp {
    let mut cp = Cp(0);

    let occupied = position.pieces.occupied();
    // Virtual mobility: treat king as a queen and the less squares it can attack the better.
    let w_sliding = position.pieces[(White, Queen)]
        | position.pieces[(White, Rook)]
        | position.pieces[(White, Bishop)];
    let b_sliding = position.pieces[(Black, Queen)]
        | position.pieces[(Black, Rook)]
        | position.pieces[(Black, Bishop)];
    let w_num_sliding = w_sliding.count_squares();
    let b_num_sliding = b_sliding.count_squares();
    let w_king = position.pieces[(White, King)];
    let b_king = position.pieces[(Black, King)];

    let w_king_open_squares = mg::queen_attacks(&w_king, &occupied).count_squares();
    let b_king_open_squares = mg::queen_attacks(&b_king, &occupied).count_squares();

    // The more sliding pieces the enemy has, the more value each open square has.
    let w_value = b_king_open_squares * w_num_sliding / 2;
    let b_value = w_king_open_squares * b_num_sliding / 2;

    let value_diff = Cp(w_value as i32 - b_value as i32);
    cp += value_diff;

    cp
}

/// Return value of number of moves that can be made from a position.
pub fn mobility(position: &Position) -> Cp {
    let w_attacks = position.attacks(&White, &position.pieces().occupied());
    let b_attacks = position.attacks(&Black, &position.pieces().occupied());

    let attack_surface_area_diff =
        w_attacks.count_squares() as i32 - b_attacks.count_squares() as i32;

    Cp(attack_surface_area_diff) * MOBILITY_CP
}

/// Returns Centipawn difference for passed pawns.
pub fn pass_pawns(position: &Position) -> Cp {
    // Base value of a passed pawn.
    const SCALAR: Cp = Cp(20);
    // Bonus value of passed pawn per rank. Pass pawns are very valuable on rank 7.
    const RANK_CP: [CpKind; NUM_RANKS] = [0, 0, 1, 2, 10, 50, 250, 900];
    let w_passed: Bitboard = pass_pawns_bb(position, White);
    let b_passed: Bitboard = pass_pawns_bb(position, Black);
    let w_num_passed = w_passed.count_squares() as i32;
    let b_num_passed = b_passed.count_squares() as i32;

    // Sum the bonus rank value of each pass pawn.
    let w_rank_bonus = w_passed
        .into_iter()
        .map(|sq| sq.rank())
        .fold(Cp(0), |acc, rank| acc + Cp(RANK_CP[rank as usize]));
    let b_rank_bonus = b_passed
        .into_iter()
        .map(|sq| sq.rank().flip())
        .fold(Cp(0), |acc, rank| acc + Cp(RANK_CP[rank as usize]));

    Cp(w_num_passed - b_num_passed) * SCALAR + w_rank_bonus - b_rank_bonus
}

/// Returns value from sliding pieces attacking opposing king on otherwise empty chessboard.
pub fn xray_king_attacks(position: &Position) -> Cp {
    // Base value of xray attackers.
    const SCALAR: Cp = Cp(8);
    let w_king = position.pieces[(White, King)].get_lowest_square().unwrap();
    let b_king = position.pieces[(Black, King)].get_lowest_square().unwrap();
    let w_king_ortho = Bitboard::from(w_king.file()) | Bitboard::from(w_king.rank());
    let b_king_ortho = Bitboard::from(b_king.file()) | Bitboard::from(b_king.rank());
    let w_king_diags = mg::bishop_pattern(w_king);
    let b_king_diags = mg::bishop_pattern(b_king);

    let w_diags = position.pieces[(White, Queen)] | position.pieces[(White, Bishop)];
    let b_diags = position.pieces[(Black, Queen)] | position.pieces[(Black, Bishop)];
    let w_ortho = position.pieces[(White, Queen)] | position.pieces[(White, Rook)];
    let b_ortho = position.pieces[(Black, Queen)] | position.pieces[(Black, Rook)];

    let w_xray_attackers_bb = (b_king_diags & w_diags) | (b_king_ortho & w_ortho);
    let b_xray_attackers_bb = (w_king_diags & b_diags) | (w_king_ortho & b_ortho);

    let w_xray_attackers: CpKind = w_xray_attackers_bb.count_squares() as CpKind;
    let b_xray_attackers: CpKind = b_xray_attackers_bb.count_squares() as CpKind;

    Cp(w_xray_attackers - b_xray_attackers) * SCALAR
}

/// A pass pawn is one with no opponent pawns in front of it on same or adjacent files.
/// This returns a bitboard with all pass pawns of given player.
#[inline]
fn pass_pawns_bb(position: &Position, player: Color) -> Bitboard {
    use Bitboard as Bb;

    let opponent_pawns = position.pieces[(!player, Pawn)];

    let spans = opponent_pawns
        .into_iter()
        .map(|sq| {
            let file = sq.file();
            let mut span = Bb::from(file);
            // Working with opponent pieces, so if finding w_pass, need to clear above sq.
            match player {
                Color::White => span.clear_square_and_above(sq),
                Color::Black => span.clear_square_and_below(sq),
            };

            span | span.to_east() | span.to_west()
        })
        .fold(Bitboard::EMPTY, |acc, bb| acc | bb);

    // Any pawn not in spans is a pass pawn.
    position.pieces[(player, Pawn)] & !spans
}

// Piece Square Tables
// TODO!

// Const Data Generation

/// Warning: Do not use, unfinished.
pub const PASS_PAWN_SIZE: usize = (NUM_SQUARES - 24) * 2;
pub const PASS_PAWN_PATTERN: [Bitboard; PASS_PAWN_SIZE] = generate_pass_pawn_pattern();

// Repeats the form: array[num] = func[num];
// where $array and $func are identifiers, followed by 1 or more literals to repeat on.
// Need to use a macro because loops are not allowed in const fn currently.
macro_rules! w_repeat_for_each {
    ($array:ident, $func:ident, $($numbers:literal),+) => {
        {
            $($array[$numbers - 8] = $func($numbers);)*
        }
    };
}

/// TODO:
/// FINISH FOR B_PAWNS.
/// Unfinished until eval is working.
/// NOTES:
/// pass_pawn_pattern does not need to be generated for:
/// * Rank 1 White (Pawns cannot be on squares)
/// * Rank 7/8 White (Cannot be blocked by pawns)
/// * Rank 8 Black ( Pawns cannot be on squares)
/// * Rank 1/2 Black (Pawns cannot be blocked by pawns)
const fn generate_pass_pawn_pattern() -> [Bitboard; PASS_PAWN_SIZE] {
    let mut array = [Bitboard::EMPTY; PASS_PAWN_SIZE];

    #[rustfmt::skip]
    w_repeat_for_each!(
        array,
        w_pass_pawn_pattern_idx,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47
    );

    array
}

const fn w_pass_pawn_pattern_idx(square: usize) -> Bitboard {
    use Bitboard as Bb;
    let square_bb: bitboard::Kind = 1u64 << square;

    if square_bb & Bitboard::FILE_A.0 > 0 {
        // On File A
        let mut pass_pawn_pat = Bitboard::FILE_A.0 | Bitboard::FILE_B.0;
        pass_pawn_pat &= !square_bb; // Remove idx square.
        pass_pawn_pat &= !(square_bb << 1); // Remove square to right of idx.
        if square != 0 {
            pass_pawn_pat &= !(square_bb - 1);
        }
        Bitboard(pass_pawn_pat)
    } else if square_bb & Bitboard::FILE_H.0 > 0 {
        // On File H
        let mut pass_pawn_pat = Bitboard::FILE_G.0 | Bitboard::FILE_H.0;
        pass_pawn_pat &= !(square_bb ^ (square_bb - 1)); // Remove square and below.
        Bitboard(pass_pawn_pat)
    } else {
        // Not Files A or H
        let mut pass_pawn_pat = match square_bb {
            bb if bb & Bb::FILE_B.0 > 0 => Bb::FILE_A.0 | Bb::FILE_B.0 | Bb::FILE_C.0,
            bb if bb & Bb::FILE_C.0 > 0 => Bb::FILE_B.0 | Bb::FILE_C.0 | Bb::FILE_D.0,
            bb if bb & Bb::FILE_D.0 > 0 => Bb::FILE_C.0 | Bb::FILE_D.0 | Bb::FILE_E.0,
            bb if bb & Bb::FILE_E.0 > 0 => Bb::FILE_D.0 | Bb::FILE_E.0 | Bb::FILE_F.0,
            bb if bb & Bb::FILE_F.0 > 0 => Bb::FILE_E.0 | Bb::FILE_F.0 | Bb::FILE_G.0,
            bb if bb & Bb::FILE_G.0 > 0 => Bb::FILE_F.0 | Bb::FILE_G.0 | Bb::FILE_H.0,
            _ => 0,
        };
        // Remove Rank of square and all below.
        pass_pawn_pat &= !(square_bb ^ (square_bb - 1)); // Remove square and below.
        pass_pawn_pat &= !(square_bb << 1);

        Bitboard(pass_pawn_pat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cp_min_and_max() {
        let min = Cp::MIN;
        let max = Cp::MAX;
        assert_eq!(min.signum(), -1);
        assert_eq!(max.signum(), 1);

        // Negated
        assert_eq!((-min).signum(), 1);
        assert_eq!((-max).signum(), -1);
    }
}