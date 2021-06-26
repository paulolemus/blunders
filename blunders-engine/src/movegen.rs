//! Functions and constants used to help with generating moves for a position.

// Temporary. TODO: Delete this
#![allow(dead_code)]

use crate::bitboard::Bitboard;
use crate::coretypes::{Castling, Color, Move, Square, SquareIndexable, NUM_SQUARES};
use crate::coretypes::{Color::*, PieceKind::*, Square::*};
use crate::movelist::MoveList;

//////////////////////////////////////
// Pre-generated move/attack Lookup //
//////////////////////////////////////

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

/// Absolute pins are where a piece is pinned to its same color king.
/// Finding absolute pins are necessary to legal move generation.
/// An absolutely pinned piece may only move along its pin direction.
/// # Parameters
/// * king: Square of king to get for pins against.
/// * us: Bitboard with occupancy of pieces of king's color.
/// * them: Bitboard with occupancy of pinning player's color.
/// * queens_rooks: Bitboard with positions of pinning player's queens and rooks.
/// * queens_bishops: Bitboard with positions of pinning player's queens and bishops.
/// Return value: (pinned, pinned_square_moves)
/// pinned -> A Bitboard with all the pinned pieces.
/// pinned_square_moves -> A mapping of a pinned_square to squares along its pin direction.
pub fn absolute_pins(
    king: Square,
    us: Bitboard,
    them: Bitboard,
    queens_rooks: Bitboard,
    queens_bishops: Bitboard,
) -> (Bitboard, [Option<(Square, Bitboard)>; 8]) {
    // There can be a maximum of 8 pins at a time.
    // Squares that an absolutely pinned piece can move to are squares
    // up to and including the pinning piece, and up to the king.
    // Algorithm:
    // Treat the king as both a rook and a bishop.
    // For orthogonal and then diagonal directions, send out a ray attack stopping at first piece hit.
    // If a same color piece was hit, it could potentially be absolutely pinned. If opposite color, no pins.
    // For each potentially pinned piece, remove it from occupancy, and then send a ray again.
    // If this new ray hits a piece in the enemy sliding piece bb, then that initial piece is pinned.
    let mut pinned = Bitboard::EMPTY;
    let mut pinned_between: [Option<(Square, Bitboard)>; 8] = [None; 8];
    let mut index = 0;
    let occupied = us | them;

    for ortho_ray in [ray_attack_no, ray_attack_ea, ray_attack_so, ray_attack_we] {
        let maybe_pinned = ortho_ray(&king, &occupied) & us; // Bb of single own piece (potentially pinned), or empty.
        if !maybe_pinned.is_empty() {
            let ray_without_pinned = ortho_ray(&king, &(occupied ^ maybe_pinned));
            let hits_queen_rook = ray_without_pinned & queens_rooks;
            if !hits_queen_rook.is_empty() {
                // Piece is pinned, store piece and its legal moves.
                pinned |= maybe_pinned;
                let pinned_square = maybe_pinned.get_lowest_square().unwrap();
                let potential_moves_bb = ray_without_pinned ^ maybe_pinned;
                pinned_between[index] = Some((pinned_square, potential_moves_bb));
                index += 1;
            }
        }
    }

    for diag_ray in [
        ray_attack_noea,
        ray_attack_nowe,
        ray_attack_soea,
        ray_attack_sowe,
    ] {
        let maybe_pinned = diag_ray(&king, &occupied) & us; // Bb of possible single own piece (potentially pinned).
        if !maybe_pinned.is_empty() {
            let ray_without_pinned = diag_ray(&king, &(occupied ^ maybe_pinned));
            let hits_queen_bishop = ray_without_pinned & queens_bishops;
            if !hits_queen_bishop.is_empty() {
                // Piece is pinned, store piece and its legal moves.
                pinned |= maybe_pinned;
                let pinned_square = maybe_pinned.get_lowest_square().unwrap();
                let potential_moves_bb = ray_without_pinned ^ maybe_pinned;
                pinned_between[index] = Some((pinned_square, potential_moves_bb));
                index += 1;
            }
        }
    }
    // Check that for each pinned piece, there exists a mapping to it's in between squares.
    debug_assert_eq!(pinned.count_squares() as usize, index);

    (pinned, pinned_between)
}

