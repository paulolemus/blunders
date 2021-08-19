//! Zobrist Hashing

use std::collections::HashSet;
use std::ops::Index;

use rand::prelude::*;

use crate::boardrepr::PieceSets;
use crate::coretypes::{Castling, Color, File, Piece, PieceKind, Rank, Square, SquareIndexable};
use crate::coretypes::{MoveInfo, MoveKind, Square::*};
use crate::coretypes::{NUM_FILES, NUM_PIECE_KINDS, NUM_SQUARES};
use crate::position::{Cache, Position};

/// HashKind is an alias for the underlying type of a Zobrist Hash.
pub type HashKind = u64;

/// Key contains all data needed to generate a hash.
pub type Key<'a> = (&'a PieceSets, &'a Color, &'a Castling, &'a Option<Square>);

/// Convert a Position reference into a Key.
impl<'a> From<&'a Position> for Key<'a> {
    fn from(pos_ref: &'a Position) -> Self {
        (
            pos_ref.pieces(),
            pos_ref.player(),
            pos_ref.castling(),
            pos_ref.en_passant(),
        )
    }
}

// Strategy:
// A TT has a maximum size. That size is pre-allocated, and never changes.
// The Key of a table is a Position, which has a cached hash.

/// Zobrist Hashing is a quick and incremental way to hash a chess position.
/// ZobristTable contains unique, pseudo-randomly generated values
/// used for calculating Zobrist Hash of a chess position.
///
/// Each Piece gets a unique number for each square.
/// A single side to move gets a unique number.
/// Each possible combination of castling rights gets a unique number.
/// Each possible file for En-Passant gets a unique number.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ZobristTable {
    piece_hash: [[HashKind; NUM_SQUARES]; NUM_PIECE_KINDS],
    ep_hash: [HashKind; NUM_FILES],
    castling_hash: [HashKind; Castling::ENUMERATIONS],
    pub(crate) player_hash: HashKind,
}

impl ZobristTable {
    const TOGGLE_PLAYER: Color = Color::Black;

    /// Returns a new ZobristTable with randomly seeded, unique values.
    pub fn new() -> Self {
        Self::with_rng(StdRng::from_entropy())
    }

    /// Returns a new ZobristTable with unique values generated from seeded rng.
    pub fn with_seed(seed: u64) -> Self {
        Self::with_rng(StdRng::seed_from_u64(seed))
    }

    /// Returns a new ZobristTable with unique values generated from rng.
    fn with_rng(mut rng: StdRng) -> Self {
        // Ensure there are no duplicates in Table. Each value used must be unique.
        let mut used_values = HashSet::new();

        let mut piece_hash = [[HashKind::default(); NUM_SQUARES]; NUM_PIECE_KINDS];
        let mut ep_hash = [HashKind::default(); NUM_FILES];
        let mut castling_hash = [HashKind::default(); Castling::ENUMERATIONS];

        // arbitrarily large number to prevent infinite loops for bad rng.
        let mut inf_loop_protection: usize = 10_000;

        // Initialize array parts of Table.
        for item in piece_hash
            .iter_mut()
            .flatten()
            .chain(ep_hash.iter_mut())
            .chain(castling_hash.iter_mut())
        {
            let mut new_unique_value: HashKind = rng.gen();
            // insert returns false if item was already in set.
            // Loop until unique value is found.
            while !used_values.insert(new_unique_value) {
                new_unique_value = rng.gen();

                inf_loop_protection -= 1;
                if inf_loop_protection < 10 {
                    panic!("Encountered excessively repeated random numbers.");
                }
            }
            *item = new_unique_value;
        }

        // Initialize scalar parts of table.
        let mut player_hash: HashKind = rng.gen();
        while !used_values.insert(player_hash) {
            player_hash = rng.gen();

            inf_loop_protection -= 1;
            if inf_loop_protection < 10 {
                panic!("Encountered excessively repeated random numbers.");
            }
        }

        Self {
            piece_hash,
            ep_hash,
            castling_hash,
            player_hash,
        }
    }

