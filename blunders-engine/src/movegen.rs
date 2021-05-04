//! Functions and constants used to help with generating moves for a position.

// TODO:
// Pawn move generation for white / black.

use crate::bitboard::Bitboard;
use crate::coretypes::{Color, NUM_SQUARES};

// Single Piece, Square Indexed, Symmetrical, Attacks == pseudo-legal Moves
pub const KNIGHT_PATTERN: [Bitboard; NUM_SQUARES] = generate_knight_patterns();
// Single Piece, Square Indexed, Symmetrical. Attacks == pseudo-legal Moves
pub const KING_PATTERN: [Bitboard; NUM_SQUARES] = generate_king_patterns();
// Single Piece, Square Indexed, Symmetrical. Attacks == pseudo-legal Moves on empty board.
pub const ROOK_PATTERN: [Bitboard; NUM_SQUARES] = generate_rook_patterns();
// Single Piece, Square Indexed, Symmetrical. Attacks == pseudo-legal Moves on empty board.
pub const BISHOP_PATTERN: [Bitboard; NUM_SQUARES] = generate_bishop_patterns();
// Single Piece, Square Indexed, Symmetrical. Attacks == pseudo-legal Moves on empty board.
pub const QUEEN_PATTERN: [Bitboard; NUM_SQUARES] = generate_queen_patterns();

// Repeats the form: array[num] = func[num];
// where $array and $func are identifiers, followed by 1 or more literals to repeat on.
// Need to use a macro because loops are not allowed in const fn currently.
macro_rules! repeat_for_each {
    ($array:ident, $func:ident, $($numbers:literal),+) => {
        {
            $($array[$numbers] = $func($numbers);)*
        }
    };
}

/// Generates an array containing a knight attack/move pattern bitboard for each square.
/// Knights move/attack in L shaped pattern.
const fn generate_knight_patterns() -> [Bitboard; NUM_SQUARES] {
    let mut pattern_arr = [Bitboard::EMPTY; NUM_SQUARES];
    #[rustfmt::skip]
    repeat_for_each!(
        pattern_arr,
        knight_pattern_index,
        0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63
    );
    pattern_arr
}

/// Generate bitboard of knight moves/attacks for a single square.
const fn knight_pattern_index(index: usize) -> Bitboard {
    let index_bb = Bitboard(1u64 << index);
    let mut bb = Bitboard::EMPTY;

    bb.0 |= index_bb.to_north().to_north().to_east().0;
    bb.0 |= index_bb.to_north().to_east().to_east().0;
    bb.0 |= index_bb.to_south().to_east().to_east().0;
    bb.0 |= index_bb.to_south().to_south().to_east().0;

    bb.0 |= index_bb.to_south().to_south().to_west().0;
    bb.0 |= index_bb.to_south().to_west().to_west().0;
    bb.0 |= index_bb.to_north().to_west().to_west().0;
    bb.0 |= index_bb.to_north().to_north().to_west().0;

    bb
}

/// Generates an array containing a king move/attack pattern bitboard for each square.
/// Kings move/attack all surrounding squares orthogonally and diagonally.
const fn generate_king_patterns() -> [Bitboard; NUM_SQUARES] {
    let mut pattern_arr = [Bitboard::EMPTY; NUM_SQUARES];
    #[rustfmt::skip]
    repeat_for_each!(
        pattern_arr,
        king_pattern_index,
        0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63
    );
    pattern_arr
}

/// Generate bitboard of king moves/attacks for a single square.
const fn king_pattern_index(index: usize) -> Bitboard {
    let mut index_bb = Bitboard(1u64 << index);
    let mut bb = Bitboard(index_bb.to_west().0 | index_bb.to_east().0);
    index_bb.0 |= bb.0;
    bb.0 |= index_bb.to_north().0;
    bb.0 |= index_bb.to_south().0;

    bb
}

/// Generate an array containing a Rook move/attack pattern bitboard for each square,
/// on an otherwise empty chess board.
/// Rooks move/attack all squares on their file and rank.
const fn generate_rook_patterns() -> [Bitboard; NUM_SQUARES] {
    let mut pattern_arr = [Bitboard::EMPTY; NUM_SQUARES];
    #[rustfmt::skip]
    repeat_for_each!(
        pattern_arr,
        rook_pattern_index,
        0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63
    );
    pattern_arr
}

macro_rules! repeat_6_times {
    ($statement:stmt) => {
        $statement
        $statement
        $statement
        $statement
        $statement
        $statement
    };
}