/// Generate castling moves and append to move list.
/// Castling is legal is there are no pieces between rook and king,
/// the king does not pass through check, and has appropriate castling rights.
/// params:
/// moves - Move list to append to.
/// player - Player that is castling.
/// castling - Castling rights for player.
/// occupied - All occupied squares on chess board.
/// attacked - All Squares directly attacked by opposite player.
pub fn legal_castling_moves(
    moves: &mut MoveList,
    player: Color,
    castling: Castling,
    occupied: Bitboard,
    attacked: Bitboard,
) {
    let (has_kingside, has_queenside, king_rank) = match player {
        White => {
            let kingside = castling.has(Castling::W_KING);
            let queenside = castling.has(Castling::W_QUEEN);
            let king_rank = Bitboard::RANK_1;
            (kingside, queenside, king_rank)
        }
        Black => {
            let kingside = castling.has(Castling::B_KING);
            let queenside = castling.has(Castling::B_QUEEN);
            let king_rank = Bitboard::RANK_8;
            (kingside, queenside, king_rank)
        }
    };
    if has_kingside {
        let between = occupied & Bitboard::KINGSIDE_BETWEEN & king_rank;
        let pass_attacked = attacked & Bitboard::KINGSIDE_PASS & king_rank;
        if between.is_empty() && pass_attacked.is_empty() {
            match player {
                White => moves.push(Move::new(E1, G1, None)),
                Black => moves.push(Move::new(E8, G8, None)),
            }
        }
    }
    if has_queenside {
        let between = occupied & Bitboard::QUEENSIDE_BETWEEN & king_rank;
        let pass_attacked = attacked & Bitboard::QUEENSIDE_PASS & king_rank;
        if between.is_empty() && pass_attacked.is_empty() {
            match player {
                White => moves.push(Move::new(E1, C1, None)),
                Black => moves.push(Move::new(E8, C8, None)),
            }
        }
    }
}

// *_pseudo_moves:
// generate a move list of pseudo legal moves for each piece, including
// pushes and attacks. These moves do not consider check, but they do consider
// occupancy.

/// Generate all pseudo-legal pawn moves and append to move list.
/// params:
/// moves - move list to add new moves to.
/// pawns - Bitboard with squares of all pawns to generate moves for.
/// color - player to generate moves for.
/// occupied - All occupied squares on board.
/// them - All squares occupied by opposing player.
/// en_passant - Optional en-passant target square.
pub fn pawn_pseudo_moves(
    moves: &mut MoveList,
    pawns: Bitboard,
    color: Color,
    occupied: Bitboard,
    them: Bitboard,
    en_passant: Option<Square>,
) {
    // Pawns can attack ep square as if it was occupied.
    let them_with_ep = match en_passant {
        Some(ep_square) => them | Bitboard::from(ep_square),
        None => them,
    };

    // Consider pushes, attacks, promotions for each pawn individually.
    for from in pawns {
        let pawn = Bitboard::from(from);
        let single_push = pawn_single_pushes(&pawn, &color) & !occupied;
        let double_push = pawn_double_pushes(&pawn, &color) & !occupied;
        let valid_double_push = double_push & pawn_single_pushes(&single_push, &color);
        let pushes = single_push | valid_double_push;
        let attacks = pawn_attacks(&pawn, &color) & them_with_ep;

        let tos = pushes.into_iter().chain(attacks.into_iter());

        for to in tos {
            if Bitboard::RANK_1.has_square(to) || Bitboard::RANK_8.has_square(to) {
                moves.push(Move::new(from, to, Some(Queen)));
                moves.push(Move::new(from, to, Some(Rook)));
                moves.push(Move::new(from, to, Some(Bishop)));
                moves.push(Move::new(from, to, Some(Knight)));
            } else {
                moves.push(Move::new(from, to, None));
            }
        }
    }
}

/// Generate all pseudo-legal knight moves and append to move list.
/// params:
/// moves - move list to add new moves to.
/// knights - Bitboard with squares of all knights to generate moves for.
/// us - Bitboard with occupancy of moving player.
pub fn knight_pseudo_moves(moves: &mut MoveList, knights: Bitboard, us: Bitboard) {
    for from in knights {
        let tos = knight_pattern(from) & !us;
        for to in tos {
            moves.push(Move::new(from, to, None));
        }
    }
}

