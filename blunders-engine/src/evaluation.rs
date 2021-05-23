//! Evaluation functions that return a centipawn.

use std::ops::{Add, AddAssign, Mul, Neg, Sub};

use crate::bitboard::{self, Bitboard};
use crate::coretypes::{Color::*, PieceKind::*};
use crate::coretypes::{PieceKind, Rank, NUM_RANKS, NUM_SQUARES};
use crate::movegen as mg;
use crate::position::Position;

/// Centipawn, a common unit of measurement in chess, where 100 Centipawn == 1 Pawn.
/// A positive centipawn value represent an advantage for White,
/// and a negative value represents an advantage for Black.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct Cp(pub(crate) CpKind);

// Type alias to make changing type easy if needed.
type CpKind = i32;

// Newtype pattern boilerplate
impl Cp {
    pub const MIN: Cp = Self(CpKind::MIN);
    pub const MAX: Cp = Self(CpKind::MAX);

    pub const fn new(value: CpKind) -> Self {
        Self(value)
    }

    pub const fn signum(&self) -> CpKind {
        self.0.signum()
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

impl PieceKind {
    /// Default, color independent value per piece.
    const fn centipawns(&self) -> Cp {
        Cp(match self {
            Pawn => 100,
            Knight => 300,
            Bishop => 300,
            Rook => 500,
            Queen => 900,
            King => 800_000,
        })
    }
}

//impl Piece {
//    const fn centipawns(&self) -> Cp {
//        Cp(match self.color {
//            White => self.piece_kind.centipawns().0,
//            Black => -self.piece_kind.centipawns().0,
//        })
//    }
//}

// Evaluation Constants
const CHECKMATE: Cp = Cp(Cp::MAX.0 / 2 - 1);

// Evaluation Functions

/// Primary evaluate function for engine.
pub fn static_evaluate(position: &Position, num_moves: usize) -> Cp {
    let cp_material = material(position);
    let cp_pass_pawns = pass_pawns(position);
    let cp_xray_king = xray_king_attacks(position);
    let cp_mobility = mobility(position, num_moves);

    let cp_total = cp_material + cp_pass_pawns + cp_xray_king + cp_mobility;
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

/// Return value of number of moves that can be made from a position.
/// This function handles checkmates and stalemates.
/// Currently treats stalemates as as losses.
pub fn mobility(position: &Position, num_moves: usize) -> Cp {
    let mut cp = Cp(0);
    if num_moves == 0 {
        // Checkmate or stalemate.
        cp = match position.player {
            White => -CHECKMATE,
            Black => CHECKMATE,
        };
    }
    cp
}

/// Returns Centipawn difference for passed pawns.
pub fn pass_pawns(position: &Position) -> Cp {
    // Base value of a passed pawn. Currently worth 50cp.
    const SCALAR: Cp = Cp(50);
    // Bonus value of passed pawn per rank. Pass pawns are very valuable on rank 7.
    const RANK_CP: [Cp; NUM_RANKS - 1] = [Cp(0), Cp(0), Cp(0), Cp(5), Cp(10), Cp(100), Cp(700)];
    let w_passed: Bitboard = w_pass_pawns(&position);
    let b_passed: Bitboard = b_pass_pawns(&position);
    let w_num_passed = w_passed.count_squares() as i32;
    let b_num_passed = b_passed.count_squares() as i32;

    let w_rank_bonus = {
        let mut bonus = Cp(0);
        for &rank in &[Rank::R4, Rank::R5, Rank::R6, Rank::R7] {
            bonus += RANK_CP[rank as usize] * (w_passed & Bitboard::from(rank)).count_squares();
        }
        bonus
    };
    let b_rank_bonus = {
        let mut bonus = Cp(0);
        for &rank in &[Rank::R4, Rank::R5, Rank::R6, Rank::R7] {
            bonus += RANK_CP[rank as usize] * (b_passed & Bitboard::from(rank)).count_squares();
        }
        bonus
    };
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

#[inline]
fn w_pass_pawns(position: &Position) -> Bitboard {
    let w_pawns = position.pieces[(White, Pawn)];
    let b_pawns = position.pieces[(Black, Pawn)];
    let mut w_passed = Bitboard::EMPTY;

    for w_pawn in w_pawns {
        let pawn_file = w_pawn.file();
        let mut passed_mask = Bitboard::from(pawn_file)
            | pawn_file
                .before()
                .map_or(Bitboard::EMPTY, |file| Bitboard::from(file))
            | pawn_file
                .after()
                .map_or(Bitboard::EMPTY, |file| Bitboard::from(file));

        // Remove all squares next to and below pawn square to get mask.
        let next_square = w_pawn.into_iter().next().unwrap();
        passed_mask.clear_square_and_below(next_square);

        if (passed_mask & b_pawns) == Bitboard::EMPTY {
            w_passed.set_square(w_pawn);
        }
    }

    w_passed
}

#[inline]
fn b_pass_pawns(position: &Position) -> Bitboard {
    let b_pawns = position.pieces[(Black, Pawn)];
    let w_pawns = position.pieces[(White, Pawn)];
    let mut b_passed = Bitboard::EMPTY;

    for b_pawn in b_pawns {
        let pawn_file = b_pawn.file();
        let mut passed_mask = Bitboard::from(pawn_file)
            | pawn_file
                .before()
                .map_or(Bitboard::EMPTY, |file| Bitboard::from(file))
            | pawn_file
                .after()
                .map_or(Bitboard::EMPTY, |file| Bitboard::from(file));

        // Remove all squares next to and above pawn square to get mask.
        passed_mask &= !Bitboard::from(b_pawn.rank());
        passed_mask.clear_square_and_above(b_pawn);

        if (passed_mask & w_pawns) == Bitboard::EMPTY {
            b_passed.set_square(b_pawn);
        }
    }

    b_passed
}

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
