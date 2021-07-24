use criterion::{black_box, criterion_group, criterion_main, Criterion};

use blunders_engine::coretypes::{Color::*, Move, Square::*};
use blunders_engine::fen::Fen;
use blunders_engine::search;
use blunders_engine::*;

pub fn criterion_mates_3_sac_knight(c: &mut Criterion) {
    // Setup
    let pos =
        Position::parse_fen("r4rk1/1b3ppp/pp2p3/2p5/P1B1NR1Q/3P3P/2q3P1/7K w - - 0 24").unwrap();
    let ply = 6;
    let bm = Move::new(E4, F6, None);
    let lead = Some(White);

    // Benchmarks

    c.bench_function("mates_3_sac_knight_alpha_beta", |b| {
        b.iter(|| {
            let result = search::alpha_beta(black_box(pos), black_box(ply));

            assert_eq!(result.score.leading(), lead);
            assert_eq!(result.best_move, bm);
        })
    });

    c.bench_function("mates_3_sac_knight_negamax", |b| {
        b.iter(|| {
            let result = search::negamax(black_box(pos), black_box(ply));

            assert_eq!(result.score.leading(), lead);
            assert_eq!(result.best_move, bm);
        })
    });

    c.bench_function("mates_3_sac_knight_ids", |b| {
        b.iter(|| {
            let result = search::ids(black_box(pos), black_box(ply));

            assert_eq!(result.score.leading(), lead);
            assert_eq!(result.best_move, bm);
        })
    });
}

pub fn criterion_mates_3_knights_and_bishop(c: &mut Criterion) {
    // Setup
    let pos = Position::parse_fen("8/1b5p/1p2NNpk/4P3/p1B4b/P7/KP6/2r5 w - - 7 37").unwrap();
    let ply = 6;
    let bm = Move::new(F6, G8, None);
    let lead = Some(White);

    // Benchmarks

    c.bench_function("mates_3_knights_and_bishop_alpha_beta", |b| {
        b.iter(|| {
            let result = search::alpha_beta(black_box(pos), black_box(ply));

            assert_eq!(result.score.leading(), lead);
            assert_eq!(result.best_move, bm);
        })
    });

    c.bench_function("mates_3_knights_and_bishop_negamax", |b| {
        b.iter(|| {
            let result = search::negamax(black_box(pos), black_box(ply));

            assert_eq!(result.score.leading(), lead);
            assert_eq!(result.best_move, bm);
        })
    });

    c.bench_function("mates_3_knights_and_bishop_ids", |b| {
        b.iter(|| {
            let result = search::ids(black_box(pos), black_box(ply));

            assert_eq!(result.score.leading(), lead);
            assert_eq!(result.best_move, bm);
        })
    });
}

criterion_group! {
    name = small_benches;
    config = Criterion::default().without_plots().sample_size(30);
    targets = criterion_mates_3_sac_knight, criterion_mates_3_knights_and_bishop
}

criterion_main!(small_benches);
