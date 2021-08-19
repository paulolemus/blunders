//! Holds Position struct, the most important data structure for the engine.
//! Position represents a chess position.
//! Positions and moves are assumed to be strictly legal,
//! and have undefined behavior for illegal activity.

use std::fmt::{self, Display};

use crate::bitboard::Bitboard;
use crate::boardrepr::PieceSets;
use crate::coretypes::{
    Castling, Color, Move, MoveCount, MoveInfo, MoveKind, Piece, PieceKind, Square,
};
use crate::coretypes::{Color::*, PieceKind::*, Square::*};
use crate::error::{self, ErrorKind};
use crate::fen::Fen;
use crate::movegen as mg;
use crate::movelist::{MoveHistory, MoveList};

/// Game contains information for an in progress game:
/// The base position the game started from, the sequence of moves that were
/// played, and the current position.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Game {
    pub base_position: Position,
    pub moves: MoveHistory,
    pub position: Position,
}

impl Game {
    /// Create a new Game from a base position and a sequence of moves.
    /// This generates the current position by applying the sequence of moves to the base.
    /// If a move in the move history was illegal, Err is returned.
    pub fn new(base_position: Position, moves: MoveHistory) -> error::Result<Self> {
        let mut position = base_position.clone();

        for move_ in &moves {
            let maybe_move_info = position.do_legal_move(*move_);
            if maybe_move_info.is_none() {
                return Err(ErrorKind::GameIllegalMove.into());
            }
        }

        Ok(Self {
            base_position,
            moves,
            position,
        })
    }

    /// Create a new game in the standard chess start position.
    pub fn start_position() -> Self {
        Self::from(Position::start_position())
    }
}

/// Convert a position to a Game with no past moves.
impl From<Position> for Game {
    fn from(position: Position) -> Self {
        Self::new(position, MoveHistory::new()).unwrap()
    }
}

/// During position.do_move, there are a number of variables
/// that are updated in one direction, which are restored from backups in MoveInfo
/// during position.undo_move. Instead of each MoveInfo keeping its own repetitive copy
/// of this undone info, they should be saved separately.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Cache {
    pub(crate) castling: Castling,
    pub(crate) en_passant: Option<Square>,
    pub(crate) halfmoves: MoveCount,
    // Number of times active player is in check, either 0, 1, or 2.
    // pub(crate) checks: u8,
    // Checks? Occupied per side?
}

impl Cache {
    /// Return a cache with garbage values.
    pub fn illegal() -> Self {
        Self {
            castling: Castling::NONE,
            en_passant: None,
            halfmoves: 1,
        }
    }
}

impl From<&Position> for Cache {
    fn from(position: &Position) -> Self {
        Self {
            castling: position.castling,
            en_passant: position.en_passant,
            halfmoves: position.halfmoves,
        }
    }
}

/// struct Position
/// A complete data set that can represent any chess position.
/// # Members:
/// * pieces - a piece-centric setwise container of all basic chess piece positions.
/// * player - Color of player whose turn it is. AKA: "side_to_move".
/// * castling - Castling rights for both players.
/// * en_passant - Indicates if en passant is possible, and for which square.
/// * halfmoves - Tracker for 50 move draw rule. Resets after capture/pawn move.
/// * fullmoves - Starts at 1, increments after each black player's move.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    pub(crate) pieces: PieceSets,
    pub(crate) player: Color,
    pub(crate) castling: Castling,
    pub(crate) en_passant: Option<Square>,
    pub(crate) halfmoves: MoveCount,
    pub(crate) fullmoves: MoveCount,
}

impl Position {
    /// Standard chess start position.
    pub fn start_position() -> Self {
        Self {
            pieces: PieceSets::start_position(),
            player: Color::White,
            castling: Castling::start_position(),
            en_passant: None,
            halfmoves: 0,
            fullmoves: 1,
        }
    }

    /// Const getters.
    pub fn pieces(&self) -> &PieceSets {
        &self.pieces
    }
    pub fn player(&self) -> &Color {
        &self.player
    }
    pub fn castling(&self) -> &Castling {
        &self.castling
    }
    pub fn en_passant(&self) -> &Option<Square> {
        &self.en_passant
    }
    pub fn halfmoves(&self) -> &MoveCount {
        &self.halfmoves
    }
    pub fn fullmoves(&self) -> &MoveCount {
        &self.fullmoves
    }

