//! Various functions to find attack rays for sliding pieces.

use crate::bitboard::Bitboard;
use crate::coretypes::Square;

// Each of 8-Directional rays, North, East, South, West, 4 Diagonals.

/// Given one of Bitboard::to_(north|south|east|west|noea|nowe|soea|sowe),
/// generate a ray from the origin exclusive to the first occupied piece inclusive along the ray direction.
#[inline(always)]
fn ray_scan(
    origin: Square,
    occupancy: Bitboard,
    direction_func: fn(&Bitboard) -> Bitboard,
) -> Bitboard {
    let mut ray = direction_func(&Bitboard::from(origin));
    for _ in 0..6 {
        if occupancy.has_any(ray) {
            return ray;
        }
        ray |= direction_func(&ray);
    }
    ray
}

/// Return all squares attacked in North-direction ray, stopping on first attacked piece.
pub(crate) fn north(origin: Square, occupancy: Bitboard) -> Bitboard {
    let rays = positive_xor_trick(origin, occupancy, Bitboard::from(origin.file()));
    debug_assert_eq!(rays, ray_scan(origin, occupancy, Bitboard::to_north));
    rays
}
/// Return all squares attacked in East-direction ray, stopping on first attacked piece.
pub(crate) fn east(origin: Square, occupancy: Bitboard) -> Bitboard {
    let rays = positive_xor_trick(origin, occupancy, Bitboard::from(origin.rank()));
    debug_assert_eq!(rays, ray_scan(origin, occupancy, Bitboard::to_east));
    rays
}
/// Return all squares attacked in South-direction ray, stopping on first attacked piece.
pub(crate) fn south(origin: Square, occupancy: Bitboard) -> Bitboard {
    ray_scan(origin, occupancy, Bitboard::to_south)
}
/// Return all squares attacked in North-direction ray, stopping on first attacked piece.
pub(crate) fn west(origin: Square, occupancy: Bitboard) -> Bitboard {
    ray_scan(origin, occupancy, Bitboard::to_west)
}
/// Return all squares attacked in NorthEast-direction ray, stopping on first attacked piece.
pub(crate) fn noea(origin: Square, occupancy: Bitboard) -> Bitboard {
    ray_scan(origin, occupancy, Bitboard::to_north_east)
}
/// Return all squares attacked in SouthEast-direction ray, stopping on first attacked piece.
pub(crate) fn soea(origin: Square, occupancy: Bitboard) -> Bitboard {
    ray_scan(origin, occupancy, Bitboard::to_south_east)
}
/// Return all squares attacked in SouthWest-direction ray, stopping on first attacked piece.
pub(crate) fn sowe(origin: Square, occupancy: Bitboard) -> Bitboard {
    ray_scan(origin, occupancy, Bitboard::to_south_west)
}
/// Return all squares attacked in NorthWest-direction ray, stopping on first attacked piece.
pub(crate) fn nowe(origin: Square, occupancy: Bitboard) -> Bitboard {
    ray_scan(origin, occupancy, Bitboard::to_north_west)
}

/// Bit trick known as [o^(o-2r)](https://www.chessprogramming.org/Subtracting_a_Rook_from_a_Blocking_Piece).
/// Only works on a single positive ray at a time, and with a specific bit layout for square indices in a bitboard.
/// Algorithm:
///
/// Arguments:
/// * `origin` - Square of sliding piece.
/// * `occupancy` - Occupancy bitboard of all pieces.
/// * `mask` - file, row, or diagonal of positive ray to find.
#[inline(always)]
pub(crate) fn positive_xor_trick(origin: Square, occupancy: Bitboard, mask: Bitboard) -> Bitboard {
    let (origin, occ, mask) = (Bitboard::from(origin).0, occupancy.0, mask.0);
    let potential_blockers = occ & mask;
    let diff = potential_blockers.wrapping_sub(origin.wrapping_mul(2));
    let changed = diff ^ occ;
    let ray = Bitboard(changed & mask);

    debug_assert!(ray.len() <= 7, "No ray can attack more than 7 squares.");
    ray
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use super::*;
    use crate::Square::*;

    fn ray_tester(
        origin: Square,
        occ: Bitboard,
        ray_funcs: [fn(Square, Bitboard) -> Bitboard; 8],
        ray_squares: Vec<Vec<Square>>,
    ) {
        assert_eq!(ray_funcs.len(), ray_squares.len());

        for (ray_func, ray_square) in zip(ray_funcs, ray_squares) {
            let ray = ray_func(origin, occ);
            assert_eq!(ray.len(), ray_square.len());
            for sq in ray_square {
                assert!(ray.has_square(sq));
            }
        }
    }

    #[test]
    fn empty_occupancy_rays() {
        let origin = D4;
        let occ = Bitboard::EMPTY;
        let ray_funcs = [north, south, east, west, noea, nowe, soea, sowe];
        let ray_squares = vec![
            vec![D5, D6, D7, D8],
            vec![D3, D2, D1],
            vec![E4, F4, G4, H4],
            vec![A4, B4, C4],
            vec![E5, F6, G7, H8],
            vec![C5, B6, A7],
            vec![E3, F2, G1],
            vec![C3, B2, A1],
        ];
        ray_tester(origin, occ, ray_funcs, ray_squares);
    }

    #[test]
    fn occupied_rays() {
        let origin = E5;
        let occ = Bitboard::from(vec![D2, D4, A5, G5, E8].as_slice());
        let ray_funcs = [north, south, east, west, noea, nowe, soea, sowe];
        let ray_squares = vec![
            vec![E6, E7, E8],
            vec![E4, E3, E2, E1],
            vec![F5, G5],
            vec![D5, C5, B5, A5],
            vec![F6, G7, H8],
            vec![D6, C7, B8],
            vec![F4, G3, H2],
            vec![D4],
        ];
        ray_tester(origin, occ, ray_funcs, ray_squares);
    }

    #[test]
    fn corner_rays() {
        let occ = Bitboard::EMPTY;
        let ray_funcs = [north, south, east, west, noea, nowe, soea, sowe];
        {
            let origin = A1;
            let ray_squares = vec![
                vec![A2, A3, A4, A5, A6, A7, A8],
                vec![],
                vec![B1, C1, D1, E1, F1, G1, H1],
                vec![],
                vec![B2, C3, D4, E5, F6, G7, H8],
                vec![],
                vec![],
                vec![],
            ];
            ray_tester(origin, occ, ray_funcs, ray_squares);
        }
        {
            let origin = H8;
            let ray_squares = vec![
                vec![],
                vec![H1, H2, H3, H4, H5, H6, H7],
                vec![],
                vec![A8, B8, C8, D8, E8, F8, G8],
                vec![],
                vec![],
                vec![],
                vec![A1, B2, C3, D4, E5, F6, G7],
            ];
            ray_tester(origin, occ, ray_funcs, ray_squares);
        }
    }
}