/// Generate all pseudo-legal queen moves and append to move list.
pub fn queen_pseudo_moves(
    moves: &mut MoveList,
    queens: Bitboard,
    occupied: Bitboard,
    us: Bitboard,
) {
    for from in queens {
        let tos = solo_queen_attacks(&from, &occupied) & !us;
        for to in tos {
            moves.push(Move::new(from, to, None));
        }
    }
}

/// Generate all pseudo-legal rook moves and append to move list.
pub fn rook_pseudo_moves(moves: &mut MoveList, rooks: Bitboard, occupied: Bitboard, us: Bitboard) {
    for from in rooks {
        let tos = solo_rook_attacks(&from, &occupied) & !us;
        for to in tos {
            moves.push(Move::new(from, to, None));
        }
    }
}

/// Generate all pseudo-legal bishop moves and append to move list.
pub fn bishop_pseudo_moves(
    moves: &mut MoveList,
    bishops: Bitboard,
    occupied: Bitboard,
    us: Bitboard,
) {
    for from in bishops {
        let tos = solo_bishop_attacks(&from, &occupied) & !us;
        for to in tos {
            moves.push(Move::new(from, to, None));
        }
    }
}

// Pushes and attacks: Calculate pushes or attacks for all pieces on a bitboard.

/// Generate pushes for all pawns of a color on otherwise empty board.
/// Currently generating separately per color because moves are not symmetrical.
pub fn pawn_pushes(pawns: &Bitboard, color: &Color) -> Bitboard {
    let single_push_bb = pawn_single_pushes(pawns, color);
    let double_push_bb = pawn_double_pushes(pawns, color);
    single_push_bb | double_push_bb
}

/// Generate pseudo-legal single push moves for all pawns of a color.
pub fn pawn_single_pushes(pawns: &Bitboard, color: &Color) -> Bitboard {
    // Single pushes are easy to generate, by pushing 1 square forward.
    match color {
        White => pawns.to_north(),
        Black => pawns.to_south(),
    }
}

/// Generate pseudo-legal double push moves for all pawns of a color.
pub fn pawn_double_pushes(pawns: &Bitboard, color: &Color) -> Bitboard {
    // Double pushes are generated only from pawns on color's starting rank.
    match color {
        White => (pawns & Bitboard::RANK_2).to_north().to_north(),
        Black => (pawns & Bitboard::RANK_7).to_south().to_south(),
    }
}

/// Generate attacks for all pawns in Bitboard for a color.
/// Attacks for any number of pawns are calculated in constant time.
pub fn pawn_attacks(pawns: &Bitboard, color: &Color) -> Bitboard {
    match color {
        White => pawns.to_north().to_east() | pawns.to_north().to_west(),
        Black => pawns.to_south().to_east() | pawns.to_south().to_west(),
    }
}

/// Generate Bitboard with squares that are attacked by exactly two pawns for a color.
pub fn pawn_double_attacks(pawns: &Bitboard, color: &Color) -> Bitboard {
    // double attacks are only possible if East and West attacks attack same square.
    match color {
        White => pawns.to_north().to_east() & pawns.to_north().to_west(),
        Black => pawns.to_south().to_east() & pawns.to_south().to_west(),
    }
}

/// Generate Bitboard with squares attacked by knights.
/// Knight attacks are a pattern, so attacks for all knights are calculated in constant time.
pub fn knight_attacks(knights: &Bitboard) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;

    attacks |= knights.to_north().to_north().to_east();
    attacks |= knights.to_north().to_east().to_east();
    attacks |= knights.to_south().to_east().to_east();
    attacks |= knights.to_south().to_south().to_east();

    attacks |= knights.to_south().to_south().to_west();
    attacks |= knights.to_south().to_west().to_west();
    attacks |= knights.to_north().to_west().to_west();
    attacks |= knights.to_north().to_north().to_west();

    attacks
}

/// Generate Bitboard with squares attacked by king, assuming exactly 1 king.
/// King attacks are found in constant time by lookup.
pub fn king_attacks(king: &Bitboard) -> Bitboard {
    king_pattern(king.get_lowest_square().unwrap())
}

