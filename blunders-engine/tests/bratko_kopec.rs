//! Bratko-Kopec Test Suite
//!
//! Tests to ensure engine passes basic strength tests.
//! [Bratko-Kopec Link](https://www.chessprogramming.org/Bratko-Kopec_Test)

use blunders_engine::coretypes::Move;
use blunders_engine::coretypes::Square::*;
use blunders_engine::fen::Fen;
use blunders_engine::search::search;
use blunders_engine::Mode;
use blunders_engine::*;

#[test]
fn bkt_1() {
    let pos = Position::parse_fen("1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 0 1").unwrap();
    let bm = Move::new(D6, D1, None);
    let mut tt = TranspositionTable::new();
    let result = search(pos, Mode::depth(5, None), &mut tt, None);
    assert_eq!(bm, result.best_move);
}
