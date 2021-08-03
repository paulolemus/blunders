//! Mates
//!
//! Tests to ensure engine finds forced checkmates.
//! They should find the best move with a small depth.

use blunders_engine::coretypes::{Color::*, Move, PieceKind::*, Square::*};
use blunders_engine::fen::Fen;
use blunders_engine::search::search;
use blunders_engine::*;

#[test]
fn mate_in_1_queen_take_pawn() {
    let pos =
        Position::parse_fen("r1bqk2r/2p2pp1/p1pp3p/2b5/2B1P1n1/2N2Q2/PPP2PPP/R1B1R1K1 w kq - 2 11")
            .unwrap();
    let bm = Move::new(F3, F7, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_2_double_bishop() {
    let pos =
        Position::parse_fen("5bk1/1b5p/1p2RBp1/p2B1p2/3n3P/PP4P1/5PKN/2r5 w - - 2 30").unwrap();
    let bm = Move::new(E6, C6, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 6, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_2_back_rank_queen() {
    let pos = Position::parse_fen("6k1/5ppp/4p3/4P2q/3P1P2/2r4P/4R1QK/8 w - - 0 3").unwrap();
    let bm = Move::new(G2, A8, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_2_force_king_moves() {
    let pos = Position::parse_fen("3n4/5pkp/p4Nb1/1p2q1PQ/8/1P6/1PP2P2/6K1 w - - 1 34").unwrap();
    let bm = Move::new(H5, H6, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_2_sac_rook() {
    let pos = Position::parse_fen("8/1p3Pkp/p5p1/8/3q4/1P4Q1/5PPP/r4RK1 b - - 0 33").unwrap();
    let bm = Move::new(A1, F1, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);
    assert_eq!(result.leading(), Some(Black));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_3_queen_promotion() {
    let pos = Position::parse_fen("8/7P/1p6/1P6/K1k5/8/5p2/8 b - - 0 53").unwrap();
    let bm = Move::new(F2, F1, Some(Queen));
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);
    assert_eq!(result.leading(), Some(Black));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_3_sac_knight() {
    let pos =
        Position::parse_fen("r4rk1/1b3ppp/pp2p3/2p5/P1B1NR1Q/3P3P/2q3P1/7K w - - 0 24").unwrap();
    let bm = Move::new(E4, F6, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 6, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_3_back_rank_sac_queen() {
    let pos =
        Position::parse_fen("4r1k1/ppp1rppp/1b6/3p2q1/3P2b1/2PB4/PP3QPP/4RRK1 w - - 5 19").unwrap();
    let bm = Move::new(F2, F7, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 6, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn mate_in_3_force_king_moves_with_bishop_rook() {
    let pos = Position::parse_fen("6k1/ppp4p/8/1RbpP3/5Bb1/2PB2P1/P1P2r1P/7K b - - 4 22").unwrap();
    let bm = Move::new(G4, F3, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 6, &mut tt);
    assert_eq!(result.leading(), Some(Black));
    assert_eq!(bm, result.best_move);
}