/// Generate and return Bitboard with squares attacked by all queens.
/// Queen attacks are found in linear time, with 8 rays calculated per queen.
pub fn queen_attacks(queens: &Bitboard, occupied: &Bitboard) -> Bitboard {
    queens
        .into_iter()
        .map(|square| solo_queen_attacks(&square, occupied))
        .fold(Bitboard::EMPTY, |acc, attacks| acc | attacks)
}

/// Generate and return Bitboard with squares attacked by all rooks.
/// Rook attacks are found in linear time, with 4 rays calculated per rook.
pub fn rook_attacks(rooks: &Bitboard, occupied: &Bitboard) -> Bitboard {
    rooks
        .into_iter()
        .map(|square| solo_rook_attacks(&square, occupied))
        .fold(Bitboard::EMPTY, |acc, attacks| acc | attacks)
}

/// Generate and return Bitboard with squares attacked by all bishops.
/// Bishop attacks are found in linear time, with 4 rays calculated per bishops.
pub fn bishop_attacks(bishops: &Bitboard, occupied: &Bitboard) -> Bitboard {
    bishops
        .into_iter()
        .map(|square| solo_bishop_attacks(&square, occupied))
        .fold(Bitboard::EMPTY, |acc, attacks| acc | attacks)
}

/// Generate and return Bitboard with squares attacked by all sliding pieces.
/// This may be a little more efficient than checking for each sliding piece individually.
pub fn slide_attacks(
    queens: &Bitboard,
    rooks: &Bitboard,
    bishops: &Bitboard,
    occupied: &Bitboard,
) -> Bitboard {
    let orthogonals = queens | *rooks;
    let diagonals = queens | *bishops;

    let orthogonal_attacks = orthogonals
        .into_iter()
        .map(|square| solo_rook_attacks(&square, &occupied))
        .fold(Bitboard::EMPTY, |acc, attacks| acc | attacks);

    let diagonal_attacks = diagonals
        .into_iter()
        .map(|square| solo_bishop_attacks(&square, &occupied))
        .fold(Bitboard::EMPTY, |acc, attacks| acc | attacks);
    orthogonal_attacks | diagonal_attacks
}

/// Generate Bitboard containing all squares that are directly attacked by a piece at origin,
/// in all 8 orthogonal and diagonal directions.
/// Directly attacked squares are all empty squares along ray up to first any piece, inclusive.
/// Individual rays stop at and include the first attacked piece regardless of piece color.
pub fn solo_queen_attacks(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    solo_rook_attacks(origin, occupancy) | solo_bishop_attacks(origin, occupancy)
}

/// Returns Bitboard with Squares directly attacked from origin in 4 orthogonal directions.
pub fn solo_rook_attacks(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    ray_attack_no(origin, occupancy)
        | ray_attack_ea(origin, occupancy)
        | ray_attack_so(origin, occupancy)
        | ray_attack_we(origin, occupancy)
}

/// Returns Bitboard with Squares directly attacked from origin in 4 diagonal directions.
pub fn solo_bishop_attacks(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    ray_attack_noea(origin, occupancy)
        | ray_attack_soea(origin, occupancy)
        | ray_attack_sowe(origin, occupancy)
        | ray_attack_nowe(origin, occupancy)
}

// attackers_to functions take a target square and an occupancy Bitboard

