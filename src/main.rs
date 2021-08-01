//! Main CLI interface to Blunders engine.

use std::io;
use std::panic;
use std::process;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use blunders_engine;
use blunders_engine::arrayvec::display;
use blunders_engine::search::{self, SearchResult};
use blunders_engine::uci::{self, UciCommand, UciOption, UciOptions, UciResponse};
use blunders_engine::{Fen, Position, TranspositionTable};

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
            Err(_) => {
                let err_str = format!("{} could not be parsed", buffer.escape_debug());
                if let Err(_) = uci::error(&err_str) {
                    return;
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

const NAME: &'static str = env!("CARGO_PKG_NAME");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");

fn main() -> io::Result<()> {
    println!("{} {} by {}", NAME, VERSION, AUTHOR);

    // Hook to print errors to STDOUT.
    panic_hook();

    // Engine Internal parameters
    let mut uci_options = UciOptions::new();
    // option name Hash type spin default 1 min 1 max 16000
    // option name Clear Hash type button
    // option name Ponder type check default false
    // option name Threads type spin default 1 min 1 max 32
    // option name Debug type check default true
    let option_hash = UciOption::new_spin("Hash", 1, 1, 16000);
    let option_clear_hash = UciOption::new_button("Clear Hash", false);
    let option_ponder = UciOption::new_check("Ponder", false);
    let option_threads = UciOption::new_spin("Threads", 1, 1, 32);
    let option_debug = UciOption::new_check("Debug", true);
    uci_options.insert(option_hash);
    uci_options.insert(option_clear_hash);
    uci_options.insert(option_ponder);
    uci_options.insert(option_threads);
    uci_options.insert(option_debug);

    // Engine global transposition table.
    let tt = TranspositionTable::with_mb(uci_options["Hash"].spin().value());
    let tt = Arc::new(Mutex::new(tt));
    // Position to search.
    let mut position = Position::start_position();
    // If set to true, allow debugging strings to be printed.
    let mut debug = uci_options["Debug"].check().value;

    // Communications between input, search, and main threads.
    let (sender, receiver) = mpsc::channel::<Message>();

    // Create input thread.
    let input_sender = sender.clone();
    let input_thread_handle = thread::spawn(move || input_handler(input_sender));

    // Search stopper, set this to stop any active searches.
    let stopper = Arc::new(AtomicBool::new(false));

    // Only a single search at a time is allowed. Handle is stored here.
    let mut search_handle: Option<JoinHandle<()>> = None;

    // Message can either be A UciCommand received from external source,
    // or the results of a search. Process accordingly.
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Command(command) => match command {
                // GUI is telling engine to use UCI protocol.
                // It requires a response of Id, available options, and an acknowledgement.
                UciCommand::Uci => {
                    UciResponse::Id.send()?;
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
                UciCommand::UciNewGame => {
                    {
                        tt.lock().unwrap().clear();
                    }
                    uci::debug(debug, "transposition table cleared")?;
                }

                // GUI commands engine to immediately stop any active search.
                UciCommand::Stop => {
                    uci::debug(debug, "stopping...")?;
                    stopper.store(true, Ordering::Relaxed);
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
                    debug = uci_options["Debug"].check().value;
                }

                // Command to change engine internal parameters.
                // This should only be sent while engine is waiting.
                UciCommand::SetOption(raw_opt) => match uci_options.update(&raw_opt) {
                    Ok(option) => {
                        // Received a new hash table capacity, so reassign tt.
                        if option.name == "Hash" {
                            let mb = option.spin().value();
                            let capacity = {
                                let mut locked_tt = tt.lock().unwrap();
                                *locked_tt = TranspositionTable::with_mb(mb);
                                locked_tt.capacity()
                            };
                            uci::debug(debug, &format!("tt mb: {}, capacity: {}", mb, capacity))?;

                        // Button was pressed to clear the hash table.
                        } else if option.name == "Clear Hash" {
                            {
                                tt.lock().unwrap().clear();
                            }
                            option.button_mut().pressed = false;
                            uci::debug(debug, "hash table cleared")?;

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
                        }
                    }
                    Err(s) => {
                        uci::error(s)?;
                    }
                },

                // Set the current position.
                UciCommand::Pos(new_position) => {
                    position = new_position;
                    uci::debug(debug, &format!("set position {}", position.to_fen()))?;
                }

                // Begin a search with provided parameters. Only search if are no other active searches.
                UciCommand::Go(_search_ctrl) => {
                    // TODO: When receive a go command, STOP the current search, wait for it, then start a new one
                    if search_handle.is_none() {
                        uci::debug(debug, "go starting search...")?;
                        // Ensure stopper is not set before starting search.
                        stopper.store(false, Ordering::SeqCst);

                        let depth = 7;
                        let handle = search::search_nonblocking(
                            position.clone(),
                            depth,
                            Arc::clone(&tt),
                            Arc::clone(&stopper),
                            sender.clone(),
                        );
                        search_handle = Some(handle);
                    } else {
                        uci::error("search already in progress. Cannot begin new search")?;
                    }
                }
            },

            // A search has finished and the results have been returned.
            Message::Search(search_result) => {
                uci::debug(debug, "search_result begin")?;
                println!(
                    "info depth {} score cp {} time {} nodes {} nps {} pv {}",
                    search_result.depth,
                    search_result.relative_score(),
                    search_result.elapsed.as_millis(),
                    search_result.nodes,
                    search_result.nps(),
                    display(&search_result.pv_line),
                );

                UciResponse::new_best_move(search_result.best_move).send()?;

                // Wait for thread to clean up.
                uci::debug(debug, "search_result join handle waiting...")?;
                let instant = Instant::now();
                search_handle.take().unwrap().join().unwrap();
                let time_str = format!("search_result join time: {:?}", instant.elapsed());
                uci::debug(debug, &time_str)?;
            }
        }
    }

    // Inform any active search to stop.
    stopper.store(true, Ordering::SeqCst);
    // Wait for threads to close out.
    input_thread_handle.join().unwrap();
    search_handle
        .into_iter()
        .for_each(|handle| handle.join().unwrap());

    Ok(())
}