/// Generate bitboard of rook moves/attacks for single square.
const fn rook_pattern_index(index: usize) -> Bitboard {
    let index_bb = Bitboard(1u64 << index);

    // Shift index a total of 7 times in each direction to get all possible moves/attacks.
    let mut north_bit_vec = index_bb.to_north();
    repeat_6_times!(north_bit_vec.0 |= north_bit_vec.to_north().0);
    let mut south_bit_vec = index_bb.to_south();
    repeat_6_times!(south_bit_vec.0 |= south_bit_vec.to_south().0);
    let mut east_bit_vec = index_bb.to_east();
    repeat_6_times!(east_bit_vec.0 |= east_bit_vec.to_east().0);
    let mut west_bit_vec = index_bb.to_west();
    repeat_6_times!(west_bit_vec.0 |= west_bit_vec.to_west().0);

    Bitboard(north_bit_vec.0 | south_bit_vec.0 | east_bit_vec.0 | west_bit_vec.0)
}

/// Generate an array containing a Bishop move/attack pattern bitboard for each square,
/// on an otherwise empty chess board.
/// Bishops move/attack all squares on their diagonal and anti-diagonal.
const fn generate_bishop_patterns() -> [Bitboard; NUM_SQUARES] {
    let mut pattern_arr = [Bitboard::EMPTY; NUM_SQUARES];
    #[rustfmt::skip]
    repeat_for_each!(
        pattern_arr,
        bishop_pattern_index,
        0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63
    );
    pattern_arr
}

/// Generate bitboard of bishop moves/attacks for single square.
const fn bishop_pattern_index(index: usize) -> Bitboard {
    let index_bb = Bitboard(1u64 << index);

    // Shift index a total of 7 times in each direction to get all possible moves/attacks.
    let mut no_ea_bit_vec = index_bb.to_north().to_east();
    repeat_6_times!(no_ea_bit_vec.0 |= no_ea_bit_vec.to_north().to_east().0);

    let mut so_ea_bit_vec = index_bb.to_south().to_east();
    repeat_6_times!(so_ea_bit_vec.0 |= so_ea_bit_vec.to_south().to_east().0);

    let mut so_we_bit_vec = index_bb.to_south().to_west();
    repeat_6_times!(so_we_bit_vec.0 |= so_we_bit_vec.to_south().to_west().0);

    let mut no_we_bit_vec = index_bb.to_north().to_west();
    repeat_6_times!(no_we_bit_vec.0 |= no_we_bit_vec.to_north().to_west().0);

    Bitboard(no_ea_bit_vec.0 | so_ea_bit_vec.0 | so_we_bit_vec.0 | no_we_bit_vec.0)
}

/// Generate an array containing a Queen move/attack pattern bitboard for each square,
/// on an otherwise empty chess board.
/// Queens move/attack all squares on their file, rank, diagonal, and anti-diagonal.
const fn generate_queen_patterns() -> [Bitboard; NUM_SQUARES] {
    let mut pattern_arr = [Bitboard::EMPTY; NUM_SQUARES];
    #[rustfmt::skip]
    repeat_for_each!(
        pattern_arr,
        queen_pattern_index,
        0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39,
        40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55,
        56, 57, 58, 59, 60, 61, 62, 63
    );
    pattern_arr
}

/// Generate bitboard of queen moves/attacks for single square.
const fn queen_pattern_index(index: usize) -> Bitboard {
    Bitboard(ROOK_PATTERN[index].0 | BISHOP_PATTERN[index].0)
}

/// Generate pseudo-legal moves for all pawns of a color.
/// Currently generating separately per color because moves are not symmetrical.
pub fn pawn_pseudo_moves(pawns: &Bitboard, color: &Color) -> Bitboard {
    // Single pushes are easy to generate, by pushing 1 square forward.
    let single_push_bb = match color {
        Color::White => pawns.to_north(),
        Color::Black => pawns.to_south(),
    };
    // Double pushes are generated only from pawns on starting rank.
    let double_push_bb = match color {
        Color::White => (pawns & Bitboard::RANK_2).to_north().to_north(),
        Color::Black => (pawns & Bitboard::RANK_7).to_south().to_south(),
    };
    single_push_bb | double_push_bb
}

/// Generate attacks for all pawns in Bitboard for a color.
pub fn pawn_attacks(pawns: &Bitboard, color: &Color) -> Bitboard {
    match color {
        Color::White => pawns.to_north().to_east() | pawns.to_north().to_west(),
        Color::Black => pawns.to_south().to_east() | pawns.to_south().to_west(),
    }
}