/// Returns Bitboard with all same-color pawns that attack target square.
/// target: square to check if attacking.
/// pawns: Bitboard with all pawns to test with.
/// color: Color of pawns in pawns Bitboard.
pub fn pawn_attackers_to(target: &Square, pawns: &Bitboard, color: &Color) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    for pawn_square in pawns.into_iter() {
        let pawn = Bitboard::from(pawn_square);
        if pawn_attacks(&pawn, color).has_square(target) {
            attackers.set_square(pawn_square);
        }
    }
    attackers
}
/// Return Bitboard with Squares of all knights from occupancy that attack target square.
pub fn knight_attackers_to(target: &Square, knights: &Bitboard) -> Bitboard {
    knights
        .into_iter()
        .filter(|square| knight_pattern(square).has_square(target))
        .fold(Bitboard::EMPTY, |acc, square| acc | Bitboard::from(square))
}
/// Return Bitboard with Squares of all kings from occupancy that attack target square.
pub fn king_attackers_to(target: &Square, kings: &Bitboard) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    for king_square in kings.into_iter() {
        if king_pattern(king_square).has_square(target) {
            attackers.set_square(king_square);
        }
    }
    attackers
}
/// Returns Bitboard with all queens that attack target square, considering occupied squares.
pub fn queen_attackers_to(target: &Square, queens: &Bitboard, occupied: &Bitboard) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    for queen_square in queens.into_iter() {
        if solo_queen_attacks(&queen_square, occupied).has_square(target) {
            attackers.set_square(queen_square);
        }
    }
    attackers
}
/// Returns Bitboard with all rooks that attack target square, considering occupied squares.
pub fn rook_attackers_to(target: &Square, rooks: &Bitboard, occupied: &Bitboard) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    for rook_square in rooks.into_iter() {
        if solo_rook_attacks(&rook_square, occupied).has_square(target) {
            attackers.set_square(rook_square);
        }
    }
    attackers
}
/// Returns Bitboard with all bishops that attack target square, considering occupied squares.
pub fn bishop_attackers_to(target: &Square, bishops: &Bitboard, occupied: &Bitboard) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    for bishop_square in bishops.into_iter() {
        if solo_bishop_attacks(&bishop_square, occupied).has_square(target) {
            attackers.set_square(bishop_square);
        }
    }
    attackers
}

// Each of 8-Directional rays, North, East, South, West, 4 Diagonals.