    /// Create a new position where the relative position is the same for the active player,
    /// but the player gets switched.
    /// This is equivalent to a vertical flip and color swap for all pieces,
    /// along with castling rights, player and en-passant.
    ///
    /// This is useful for checking that an evaluation function scores the same scenario
    /// presented to either player.
    pub fn color_flip(&self) -> Self {
        let mut flipped = self.clone();

        // For each piece, flip its rank and color
        let mut pieces = PieceSets::new();
        for color in Color::iter() {
            for piece_kind in PieceKind::iter() {
                for sq in self.pieces[(color, piece_kind)] {
                    let flipped_sq = Square::from((sq.file(), sq.rank().flip()));
                    pieces[(!color, piece_kind)].set_square(flipped_sq);
                }
            }
        }
        flipped.pieces = pieces;

        // Flip side to move
        flipped.player = !self.player;

        // Flip castling rights
        let mut cr = Castling::NONE;
        self.castling
            .has(Castling::W_KING)
            .then(|| cr.set(Castling::B_KING));
        self.castling
            .has(Castling::W_QUEEN)
            .then(|| cr.set(Castling::B_QUEEN));
        self.castling
            .has(Castling::B_KING)
            .then(|| cr.set(Castling::W_KING));
        self.castling
            .has(Castling::B_QUEEN)
            .then(|| cr.set(Castling::W_QUEEN));
        flipped.castling = cr;

        // Flip ep passant square
        flipped.en_passant = self
            .en_passant
            .map(|sq| Square::from((sq.file(), sq.rank().flip())));

        debug_assert!(flipped.pieces().is_valid());
        debug_assert!(flipped.castling().is_mask_valid());
        flipped
    }

    /// Returns true if the positions are the same, in context of FIDE laws for position repetition.
    /// They are the same if the player to move, piece kind and color per square, en passant,
    /// and castling rights are the same.
    pub fn is_same_as(&self, other: &Self) -> bool {
        self.player == other.player
            && self.castling == other.castling
            && self.en_passant == other.en_passant
            && self.pieces == other.pieces
    }

    /// Returns true if the fifty-move rule has been reached by this position, indicating that it is drawn.
    /// `num_legal_moves`: number of legal moves for this position.
    pub fn fifty_move_rule(&self, num_legal_moves: usize) -> bool {
        self.halfmoves >= 100 && num_legal_moves != 0
    }

    /// Generate a MoveInfo for this position from a given Move.
    pub fn move_info(&self, move_: Move) -> MoveInfo {
        let moved_piece_kind = self
            .pieces
            .on_player_square(self.player, move_.from)
            .expect("no piece on `from` square");

        let mut move_kind = MoveKind::Quiet;

        // Check for Capture. Each mode below is mutually exclusive.
        if let Some(pk) = self.pieces.on_player_square(!self.player, move_.to) {
            move_kind = MoveKind::Capture(pk);
        }
        // Check for EnPassant.
        else if moved_piece_kind == Pawn {
            if let Some(ep_square) = self.en_passant {
                if move_.to == ep_square {
                    move_kind = MoveKind::EnPassant;
                }
            }
        }
        // Check for Castling
        else if moved_piece_kind == King {
            match (move_.from, move_.to) {
                (E1, C1) | (E1, G1) | (E8, C8) | (E8, G8) => move_kind = MoveKind::Castle,
                _ => (),
            };
        }

        MoveInfo::new(move_, moved_piece_kind, move_kind)
    }

    /// Returns this position's cached state.
    pub fn cache(&self) -> Cache {
        Cache::from(self)
    }

    /// Halfmoves is set to zero after a capture or pawn move, incremented otherwise.
    /// There is no unset because this value is cached.
    fn step_halfmoves(&mut self, move_info: &MoveInfo) {
        let is_pawn_move = move_info.piece_kind == Pawn;
        let is_capture = matches!(move_info.move_kind, MoveKind::Capture(_));

        match is_pawn_move || is_capture {
            true => self.halfmoves = 0,
            false => self.halfmoves += 1,
        }
    }

    /// Fullmoves is incremented after each Black player's move.
    fn step_fullmoves(&mut self) {
        if *self.player() == Black {
            self.fullmoves += 1;
        }
    }

    // Fullmoves increments after Black move, so decrements before White move.
    // It starts at 1, so cannot go below that.
    fn unstep_fullmoves(&mut self) {
        if *self.player() == White && *self.fullmoves() > 1 {
            self.fullmoves -= 1;
        }
    }

    /// Apply a move to self, in place.
    /// `do_move` does not check if the move is legal or not,
    /// it simply executes it while assuming legality.
    /// Castling is described by moving king 2 squares, as defined in UCI protocol.
    /// Assumptions:
    /// There is an active player's piece on from square.
    /// There is no active player's piece on to square.
    /// A double king move from starting position is a castling.
    /// Current behavior:
    /// Removes from square from active player piece on that square.
    /// Removes to square from all passive player pieces.
    /// Panics if from square has no active player piece.
    pub fn do_move(&mut self, move_: Move) -> MoveInfo {
        let move_info = self.move_info(move_);
        self.do_move_info(move_info);
        move_info
    }