    /// Generate a hash value from provided key in context of this ZobristTable.
    pub fn generate_hash(&self, key: Key) -> HashKind {
        let mut hash = HashKind::default();

        // For each piece, xor its value from ztable into the hash.
        for color in Color::iter() {
            for piece_kind in PieceKind::iter() {
                let piece = Piece::new(color, piece_kind);
                let squares = key.0[piece];

                for square in squares {
                    hash ^= self[(piece, square)];
                }
            }
        }

        // Hash the en-passant file if it exists.
        if let Some(ep_square) = key.3 {
            hash ^= self[ep_square.file()];
        }

        // Hash castling rights.
        hash ^= self[*key.2];

        // Hash player. Only need to hash when active player is Black.
        // This allows incremental hashing when making a move in a single xor for player.
        if *key.1 == ZobristTable::TOGGLE_PLAYER {
            hash ^= self.player_hash;
        }

        hash
    }

    /// Update a hash from a Position and its MoveInfo. The move that resulted in MoveInfo
    /// must already be applied to the position.
    /// update_hash works both directions, it can apply and remove a move from a position's hash.
    ///
    /// # Arguments
    /// `hash`: The hash value to directly update.
    /// `key`: A key taken from an updated Position.
    /// `move_info`: The MoveInfo that was applied to some position.
    /// `cache`: The original cache of the Position, before a move.
    pub fn update_hash(&self, hash: &mut HashKind, key: Key, move_info: MoveInfo, cache: Cache) {
        let moved_player = !key.1;
        let passive_player = *key.1;

        // Always toggle player hash because player always alternates.
        *hash ^= self.player_hash;
        // Always toggle both old and new Castling, as each enumeration even none has a hash.
        *hash ^= self[cache.castling];
        *hash ^= self[*key.2];
        // Always toggle piece on "from" square.
        let moved_piece = Piece::new(moved_player, move_info.piece_kind);
        *hash ^= self[(moved_piece, move_info.from)];

        // Toggle both old and new en-passant, if they exist.
        let old_ep = cache.en_passant;
        let new_ep = key.3;
        if let Some(ep_square) = old_ep {
            *hash ^= self[ep_square.file()];
        }
        if let Some(ep_square) = new_ep {
            *hash ^= self[ep_square.file()];
        }

        // Toggle moved piece on "to" square. If promoted, instead toggle promoted_piece.
        let to_piece_kind: PieceKind = match move_info.promotion {
            Some(promoted_pk) => promoted_pk,
            None => move_info.piece_kind,
        };
        let to_piece = Piece::new(moved_player, to_piece_kind);
        *hash ^= self[(to_piece, move_info.to)];

        match move_info.move_kind {
            // Toggle passive player's captured piece if a normal capture occurred.
            MoveKind::Capture(captured_pk) => {
                let captured_piece = Piece::new(passive_player, captured_pk);
                *hash ^= self[(captured_piece, move_info.to)];
            }

            // Toggle passive player's pawn if en-passant occurred.
            MoveKind::EnPassant => {
                let ep_square = cache.en_passant.unwrap();
                let pawn_square = match ep_square.rank() {
                    Rank::R3 => ep_square.increment_rank().unwrap(),
                    _ => ep_square.decrement_rank().unwrap(),
                };
                let captured_pawn = Piece::new(passive_player, PieceKind::Pawn);
                *hash ^= self[(captured_pawn, pawn_square)];
            }

            // Toggle both castling squares for rook.
            MoveKind::Castle => {
                let (rook_from, rook_to) = match move_info.to {
                    G1 => (H1, F1),
                    C1 => (A1, D1),
                    G8 => (H8, F8),
                    C8 => (A8, D8),
                    _ => panic!("Processing Castling Move, but to square was not valid."),
                };
                let castled_rook = Piece::new(moved_player, PieceKind::Rook);
                *hash ^= self[(castled_rook, rook_from)];
                *hash ^= self[(castled_rook, rook_to)];
            }

            // Nothing extra is toggled for quiet moves.
            MoveKind::Quiet => (),
        };
    }
}

/// Default for ZobristTable is a table with a random seed.
impl Default for ZobristTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Index used for accessing piece_hash.
impl Index<(Piece, Square)> for ZobristTable {
    type Output = HashKind;
    fn index(&self, index: (Piece, Square)) -> &Self::Output {
        let (piece, square) = index;
        &self.piece_hash[piece.zobrist_offset()][square.idx()]
    }
}

// Index used for accessing ep_hash (en-passant hash).
impl Index<File> for ZobristTable {
    type Output = HashKind;
    fn index(&self, index: File) -> &Self::Output {
        &self.ep_hash[index as usize]
    }
}