/// Return all squares attacked in North-direction ray, stopping on first attacked piece.
fn ray_attack_no(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_north();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_north();
    }
    ray
}
/// Return all squares attacked in East-direction ray, stopping on first attacked piece.
fn ray_attack_ea(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_east();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_east();
    }
    ray
}
/// Return all squares attacked in South-direction ray, stopping on first attacked piece.
fn ray_attack_so(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_south();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_south();
    }
    ray
}
/// Return all squares attacked in North-direction ray, stopping on first attacked piece.
fn ray_attack_we(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_west();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_west();
    }
    ray
}
/// Return all squares attacked in NorthEast-direction ray, stopping on first attacked piece.
fn ray_attack_noea(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_north_east();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_north_east();
    }
    ray
}
/// Return all squares attacked in SouthEast-direction ray, stopping on first attacked piece.
fn ray_attack_soea(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_south_east();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_south_east();
    }
    ray
}
/// Return all squares attacked in SouthWest-direction ray, stopping on first attacked piece.
fn ray_attack_sowe(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_south_west();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_south_west();
    }
    ray
}
/// Return all squares attacked in NorthWest-direction ray, stopping on first attacked piece.
fn ray_attack_nowe(origin: &Square, occupancy: &Bitboard) -> Bitboard {
    let mut ray = Bitboard::from(origin).to_north_west();
    for _ in 0..6 {
        if occupancy.has_any(&ray) {
            return ray;
        }
        ray |= ray.to_north_west();
    }
    ray
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

#[cfg(test)]
mod tests {
    use super::*;
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
            assert_eq!(a1.count_squares(), 14);
            for square in [A2, A3, A4, A5, A6, A7, A8, B1, C1, D1, E1, F1, G1, H1] {
                assert!(a1.has_square(&square));
            }
        }
        {
            let h8 = ROOK_PATTERN[H8.idx()];
            assert_eq!(h8.count_squares(), 14);
            for square in [A8, B8, C8, D8, E8, F8, G8, H1, H2, H3, H4, H5, H6, H7] {
                assert!(h8.has_square(&square));
            }
        }
        {
            let f3 = ROOK_PATTERN[F3.idx()];
            assert_eq!(f3.count_squares(), 14);
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
            assert_eq!(a1.count_squares(), 7);
            for square in [B2, C3, D4, E5, F6, G7, H8] {
                assert!(a1.has_square(&square));
            }
        }
        {
            let h1 = BISHOP_PATTERN[H1.idx()];
            assert_eq!(h1.count_squares(), 7);
            for square in [A8, B7, C6, D5, E4, F3, G2] {
                assert!(h1.has_square(&square));
            }
        }
        {
            let h8 = BISHOP_PATTERN[H8.idx()];
            assert_eq!(h8.count_squares(), 7);
            for square in [A1, B2, C3, D4, E5, F6, G7] {
                assert!(h8.has_square(&square));
            }
        }
        {
            let c6 = BISHOP_PATTERN[C6.idx()];
            assert_eq!(c6.count_squares(), 11);
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
            assert_eq!(a1.count_squares(), 21);
            for square in [B1, C1, D1, E1, F1, G1, H1, A2, A3, A4, A5, A6, A7, A8] {
                assert!(a1.has_square(&square)); // Orthogonal squares.
            }
            for square in [B2, C3, D4, E5, F6, G7, H8] {
                assert!(a1.has_square(&square)); // Diagonal squares.
            }
        }
        {
            let h1 = QUEEN_PATTERN[H1.idx()];
            assert_eq!(h1.count_squares(), 21);
            for square in [A1, B1, C1, D1, E1, F1, G1, H2, H3, H4, H5, H6, H7, H8] {
                assert!(h1.has_square(&square)); // Orthogonal squares.
            }
            for square in [A8, B7, C6, D5, E4, F3, G2] {
                assert!(h1.has_square(&square)); // Diagonal squares.
            }
        }
        {
            let c6 = QUEEN_PATTERN[C6.idx()];
            assert_eq!(c6.count_squares(), 25);
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

    #[test]
    fn check_pawn_pseudo_moves() {
        {
            // B pawn at end of file has no moves.
            let a1 = Bitboard::from(A1);
            let a1_moves = pawn_pushes(&a1, &Color::Black);
            assert_eq!(a1_moves.count_squares(), 0);
        }
        {
            // W pawn on starting row has 2 moves, B pawn has 1.
            let a2 = Bitboard::from(A2);
            let a2_moves = pawn_pushes(&a2, &Color::White);
            assert_eq!(a2_moves.count_squares(), 2);
            assert!(a2_moves.has_square(A3));
            assert!(a2_moves.has_square(A4));
            let a2_moves = pawn_pushes(&a2, &Color::Black);
            assert_eq!(a2_moves.count_squares(), 1);
            assert!(a2_moves.has_square(A1));
        }
        {
            let f3 = Bitboard::from(F3);
            let f3_moves = pawn_pushes(&f3, &Color::White);
            assert_eq!(f3_moves.count_squares(), 1);
            assert!(f3_moves.has_square(F4));
            let f3_moves = pawn_pushes(&f3, &Color::Black);
            assert_eq!(f3_moves.count_squares(), 1);
            assert!(f3_moves.has_square(F2));
        }
        {
            let h7 = Bitboard::from(H7);
            let h7_moves = pawn_pushes(&h7, &Color::White);
            assert_eq!(h7_moves.count_squares(), 1);
            assert!(h7_moves.has_square(H8));
            let h7_moves = pawn_pushes(&h7, &Color::Black);
            assert_eq!(h7_moves.count_squares(), 2);
            assert!(h7_moves.has_square(H6));
            assert!(h7_moves.has_square(H5));
        }
        {
            let pawns = Bitboard::from(vec![B2, C3, F7, H8].as_slice());
            let w_pawn_moves = pawn_pushes(&pawns, &Color::White);
            assert_eq!(w_pawn_moves.count_squares(), 4);
            assert!(w_pawn_moves.has_square(B3));
            assert!(w_pawn_moves.has_square(B4));
            assert!(w_pawn_moves.has_square(C4));
            assert!(w_pawn_moves.has_square(F8));

            let b_pawn_moves = pawn_pushes(&pawns, &Color::Black);
            assert_eq!(b_pawn_moves.count_squares(), 5);
            assert!(b_pawn_moves.has_square(B1));
            assert!(b_pawn_moves.has_square(C2));
            assert!(b_pawn_moves.has_square(F6));
            assert!(b_pawn_moves.has_square(F5));
            assert!(b_pawn_moves.has_square(H7));
        }
        // Does not attack own square.
        for square in Square::iter() {
            let pawn = Bitboard::from(square);
            assert!(!pawn_pushes(&pawn, &Color::Black).has_square(square));
            assert!(!pawn_pushes(&pawn, &Color::White).has_square(square));
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