    /// Incrementally apply a move to self, in place.
    /// This assumes the given move_info is legal.
    pub fn do_move_info(&mut self, move_info: MoveInfo) {
        let player = *self.player();
        let active_piece = Piece::new(player, move_info.piece_kind);

        // These always get updated, regardless of move.
        self.step_halfmoves(&move_info);
        self.step_fullmoves();
        self.en_passant = None;
        self.pieces[active_piece].clear_square(move_info.from);
        self.player = !self.player;

        // If promoting, place promoting piece. Otherwise place active piece.
        if let Some(promoting_piece_kind) = move_info.promotion {
            let promoting_piece = Piece::new(player, promoting_piece_kind);
            self.pieces[promoting_piece].set_square(move_info.to);
        } else {
            self.pieces[active_piece].set_square(move_info.to);
        }

        // Handle all special moves.
        match move_info.move_kind {
            // Clear opposing player's captured piece.
            MoveKind::Capture(piece_kind) => {
                let captured_piece = Piece::new(!player, piece_kind);
                self.pieces[captured_piece].clear_square(move_info.to);
            }
            // Remove captured pawn near the en-passant square.
            MoveKind::EnPassant => {
                let to = Bitboard::from(move_info.to);
                let captured_pawn = mg::pawn_single_pushes(to, !player);
                self.pieces[(!player, Pawn)].remove(&captured_pawn);
            }
            // Move Rook to castling square and clear castling rights.
            MoveKind::Castle => {
                let castling_rook_squares = match (move_info.from, move_info.to) {
                    (E1, G1) => (H1, F1), // White Kingside
                    (E1, C1) => (A1, D1), // White Queenside
                    (E8, G8) => (H8, F8), // Black Kingside
                    (E8, C8) => (A8, D8), // Black Queenside
                    _ => panic!("move_kind is Castle however squares are illegal"),
                };
                let (clear, set) = castling_rook_squares;
                let active_rook = (active_piece.color, Rook);
                self.pieces[active_rook].clear_square(clear);
                self.pieces[active_rook].set_square(set);

                self.castling.clear_color(player);
            }

            // Handle special quiet case where pawn double jumps.
            MoveKind::Quiet => {
                if move_info.piece_kind == Pawn {
                    let from = Bitboard::from(move_info.from);
                    let to = Bitboard::from(move_info.to);
                    let from_start_row = from & (Bitboard::RANK_2 | Bitboard::RANK_7);
                    let to_jump_row = to & (Bitboard::RANK_4 | Bitboard::RANK_5);

                    if !from_start_row.is_empty() && !to_jump_row.is_empty() {
                        let pawn = Bitboard::from(move_info.from);
                        self.en_passant = mg::pawn_single_pushes(pawn, player).get_lowest_square();
                    }
                }
            }
        };

        // If any corner square is moved from or in to, remove those castling rights.
        // This covers active player moving rook, and passive player losing a rook.
        let moved_rights = match move_info.from {
            A1 => Castling::W_QUEEN,
            A8 => Castling::B_QUEEN,
            H1 => Castling::W_KING,
            H8 => Castling::B_KING,
            _ => Castling::NONE,
        };
        let captured_rights = match move_info.to {
            A1 => Castling::W_QUEEN,
            A8 => Castling::B_QUEEN,
            H1 => Castling::W_KING,
            H8 => Castling::B_KING,
            _ => Castling::NONE,
        };
        self.castling.clear(moved_rights | captured_rights);

        // If King has moved, remove all castling rights.
        if move_info.piece_kind == King {
            self.castling.clear_color(player);
        }

        debug_assert!(self.pieces().is_valid());
    }

