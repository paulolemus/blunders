//! Mates
//!
//! Tests to ensure engine finds forced checkmates.
//! They should find the best move with a small depth.

use blunders_engine::coretypes::{Color, Color::*, Move, PieceKind::*, Square::*};
use blunders_engine::fen::Fen;
use blunders_engine::search::search;
use blunders_engine::*;

#[inline(always)]
fn mate_tester(fen_str: &str, best_move: Move, ply: u8, winner: Color) {
    let pos = Position::parse_fen(fen_str).unwrap();
    let mut tt = TranspositionTable::new();
    let result = search(pos, Mode::depth(ply, None), &mut tt, None);
    assert_eq!(result.leading(), Some(winner));
    assert_eq!(result.best_move, best_move);
    assert!(result.score.is_mate());
}

/// A unique position where a king should not be able to capture a checking queen,
/// even though recapturing piece is pinned to opposing king.
/// https://support.chess.com/article/373-checkmate-with-a-pinned-piece-whats-going-on
#[test]
fn mate_with_pinned_piece() {
    let pos = Position::parse_fen("k7/1r6/8/8/4B3/8/1q6/K7 w - - 0 1").unwrap();
    assert!(pos.is_checkmate());
}

#[test]
fn mate_in_1_queen_take_pawn() {
    let pos = "r1bqk2r/2p2pp1/p1pp3p/2b5/2B1P1n1/2N2Q2/PPP2PPP/R1B1R1K1 w kq - 2 11";
    let bm = Move::new(F3, F7, None);
    mate_tester(pos, bm, 5, White);
}

#[test]
fn mate_in_2_double_bishop() {
    let pos = "5bk1/1b5p/1p2RBp1/p2B1p2/3n3P/PP4P1/5PKN/2r5 w - - 2 30";
    let bm = Move::new(E6, C6, None);
    mate_tester(pos, bm, 6, White);
}

#[test]
fn mate_in_2_back_rank_queen() {
    let pos = "6k1/5ppp/4p3/4P2q/3P1P2/2r4P/4R1QK/8 w - - 0 3";
    let bm = Move::new(G2, A8, None);
    mate_tester(pos, bm, 5, White);
}

#[test]
fn mate_in_2_force_king_moves() {
    let pos = "3n4/5pkp/p4Nb1/1p2q1PQ/8/1P6/1PP2P2/6K1 w - - 1 34";
    let bm = Move::new(H5, H6, None);
    mate_tester(pos, bm, 5, White);
}

#[test]
fn mate_in_2_sac_rook() {
    let pos = "8/1p3Pkp/p5p1/8/3q4/1P4Q1/5PPP/r4RK1 b - - 0 33";
    let bm = Move::new(A1, F1, None);
    mate_tester(pos, bm, 5, Black);
}

#[test]
fn mate_in_3_queen_promotion() {
    let pos = "8/7P/1p6/1P6/K1k5/8/5p2/8 b - - 0 53";
    let bm = Move::new(F2, F1, Some(Queen));
    mate_tester(pos, bm, 5, Black);
}

#[test]
fn mate_in_3_sac_knight() {
    let pos = "r4rk1/1b3ppp/pp2p3/2p5/P1B1NR1Q/3P3P/2q3P1/7K w - - 0 24";
    let bm = Move::new(E4, F6, None);
    mate_tester(pos, bm, 6, White);
}

#[test]
fn mate_in_3_back_rank_sac_queen() {
    let pos = "4r1k1/ppp1rppp/1b6/3p2q1/3P2b1/2PB4/PP3QPP/4RRK1 w - - 5 19";
    let bm = Move::new(F2, F7, None);
    mate_tester(pos, bm, 6, White);
}

#[test]
fn mate_in_3_force_king_moves_with_bishop_rook() {
    let pos = "6k1/ppp4p/8/1RbpP3/5Bb1/2PB2P1/P1P2r1P/7K b - - 4 22";
    let bm = Move::new(G4, F3, None);
    mate_tester(pos, bm, 6, Black);
}
