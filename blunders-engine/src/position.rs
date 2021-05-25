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
use crate::fen::Fen;
use crate::movegen as mg;
use crate::movelist::MoveList;

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

    /// Updates the En-Passant position square, and handles any en-passant capture.
    /// En Passant square is set after any double pawn push.
    fn update_en_passant(&mut self, move_: &Move, active_piece: &Piece, move_info: &mut MoveInfo) {
        // Non-pawn pushes set to None.
        if active_piece.piece_kind != Pawn {
            self.en_passant = None;
            return;
        }
        let pawn = Bitboard::from(move_.from);
        let to = Bitboard::from(move_.to);
        let double_push = mg::pawn_double_pushes(&pawn, &active_piece.color);

        // Handle en passant capture. A pawn moving to an en-passant square is
        // only possible in en-passant capture.
        if let Some(ep_square) = self.en_passant() {
            if move_.to == *ep_square {
                move_info.move_kind = MoveKind::EnPassant;
                let passive_player = !active_piece.color;
                let captured_pawn = mg::pawn_single_pushes(&to, &passive_player);
                self.pieces[(passive_player, Pawn)].remove(&captured_pawn);
            }
        }

        // Set en passant square.
        if to == double_push {
            self.en_passant =
                mg::pawn_single_pushes(&pawn, &active_piece.color).get_lowest_square();
        } else {
            self.en_passant = None;
        }
    }

    /// Update Position move counters, as if move_ was applied to self.
    /// halfmoves is set to zero after a capture or pawn move, incremented otherwise.
    /// fullmoves is incremented after each Black player's move.
    fn update_move_counters(&mut self, move_: &Move, active_piece: &Piece) {
        // Update halfmoves
        let is_pawn_move = *active_piece.piece_kind() == Pawn;
        let is_capture = {
            let passive_player = !active_piece.color;
            self.pieces()[&passive_player]
                .iter()
                .any(|bb| bb.has_square(move_.to()))
        };

        if is_pawn_move || is_capture {
            self.halfmoves = 0;
        } else {
            self.halfmoves += 1;
        }

        // Update fullmoves
        if active_piece.color == Black {
            self.fullmoves += 1;
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
        // Find piece on `from` square for active player.
        let active_piece: Piece = PieceKind::iter()
            .map(|piece_kind| Piece::new(*self.player(), piece_kind))
            .find(|piece| self.pieces()[piece].has_square(move_.from))
            .expect("No piece on moving square.");

        // Store info on current position, before applying move.
        let mut move_info = MoveInfo::new(
            move_,
            active_piece.piece_kind,
            MoveKind::Quiet,
            *self.castling(),
            *self.en_passant(),
            *self.halfmoves(),
        );

        // These always get updated, regardless of move.
        // Note: Do not use self.player, instead refer to active_piece.color.
        self.update_en_passant(&move_, &active_piece, &mut move_info);
        self.update_move_counters(&move_, &active_piece);
        self.pieces[&active_piece].clear_square(move_.from);
        self.player = !self.player;

        // If promoting, place promoting piece. Otherwise place active piece.
        if let Some(promoting_piece_kind) = move_.promotion {
            let promoting_piece = Piece::new(active_piece.color, promoting_piece_kind);
            self.pieces[&promoting_piece].set_square(move_.to);
        } else {
            self.pieces[&active_piece].set_square(move_.to);
        }

        // If king moves, check if castling and remove castling rights.
        // (Castling: either king moved 2 squares sideways)
        if active_piece.piece_kind == King {
            let castling_rook_squares = match (move_.from, move_.to) {
                (E1, G1) => Some((H1, F1)), // White Kingside
                (E1, C1) => Some((A1, D1)), // White Queenside
                (E8, G8) => Some((H8, F8)), // Black Kingside
                (E8, C8) => Some((A8, D8)), // Black Queenside
                _ => None,
            };
            if let Some((clear, set)) = castling_rook_squares {
                move_info.move_kind = MoveKind::Castle;
                let active_rooks = (active_piece.color, Rook);
                self.pieces[active_rooks].clear_square(clear);
                self.pieces[active_rooks].set_square(set);
            }
            self.castling.clear_color(&active_piece.color);
        }

        // If any corner square is moved from or in to, remove those castling rights.
        // This covers active player moving rook, and passive player losing a rook.
        let moved_rights = match move_.from {
            A1 => Castling::W_QUEEN,
            A8 => Castling::B_QUEEN,
            H1 => Castling::W_KING,
            H8 => Castling::B_KING,
            _ => Castling::NONE,
        };
        let captured_rights = match move_.to {
            A1 => Castling::W_QUEEN,
            A8 => Castling::B_QUEEN,
            H1 => Castling::W_KING,
            H8 => Castling::B_KING,
            _ => Castling::NONE,
        };
        self.castling.clear(moved_rights | captured_rights);

        // Clear passive (non-playing) player's piece on `to` square if exists.
        let passive_player = !active_piece.color;
        PieceKind::iter()
            .find(|piece_kind| self.pieces()[(passive_player, *piece_kind)].has_square(move_.to))
            .into_iter()
            .for_each(|piece_kind| {
                move_info.move_kind = MoveKind::Capture(piece_kind);
                self.pieces[(passive_player, piece_kind)].clear_square(move_.to);
            });

        debug_assert!(self.pieces().is_valid());
        move_info
    }

    /// Undo the application of a move, in place.
    pub fn undo_move(&mut self, move_info: MoveInfo) {
        // Fullmoves increments after Black move, so decrements before White move.
        // It starts at 1, so cannot go below that.
        if *self.player() == White && *self.fullmoves() > 1 {
            self.fullmoves -= 1;
        }
        self.player = !self.player();
        self.castling = move_info.castling;
        self.en_passant = move_info.en_passant;
        self.halfmoves = move_info.halfmoves;

        // Side-to-move of self is currently set to player who made original move.
        // If the player captured a piece, need to restore captured piece.
        // If player did en-passant, need to restore captured piece.
        // If player castled, need to restore king and rook positions.
        // If player promoted, need to remove promoted piece on to square, add original piece to from square.
        let player = *self.player();

        // Restore explicitly moved piece of move's active player.
        let moved_piece = Piece::new(player, move_info.piece_kind);
        self.pieces[&moved_piece].set_square(move_info.move_.from);
        self.pieces[&moved_piece].clear_square(move_info.move_.to);
        if let Some(promoted) = move_info.move_.promotion {
            self.pieces[(player, promoted)].clear_square(move_info.move_.to);
        }

        // Handle special MoveKind cases.
        match move_info.move_kind {
            MoveKind::Capture(piece_kind) => {
                self.pieces[(!player, piece_kind)].set_square(move_info.move_.to);
            }

            MoveKind::Castle => {
                // Identify what kind of castle.
                let (rook_from, rook_to) = match move_info.move_.to {
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
                let ep_square = move_info
                    .en_passant
                    .expect("MoveKind is EnPassant, but en_passant square is not set.");
                let ep_bb = Bitboard::from(ep_square);
                let original_bb = mg::pawn_single_pushes(&ep_bb, &!player);
                self.pieces[(!player, Pawn)] |= original_bb;
            }

            MoveKind::Quiet => (),
        }
        debug_assert!(self.pieces().is_valid());
    }

    /// Checks if move is legal before applying it.
    /// If move is legal, the move is applied and returns true.
    /// Otherwise, no action is taken and returns false.
    /// This is best used as a CLI function, not in the engine.
    pub fn do_legal_move(&mut self, move_: Move) -> (bool, Option<MoveInfo>) {
        let legal_moves = self.get_legal_moves();
        if legal_moves.contains(&move_) {
            (true, Some(self.do_move(move_)))
        } else {
            (false, None)
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
    pub fn is_legal_move(&self, _move_: Move) -> bool {
        todo!()
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
        let king_attackers = self.attackers_to(&king_square, &!self.player);
        king_attackers.count_squares()
    }

    /// Returns bitboard with positions of all pieces of a player attacking a square.
    /// Assumes there is no overlap for pieces of a color.
    pub fn attackers_to(&self, target: &Square, attacking: &Color) -> Bitboard {
        let pawns = self.pieces[(*attacking, Pawn)];
        let knights = self.pieces[(*attacking, Knight)];
        let king = self.pieces[(*attacking, King)];
        let bishops = self.pieces[(*attacking, Bishop)];
        let rooks = self.pieces[(*attacking, Rook)];
        let queens = self.pieces[(*attacking, Queen)];

        let occupied = self.pieces().occupied();

        mg::pawn_attackers_to(target, &pawns, &attacking)
            | mg::knight_attackers_to(target, &knights)
            | mg::king_attackers_to(target, &king)
            | mg::bishop_attackers_to(target, &bishops, &occupied)
            | mg::rook_attackers_to(target, &rooks, &occupied)
            | mg::queen_attackers_to(target, &queens, &occupied)
    }

    /// Returns true if target square is attacked by any piece of attacking color.
    pub fn is_attacked_by(&self, target: &Square, attacking: &Color) -> bool {
        self.attackers_to(target, attacking).count_squares() > 0
    }

    /// Returns bitboard with all squares attacked by a player's pieces.
    pub fn attacks(&self, attacking: &Color, occupied: &Bitboard) -> Bitboard {
        let pawns = self.pieces[(*attacking, Pawn)];
        let knights = self.pieces[(*attacking, Knight)];
        let king = self.pieces[(*attacking, King)];
        let rooks = self.pieces[(*attacking, Rook)];
        let bishops = self.pieces[(*attacking, Bishop)];
        let queens = self.pieces[(*attacking, Queen)];

        mg::pawn_attacks(&pawns, &attacking)
            | mg::knight_attacks(&knights)
            | mg::king_attacks(&king)
            | mg::slide_attacks(&queens, &rooks, &bishops, &occupied)
    }

    /// Returns a list of all legal moves for active player in current position.
    /// This operation is expensive.
    /// Notes:
    /// If En-Passant, need to check for sliding piece check discovery.
    /// If king is in check, number of moves are restricted.
    /// If king is pinned, number of moves are restricted.
    /// If not pinned or
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

    /// Generate king moves assuming double check.
    /// Only the king can move when in double check.
    fn generate_legal_double_check_moves(&self) -> MoveList {
        let king = self.pieces[(self.player, King)];

        // Generate bitboard with all squares attacked by passive player.
        // Sliding pieces x-ray king.
        let passive_player = !self.player;
        let occupied_without_king = self.pieces.occupied() & !king;
        let attacked = self.attacks(&passive_player, &occupied_without_king);

        // Filter illegal moves from pseudo-legal king moves.
        // King cannot move into attacked square, or into piece of same color.
        let mut possible_moves = mg::king_attacks(&king);
        possible_moves.remove(&attacked);
        possible_moves.remove(&self.pieces.color_occupied(&self.player));

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
        let us = self.pieces.color_occupied(&self.player);
        let them = self.pieces.color_occupied(&passive_player);
        let occupied = self.pieces.occupied();

        // Generate all legal king moves.
        let occupied_without_king = occupied & !king;
        let attacked_xray_king = self.attacks(&passive_player, &occupied_without_king);
        let mut possible_moves = mg::king_attacks(&king);
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
        pseudo_moves
            .into_iter()
            .filter(|pseudo_move| {
                let move_info = position.do_move(*pseudo_move);
                let is_legal = !position.is_attacked_by(&king_square, &passive_player);
                position.undo_move(move_info);
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
        let us = self.pieces.color_occupied(&self.player);
        let them = self.pieces.color_occupied(&passive_player);
        let occupied = us | them;
        let attacked = self.attacks(&passive_player, &occupied);

        let (absolute_pins, _pinned_moves) = {
            let queens = self.pieces[(passive_player, Queen)];
            let rooks = self.pieces[(passive_player, Rook)];
            let bishops = self.pieces[(passive_player, Bishop)];

            mg::absolute_pins(king_square, us, them, queens | rooks, queens | bishops)
        };

        // Generate all normal legal king moves.
        let mut king_tos = mg::king_attacks(&king);
        king_tos.remove(&us);
        king_tos.remove(&attacked);
        for to in king_tos {
            legal_moves.push(Move::new(king_square, to, None));
        }

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
        let rooks_free = rooks & !absolute_pins;
        let queens_free = queens & !absolute_pins;
        mg::knight_pseudo_moves(&mut legal_moves, knights_free, us);
        mg::bishop_pseudo_moves(&mut legal_moves, bishops_free, occupied, us);
        mg::rook_pseudo_moves(&mut legal_moves, rooks_free, occupied, us);
        mg::queen_pseudo_moves(&mut legal_moves, queens_free, occupied, us);

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
        pseudo_moves
            .into_iter()
            .filter(|pseudo_move| {
                let move_info = position.do_move(*pseudo_move);
                let is_legal = !position.is_attacked_by(&king_square, &passive_player);
                position.undo_move(move_info);
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
            let mut pos_moved = pos.clone();
            let move_ = Move::new(E2, E4, None);
            let move_info = pos_moved.do_move(move_);
            pos_moved.undo_move(move_info);
            assert_eq!(pos, pos_moved);
            assert_eq!(move_info.move_, move_);
            assert_eq!(move_info.piece_kind, Pawn);
            assert_eq!(move_info.move_kind, MoveKind::Quiet);
            assert_eq!(move_info.castling, Castling::ALL);
            assert_eq!(move_info.en_passant, None);
        }
        {
            // En passant
            let pos = Position::parse_fen(
                "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
            )
            .unwrap();
            let mut pos_moved = pos.clone();
            let move_ = Move::new(E5, F6, None);
            let move_info = pos_moved.do_move(move_);
            pos_moved.undo_move(move_info);
            assert_eq!(pos, pos_moved);
            assert_eq!(move_info.move_, move_);
            assert_eq!(move_info.piece_kind, Pawn);
            assert_eq!(move_info.move_kind, MoveKind::EnPassant);
            assert_eq!(move_info.castling, Castling::ALL);
            assert_eq!(move_info.en_passant, Some(F6));
        }
        {
            // Kingside Castling
            let pos = Position::parse_fen(
                "rn1qkbnr/ppp3pp/5p2/8/2Bp2b1/5N2/PPPP1PPP/RNBQK2R w KQkq - 2 6",
            )
            .unwrap();
            let mut pos_moved = pos.clone();
            let move_ = Move::new(E1, G1, None);
            let move_info = pos_moved.do_move(move_);
            pos_moved.undo_move(move_info);
            assert_eq!(pos, pos_moved);
            assert_eq!(move_info.move_, move_);
            assert_eq!(move_info.piece_kind, King);
            assert_eq!(move_info.move_kind, MoveKind::Castle);
            assert_eq!(move_info.castling, Castling::ALL);
            assert_eq!(move_info.en_passant, None);
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
        let pos1 =
            Position::parse_fen("rnb1k1nr/ppp2ppp/4p3/8/P7/1Pb3BQ/3qPPPP/4KBNR w Kkq - 0 14")
                .unwrap();
        let moves1 = pos1.get_legal_moves();
        assert_eq!(moves1.len(), 0);
    }

    #[test]
    fn stalemated() {
        let pos1 = Position::parse_fen("8/8/8/8/p7/P3k3/4p3/4K3 w - - 1 2").unwrap();

        let moves1 = pos1.get_legal_moves();
        assert_eq!(moves1.len(), 0);
    }
}