// Index used for accessing castling_hash.
impl Index<Castling> for ZobristTable {
    type Output = HashKind;
    fn index(&self, index: Castling) -> &Self::Output {
        &self.castling_hash[index.bits() as usize]
    }
}

// Implementations for Color, PieceKind, and Piece to allow indexing into ZobristTable.
// This code is copied from piece_sets.rs.
// TODO: Consider consolidating.

impl Color {
    /// Get the position of the start of the block for a color.
    /// There are 6 piece_kinds per color, so one should start at 0, and the other at 6.
    #[inline(always)]
    const fn zobrist_offset_block(&self) -> usize {
        match self {
            Color::White => 0,
            Color::Black => 6,
        }
    }
}

impl PieceKind {
    /// Get the offset of a piece_kind within a block.
    /// Values must cover all numbers of [0, 1, 2, 3, 4, 5].
    #[inline(always)]
    const fn zobrist_offset_pk(&self) -> usize {
        match self {
            PieceKind::King => 0,
            PieceKind::Pawn => 1,
            PieceKind::Knight => 2,
            PieceKind::Queen => 3,
            PieceKind::Rook => 4,
            PieceKind::Bishop => 5,
        }
    }
}

impl Piece {
    /// Get the completely qualified index for a piece.
    #[inline(always)]
    const fn zobrist_offset(&self) -> usize {
        self.color.zobrist_offset_block() + self.piece_kind.zobrist_offset_pk()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::Move;
    use crate::fen::Fen;
    use crate::Position;

    fn test_before_and_after(
        table: ZobristTable,
        before: Position,
        after: Position,
        legal_move: Move,
    ) {
        let hash_before = table.generate_hash(Key::from(&before));
        let hash_after = table.generate_hash(Key::from(&after));

        // Multiple hashes generated from same table and position are equal.
        let hash_extra_before = table.generate_hash(Key::from(&before));
        let hash_extra_after = table.generate_hash(Key::from(&after));
        assert_eq!(hash_before, hash_extra_before);
        assert_eq!(hash_after, hash_extra_after);

        // Check that hashes and positions are the same before applying move.
        let mut pos = before.clone();
        let mut hash = table.generate_hash(Key::from(&pos));
        assert_eq!(pos, before);
        assert_eq!(hash, hash_before);

        // Check that newly updated hash and position
        // equal individually generated hash and position.
        let cache = pos.cache();
        let move_info = pos.do_move(legal_move);
        table.update_hash(&mut hash, Key::from(&pos), move_info, cache);
        assert_eq!(pos, after);
        assert_eq!(hash, hash_after);

        // Check that updating hash a second time undoes the previous update.
        table.update_hash(&mut hash, Key::from(&pos), move_info, cache);
        assert_eq!(hash, hash_before);

        // Check that updating hash a third time acts like the first update.
        table.update_hash(&mut hash, Key::from(&pos), move_info, cache);
        assert_eq!(hash, hash_after);
    }

    #[test]
    fn hash_start_position() {
        let table = ZobristTable::new();
        let legal_move = Move::new(D2, D4, None);
        let start_position = Position::start_position();
        let queens_pawn_game = start_position.make_move(legal_move);

        test_before_and_after(table, start_position, queens_pawn_game, legal_move);
    }

    #[test]
    fn hash_en_passant_position() {
        let table = ZobristTable::new();
        let legal_move = Move::new(D5, E6, None);
        let pos_before =
            Position::parse_fen("rnbqkbnr/pp1p1ppp/8/2pPp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 3")
                .unwrap();
        let pos_after =
            Position::parse_fen("rnbqkbnr/pp1p1ppp/4P3/2p5/8/8/PPP1PPPP/RNBQKBNR b KQkq - 0 3")
                .unwrap();

        test_before_and_after(table, pos_before, pos_after, legal_move);
    }

    #[test]
    fn hash_castling_position() {
        let table = ZobristTable::new();
        let legal_move = Move::new(E1, G1, None);
        let pos_before = Position::parse_fen(
            "rnb1k1nr/pp3ppp/3bp3/q2p4/2Pp4/2NBPN2/PP3PPP/R1BQK2R w KQkq - 0 7",
        )
        .unwrap();
        let pos_after =
            Position::parse_fen("rnb1k1nr/pp3ppp/3bp3/q2p4/2Pp4/2NBPN2/PP3PPP/R1BQ1RK1 b kq - 1 7")
                .unwrap();

        test_before_and_after(table, pos_before, pos_after, legal_move);
    }
}
