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

    /// Updates the En-Passant position square, and handles any en-passant capture.
    /// En Passant square is set after any double pawn push.
    fn update_en_passant(&mut self, move_: &Move, active_piece: &Piece) {
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
                let passive_player = !active_piece.color;
                let captured_pawn = mg::pawn_single_pushes(&to, &passive_player);
                self.pieces[&(passive_player, Pawn)].remove(&captured_pawn);
            }
        }

        // Set en passant square.
        if to == double_push {
            self.en_passant = mg::pawn_single_pushes(&pawn, &active_piece.color)
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
    pub fn do_move(&mut self, move_: Move) {
        use Square::*;
        // Find piece on `from` square for active player.
        let active_piece: Piece = PieceKind::iter()
            .map(|piece_kind| Piece::new(*self.side_to_move(), piece_kind))
            .find(|piece| self.pieces()[piece].has_square(move_.from))
            .expect("No piece on moving square.");

        // These always get updated, regardless of move.
        // Note: Do not use self.side_to_move, instead refer to active_piece.color.
        self.update_en_passant(&move_, &active_piece);
        self.update_move_counters(&move_, &active_piece);
        self.pieces[&active_piece].clear_square(move_.from);
        self.side_to_move = !self.side_to_move;

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
            let active_rooks = Piece::new(active_piece.color, Rook);
            if let Some((clear, set)) = castling_rook_squares {
                self.pieces[&active_rooks].clear_square(clear);
                self.pieces[&active_rooks].set_square(set);
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

        // Clear all passive (non-playing) player's pieces on `to` square.
        let passive_player = !active_piece.color;
        self.pieces[&passive_player]
            .iter_mut()
            .for_each(|bb| bb.clear_square(move_.to));
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

    pub fn num_active_king_checks(&self) -> u32 {
        let king_bb = self.pieces[&(self.side_to_move, King)];
        let king_square = king_bb.squares()[0];
        let king_attackers = self.attackers_to(&king_square, &!self.side_to_move);
        king_attackers.count_squares()
    }

    /// Returns bitboard with positions of all pieces of a player attacking a square.
    /// Assumes there is no overlap for pieces of a color.
    pub fn attackers_to(&self, target: &Square, attacking: &Color) -> Bitboard {
        let pawns = self.pieces[&(*attacking, Pawn)];
        let knights = self.pieces[&(*attacking, Knight)];
        let king = self.pieces[&(*attacking, King)];
        let bishops = self.pieces[&(*attacking, Bishop)];
        let rooks = self.pieces[&(*attacking, Rook)];
        let queens = self.pieces[&(*attacking, Queen)];

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
            | mg::rook_attacks(&rooks, &occupied)
            | mg::bishop_attacks(&bishops, &occupied)
            | mg::queen_attacks(&queens, &occupied)
    }

    /// Returns a list of all legal moves for active player in current position.
    /// This operation is expensive.
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
        // Sliding pieces x-ray king.
        let passive_player = !self.side_to_move;
        let occupied_without_king = self.pieces.occupied() & !king;
        let attacked = self.attacks(&passive_player, &occupied_without_king);

        // Filter illegal moves from pseudo-legal king moves.
        // King cannot move into attacked square, or into piece of same color.
        let mut possible_moves = mg::king_attacks(&king);
        possible_moves.remove(&attacked);
        possible_moves.remove(&self.pieces.color_occupied(&self.side_to_move));

        // Convert remaining move squares into Move structs.
        let mut legal_moves = Vec::with_capacity(8); // 8 max possible moves.
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
        let mut legal_moves: Vec<Move> = Vec::new();

        let king = self.pieces[(self.side_to_move, King)];
        let king_square = king.squares()[0];
        let passive_player = !self.side_to_move;
        let us = self.pieces.color_occupied(&self.side_to_move);
        let them = self.pieces.color_occupied(&passive_player);
        let occupied = self.pieces.occupied();

        // Generate all legal king moves.
        let occupied_without_king = occupied & !king;
        let attacked_xray_king = self.attacks(&passive_player, &occupied_without_king);
        let mut possible_moves = mg::king_attacks(&king);
        possible_moves.remove(&attacked_xray_king);
        possible_moves.remove(&us);
        for to in possible_moves.squares() {
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
        let queens = self.pieces[&(self.side_to_move, Queen)];
        let rooks = self.pieces[&(self.side_to_move, Rook)];
        let bishops = self.pieces[&(self.side_to_move, Bishop)];
        let knights = self.pieces[&(self.side_to_move, Knight)];
        let pawns = self.pieces[&(self.side_to_move, Pawn)];

        let mut pseudo_moves = Vec::new();
        mg::queen_pseudo_moves(&mut pseudo_moves, queens, occupied, us);
        mg::rook_pseudo_moves(&mut pseudo_moves, rooks, occupied, us);
        mg::bishop_pseudo_moves(&mut pseudo_moves, bishops, occupied, us);
        mg::knight_pseudo_moves(&mut pseudo_moves, knights, us);
        mg::pawn_pseudo_moves(
            &mut pseudo_moves,
            pawns,
            self.side_to_move,
            occupied,
            them,
            self.en_passant,
        );

        pseudo_moves
            .into_iter()
            .filter(|pseudo_move| {
                !self
                    .make_move(*pseudo_move)
                    .is_attacked_by(&king_square, &passive_player)
            })
            .for_each(|legal_move| legal_moves.push(legal_move));

        legal_moves
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
        // Most positions will have fewer moves than this capacity.
        let mut legal_moves = Vec::with_capacity(128);

        let king = self.pieces[(self.side_to_move, King)];
        let king_square = king.squares()[0];
        let passive_player = !self.side_to_move;
        let us = self.pieces.color_occupied(&self.side_to_move);
        let them = self.pieces.color_occupied(&passive_player);
        let occupied = us | them;
        let attacked = self.attacks(&passive_player, &occupied);

        // Generate all normal legal king moves.
        let mut king_tos = mg::king_attacks(&king);
        king_tos.remove(&us);
        king_tos.remove(&attacked);
        for to in king_tos.squares() {
            legal_moves.push(Move::new(king_square, to, None));
        }

        // Generate all normal Queen, Rook, Bishop, Knight moves.
        // Generate all normal and special Pawn moves (single/double push, attacks, ep).
        let queens = self.pieces[&(self.side_to_move, Queen)];
        let rooks = self.pieces[&(self.side_to_move, Rook)];
        let bishops = self.pieces[&(self.side_to_move, Bishop)];
        let knights = self.pieces[&(self.side_to_move, Knight)];
        let pawns = self.pieces[&(self.side_to_move, Pawn)];

        let mut pseudo_moves = Vec::with_capacity(128);
        mg::queen_pseudo_moves(&mut pseudo_moves, queens, occupied, us);
        mg::rook_pseudo_moves(&mut pseudo_moves, rooks, occupied, us);
        mg::bishop_pseudo_moves(&mut pseudo_moves, bishops, occupied, us);
        mg::knight_pseudo_moves(&mut pseudo_moves, knights, us);
        mg::pawn_pseudo_moves(
            &mut pseudo_moves,
            pawns,
            self.side_to_move,
            occupied,
            them,
            self.en_passant,
        );
        pseudo_moves
            .into_iter()
            .filter(|pseudo_move| {
                !self
                    .make_move(*pseudo_move)
                    .is_attacked_by(&king_square, &passive_player)
            })
            .for_each(|legal_move| legal_moves.push(legal_move));

        // Generate Castling moves
        // Check if current player can castle. If can, for each side that can castle,
        // check if there are any pieces between king and castling rook.
        // check if king will pass through an attacked square.
        mg::legal_castling_moves(
            &mut legal_moves,
            self.side_to_move,
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