    /// Undo the application of a move, in place.
    pub fn undo_move(&mut self, move_info: MoveInfo, cache: Cache) {
        self.unstep_fullmoves();
        self.player = !self.player;
        self.castling = cache.castling;
        self.en_passant = cache.en_passant;
        self.halfmoves = cache.halfmoves;

        // Side-to-move of self is currently set to player who made original move.
        // If the player captured a piece, need to restore captured piece.
        // If player did en-passant, need to restore captured piece.
        // If player castled, need to restore king and rook positions.
        // If player promoted, need to remove promoted piece on to square, add original piece to from square.
        let player = *self.player();

        // Restore explicitly moved piece of move's active player.
        let moved_piece = Piece::new(player, move_info.piece_kind);
        self.pieces[moved_piece].set_square(move_info.from);
        self.pieces[moved_piece].clear_square(move_info.to);
        if let Some(promoted) = move_info.promotion {
            self.pieces[(player, promoted)].clear_square(move_info.to);
        }

        // Handle special MoveKind cases.
        match move_info.move_kind {
            MoveKind::Capture(piece_kind) => {
                self.pieces[(!player, piece_kind)].set_square(move_info.to);
            }

            MoveKind::Castle => {
                // Identify what kind of castle.
                let (rook_from, rook_to) = match move_info.to {
                    C1 => (A1, D1), // White Queenside
                    G1 => (H1, F1), // White Kingside
                    C8 => (A8, D8), // Black Queenside
                    G8 => (H8, F8), // Black Kingside
                    _ => panic!("MoveKind is Castle but Move is not a castling move."),
                };
                // Restore Rook position before castling.
                self.pieces[(player, Rook)].set_square(rook_from);
                self.pieces[(player, Rook)].clear_square(rook_to);
            }

            MoveKind::EnPassant => {
                let ep_square = cache
                    .en_passant
                    .expect("MoveKind is EnPassant, but en_passant square is not set.");
                let ep_bb = Bitboard::from(ep_square);
                let original_bb = mg::pawn_single_pushes(ep_bb, !player);
                self.pieces[(!player, Pawn)] |= original_bb;
            }

            _ => (),
        }
        debug_assert!(self.pieces().is_valid());
    }

    /// Checks if move is legal before applying it.
    /// If move is legal, the move is applied and returns the resulting MoveInfo.
    /// Otherwise, no action is taken and returns None.
    /// This is best used as a CLI function, not in the engine.
    pub fn do_legal_move(&mut self, move_: Move) -> Option<MoveInfo> {
        if self.is_legal_move(move_) {
            Some(self.do_move(move_))
        } else {
            None
        }
    }

    /// Check if the current position is checkmated.
    /// Returns true if it is mate, false otherwise.
    pub fn is_checkmate(&self) -> bool {
        let legal_moves = self.get_legal_moves();
        if legal_moves.len() == 0 && self.is_in_check() {
            true
        } else {
            false
        }
    }

    /// Check if the current position is stalemated.
    /// Returns true if it is stalemate, false otherwise.
    pub fn is_stalemate(&self) -> bool {
        let legal_moves = self.get_legal_moves();
        if legal_moves.len() == 0 && !self.is_in_check() {
            true
        } else {
            false
        }
    }

    /// Generates a new Position from applying move on current Position.
    pub fn make_move(&self, move_: Move) -> Self {
        let mut position_clone: Position = self.clone();
        position_clone.do_move(move_);
        position_clone
    }

    /// Checks if given move is legal for current position.
    pub fn is_legal_move(&self, move_: Move) -> bool {
        let legal_moves = self.get_legal_moves();
        legal_moves.contains(&move_)
    }

    /// Returns true if active player's king is in any check.
    pub fn is_in_check(&self) -> bool {
        self.num_active_king_checks() > 0
    }

    /// Returns tuple representing if current player's king is in single or double check.
    /// Tuple format: (is_in_single_check, is_in_double_check).
    pub fn active_king_checks(&self) -> (bool, bool) {
        let num_checks = self.num_active_king_checks();
        let single_check = num_checks >= 1;
        let double_check = num_checks >= 2;
        (single_check, double_check)
    }

    pub fn num_active_king_checks(&self) -> u32 {
        let king_bb = self.pieces[(self.player, King)];
        let king_square = king_bb.get_lowest_square().unwrap();
        let king_attackers = self.attackers_to(king_square, !self.player);
        king_attackers.count_squares()
    }

    /// Returns bitboard with positions of all pieces of a player attacking a square.
    /// Assumes there is no overlap for pieces of a color.
    pub fn attackers_to(&self, target: Square, attacking: Color) -> Bitboard {
        let pawns = self.pieces[(attacking, Pawn)];
        let knights = self.pieces[(attacking, Knight)];
        let king = self.pieces[(attacking, King)];
        let bishops = self.pieces[(attacking, Bishop)];
        let rooks = self.pieces[(attacking, Rook)];
        let queens = self.pieces[(attacking, Queen)];

        let occupied = self.pieces().occupied();

        mg::pawn_attackers_to(target, pawns, attacking)
            | mg::knight_attackers_to(target, knights)
            | mg::king_attackers_to(target, king)
            | mg::bishop_attackers_to(target, bishops, occupied)
            | mg::rook_attackers_to(target, rooks, occupied)
            | mg::queen_attackers_to(target, queens, occupied)
    }

