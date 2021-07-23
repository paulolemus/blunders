# Blunders Engine

### What is Blunders Engine?

Blunders Engine is a WIP chess engine built from scratch that has:
* Bitboard and mailbox representations for Chess Positions
* Legal move generator
* UCI communication facilities
* Transposition Table
* Minimax with Alpha-Beta pruning based search

### Basic Usage

Search the start position to a depth of 8-ply using a Transposition Table with 10 megabytes of capacity:
```rust
use blunders_engine::{search, Position, TranspositionTable};

let mut tt = TranspositionTable::with_mb(10);
let position = Position::start_position();
let ply = 8;

let search_results = search::search_with_tt(position, ply, &mut tt);
println!("best move: {}, nodes/sec: {}", search_results.best_move, search_results.nps());
```