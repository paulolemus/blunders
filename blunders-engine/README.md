# Blunders Engine

Blunders Engine is the core library of the Blunders Chess Engine application.

## Features

* Bitboard and mailbox representations for Chess Positions / Board state.
* Legal move generator.
* UCI communication facilities.
* A two-layer shared Transposition Table, that uses either a mutex lock or atomics for synchronization.
* Minimax with Alpha-Beta pruning based search, iterative deepening, quiescence search.
* Unified Error type.
* Incremental Zobrist hashing.
* Hand-crafted evaluation.
* Simple time management strategy.

## Basic Usage

Blunders Engine can either be used by composing the raw components manually, or using the `Engine` API.

Search the start position to a depth of 4-ply using a Transposition Table with 10 megabytes of capacity:
```rust
use blunders_engine::{search, Position, Mode, TranspositionTable};

let tt = TranspositionTable::with_mb(10);
let position = Position::start_position();
let mode = Mode::depth(4, None);

let search_results = search::search(position, mode, &tt, None);
println!("best move: {}, nodes/sec: {}", search_results.best_move, search_results.nps());
assert_eq!(search_results.depth, 4);
```

Do the same as above with the engine API:
```rust
use blunders_engine::{EngineBuilder, Position, Mode};

let mut engine = EngineBuilder::new()
    .position(Position::start_position())
    .transpositions_mb(10)
    .build();

let search_results = engine.search_sync(Mode::depth(4, None), None);
println!("best move: {}, nodes/sec: {}", search_results.best_move, search_results.nps());
assert_eq!(search_results.depth, 4);
```
