//! Main CLI interface to Blunders engine.

use std::convert::TryFrom;
use std::io;
use std::panic;
use std::process;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use blunders_engine::arrayvec::display;
use blunders_engine::uci::{self, UciCommand, UciOption, UciOptions, UciResponse};
use blunders_engine::{EngineBuilder, Fen, Game, Mode, SearchResult};

/// Message type passed over channels.
#[derive(Debug, Clone)]
enum Message {
    Command(UciCommand),
    Search(SearchResult),
}

impl From<UciCommand> for Message {
    fn from(uci_command: UciCommand) -> Self {
        Message::Command(uci_command)
    }
}

impl From<SearchResult> for Message {
    fn from(search_result: SearchResult) -> Self {
        Message::Search(search_result)
    }
}

/// Input Handler thread function.
/// Input is parsed in a separate thread from main so Blunders may process
/// both input and search results at the same time.
fn input_handler(sender: mpsc::Sender<Message>) {
    loop {
        // Wait to receive a line of input.
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();

        // Try to parse into valid input.
        match UciCommand::from_str(&buffer) {
            // On success, send to main thread. If command was quit, exit.
            Ok(command) => {
                let is_quit = command == UciCommand::Quit;
                let send_result = sender.send(command.into());

                if is_quit || send_result.is_err() {
                    return;
                }
            }

            // On error, report over UCI.
            // On error reporting, exit.
            Err(err) => {
                let err_str = format!("{} could not be parsed, {}", buffer.escape_debug(), err);
                if let Err(err) = uci::error(&err_str) {
                    panic!("{}", err);
                }
            }
        }
    }
}

/// Function that adds to panic hook by printing error data to stdout,
/// so they are visible in GUI.
fn panic_hook() {
    // Print panic errors to STDOUT so they are visible in GUI.
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Print Error payload.
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            uci::error(s).unwrap();
        }
        // Print Error location information.
        if let Some(location) = panic_info.location() {
            let err_str = format!(
                "panic in file '{}' at line {}",
                location.file(),
                location.line()
            );
            uci::error(&err_str).unwrap();
        }

        // Run original hook then exit.
        orig_hook(panic_info);
        process::exit(1);
    }));
}

const NAME_VERSION: &'static str = concat!(env!("CARGO_PKG_NAME"), ' ', env!("CARGO_PKG_VERSION"));
const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");