    /// Returns true if target square is attacked by any piece of attacking color.
    pub fn is_attacked_by(&self, target: Square, attacking: Color) -> bool {
        self.attackers_to(target, attacking).count_squares() > 0
    }

    /// Returns bitboard with all squares attacked by a player's pieces.
    pub fn attacks(&self, attacking: Color, occupied: Bitboard) -> Bitboard {
        let pawns = self.pieces[(attacking, Pawn)];
        let knights = self.pieces[(attacking, Knight)];
        let king = self.pieces[(attacking, King)];
        let rooks = self.pieces[(attacking, Rook)];
        let bishops = self.pieces[(attacking, Bishop)];
        let queens = self.pieces[(attacking, Queen)];

        mg::pawn_attacks(pawns, attacking)
            | mg::knight_attacks(knights)
            | mg::king_attacks(king)
            | mg::slide_attacks(queens, rooks, bishops, occupied)
    }

    /// Returns a list of all legal moves for active player in current position.
    /// This operation is expensive.
    /// Notes:
    /// If En-Passant, need to check for sliding piece check discovery.
    /// If king is in check, number of moves are restricted.
    /// If king is pinned, number of moves are restricted.
    pub fn get_legal_moves(&self) -> MoveList {
        let (single_check, double_check) = self.active_king_checks();

        if double_check {
            self.generate_legal_double_check_moves()
        } else if single_check {
            self.generate_legal_single_check_moves()
        } else {
            self.generate_legal_no_check_moves()
        }
    }

    /// Generate a list of all legal capture moves the active player can make in
    /// the current position.
    //pub fn get_legal_captures(&self) -> MoveInfoList {
    //    let (single_check, double_check) = self.active_king_checks();

    //    if double_check {
    //        self.generate_legal_double_check_captures()
    //    } else if single_check {
    //        self.generate_legal_single_check_captures()
    //    } else {
    //        self.generate_legal_no_check_captures()
    //    }
    //}

    /// Generate king moves assuming double check.
    /// Only the king can move when in double check.
    fn generate_legal_double_check_moves(&self) -> MoveList {
        let king = self.pieces[(self.player, King)];

        // Generate bitboard with all squares attacked by passive player.
        // Sliding pieces x-ray king.
        let passive_player = !self.player;
        let occupied_without_king = self.pieces.occupied() & !king;
        let attacked = self.attacks(passive_player, occupied_without_king);

        // Filter illegal moves from pseudo-legal king moves.
        // King cannot move into attacked square, or into piece of same color.
        let mut possible_moves = mg::king_attacks(king);
        possible_moves.remove(&attacked);
        possible_moves.remove(&self.pieces.color_occupied(self.player));

        // Convert remaining move squares into Move structs.
        let mut legal_moves = MoveList::new(); // Eight max possible moves.
        let from = king.get_lowest_square().unwrap();
        for to in possible_moves {
            legal_moves.push(Move::new(from, to, None));
        }

        legal_moves
    }

    /// Generate moves assuming active player is in single check.
    fn generate_legal_single_check_moves(&self) -> MoveList {
        // Can capture checking piece with non-absolute-pinned piece,
        // move king to non-attacked squares,
        // block checking piece with non-absolute-pinned piece
        let mut legal_moves: MoveList = MoveList::new();

        let king = self.pieces[(self.player, King)];
        let king_square = king.get_lowest_square().unwrap();
        let passive_player = !self.player;
        let us = self.pieces.color_occupied(self.player);
        let them = self.pieces.color_occupied(passive_player);
        let occupied = self.pieces.occupied();

        // Generate all legal king moves.
        let occupied_without_king = occupied & !king;
        let attacked_xray_king = self.attacks(passive_player, occupied_without_king);
        let mut possible_moves = mg::king_attacks(king);
        possible_moves.remove(&attacked_xray_king);
        possible_moves.remove(&us);
        for to in possible_moves {
            legal_moves.push(Move::new(king_square, to, None));
        }

        // Notes
        // Only sliding pieces can cause absolute pins and pins in general.
        // If a piece is absolutely pinned, it can only move along pinned direction.
        // a pinning piece must already pseudo attack the king to absolutely pin.
        // If there are multiple in between pieces, there is no pin.
        // Once a piece is known to be pinned, how to determine where it can move?
        // Algorithm:
        // For each sliding piece, check if it pseudo checks the king.
        // If it does, need to find if there is a single piece between them of active color.
        // Sliding checker can be blocked or captured with non-pinned piece.
        // If not sliding, then checker can be captured with non-pinned piece.
        // TODO: Make more efficient (change from verifying by making move).
        let queens = self.pieces[(self.player, Queen)];
        let rooks = self.pieces[(self.player, Rook)];
        let bishops = self.pieces[(self.player, Bishop)];
        let knights = self.pieces[(self.player, Knight)];
        let pawns = self.pieces[(self.player, Pawn)];

        let mut pseudo_moves = MoveList::new();
        mg::queen_pseudo_moves(&mut pseudo_moves, queens, occupied, us);
        mg::rook_pseudo_moves(&mut pseudo_moves, rooks, occupied, us);
        mg::bishop_pseudo_moves(&mut pseudo_moves, bishops, occupied, us);
        mg::knight_pseudo_moves(&mut pseudo_moves, knights, us);
        mg::pawn_pseudo_moves(
            &mut pseudo_moves,
            pawns,
            self.player,
            occupied,
            them,
            self.en_passant,
        );

        let mut position = self.clone();
        let cache = position.cache();
        pseudo_moves
            .into_iter()
            .filter(|pseudo_move| {
                let move_info = position.do_move(*pseudo_move);
                let is_legal = !position.is_attacked_by(king_square, passive_player);
                position.undo_move(move_info, cache);
                is_legal
            })
            .for_each(|legal_move| legal_moves.push(legal_move));

        legal_moves
    }

