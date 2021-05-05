//! Holds Position struct, the most important data structure for the engine.
//! Position represents a chess position.

use std::fmt::{self, Display};

use crate::boardrepr::PieceSets;
use crate::coretypes::{Castling, Color, Move, MoveCount, Piece, PieceKind, Square};
use crate::fen::Fen;

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

    /// Apply a move to self, in place.
    /// `do_move` does not check if the move is legal or not,
    /// it simply executes it.
    /// Current behavior:
    /// Does nothing if active player has no piece on from square.
    /// Removes from square from active player piece on that square.
    /// Removes to square from all passive player pieces.
    pub fn do_move(&mut self, move_: Move) {
        // Find piece on `from` square for active player if exists.
        let maybe_active_piece: Option<Piece> = PieceKind::iter()
            .map(|piece_kind| Piece::new(self.side_to_move, piece_kind))
            .find(|piece| self.pieces[piece].has_square(move_.from));

        // Only do something if we have a piece to move.
        if let Some(active_piece) = maybe_active_piece {
            self.pieces[&active_piece].clear_square(move_.from);

            // Clear all passive (non-playing) player's pieces on to square.
            let passive_player = !self.side_to_move;
            PieceKind::iter()
                .map(|piece_kind| Piece::new(passive_player, piece_kind))
                .for_each(|passive_piece| self.pieces[&passive_piece].clear_square(move_.to));

            // If promoting, set promoting piece. Otherwise set active piece.
            if let Some(promoting_piece_kind) = move_.promotion {
                let promoting_piece = Piece::new(self.side_to_move, promoting_piece_kind);
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

    /// Returns true if king is in check.
    pub fn is_in_check(&self) -> bool {
        todo!()
    }

    /// Returns a list of all legal moves for active player.
    /// Notes:
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
}
