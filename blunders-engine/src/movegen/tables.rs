//! Various lookup tables useful for move generation.

// TODO: Consider removing down the line.
// Some functions are unused but complete symmetry for all piece types.
#![allow(dead_code)]

use crate::bitboard::Bitboard;
use crate::coretypes::{SquareIndexable, NUM_SQUARES};

///////////////////////////////////
// Pre-generated move/attack Lookup
//
// Bitboards representing how each piece type moves and attacks on an otherwise empty board.
// Arrays are indexed by Square's discriminant.

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

///////////////////////////////////////
// Runtime Move Generation Functions //
///////////////////////////////////////

/// Convenience function for pre-generated lookup array.
pub fn knight_pattern<I: SquareIndexable>(idx: I) -> Bitboard {
    KNIGHT_PATTERN[idx.idx()]
}
/// Convenience function for pre-generated lookup array.
pub fn king_pattern<I: SquareIndexable>(idx: I) -> Bitboard {
    KING_PATTERN[idx.idx()]
}
/// Convenience function for pre-generated lookup array.
pub fn rook_pattern<I: SquareIndexable>(idx: I) -> Bitboard {
    ROOK_PATTERN[idx.idx()]
}
/// Convenience function for pre-generated lookup array.
pub fn bishop_pattern<I: SquareIndexable>(idx: I) -> Bitboard {
    BISHOP_PATTERN[idx.idx()]
}
/// Convenience function for pre-generated lookup array.
pub fn queen_pattern<I: SquareIndexable>(idx: I) -> Bitboard {
    ROOK_PATTERN[idx.idx()] | BISHOP_PATTERN[idx.idx()]
}

