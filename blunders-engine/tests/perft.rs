//! Performance Test (perft)
//!
//! Tests to ensure engine passes Perft test by checking against pre-determined results.
//! [Perft Results](https://www.chessprogramming.org/Perft_Results)

use num_cpus;

use blunders_engine::fen::Fen;
use blunders_engine::perft::*;
use blunders_engine::*;

const ONE_THREAD: usize = 1;

fn cpu_threads() -> usize {
    num_cpus::get()
}

#[test]
fn perft_starting_position() {
    let position = Position::start_position();
    let ply0 = perft(position, 0, ONE_THREAD);
    let ply1 = perft(position, 1, ONE_THREAD);
    let ply2 = perft(position, 2, ONE_THREAD);
    let ply3 = perft(position, 3, ONE_THREAD);
    let ply4 = perft(position, 4, ONE_THREAD);

    println!("perft(0): {:?}", ply0);
    println!("perft(1): {:?}", ply1);
    println!("perft(2): {:?}", ply2);
    println!("perft(3): {:?}", ply3);
    println!("perft(4): {:?}", ply4);

    assert_eq!(ply0.nodes, 1);
    assert_eq!(ply1.nodes, 20);
    assert_eq!(ply2.nodes, 400);
    assert_eq!(ply3.nodes, 8_902);
    assert_eq!(ply4.nodes, 197_281);

    let threaded_ply2 = perft(position, 2, cpu_threads());
    let threaded_ply3 = perft(position, 3, cpu_threads());
    let threaded_ply4 = perft(position, 4, cpu_threads());
    assert_eq!(threaded_ply2, ply2);
    assert_eq!(threaded_ply3, ply3);
    assert_eq!(threaded_ply4, ply4);
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
    let position = kiwipete_position();
    let ply0 = perft(position, 0, ONE_THREAD);
    let ply1 = perft(position, 1, ONE_THREAD);
    let ply2 = perft(position, 2, ONE_THREAD);
    let ply3 = perft(position, 3, ONE_THREAD);

    println!("perft(0): {:?}", ply0);
    println!("perft(1): {:?}", ply1);
    println!("perft(2): {:?}", ply2);
    println!("perft(3): {:?}", ply3);

    // Perft results used found in link above.
    assert_eq!(ply0.nodes, 1);
    assert_eq!(ply1.nodes, 48);
    assert_eq!(ply2.nodes, 2_039);
    assert_eq!(ply3.nodes, 97_862);

    let threaded_ply2 = perft(position, 2, cpu_threads());
    let threaded_ply3 = perft(position, 3, cpu_threads());
    assert_eq!(threaded_ply2, ply2);
    assert_eq!(threaded_ply3, ply3);
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
    let position = position_3();
    let ply0 = perft(position, 0, ONE_THREAD);
    let ply1 = perft(position, 1, ONE_THREAD);
    let ply2 = perft(position, 2, ONE_THREAD);
    let ply3 = perft(position, 3, ONE_THREAD);
    let ply4 = perft(position, 4, ONE_THREAD);

    println!("perft(0): {:?}", ply0);
    println!("perft(1): {:?}", ply1);
    println!("perft(2): {:?}", ply2);
    println!("perft(3): {:?}", ply3);
    println!("perft(4): {:?}", ply4);

    // Perft results used found in link above.
    assert_eq!(ply0.nodes, 1);
    assert_eq!(ply1.nodes, 14);
    assert_eq!(ply2.nodes, 191);
    assert_eq!(ply3.nodes, 2_812);
    assert_eq!(ply4.nodes, 43_238);

    let threaded_ply2 = perft(position, 2, cpu_threads());
    let threaded_ply3 = perft(position, 3, cpu_threads());
    let threaded_ply4 = perft(position, 4, cpu_threads());
    assert_eq!(threaded_ply2, ply2);
    assert_eq!(threaded_ply3, ply3);
    assert_eq!(threaded_ply4, ply4);
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
    let position = position_4();
    let ply0 = perft(position, 0, ONE_THREAD);
    let ply1 = perft(position, 1, ONE_THREAD);
    let ply2 = perft(position, 2, ONE_THREAD);
    let ply3 = perft(position, 3, ONE_THREAD);
    let ply4 = perft(position, 4, ONE_THREAD);

    println!("perft(0): {:?}", ply0);
    println!("perft(1): {:?}", ply1);
    println!("perft(2): {:?}", ply2);
    println!("perft(3): {:?}", ply3);
    println!("perft(4): {:?}", ply4);

    // Perft results used found in link above.
    assert_eq!(ply0.nodes, 1);
    assert_eq!(ply1.nodes, 6);
    assert_eq!(ply2.nodes, 264);
    assert_eq!(ply3.nodes, 9_467);
    assert_eq!(ply4.nodes, 422_333);

    let threaded_ply2 = perft(position, 2, cpu_threads());
    let threaded_ply3 = perft(position, 3, cpu_threads());
    let threaded_ply4 = perft(position, 4, cpu_threads());
    assert_eq!(threaded_ply2, ply2);
    assert_eq!(threaded_ply3, ply3);
    assert_eq!(threaded_ply4, ply4);
}

fn position_5() -> Position {
    // https://www.chessprogramming.org/Perft_Results#Position_5
    Position::parse_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap()
}

#[test]
fn perft_test_position_5() {
    // https://www.chessprogramming.org/Perft_Results#Position_5
    let position = position_5();
    let ply0 = perft(position, 0, ONE_THREAD);
    let ply1 = perft(position, 1, ONE_THREAD);
    let ply2 = perft(position, 2, ONE_THREAD);
    let ply3 = perft(position, 3, ONE_THREAD);

    println!("perft(0): {:?}", ply0);
    println!("perft(1): {:?}", ply1);
    println!("perft(2): {:?}", ply2);
    println!("perft(3): {:?}", ply3);

    // Perft results used found in link above.
    assert_eq!(ply0.nodes, 1);
    assert_eq!(ply1.nodes, 44);
    assert_eq!(ply2.nodes, 1_486);
    assert_eq!(ply3.nodes, 62_379);

    let threaded_ply2 = perft(position, 2, cpu_threads());
    let threaded_ply3 = perft(position, 3, cpu_threads());
    assert_eq!(threaded_ply2, ply2);
    assert_eq!(threaded_ply3, ply3);
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
    let position = position_6();
    let ply0 = perft(position, 0, ONE_THREAD);
    let ply1 = perft(position, 1, ONE_THREAD);
    let ply2 = perft(position, 2, ONE_THREAD);
    let ply3 = perft(position, 3, ONE_THREAD);

    println!("perft(0): {:?}", ply0);
    println!("perft(1): {:?}", ply1);
    println!("perft(2): {:?}", ply2);
    println!("perft(3): {:?}", ply3);

    // Perft results used found in link above.
    assert_eq!(ply0.nodes, 1);
    assert_eq!(ply1.nodes, 46);
    assert_eq!(ply2.nodes, 2_079);
    assert_eq!(ply3.nodes, 89_890);

    let threaded_ply2 = perft(position, 2, cpu_threads());
    let threaded_ply3 = perft(position, 3, cpu_threads());
    assert_eq!(threaded_ply2, ply2);
    assert_eq!(threaded_ply3, ply3);
}

#[test]
#[ignore]
fn perft_test_position_6_expensive() {
    let position = position_6();

    let ply4 = perft(position, 4, ONE_THREAD);
    println!("perft(4): {:?}", ply4);
    assert_eq!(ply4.nodes, 3_894_594);
}
