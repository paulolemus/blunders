//! Holds Position struct, the most important data structure for the engine.
//! Position represents a chess position.
//! Positions and moves are assumed to be strictly legal,
//! and have undefined behavior for illegal activity.

use std::fmt::{self, Display};

use crate::bitboard::Bitboard;
use crate::boardrepr::PieceSets;
use crate::coretypes::{Castling, Color, Move, MoveCount, Piece, PieceKind, Square};
use crate::coretypes::{Color::*, PieceKind::*};
use crate::fen::Fen;
use crate::movegen as mg;

/// struct Position
/// A complete data set that can represent any chess position.
/// # Members:
/// * pieces - a piece-centric setwise container of all basic chess piece positions.
/// * side_to_move - Color of player whose turn it is.
/// * castling - Castling rights for both players.
/// * en_passant - Indicates if en passant is possible, and for which square.
/// * halfmoves - Tracker for 50 move draw rule. Resets after capture/pawn move.
/// * fullmoves - Starts at 1, increments after each black player's move.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    pub(crate) pieces: PieceSets,
    pub(crate) side_to_move: Color,
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
            side_to_move: Color::White,
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
    pub fn side_to_move(&self) -> &Color {
        &self.side_to_move
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

    /// En Passant square is set to a square after any double pawn push.
    /// Any other kind of push is set to None.
    fn update_en_passant(&mut self, move_: &Move, active_piece: &Piece) {
        // Non-pawn pushes set to None.
        if active_piece.piece_kind != Pawn {
            self.en_passant = None;
            return;
        }

        let pawn = Bitboard::from(move_.from());
        let to = Bitboard::from(move_.to());
        let double_push = mg::pawn_double_pushes(&pawn, active_piece.color());

        if to == double_push {
            self.en_passant = mg::pawn_single_pushes(&pawn, active_piece.color())
                .squares()
                .into_iter()
                .next();
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
            let passive_player = !self.side_to_move();
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
        if *self.side_to_move() == Black {
            self.fullmoves += 1;
        }
    }

    /// Apply a move to self, in place.
    /// `do_move` does not check if the move is legal or not,
    /// it simply executes it while assuming legality.
    /// Current behavior:
    /// Removes from square from active player piece on that square.
    /// Removes to square from all passive player pieces.
    /// Panics if from square has no active player piece.
    pub fn do_move(&mut self, move_: Move) {
        // Find piece on `from` square for active player.
        let active_piece: Piece = PieceKind::iter()
            .map(|piece_kind| Piece::new(*self.side_to_move(), piece_kind))
            .find(|piece| self.pieces()[piece].has_square(move_.from))
            .unwrap();

        self.update_en_passant(&move_, &active_piece);
        self.update_move_counters(&move_, &active_piece);

        self.pieces[&active_piece].clear_square(move_.from);

        // Clear all passive (non-playing) player's pieces on `to` square.
        let passive_player = !self.side_to_move();
        self.pieces[&passive_player]
            .iter_mut()
            .for_each(|bb| bb.clear_square(move_.to));

        // If promoting, set promoting piece. Otherwise set active piece.
        if let Some(promoting_piece_kind) = move_.promotion {
            let promoting_piece = Piece::new(*self.side_to_move(), promoting_piece_kind);
            self.pieces[&promoting_piece].set_square(move_.to);
        } else {
            self.pieces[&active_piece].set_square(move_.to);
        }
    }

    /// Undo the application of a move, in place.
    pub fn undo_move(&mut self, _move_: Move) {
        todo!()
    }

    /// Checks if move is legal before applying it.
    pub fn do_legal_move(&mut self, _move_: Move) -> Result<(), &'static str> {
        todo!()
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

    /// Counts and returns number of checks on current player's king.
    pub(crate) fn num_active_king_checks(&self) -> u32 {
        let passive_player = !self.side_to_move();
        let passive_pawns = self.pieces[&(passive_player, Pawn)];
        let passive_knights = self.pieces[&(passive_player, Knight)];
        let passive_king = self.pieces[&(passive_player, King)];
        let passive_bishops = self.pieces[&(passive_player, Bishop)];
        let passive_rooks = self.pieces[&(passive_player, Rook)];
        let passive_queens = self.pieces[&(passive_player, Queen)];

        let occupied = self.pieces().occupied();
        let active_king = self.pieces[&(self.side_to_move, King)];
        let king = active_king.squares()[0];

        let pawn_attackers = mg::pawn_attackers_to(&king, &passive_pawns, &passive_player);
        let knight_attackers = mg::knight_attackers_to(&king, &passive_knights);
        let king_attackers = mg::king_attackers_to(&king, &passive_king);
        let bishop_attackers = mg::bishop_attackers_to(&king, &passive_bishops, &occupied);
        let rook_attackers = mg::rook_attackers_to(&king, &passive_rooks, &occupied);
        let queen_attackers = mg::queen_attackers_to(&king, &passive_queens, &occupied);

        pawn_attackers.count_squares()
            + knight_attackers.count_squares()
            + king_attackers.count_squares()
            + bishop_attackers.count_squares()
            + rook_attackers.count_squares()
            + queen_attackers.count_squares()
    }

    /// Returns a list of all legal moves for active player in current position.
    /// Notes:
    /// If En-Passant, need to check for sliding piece check discovery.
    /// If king is in check, number of moves are restricted.
    /// If king is pinned, number of moves are restricted.
    /// If not pinned or
    pub fn get_legal_moves(&self) -> Vec<Move> {
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
    fn generate_legal_double_check_moves(&self) -> Vec<Move> {
        let king = self.pieces[(self.side_to_move, King)];

        // Generate bitboard with all squares attacked by passive player.
        let passive_player = !self.side_to_move;
        let passive_pawns = self.pieces[(passive_player, Pawn)];
        let passive_knights = self.pieces[(passive_player, Knight)];
        let passive_king = self.pieces[(passive_player, King)];
        let passive_rooks = self.pieces[(passive_player, Rook)];
        let passive_bishops = self.pieces[(passive_player, Bishop)];
        let passive_queens = self.pieces[(passive_player, Queen)];

        // Sliding pieces x-ray king.
        let occupied_without_king = self.pieces.occupied() & !king;
        let attacked = mg::pawn_attacks(&passive_pawns, &passive_player)
            | mg::knight_attacks(&passive_knights)
            | mg::king_attacks(&passive_king)
            | mg::rook_all_attacks(&passive_rooks, &occupied_without_king)
            | mg::bishop_all_attacks(&passive_bishops, &occupied_without_king)
            | mg::queen_all_attacks(&passive_queens, &occupied_without_king);

        // Filter illegal moves from pseudo-legal king moves.
        // King cannot move into attacked square, or into piece of same color.
        let mut possible_moves = mg::king_attacks(&king);
        possible_moves.remove(&attacked);
        possible_moves.remove(&self.pieces.color_occupied(&self.side_to_move));

        // Convert remaining move squares into Move structs.
        let mut legal_moves = Vec::with_capacity(8); // 8 max possible moves, although unlikely.
        let from = king.squares()[0];
        for to in possible_moves.squares() {
            legal_moves.push(Move::new(from, to, None));
        }

        legal_moves
    }

    /// Generate moves assuming active player is in single check.
    fn generate_legal_single_check_moves(&self) -> Vec<Move> {
        // Can capture checking piece with non-absolute-pinned piece,
        // move king to non-attacked squares,
        // block checking piece with non-absolute-pinned piece
        todo!()
    }

    /// Generate moves assuming active player is not in check.
    fn generate_legal_no_check_moves(&self) -> Vec<Move> {
        // moves:
        // move absolutely-pinned piece along pin direction
        // Castling with no pieces or attacked squares between
        // en-passant with horizontal move test and conditionally pinned?

        // king not attacked by pawns, knights, kings.
        // For non king moves, only need to consider leaving absolute pin.
        // For king moves, need to consider all attacked squares.

        todo!()
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
    use Square::*;

    #[test]
    fn pretty_print_position() {
        let start_pos = Position::start_position();
        println!("{}", start_pos);
    }

    #[test]
    fn do_move_with_legal_move() {
        let move1 = Move::new(Square::E2, Square::E4, None);
        let move1_piece = Piece::new(Color::White, PieceKind::Pawn);
        let mut position = Position::start_position();
        position.do_move(move1);
        assert!(position.pieces[&move1_piece].has_square(Square::E4));
        assert!(!position.pieces[&move1_piece].has_square(Square::E2));
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
}