//////////////////////////////////////
// Generate Constant Lookup Helpers //
//////////////////////////////////////

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
    let mut no_ea_bit_vec = index_bb.to_north_east();
    repeat_6_times!(no_ea_bit_vec.0 |= no_ea_bit_vec.to_north_east().0);

    let mut so_ea_bit_vec = index_bb.to_south_east();
    repeat_6_times!(so_ea_bit_vec.0 |= so_ea_bit_vec.to_south_east().0);

    let mut so_we_bit_vec = index_bb.to_south_west();
    repeat_6_times!(so_we_bit_vec.0 |= so_we_bit_vec.to_south_west().0);

    let mut no_we_bit_vec = index_bb.to_north_west();
    repeat_6_times!(no_we_bit_vec.0 |= no_we_bit_vec.to_north_west().0);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::{Square, Square::*};

    #[test]
    fn check_knight_patterns() {
        let a1 = KNIGHT_PATTERN[A1.idx()];
        println!("a1: {:?}", a1);
        println!("a1 knight attack squares: {:?}", a1.squares());
        assert_eq!(a1.len(), 2);
        assert!(a1.has_square(C2));
        assert!(a1.has_square(B3));

        let h1 = KNIGHT_PATTERN[H1.idx()];
        assert_eq!(h1.len(), 2);
        assert!(h1.has_square(F2));
        assert!(h1.has_square(G3));

        let h8 = KNIGHT_PATTERN[H8.idx()];
        assert_eq!(h8.len(), 2);
        assert!(h8.has_square(F7));
        assert!(h8.has_square(G6));

        let d4 = KNIGHT_PATTERN[D4.idx()];
        assert_eq!(d4.len(), 8);
        for square in [E6, F5, F3, E2, C2, B3, B5, C6] {
            assert!(d4.has_square(&square));
        }

        // Does not attack own square.
        for square in Square::iter() {
            assert!(!knight_pattern(square).has_square(square));
        }
    }

    #[test]
    fn check_king_patterns() {
        {
            let a1 = KING_PATTERN[A1.idx()];
            assert_eq!(a1.len(), 3);
            assert!(a1.has_square(A2));
            assert!(a1.has_square(B2));
            assert!(a1.has_square(B1));
        }
        {
            let a8 = KING_PATTERN[A8.idx()];
            assert_eq!(a8.len(), 3);
            assert!(a8.has_square(A7));
            assert!(a8.has_square(B7));
            assert!(a8.has_square(B8));
        }
        {
            let h1 = KING_PATTERN[H1.idx()];
            assert_eq!(h1.len(), 3);
            assert!(h1.has_square(G1));
            assert!(h1.has_square(G2));
            assert!(h1.has_square(H2));
        }
        {
            let h8 = KING_PATTERN[H8.idx()];
            assert_eq!(h8.len(), 3);
            assert!(h8.has_square(G7));
            assert!(h8.has_square(G8));
            assert!(h8.has_square(H7));
        }
        {
            let d6 = KING_PATTERN[D6.idx()];
            assert_eq!(d6.len(), 8);
            for square in [C5, C6, C7, D5, D7, E5, E6, E7] {
                assert!(d6.has_square(&square));
            }
        }
        // Does not attack own square.
        for square in Square::iter() {
            assert!(!king_pattern(square).has_square(square));
        }
    }

    #[test]
    fn check_rook_patterns() {
        {
            let a1 = ROOK_PATTERN[A1.idx()];
            assert_eq!(a1.len(), 14);
            for square in [A2, A3, A4, A5, A6, A7, A8, B1, C1, D1, E1, F1, G1, H1] {
                assert!(a1.has_square(&square));
            }
        }
        {
            let h8 = ROOK_PATTERN[H8.idx()];
            assert_eq!(h8.len(), 14);
            for square in [A8, B8, C8, D8, E8, F8, G8, H1, H2, H3, H4, H5, H6, H7] {
                assert!(h8.has_square(&square));
            }
        }
        {
            let f3 = ROOK_PATTERN[F3.idx()];
            assert_eq!(f3.len(), 14);
            for square in [A3, B3, C3, D3, E3, G3, H3, F1, F2, F4, F5, F6, F7, F8] {
                assert!(f3.has_square(&square));
            }
        }
        // Does not attack own square.
        for square in Square::iter() {
            assert!(!rook_pattern(square).has_square(square));
        }
    }

    #[test]
    fn check_bishop_patterns() {
        {
            let a1 = BISHOP_PATTERN[A1.idx()];
            assert_eq!(a1.len(), 7);
            for square in [B2, C3, D4, E5, F6, G7, H8] {
                assert!(a1.has_square(&square));
            }
        }
        {
            let h1 = BISHOP_PATTERN[H1.idx()];
            assert_eq!(h1.len(), 7);
            for square in [A8, B7, C6, D5, E4, F3, G2] {
                assert!(h1.has_square(&square));
            }
        }
        {
            let h8 = BISHOP_PATTERN[H8.idx()];
            assert_eq!(h8.len(), 7);
            for square in [A1, B2, C3, D4, E5, F6, G7] {
                assert!(h8.has_square(&square));
            }
        }
        {
            let c6 = BISHOP_PATTERN[C6.idx()];
            assert_eq!(c6.len(), 11);
            for square in [A4, B5, D7, E8, A8, B7, D5, E4, F3, G2, H1] {
                assert!(c6.has_square(&square));
            }
        }
        // Does not attack own square.
        for square in Square::iter() {
            assert!(!bishop_pattern(square).has_square(square));
        }
    }
    #[test]
    fn check_queen_patterns() {
        {
            let a1 = QUEEN_PATTERN[A1.idx()];
            assert_eq!(a1.len(), 21);
            for square in [B1, C1, D1, E1, F1, G1, H1, A2, A3, A4, A5, A6, A7, A8] {
                assert!(a1.has_square(&square)); // Orthogonal squares.
            }
            for square in [B2, C3, D4, E5, F6, G7, H8] {
                assert!(a1.has_square(&square)); // Diagonal squares.
            }
        }
        {
            let h1 = QUEEN_PATTERN[H1.idx()];
            assert_eq!(h1.len(), 21);
            for square in [A1, B1, C1, D1, E1, F1, G1, H2, H3, H4, H5, H6, H7, H8] {
                assert!(h1.has_square(&square)); // Orthogonal squares.
            }
            for square in [A8, B7, C6, D5, E4, F3, G2] {
                assert!(h1.has_square(&square)); // Diagonal squares.
            }
        }
        {
            let c6 = QUEEN_PATTERN[C6.idx()];
            assert_eq!(c6.len(), 25);
            for square in [C1, C2, C3, C4, C5, C7, C8, A6, B6, D6, E6, F6, G6, H6] {
                assert!(c6.has_square(&square)); // Orthogonal squares.
            }
            for square in [A4, B5, D7, E8, A8, B7, D5, E4, F3, G2, H1] {
                assert!(c6.has_square(&square)); // Diagonal squares.
            }
        }
        // Does not attack own square.
        for square in Square::iter() {
            assert!(!queen_pattern(square).has_square(square));
        }
    }
}