fn main() -> io::Result<()> {
    println!("{} by {}", NAME_VERSION, AUTHOR);

    // Hook to print errors to STDOUT on panic.
    panic_hook();

    // Engine Internal parameters
    // option name Hash type spin default 1 min 1 max 16000
    // option name Clear Hash type button
    // option name Ponder type check default false
    // option name Threads type spin default 1 min 1 max 32
    // option name Debug type check default true
    let mut uci_options = UciOptions::new();
    uci_options.insert(UciOption::new_spin("Hash", 1, 1, 16000));
    uci_options.insert(UciOption::new_button("Clear Hash", false));
    uci_options.insert(UciOption::new_check("Ponder", false));
    uci_options.insert(UciOption::new_spin("Threads", 1, 1, 32));
    uci_options.insert(UciOption::new_check("Debug", true));

    // Current chess game with move history.
    let mut game = Game::start_position();

    // If set to true, allow debugging strings to be printed.
    let mut debug = uci_options["Debug"].check().value;

    // Communications between input, search, and main threads.
    let (sender, receiver) = mpsc::channel::<Message>();

    // Create input thread.
    let input_sender = sender.clone();
    let input_thread_handle = thread::spawn(move || input_handler(input_sender));

    // Main Engine instance.
    let mut engine = EngineBuilder::new()
        .transpositions_mb(uci_options["Hash"].spin().value())
        .threads(uci_options["Threads"].spin().value())
        .debug(debug)
        .game(game.clone())
        .build();

    // Message can either be A UciCommand received from external source,
    // or the results of a search. Process accordingly.
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Command(command) => match command {
                // GUI is telling engine to use UCI protocol.
                // It requires a response of Id, available options, and an acknowledgement.
                UciCommand::Uci => {
                    UciResponse::new_id(NAME_VERSION, AUTHOR).send()?;
                    for uci_opt in uci_options.values() {
                        UciResponse::new_option(uci_opt.clone()).send()?;
                    }
                    UciResponse::UciOk.send()?;
                }

                // Command used to sync GUI with engine. Requires acknowledgement response.
                UciCommand::IsReady => {
                    UciResponse::ReadyOk.send()?;
                }

                // The next search will be from a different game.
                // Clearing the transposition table of all entries allows engine
                // to enter new game without prior information.
                UciCommand::UciNewGame => match engine.new_game() {
                    Ok(()) => uci::debug(debug, "transposition table cleared")?,
                    Err(err) => uci::error(&err.to_string())?,
                },

                // GUI commands engine to immediately stop any active search.
                UciCommand::Stop => {
                    uci::debug(debug, "stopping...")?;
                    engine.stop();
                }

                // Inform the engine that user has played an expected move and may
                // continue its search of that move if applicable.
                UciCommand::PonderHit => {}

                // Shutdown engine.
                UciCommand::Quit => break,

                // Tells engine to send extra `info string` to the GUI.
                // Command can be sent anytime.
                UciCommand::Debug(new_debug_value) => {
                    uci::debug(
                        debug | new_debug_value,
                        &format!("set debug {}", new_debug_value),
                    )?;

                    // Update both engine options and global debug flag.
                    uci_options["Debug"].check_mut().value = new_debug_value;
                    debug = new_debug_value;
                    engine.set_debug(new_debug_value);
                }

                // Command to change engine internal parameters.
                // This should only be sent while engine is waiting.
                UciCommand::SetOption(raw_opt) => match uci_options.update(&raw_opt) {
                    Ok(option) => {
                        // Received a new hash table capacity, so reassign tt.
                        if option.name == "Hash" {
                            let mb = option.spin().value();

                            match engine.try_set_transpositions_mb(mb) {
                                Ok(capacity) => {
                                    let s = format!("tt mb: {}, capacity: {}", mb, capacity);
                                    uci::debug(debug, &s)?;
                                }
                                Err(err) => uci::error(&err.to_string())?,
                            };

                        // Button was pressed to clear the hash table.
                        } else if option.name == "Clear Hash" {
                            option.button_mut().pressed = false;

                            match engine.try_clear_transpositions() {
                                Ok(()) => uci::debug(debug, "hash table cleared")?,
                                Err(err) => uci::error(&err.to_string())?,
                            };

                        // Engine was informed if pondering is possible or not.
                        } else if option.name == "Ponder" {
                            let response = format!("setoption Ponder: {}", option.check().value);
                            uci::debug(debug, &response)?;

                        // Engine was given the number of threads it can use.
                        } else if option.name == "Threads" {
                            let response = format!("setoption Threads: {}", option.spin().value);
                            uci::debug(debug, &response)?;

                        // Engine debug mode was set.
                        } else if option.name == "Debug" {
                            let new_debug_value = option.check().value;
                            let response = format!("setoption Debug {}", new_debug_value);
                            uci::debug(debug | new_debug_value, &response)?;

                            debug = new_debug_value;
                            engine.set_debug(new_debug_value);
                        }
                    }
                    Err(err) => uci::error(&err.to_string())?,
                },

                // Set the current position.
                UciCommand::Pos(new_game) => {
                    game = new_game;
                    engine.set_game(game.clone());
                    uci::debug(debug, &format!("set position {}", game.position.to_fen()))?;
                }

                // Begin a search with provided parameters. Only search if are no other active searches.
                UciCommand::Go(search_ctrl) => {
                    let mode = match Mode::try_from(search_ctrl) {
                        Ok(mode) => mode,
                        Err(err) => {
                            uci::error(&err.to_string())?;
                            uci::error("falling back to depth search")?;
                            Mode::depth(6, None)
                        }
                    };

                    // TODO: consider stopping any active search to ensure new search can always start.
                    match engine.search(mode, sender.clone()) {
                        Ok(()) => uci::debug(debug, "go starting search...")?,
                        Err(err) => uci::error(&err.to_string())?,
                    };
                }
            },

            // A search has finished and the results have been returned.
            Message::Search(search_result) => {
                uci::debug(debug, "search_result begin")?;
                let extras = format!(
                    "string q_nodes {} q_nps {} q_ratio {:.2} tt_cuts {} tt_hits {} cut_ratio {:.2}",
                    search_result.q_nodes,
                    search_result.q_nps(),
                    search_result.quiescence_ratio(),
                    search_result.tt_cuts,
                    search_result.tt_hits,
                    search_result.tt_cut_ratio()
                );
                println!(
                    "info depth {} score cp {} time {} nodes {} nps {} pv {} {}",
                    search_result.depth,
                    search_result.relative_score(),
                    search_result.elapsed.as_millis(),
                    search_result.nodes,
                    search_result.nps(),
                    display(&search_result.pv),
                    extras
                );
                UciResponse::new_best_move(search_result.best_move).send()?;

                // Wait for engine to clean up.
                uci::debug(debug, "engine waiting...")?;
                let instant = Instant::now();
                engine.wait();
                let time_str = format!("engine wait time: {:?}", instant.elapsed());
                uci::debug(debug, &time_str)?;
            }
        };
    }

    // Inform any active search to stop.
    engine.shutdown();
    // Wait for input thread to close out.
    input_thread_handle.join().unwrap();

    Ok(())
}
