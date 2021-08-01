# Blunders

A Universal Chess Interface ([UCI](https://www.shredderchess.com/chess-features/uci-universal-chess-interface.html)) chess engine.  
Blunders is currently a work in progress.

# Building and running Blunders

Blunders supports Windows 10 and Linux, the following commands should work on both platforms. Building for MacOS is untested.

## Via Cargo

Install the most recent stable rust compiler and cargo through [rustup](https://rustup.rs/).
Download or clone the Blunders repository and navigate into the root folder.

To build only, run the command `cargo build --release`  
To build and run, run the command `cargo run --release`

The default location for the Blunders executable is `blunders/target/release/blunders`.

# Using Blunders

Blunders is a UCI compatible chess engine, and is most easily used from a chess GUI or CLI program instead of running it directly.
Blunders uses the chess GUI [Cute Chess](https://github.com/cutechess/cutechess) during development and is known to work well within it.

To use Blunders directly, look at the UCI specification to find complete instructions on how to interact with Blunders in UCI mode.
Eventually Blunders will get a non-standard set of commands to make it easy to use directly.

## Blunders runtime settings

* `Hash x`: an integer size in megabytes `x` to set the size of the engine's hash table
* `Clear Hash`: a button command telling the engine to clear its hash table, effectively forgetting its search history
* `Ponder bool`: tells engine whether pondering is allowed or not. Allowing this means the engine may be allowed to search during an opponent's turn
* `Threads x`: an integer `x` telling engine the maximum number of threads it may use to search. This is best set to the number of threads your computer cpu supports
* `Debug bool`: tell engine to print debugging or extra information strings


## Direct use through UCI examples

Change default settings, then quit:
```shell
blunders/target/release>./blunders
setoption name Hash value 20
setoption name Ponder value false
setoption name Threads value 4
setoption name Debug value true
setoption name Clear Hash
quit
blunders/target/release>

```

Search the starting position to depth 3 to get info and bestmove output, then quit:
```shell
blunders/target/release>./blunders
position startpos
go depth 3
info depth 3 score cp +10 time 6 nodes 10000 nps 1666666 pv d2d4 d7d5 c2c4
bestmove d2d4
quit
blunders/target/release>

```

# Testing Blunders

Testing is done through `cargo`. There are several commands that can be run to test all crates. Note that there are extra debug assertions so it may be worth it to test in both debug and release modes.

Run relatively quick tests: `cargo test --all` or `cargo test --all --release`  
Run expensive tests: `cargo test --all -- --ignored` or `cargo test --all --release -- --ignored`

# Benchmarking Blunders

Blunders has some simple benchmarks that can be run with `cargo bench --all`.

# Checklist for 1.0

- [ ] Develop stable engine API
- [ ] Support single and multithreaded search
- [x] Blocking and non-blocking `search`
- [ ] Compile for WASM
- [ ] Add Blunders non-UCI commands for GUI-less play vs engine
- [ ] Clean library docs for `blunders-engine`
- [ ] Write User Starting Guide