    /// Generate moves assuming active player is not in check.
    fn generate_legal_no_check_moves(&self) -> MoveList {
        // moves:
        // move absolutely-pinned piece along pin direction
        // Castling with no pieces or attacked squares between
        // en-passant with horizontal move test and conditionally pinned?
        // king not attacked by pawns, knights, kings.
        // For non king moves, only need to consider leaving absolute pin.
        // For king moves, need to consider all attacked squares.
        // Most positions will have fewer moves than this capacity.
        let mut legal_moves = MoveList::new();

        let king = self.pieces[(self.player, King)];
        let king_square = king.get_lowest_square().unwrap();
        let passive_player = !self.player;
        let us = self.pieces.color_occupied(self.player);
        let them = self.pieces.color_occupied(passive_player);
        let occupied = us | them;
        let attacked = self.attacks(passive_player, occupied);

        let (absolute_pins, _pinned_moves) = {
            let queens = self.pieces[(passive_player, Queen)];
            let rooks = self.pieces[(passive_player, Rook)];
            let bishops = self.pieces[(passive_player, Bishop)];

            mg::absolute_pins(king_square, us, them, queens | rooks, queens | bishops)
        };

        // Generate all normal Queen, Rook, Bishop, Knight moves.
        // Generate all normal and special Pawn moves (single/double push, attacks, ep).
        let queens = self.pieces[(self.player, Queen)];
        let rooks = self.pieces[(self.player, Rook)];
        let bishops = self.pieces[(self.player, Bishop)];
        let knights = self.pieces[(self.player, Knight)];
        let pawns = self.pieces[(self.player, Pawn)];

        // Generate strictly legal moves using pinned data.
        let knights_free = knights & !absolute_pins;
        let bishops_free = bishops & !absolute_pins;
        let queens_free = queens & !absolute_pins;
        let rooks_free = rooks & !absolute_pins;
        mg::knight_pseudo_moves(&mut legal_moves, knights_free, us);
        mg::bishop_pseudo_moves(&mut legal_moves, bishops_free, occupied, us);
        mg::queen_pseudo_moves(&mut legal_moves, queens_free, occupied, us);
        mg::rook_pseudo_moves(&mut legal_moves, rooks_free, occupied, us);

        // Generate all normal legal king moves.
        let mut king_tos = mg::king_attacks(king);
        king_tos.remove(&us);
        king_tos.remove(&attacked);
        for to in king_tos {
            legal_moves.push(Move::new(king_square, to, None));
        }

        // Generate pseudo moves and check for legality with "do/undo".
        let mut pseudo_moves = MoveList::new();
        let bishops_pinned = bishops & absolute_pins;
        let rooks_pinned = rooks & absolute_pins;
        let queens_pinned = queens & absolute_pins;

        mg::queen_pseudo_moves(&mut pseudo_moves, queens_pinned, occupied, us);
        mg::rook_pseudo_moves(&mut pseudo_moves, rooks_pinned, occupied, us);
        mg::bishop_pseudo_moves(&mut pseudo_moves, bishops_pinned, occupied, us);
        mg::pawn_pseudo_moves(
            &mut pseudo_moves,
            pawns,
            self.player,
            occupied,
            them,
            self.en_passant,
        );

        let mut position = self.clone();
        let cache = position.cache();
        pseudo_moves
            .into_iter()
            .filter(|pseudo_move| {
                let move_info = position.do_move(*pseudo_move);
                let is_legal = !position.is_attacked_by(king_square, passive_player);
                position.undo_move(move_info, cache);
                is_legal
            })
            .for_each(|legal_move| legal_moves.push(legal_move));

        // Generate Castling moves
        // Check if current player can castle. If can, for each side that can castle,
        // check if there are any pieces between king and castling rook.
        // check if king will pass through an attacked square.
        mg::legal_castling_moves(
            &mut legal_moves,
            self.player,
            self.castling,
            occupied,
            attacked,
        );

        legal_moves
    }

