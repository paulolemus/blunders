//! Holds Position struct, the most important data structure for the engine.
//! Position represents a chess position.

use std::ops::{Index, IndexMut};

use crate::bitboard::Bitboard;
use crate::coretypes::{Castling, Color, Move, MoveCount, Piece, PieceKind, Square};
use crate::fen::Fen;
use crate::mailbox::Mailbox;

/// A Piece-Centric representation of pieces on a chessboard.
/// A Bitboard is used to encode the squares of each chess piece.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Pieces {
    pieces: [Bitboard; Self::SIZE],
}

impl Pieces {
    const SIZE: usize = 12; // 1 White, 1 Black BB for each piece type.
    fn new() -> Self {
        Pieces {
            pieces: [Bitboard::EMPTY; Self::SIZE],
        }
    }
}

impl Index<&Piece> for Pieces {
    type Output = Bitboard;
    fn index(&self, piece: &Piece) -> &Self::Output {
        &self.pieces[piece.color as usize + piece.piece_kind as usize]
    }
}

impl IndexMut<&Piece> for Pieces {
    fn index_mut(&mut self, piece: &Piece) -> &mut Self::Output {
        &mut self.pieces[piece.color as usize + piece.piece_kind as usize]
    }
}

/// Get a slice of all pieces of same color.
/// ```rust
/// # use blunders_engine::coretypes::Color;
/// # assert!((Color::White as usize) < Color::Black as usize);
/// ```
impl Index<&Color> for Pieces {
    type Output = [Bitboard];
    fn index(&self, color: &Color) -> &Self::Output {
        match color {
            Color::White => &self.pieces[Color::White as usize..Color::Black as usize],
            Color::Black => &self.pieces[Color::Black as usize..Self::SIZE],
        }
    }
}

impl IndexMut<&Color> for Pieces {
    fn index_mut(&mut self, color: &Color) -> &mut Self::Output {
        match color {
            Color::White => &mut self.pieces[Color::White as usize..Color::Black as usize],
            Color::Black => &mut self.pieces[Color::Black as usize..Self::SIZE],
        }
    }
}

impl From<&Mailbox> for Pieces {
    fn from(mb: &Mailbox) -> Self {
        let mut pieces = Pieces::new();

        for square in Square::iter() {
            if let Some(ref piece) = mb[square] {
                pieces[piece].set_square(square);
            }
        }
        pieces
    }
}

impl From<&Pieces> for Mailbox {
    fn from(pieces: &Pieces) -> Mailbox {
        let mut mb = Mailbox::new();

        for color in Color::iter() {
            for piece_kind in PieceKind::iter() {
                let piece = Piece::new(color, piece_kind);
                pieces[&piece]
                    .squares()
                    .into_iter()
                    .for_each(|square| mb[square] = Some(piece));
            }
        }
        mb
    }
}

/// Defaults to standard chess piece starting positions.
impl Default for Pieces {
    fn default() -> Self {
        let mb: Mailbox = Mailbox::default();
        Self::from(&mb)
    }
}

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
    pieces: Pieces,
    side_to_move: Color,
    castling: Castling,
    en_passant: Option<Square>,
    halfmoves: MoveCount,
    fullmoves: MoveCount,
}

impl Position {
    /// Standard chess start position.
    pub fn start_position() -> Self {
        Default::default()
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

    /// Generates a new Position from applying move on current move.
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
        Self {
            pieces: Default::default(),
            side_to_move: Color::White,
            castling: Default::default(),
            en_passant: None,
            halfmoves: 0,
            fullmoves: 1,
        }
    }
}

impl From<Fen> for Position {
    fn from(fen: Fen) -> Self {
        Self {
            pieces: fen.placement().into(),
            side_to_move: *fen.side_to_move(),
            castling: *fen.castling(),
            en_passant: *fen.en_passant(),
            halfmoves: *fen.halfmove_clock(),
            fullmoves: *fen.fullmove_number(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_pos_equal_to_fen_start_pos() {
        let start_pos = Position::start_position();
        let from_fen_start_pos = Position::from(Fen::start_position());
        assert_eq!(start_pos, from_fen_start_pos);
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