/// Generate only double attacks from pawns in Bitboard for a color.
pub fn pawn_double_attacks(pawns: &Bitboard, color: &Color) -> Bitboard {
    // double attacks are only possible if East and West attacks attack same square.
    match color {
        Color::White => pawns.to_north().to_east() & pawns.to_north().to_west(),
        Color::Black => pawns.to_south().to_east() & pawns.to_south().to_west(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::Square::*;
    use crate::coretypes::*;

    #[test]
    fn check_knight_patterns() {
        let a1 = KNIGHT_PATTERN[A1.idx()];
        println!("a1: {:?}", a1);
        println!("a1 knight attack squares: {:?}", a1.squares());
        assert_eq!(a1.count_squares(), 2);
        assert!(a1.has_square(C2));
        assert!(a1.has_square(B3));

        let h1 = KNIGHT_PATTERN[H1.idx()];
        assert_eq!(h1.count_squares(), 2);
        assert!(h1.has_square(F2));
        assert!(h1.has_square(G3));

        let h8 = KNIGHT_PATTERN[H8.idx()];
        assert_eq!(h8.count_squares(), 2);
        assert!(h8.has_square(F7));
        assert!(h8.has_square(G6));

        let d4 = KNIGHT_PATTERN[D4.idx()];
        assert_eq!(d4.count_squares(), 8);
        for &square in &[E6, F5, F3, E2, C2, B3, B5, C6] {
            assert!(d4.has_square(square));
        }
    }

    #[test]
    fn check_king_patterns() {
        {
            let a1 = KING_PATTERN[A1.idx()];
            assert_eq!(a1.count_squares(), 3);
            assert!(a1.has_square(A2));
            assert!(a1.has_square(B2));
            assert!(a1.has_square(B1));
        }
        {
            let a8 = KING_PATTERN[A8.idx()];
            assert_eq!(a8.count_squares(), 3);
            assert!(a8.has_square(A7));
            assert!(a8.has_square(B7));
            assert!(a8.has_square(B8));
        }
        {
            let h1 = KING_PATTERN[H1.idx()];
            assert_eq!(h1.count_squares(), 3);
            assert!(h1.has_square(G1));
            assert!(h1.has_square(G2));
            assert!(h1.has_square(H2));
        }
        {
            let h8 = KING_PATTERN[H8.idx()];
            assert_eq!(h8.count_squares(), 3);
            assert!(h8.has_square(G7));
            assert!(h8.has_square(G8));
            assert!(h8.has_square(H7));
        }
        {
            let d6 = KING_PATTERN[D6.idx()];
            assert_eq!(d6.count_squares(), 8);
            for &square in &[C5, C6, C7, D5, D7, E5, E6, E7] {
                assert!(d6.has_square(square));
            }
        }
    }

    #[test]
    fn check_rook_patterns() {
        {
            let a1 = ROOK_PATTERN[A1.idx()];
            assert_eq!(a1.count_squares(), 14);
            for &square in &[A2, A3, A4, A5, A6, A7, A8, B1, C1, D1, E1, F1, G1, H1] {
                assert!(a1.has_square(square));
            }
        }
        {
            let h8 = ROOK_PATTERN[H8.idx()];
            assert_eq!(h8.count_squares(), 14);
            for &square in &[A8, B8, C8, D8, E8, F8, G8, H1, H2, H3, H4, H5, H6, H7] {
                assert!(h8.has_square(square));
            }
        }
        {
            let f3 = ROOK_PATTERN[F3.idx()];
            assert_eq!(f3.count_squares(), 14);
            for &square in &[A3, B3, C3, D3, E3, G3, H3, F1, F2, F4, F5, F6, F7, F8] {
                assert!(f3.has_square(square));
            }
        }
    }

    #[test]
    fn check_bishop_patterns() {
        {
            let a1 = BISHOP_PATTERN[A1.idx()];
            assert_eq!(a1.count_squares(), 7);
            for &square in &[B2, C3, D4, E5, F6, G7, H8] {
                assert!(a1.has_square(square));
            }
        }
        {
            let h1 = BISHOP_PATTERN[H1.idx()];
            assert_eq!(h1.count_squares(), 7);
            for &square in &[A8, B7, C6, D5, E4, F3, G2] {
                assert!(h1.has_square(square));
            }
        }
        {
            let h8 = BISHOP_PATTERN[H8.idx()];
            assert_eq!(h8.count_squares(), 7);
            for &square in &[A1, B2, C3, D4, E5, F6, G7] {
                assert!(h8.has_square(square));
            }
        }
        {
            let c6 = BISHOP_PATTERN[C6.idx()];
            assert_eq!(c6.count_squares(), 11);
            for &square in &[A4, B5, D7, E8, A8, B7, D5, E4, F3, G2, H1] {
                assert!(c6.has_square(square));
            }
        }
    }
    #[test]
    fn check_queen_patterns() {
        {
            let a1 = QUEEN_PATTERN[A1.idx()];
            assert_eq!(a1.count_squares(), 21);
            for &square in &[B1, C1, D1, E1, F1, G1, H1, A2, A3, A4, A5, A6, A7, A8] {
                assert!(a1.has_square(square)); // Orthogonal squares.
            }
            for &square in &[B2, C3, D4, E5, F6, G7, H8] {
                assert!(a1.has_square(square)); // Diagonal squares.
            }
        }
        {
            let h1 = QUEEN_PATTERN[H1.idx()];
            assert_eq!(h1.count_squares(), 21);
            for &square in &[A1, B1, C1, D1, E1, F1, G1, H2, H3, H4, H5, H6, H7, H8] {
                assert!(h1.has_square(square)); // Orthogonal squares.
            }
            for &square in &[A8, B7, C6, D5, E4, F3, G2] {
                assert!(h1.has_square(square)); // Diagonal squares.
            }
        }
        {
            let c6 = QUEEN_PATTERN[C6.idx()];
            assert_eq!(c6.count_squares(), 25);
            for &square in &[C1, C2, C3, C4, C5, C7, C8, A6, B6, D6, E6, F6, G6, H6] {
                assert!(c6.has_square(square)); // Orthogonal squares.
            }
            for &square in &[A4, B5, D7, E8, A8, B7, D5, E4, F3, G2, H1] {
                assert!(c6.has_square(square)); // Diagonal squares.
            }
        }
    }

    #[test]
    fn check_pawn_pseudo_moves() {
        {
            // B pawn at end of file has no moves.
            let a1 = Bitboard::from(A1);
            let a1_moves = pawn_pseudo_moves(&a1, &Color::Black);
            assert_eq!(a1_moves.count_squares(), 0);
        }
        {
            // W pawn on starting row has 2 moves, B pawn has 1.
            let a2 = Bitboard::from(A2);
            let a2_moves = pawn_pseudo_moves(&a2, &Color::White);
            assert_eq!(a2_moves.count_squares(), 2);
            assert!(a2_moves.has_square(A3));
            assert!(a2_moves.has_square(A4));
            let a2_moves = pawn_pseudo_moves(&a2, &Color::Black);
            assert_eq!(a2_moves.count_squares(), 1);
            assert!(a2_moves.has_square(A1));
        }
        {
            let f3 = Bitboard::from(F3);
            let f3_moves = pawn_pseudo_moves(&f3, &Color::White);
            assert_eq!(f3_moves.count_squares(), 1);
            assert!(f3_moves.has_square(F4));
            let f3_moves = pawn_pseudo_moves(&f3, &Color::Black);
            assert_eq!(f3_moves.count_squares(), 1);
            assert!(f3_moves.has_square(F2));
        }
        {
            let h7 = Bitboard::from(H7);
            let h7_moves = pawn_pseudo_moves(&h7, &Color::White);
            assert_eq!(h7_moves.count_squares(), 1);
            assert!(h7_moves.has_square(H8));
            let h7_moves = pawn_pseudo_moves(&h7, &Color::Black);
            assert_eq!(h7_moves.count_squares(), 2);
            assert!(h7_moves.has_square(H6));
            assert!(h7_moves.has_square(H5));
        }
        {
            let pawns = Bitboard::from(vec![B2, C3, F7, H8].as_slice());
            let w_pawn_moves = pawn_pseudo_moves(&pawns, &Color::White);
            assert_eq!(w_pawn_moves.count_squares(), 4);
            assert!(w_pawn_moves.has_square(B3));
            assert!(w_pawn_moves.has_square(B4));
            assert!(w_pawn_moves.has_square(C4));
            assert!(w_pawn_moves.has_square(F8));

            let b_pawn_moves = pawn_pseudo_moves(&pawns, &Color::Black);
            assert_eq!(b_pawn_moves.count_squares(), 5);
            assert!(b_pawn_moves.has_square(B1));
            assert!(b_pawn_moves.has_square(C2));
            assert!(b_pawn_moves.has_square(F6));
            assert!(b_pawn_moves.has_square(F5));
            assert!(b_pawn_moves.has_square(H7));
        }
    }
    #[test]
    fn check_pawn_attacks() {
        {
            let c2 = Bitboard::from(C2);
            let c2_attacks = pawn_attacks(&c2, &Color::White);
            assert_eq!(c2_attacks.count_squares(), 2);
            assert!(c2_attacks.has_square(B3));
            assert!(c2_attacks.has_square(D3));
            let c2_attacks = pawn_attacks(&c2, &Color::Black);
            assert_eq!(c2_attacks.count_squares(), 2);
            assert!(c2_attacks.has_square(B1));
            assert!(c2_attacks.has_square(D1));
        }
        {
            let a1 = Bitboard::from(A1);
            let a1_attacks = pawn_attacks(&a1, &Color::White);
            assert_eq!(a1_attacks.count_squares(), 1);
            assert!(a1_attacks.has_square(B2));
            let a1_attacks = pawn_attacks(&a1, &Color::Black);
            assert_eq!(a1_attacks.count_squares(), 0);
        }
    }
}
