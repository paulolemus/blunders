use criterion::{black_box, criterion_group, criterion_main, Criterion};

use blunders_engine::perft::*;
use blunders_engine::*;

pub fn criterion_perft_benchmark(c: &mut Criterion) {
    // Setup
    let starting_position = Position::start_position();

    // Benchmarks
    c.bench_function("perft(1)", |b| {
        b.iter(|| {
            let info = perft(black_box(starting_position), black_box(1));
            assert_eq!(info.nodes, 20);
        })
    });

    c.bench_function("perft(2)", |b| {
        b.iter(|| {
            let info = perft(black_box(starting_position), black_box(2));
            assert_eq!(info.nodes, 400);
        })
    });

    c.bench_function("perft(3)", |b| {
        b.iter(|| {
            let _info = perft(black_box(starting_position), black_box(3));
            //assert_eq!(info.nodes, 8_902);
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().without_plots();
    targets = criterion_perft_benchmark
}
criterion_main!(benches);