    // Generate all captures possible while in double check, where only king can move.
    // fn generate_legal_double_check_captures(&self) -> MoveInfoList {
    //     let king_bb = self.pieces[(self.player, King)];
    //     let enemy = !self.player;

    //     // Generate bitboard with all squares attacked by enemy player.
    //     // Remove king so enemy attacks x-ray the king.
    //     let occupied_without_king = self.pieces.occupied() & !king_bb;
    //     let attacked = self.attacks(enemy, occupied_without_king);

    //     // Extract only legal captures by removing attacked squares and non-enemy squares.
    //     let mut possible_captures = mg::king_attacks(king_bb);
    //     possible_captures.remove(&attacked);
    //     possible_captures.remove(&!self.pieces.color_occupied(enemy));

    //     let mut legal_captures = MoveInfoList::new();

    //     // Convert each capture into a MoveInfo.
    //     let from = king_bb.get_lowest_square().unwrap();
    //     for to in possible_captures {
    //         let captured_pk = self.pieces.on_player_square(enemy, to).unwrap();
    //         let move_kind = MoveKind::Capture(captured_pk);

    //         legal_captures.push(MoveInfo::new(
    //             Move::new(from, to, None),
    //             King,
    //             move_kind,
    //             self.castling,
    //             self.en_passant,
    //             self.halfmoves,
    //         ));
    //     }

    //     legal_captures
    // }
}

/// Defaults to standard chess start position.
impl Default for Position {
    fn default() -> Self {
        Self::start_position()
    }
}

