//! Simple Tactics
//!
//! Tests to ensure engine passes basic strength tests.
//! They should find the best move with a small depth.

use blunders_engine::coretypes::{Color::*, Move, PieceKind::*, Square::*};
use blunders_engine::fen::Fen;
use blunders_engine::search::search;
use blunders_engine::*;

#[test]
fn trade_rooks_win_queen() {
    let pos = Position::parse_fen("7k/6p1/3p3p/p3p3/q3Pp1P/3P1P2/2R5/1rRK2Q1 b - - 8 44").unwrap();
    let bm = Move::new(B1, C1, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);

    assert_eq!(result.leading(), Some(Black));
    assert_eq!(bm, result.best_move);
}

#[test]
fn win_bishop_after_trading_bishop_for_knight() {
    let pos = Position::parse_fen("r2qk2r/p1pp1ppp/1p2pn2/8/2P1b3/2B5/PPP1QPPP/2KR2NR w kq - 0 11")
        .unwrap();
    let bm = Move::new(C3, F6, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);

    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn tempo_on_king_capture_queen() {
    let pos = Position::parse_fen("4r3/p4ppk/2p5/8/P1pq4/1r2P1P1/4Q2P/R1B3K1 w - - 0 27").unwrap();
    let bm = Move::new(E2, H5, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, 5, &mut tt);

    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
fn underpromote_to_knight_fork_queen() {
    let pos = Position::parse_fen("5K2/2q1P3/5kp1/7p/8/6PP/8/8 w - - 0 58").unwrap();
    let bm = Move::new(E7, E8, Some(Knight));
    let mut tt = TranspositionTable::new();
    let result = search(pos, 6, &mut tt);
    assert_eq!(result.leading(), Some(White));
    assert_eq!(bm, result.best_move);
}

#[test]
#[ignore]
fn trade_queen_that_cannot_escape() {
    // While playing as white against engine with tt, found that it engine moved Qxd3 pawn
    // instead of queen.
    let pos =
        Position::parse_fen("r1b1k2r/1ppp1pp1/p1n2n1p/4p3/4P3/R1NPBNP1/2q2PBP/1Q3RK1 b kq - 0 1")
            .unwrap();
    let bm = Move::new(C2, B1, None);
    let depth = 7;
    let mut tt = TranspositionTable::new();
    let result = search(pos, depth, &mut tt);

    let expected = || {
        let mut pos = pos;
        pos.do_move(bm);
        let mut tt = TranspositionTable::new();
        search(pos, depth - 1, &mut tt)
    };
    assert_eq!(
        bm,
        result.best_move,
        "result: {}, result of bm: {}",
        result,
        expected()
    );
}
