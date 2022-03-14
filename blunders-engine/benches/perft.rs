use std::thread::available_parallelism;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use blunders_engine::perft::*;
use blunders_engine::*;

pub fn criterion_perft_small_benchmark(c: &mut Criterion) {
    // Setup
    let starting_position = Position::start_position();
    let num_threads = available_parallelism()
        .map(|inner| inner.get())
        .unwrap_or(1);

    // Benchmarks

    c.bench_function("start_position: perft(1) threads: 1", |b| {
        b.iter(|| {
            let info = perft(black_box(starting_position), black_box(1), black_box(1));
            assert_eq!(info.nodes, 20);
        })
    });
    c.bench_function(
        &format!("start_position: perft(1) threads: {num_threads}"),
        |b| {
            b.iter(|| {
                let info = perft(
                    black_box(starting_position),
                    black_box(1),
                    black_box(num_threads),
                );
                assert_eq!(info.nodes, 20);
            })
        },
    );

    c.bench_function("start_position: perft(2) threads: 1", |b| {
        b.iter(|| {
            let info = perft(black_box(starting_position), black_box(2), black_box(1));
            assert_eq!(info.nodes, 400);
        })
    });
    c.bench_function(
        &format!("start_position: perft(2) threads: {num_threads}"),
        |b| {
            b.iter(|| {
                let info = perft(
                    black_box(starting_position),
                    black_box(2),
                    black_box(num_threads),
                );
                assert_eq!(info.nodes, 400);
            })
        },
    );

    c.bench_function("start_position: perft(3) threads: 1", |b| {
        b.iter(|| {
            let info = perft(black_box(starting_position), black_box(3), black_box(1));
            assert_eq!(info.nodes, 8_902);
        })
    });
    c.bench_function(
        &format!("start_position: perft(3) threads: {num_threads}"),
        |b| {
            b.iter(|| {
                let info = perft(
                    black_box(starting_position),
                    black_box(3),
                    black_box(num_threads),
                );
                assert_eq!(info.nodes, 8_902);
            })
        },
    );

    c.bench_function("start_position: perft(4) threads: 1", |b| {
        b.iter(|| {
            let info = perft(black_box(starting_position), black_box(4), black_box(1));
            assert_eq!(info.nodes, 197_281);
        })
    });
    c.bench_function(
        &format!("start_position: perft(4) threads: {num_threads}"),
        |b| {
            b.iter(|| {
                let info = perft(
                    black_box(starting_position),
                    black_box(4),
                    black_box(num_threads),
                );
                assert_eq!(info.nodes, 197_281);
            })
        },
    );

    c.bench_function(
        &format!("start_position: perft(5) threads: {num_threads}"),
        |b| {
            b.iter(|| {
                let info = perft(
                    black_box(starting_position),
                    black_box(5),
                    black_box(num_threads),
                );
                assert_eq!(info.nodes, 4_865_609);
            })
        },
    );
}

/// Large number of positions to search, > 100,000,000
pub fn criterion_perft_large_benchmark(c: &mut Criterion) {
    // Setup
    let starting_position = Position::start_position();
    let num_threads = available_parallelism()
        .map(|inner| inner.get())
        .unwrap_or(1);

    c.bench_function(
        &format!("start_position: perft(6) threads: {num_threads}"),
        |b| {
            b.iter(|| {
                let info = perft(
                    black_box(starting_position),
                    black_box(6),
                    black_box(num_threads),
                );
                assert_eq!(info.nodes, 119_060_324);
            })
        },
    );
}

criterion_group! {
    name = small_benches;
    config = Criterion::default().without_plots().sample_size(70);
    targets = criterion_perft_small_benchmark
}
criterion_group! {
    name = large_benches;
    config = Criterion::default().without_plots().sample_size(10);
    targets = criterion_perft_large_benchmark
}
criterion_main!(small_benches, large_benches);
