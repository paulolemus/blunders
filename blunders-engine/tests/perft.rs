//! Performance Test (perft)
//!
//! Tests to ensure engine passes Perft test by checking against pre-determined results.
//! [Perft Results](https://www.chessprogramming.org/Perft_Results)

use std::thread::available_parallelism;

use blunders_engine::fen::Fen;
use blunders_engine::perft::*;
use blunders_engine::*;

const ONE_THREAD: usize = 1;

fn cpu_threads() -> usize {
    available_parallelism()
        .map(|inner| inner.get())
        .unwrap_or(1)
}

/// Run single and multithreaded perft `expected_nodes.len()` times.
/// The index of each expected_node value is its ply.
#[inline(always)]
fn perft_tester(position: Position, expected_nodes: Vec<u64>) {
    for (ply, expected_node) in expected_nodes.into_iter().enumerate() {
        let single_thread_result = perft(position, ply as u8, ONE_THREAD);
        let multi_thread_result = perft(position, ply as u8, cpu_threads());

        println!("perft({ply}): {single_thread_result:?}");
        assert_eq!(single_thread_result.nodes, expected_node);
        assert_eq!(single_thread_result, multi_thread_result);
    }
}

#[test]
fn perft_starting_position() {
    perft_tester(Position::start_position(), vec![1, 20, 400, 8_902, 197_281]);
}

#[test]
#[ignore]
fn perft_starting_position_expensive() {
    let position = Position::start_position();
    let ply5 = perft(position, 5, ONE_THREAD);
    let ply6 = perft(position, 6, ONE_THREAD);

    println!("perft(5): {:?}", ply5);
    println!("perft(6): {:?}", ply6);

    assert_eq!(ply5.nodes, 4_865_609);
    assert_eq!(ply6.nodes, 119_060_324);
}

fn kiwipete_position() -> Position {
    // https://www.chessprogramming.org/Perft_Results#Position_2
    Position::parse_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
        .unwrap()
}

#[test]
fn perft_kiwipete_position() {
    // https://www.chessprogramming.org/Perft_Results#Position_2
    perft_tester(kiwipete_position(), vec![1, 48, 2_039, 97_862]);
}

#[test]
#[ignore]
fn perft_kiwipete_position_expensive() {
    let position = kiwipete_position();

    let ply4 = perft(position, 4, ONE_THREAD);
    println!("perft(4): {:?}", ply4);
    assert_eq!(ply4.nodes, 4_085_603);
}

fn position_3() -> Position {
    // https://www.chessprogramming.org/Perft_Results#Position_3
    Position::parse_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap()
}

#[test]
fn perft_test_position_3() {
    // https://www.chessprogramming.org/Perft_Results#Position_3
    perft_tester(position_3(), vec![1, 14, 191, 2_812, 43_238]);
}

#[test]
#[ignore]
fn perft_test_position_3_expensive() {
    let position = position_3();

    let ply5 = perft(position, 5, ONE_THREAD);
    let ply6 = perft(position, 6, ONE_THREAD);
    println!("perft(5): {:?}", ply5);
    println!("perft(6): {:?}", ply6);
    assert_eq!(ply5.nodes, 674_624);
    assert_eq!(ply6.nodes, 11_030_083);
}

fn position_4() -> Position {
    // https://www.chessprogramming.org/Perft_Results#Position_4
    Position::parse_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap()
}

#[test]
fn perft_test_position_4() {
    // https://www.chessprogramming.org/Perft_Results#Position_4
    perft_tester(position_4(), vec![1, 6, 264, 9_467, 422_333]);
}

fn position_5() -> Position {
    // https://www.chessprogramming.org/Perft_Results#Position_5
    Position::parse_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap()
}

#[test]
fn perft_test_position_5() {
    // https://www.chessprogramming.org/Perft_Results#Position_5
    perft_tester(position_5(), vec![1, 44, 1_486, 62_379]);
}

#[test]
#[ignore]
fn perft_test_position_5_expensive() {
    let position = position_5();
    let ply4 = perft(position, 4, ONE_THREAD);
    println!("perft(4): {:?}", ply4);
    assert_eq!(ply4.nodes, 2_103_487);
}

fn position_6() -> Position {
    // https://www.chessprogramming.org/Perft_Results#Position_6
    Position::parse_fen("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10")
        .unwrap()
}

#[test]
fn perft_test_position_6() {
    // https://www.chessprogramming.org/Perft_Results#Position_6
    perft_tester(position_6(), vec![1, 46, 2_079, 89_890]);
}

#[test]
#[ignore]
fn perft_test_position_6_expensive() {
    let position = position_6();

    let ply4 = perft(position, 4, ONE_THREAD);
    let ply5 = perft(position, 5, ONE_THREAD);
    println!("perft(4): {ply4:?}");
    println!("perft(5): {ply5:?}");
    assert_eq!(ply4.nodes, 3_894_594);
    assert_eq!(ply5.nodes, 164_075_551);
}
