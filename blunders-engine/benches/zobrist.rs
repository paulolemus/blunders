//! Benchmarks for Zobrist Hash generation and incremental update.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use blunders_engine::{Move, Position, Square::*, ZobristTable};

pub fn zobrist_generate_and_update(c: &mut Criterion) {
    // Setup
    let ztable = ZobristTable::new();
    let move_ = Move::new(E2, E4, None);

    let initial_pos = Position::start_position();
    let cache = initial_pos.cache();

    let mut final_pos = initial_pos.clone();
    let move_info = final_pos.do_move(move_);

    let initial_hash = ztable.generate_hash((&initial_pos).into());
    let final_hash = ztable.generate_hash((&final_pos).into());

    // Benchmarks

    c.bench_function("zobrist initial position generate hash", |b| {
        b.iter(|| {
            let hash = ztable.generate_hash(black_box((&initial_pos).into()));
            assert_eq!(hash, initial_hash);
        });
    });

    c.bench_function("zobrist final position generate hash", |b| {
        b.iter(|| {
            let hash = ztable.generate_hash(black_box((&final_pos).into()));
            assert_eq!(hash, final_hash);
        });
    });

    c.bench_function("zobrist final position update hash", |b| {
        b.iter(|| {
            let mut hash = initial_hash;
            ztable.update_hash(
                black_box(&mut hash),
                black_box((&final_pos).into()),
                black_box(move_info),
                black_box(cache),
            );
            assert_eq!(hash, final_hash);
        });
    });
}

criterion_group! {
    name = zobrist_hashing;
    config = Criterion::default().without_plots().sample_size(100);
    targets = zobrist_generate_and_update
}

criterion_main!(zobrist_hashing);
