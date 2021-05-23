//! Simple Tactics
//!
//! Tests to ensure engine passes basic strength tests.
//! They should find the best move with a small depth.

use blunders_engine::coretypes::{Move, Square::*};
use blunders_engine::fen::Fen;
use blunders_engine::search::alpha_beta;
use blunders_engine::*;

#[test]
fn force_rook_for_pawn() {
    let pos = Position::parse_fen("r4r1k/P6p/BP4p1/2p5/8/1PnP2P1/3b2KP/R7 w - - 5 39").unwrap();
    let bm = Move::new(A6, B7, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == 1);
    assert_eq!(bm, best_move);
}

#[test]
fn trade_rooks_win_queen() {
    let pos = Position::parse_fen("7k/6p1/3p3p/p3p3/q3Pp1P/3P1P2/2R5/1rRK2Q1 b - - 8 44").unwrap();
    let bm = Move::new(B1, C1, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == -1);
    assert_eq!(bm, best_move);
}

#[test]
fn force_mate_in_2_sac_rook() {
    let pos = Position::parse_fen("8/1p3Pkp/p5p1/8/3q4/1P4Q1/5PPP/r4RK1 b - - 0 33").unwrap();
    let bm = Move::new(A1, F1, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == -1);
    assert_eq!(bm, best_move);
}

#[test]
fn win_bishop_after_trading_bishop_for_knight() {
    let pos = Position::parse_fen("r2qk2r/p1pp1ppp/1p2pn2/8/2P1b3/2B5/PPP1QPPP/2KR2NR w kq - 0 11")
        .unwrap();
    let bm = Move::new(C3, F6, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == 1);
    assert_eq!(bm, best_move);
}

#[test]
fn simple_mate_in_one_queen_take_pawn() {
    let pos =
        Position::parse_fen("r1bqk2r/2p2pp1/p1pp3p/2b5/2B1P1n1/2N2Q2/PPP2PPP/R1B1R1K1 w kq - 2 11")
            .unwrap();
    let bm = Move::new(F3, F7, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == 1);
    assert_eq!(bm, best_move);
}

#[test]
fn tempo_on_king_capture_queen() {
    let pos = Position::parse_fen("4r3/p4ppk/2p5/8/P1pq4/1r2P1P1/4Q2P/R1B3K1 w - - 0 27").unwrap();
    let bm = Move::new(E2, H5, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == 1);
    assert_eq!(bm, best_move);
}

#[test]
fn back_rank_mate_with_queen() {
    let pos = Position::parse_fen("6k1/5ppp/4p3/4P2q/3P1P2/2r4P/4R1QK/8 w - - 0 32").unwrap();
    let bm = Move::new(G2, A8, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == 1);
    assert_eq!(bm, best_move);
}

#[test]
fn pin_king_to_rook_win_rook() {
    let pos =
        Position::parse_fen("3r2k1/ppN3pp/2p2b2/2Q5/q3PB2/2P2P2/1P4PP/2K4R b - - 10 27").unwrap();
    let bm = Move::new(A4, A1, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == -1);
    assert_eq!(bm, best_move);
}

#[test]
fn mate_in_2_force_king_moves() {
    let pos = Position::parse_fen("3n4/5pkp/p4Nb1/1p2q1PQ/8/1P6/1PP2P2/6K1 w - - 1 34").unwrap();
    let bm = Move::new(H5, H6, None);
    let (cp, best_move) = alpha_beta(pos, 5);

    assert!(cp.signum() == 1);
    assert_eq!(bm, best_move);
}
