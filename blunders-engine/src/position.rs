//! Holds Position struct, the most important data structure for the engine.
//! Position represents a chess position.

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
        let double_push = mg::pawn_pseudo_double_moves(&pawn, active_piece.color());

        if to == double_push {
            self.en_passant = mg::pawn_pseudo_single_moves(&pawn, active_piece.color())
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
    /// Does nothing if active player has no piece on from square.
    /// Removes from square from active player piece on that square.
    /// Removes to square from all passive player pieces.
    pub fn do_move(&mut self, move_: Move) {
        // Find piece on `from` square for active player if exists.
        let maybe_active_piece: Option<Piece> = PieceKind::iter()
            .map(|piece_kind| Piece::new(*self.side_to_move(), piece_kind))
            .find(|piece| self.pieces()[piece].has_square(move_.from));

        // Only do something if we have a piece to move.
        if let Some(active_piece) = maybe_active_piece {
            self.update_en_passant(&move_, &active_piece);
            self.update_move_counters(&move_, &active_piece);

            self.pieces[&active_piece].clear_square(move_.from);

            // Clear all passive (non-playing) player's pieces on `to` square.
            let passive_player = !self.side_to_move();
            PieceKind::iter()
                .map(|piece_kind| Piece::new(passive_player, piece_kind))
                .for_each(|passive_piece| self.pieces[&passive_piece].clear_square(move_.to));

            // If promoting, set promoting piece. Otherwise set active piece.
            if let Some(promoting_piece_kind) = move_.promotion {
                let promoting_piece = Piece::new(*self.side_to_move(), promoting_piece_kind);
                self.pieces[&promoting_piece].set_square(move_.to);
            } else {
                self.pieces[&active_piece].set_square(move_.to);
            }
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
        let active_king = self.pieces[&(self.side_to_move, King)];
        let king = active_king.squares()[0];
        let passive_player = !self.side_to_move();

        let passive_pawns = self.pieces[&(passive_player, Pawn)];
        let passive_knights = self.pieces[&(passive_player, Knight)];
        let passive_king = self.pieces[&(passive_player, King)];
        let passive_bishops = self.pieces[&(passive_player, Bishop)];
        let passive_rooks = self.pieces[&(passive_player, Rook)];
        let passive_queens = self.pieces[&(passive_player, Queen)];

        let occupied = self.pieces().occupied();

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

    /// Returns a list of all legal moves for active player.
    /// Notes:
    /// If En-Passant, need to check for sliding piece check discovery.
    /// If king is in check, number of moves are restricted.
    /// If king is pinned, number of moves are restricted.
    /// If not pinned or
    pub fn generate_legal_moves(&self) -> Vec<Move> {
        let legal_moves = Vec::with_capacity(64);

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
}
