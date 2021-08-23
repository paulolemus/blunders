//! Time-to-depth benchmarks for Engine's main search.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use blunders_engine::{EngineBuilder, Mode, Move, Position};

pub fn start_pos_search_time_to_depth(c: &mut Criterion) {
    // Setup
    let engine_builder = EngineBuilder::new()
        .position(Position::start_position())
        .threads(1)
        .debug(false)
        .transpositions_mb(100);

    // Benchmarks
    let mut result = Default::default();
    c.bench_function("search start position ttd 2", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(2, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }

    c.bench_function("search start position ttd 3", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(3, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }

    c.bench_function("search start position ttd 4", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(4, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }

    c.bench_function("search start position ttd 5", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(5, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }

    c.bench_function("search start position ttd 6", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(6, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }
}

pub fn start_pos_search_time_to_depth_long(c: &mut Criterion) {
    // Setup
    let engine_builder = EngineBuilder::new()
        .position(Position::start_position())
        .threads(1)
        .debug(false)
        .transpositions_mb(100);

    // Benchmarks
    let mut result = Default::default();
    c.bench_function("search start position ttd 7", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(7, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }

    c.bench_function("search start position ttd 8", |b| {
        b.iter(|| {
            let mut engine = engine_builder.build();
            result = engine.search_sync(black_box(Mode::depth(8, None)));
        });
    });
    if result.best_move != Move::illegal() {
        println!("{}", result);
    }
}

criterion_group! {
    name = time_to_depth;
    config = Criterion::default().without_plots().sample_size(30);
    targets = start_pos_search_time_to_depth
}

criterion_group! {
    name = time_to_depth_long;
    config = Criterion::default().without_plots().sample_size(10);
    targets = start_pos_search_time_to_depth_long
}

criterion_main!(time_to_depth, time_to_depth_long);
