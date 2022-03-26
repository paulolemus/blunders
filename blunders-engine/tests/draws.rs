//! Draws
//!
//! Tests to ensure threefold repetition and 50-move rule draws
//! are correctly evaluated.

use blunders_engine::coretypes::{Color::*, Move, Square::*};
use blunders_engine::fen::Fen;
use blunders_engine::movelist::MoveHistory;
use blunders_engine::*;

#[test]
fn threefold_repetition_perpetual_check_1() {
    // White has huge material advantage but black can perpetually mate.
    let pos = Position::parse_fen("k7/1p2QP2/4PP2/8/1P5q/8/6P1/1RRN2K1 b - - 0 1").unwrap();
    let moves: MoveHistory = [
        Move::new(H4, E1, None),
        Move::new(G1, H2, None),
        Move::new(E1, H4, None),
        Move::new(H2, G1, None),
    ]
    .into_iter()
    .collect();

    let repeated_game = Game::new(pos, moves).unwrap();
    let mode = Mode::depth(5, None);
    let mut engine = Engine::new();

    {
        // Search once without the history of repeated moves -> Losing.
        engine.set_game(repeated_game.position);
        let search_result = engine.search_sync(mode);
        assert_eq!(search_result.best_move, Move::new(H4, E1, None));
        assert_eq!(search_result.leading(), Some(White));
    }

    {
        // Search again with repeated moves -> Draw.
        engine.new_game().unwrap();
        engine.set_game(repeated_game);
        let search_result = engine.search_sync(mode);
        assert_eq!(search_result.best_move, Move::new(H4, E1, None));
        // assert_eq!(search_result.leading(), None); How to assess draw with contempt?
    }
}