/// Displays pretty-printed chess board and Fen string representing Position.
impl Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // print: position, FEN string
        write!(f, "{}\n Fen: {}\n", self.pieces, self.to_fen())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_print_position() {
        let start_pos = Position::start_position();
        println!("{}", start_pos);
    }

    #[test]
    fn do_move_with_legal_move() {
        let move1 = Move::new(E2, E4, None);
        let move1_piece = Piece::new(White, Pawn);
        let mut position = Position::start_position();
        position.do_move(move1);
        assert!(position.pieces[&move1_piece].has_square(E4));
        assert!(!position.pieces[&move1_piece].has_square(E2));
    }

    #[test]
    fn undo_move_with_legal_move() {
        {
            // Start position
            let pos = Position::start_position();
            let cache = pos.cache();
            let mut pos_moved = pos.clone();
            let move_ = Move::new(E2, E4, None);
            let move_info = pos_moved.do_move(move_);
            pos_moved.undo_move(move_info, cache);
            assert_eq!(pos, pos_moved);
            assert_eq!(Move::from(move_info), move_);
            assert_eq!(move_info.piece_kind, Pawn);
            assert_eq!(move_info.move_kind, MoveKind::Quiet);
            assert_eq!(cache.castling, Castling::ALL);
            assert_eq!(cache.en_passant, None);
        }
        {
            // En passant
            let pos = Position::parse_fen(
                "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
            )
            .unwrap();
            let mut pos_moved = pos.clone();
            let cache = pos_moved.cache();
            let move_ = Move::new(E5, F6, None);
            let move_info = pos_moved.do_move(move_);
            pos_moved.undo_move(move_info, cache);
            assert_eq!(pos, pos_moved);
            assert_eq!(Move::from(move_info), move_);
            assert_eq!(move_info.piece_kind, Pawn);
            assert_eq!(move_info.move_kind, MoveKind::EnPassant);
            assert_eq!(cache.castling, Castling::ALL);
            assert_eq!(cache.en_passant, Some(F6));
        }
        {
            // Kingside Castling
            let pos = Position::parse_fen(
                "rn1qkbnr/ppp3pp/5p2/8/2Bp2b1/5N2/PPPP1PPP/RNBQK2R w KQkq - 2 6",
            )
            .unwrap();
            let mut pos_moved = pos.clone();
            let cache = pos_moved.cache();
            let move_ = Move::new(E1, G1, None);
            let move_info = pos_moved.do_move(move_);
            pos_moved.undo_move(move_info, cache);
            assert_eq!(pos, pos_moved);
            assert_eq!(Move::from(move_info), move_);
            assert_eq!(move_info.piece_kind, King);
            assert_eq!(move_info.move_kind, MoveKind::Castle);
            assert_eq!(cache.castling, Castling::ALL);
            assert_eq!(cache.en_passant, None);
        }
    }

    #[test]
    fn king_checks() {
        let check1_1 = Position::parse_fen("8/8/8/8/3K3r/8/8/8 w - - 0 1").unwrap();
        let check1_2 =
            Position::parse_fen("rnb1kbnr/ppp1pppp/8/3p4/1qPPP3/8/PP3PPP/RNBQKBNR w KQkq - 1 4")
                .unwrap();
        let check2_1 = Position::parse_fen("3q4/8/4b3/3k4/4P1n1/8/3Q4/2R5 b - - 0 1").unwrap();
        let check4_1 =
            Position::parse_fen("6b1/2r1r3/pp4n1/4K2r/2p5/7p/1p1q2q1/4r2r w - - 0 1").unwrap();
        let check5_1 = Position::parse_fen("4r3/8/2b2n2/5p2/4K3/5q2/8/8 w - - 0 1").unwrap();
        let check5_2 = Position::parse_fen("8/8/5n2/3brp2/Q3K2q/5P2/3N4/1B2R3 w - - 0 1").unwrap();

        assert_eq!(check1_1.num_active_king_checks(), 1);
        assert_eq!(check1_2.num_active_king_checks(), 1);
        assert_eq!(check2_1.num_active_king_checks(), 2);
        assert_eq!(check4_1.num_active_king_checks(), 4);
        assert_eq!(check5_1.num_active_king_checks(), 5);
        assert_eq!(check5_2.num_active_king_checks(), 5);
    }

    #[test]
    fn legal_double_check_moves() {
        let pos0_1 = Position::parse_fen("4R2k/7p/6p1/8/8/2B5/8/1K6 b - - 0 1").unwrap();
        let pos1_1 = Position::parse_fen("8/5K2/8/3Qk3/4R3/8/8/8 b - - 0 1").unwrap();
        let pos3_1 = Position::parse_fen("8/2k5/8/8/4Kr2/4r3/8/8 w - - 0 1").unwrap();

        let moves0_1 = pos0_1.generate_legal_double_check_moves();
        let moves1_1 = pos1_1.generate_legal_double_check_moves();
        let moves3_1 = pos3_1.generate_legal_double_check_moves();
        assert_eq!(moves0_1.len(), 0);
        assert_eq!(moves1_1.len(), 1);
        assert_eq!(moves3_1.len(), 3);

        assert!(moves1_1.contains(&Move::new(E5, D5, None)));

        assert!(moves3_1.contains(&Move::new(E4, D5, None)));
        assert!(moves3_1.contains(&Move::new(E4, E3, None)));
        assert!(moves3_1.contains(&Move::new(E4, F4, None)));
    }

    #[test]
    fn checkmated() {
        {
            let pos1 =
                Position::parse_fen("rnb1k1nr/ppp2ppp/4p3/8/P7/1Pb3BQ/3qPPPP/4KBNR w Kkq - 0 14")
                    .unwrap();
            let moves1 = pos1.get_legal_moves();
            assert_eq!(moves1.len(), 0);
        }

        {
            let pos2 =
                Position::parse_fen("r4r1k/1b3p1p/pp2pQ2/2p5/P1B3R1/3P3P/2q3P1/7K b - - 0 26")
                    .unwrap();
            let moves2 = pos2.get_legal_moves();
            assert_eq!(moves2.len(), 0);
        }
    }

    #[test]
    fn stalemated() {
        let pos1 = Position::parse_fen("8/8/8/8/p7/P3k3/4p3/4K3 w - - 1 2").unwrap();

        let moves1 = pos1.get_legal_moves();
        assert_eq!(moves1.len(), 0);
    }

    #[test]
    fn color_flipped_eq() {
        // Manually check flipped positions.
        // Flipping one gets the other, and vice-versa.
        let w_giuoco = Position::parse_fen(
            "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 6 4",
        )
        .unwrap();
        let b_giuoco = Position::parse_fen(
            "rnbqk2r/pppp1ppp/5n2/2b1p3/2B1P3/2N5/PPPP1PPP/R1BQK1NR b KQkq - 6 4",
        )
        .unwrap();

        assert_eq!(w_giuoco.color_flip(), b_giuoco);
        assert_eq!(b_giuoco.color_flip(), w_giuoco);
    }
}
