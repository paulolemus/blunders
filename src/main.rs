//! Main CLI interface to Blunders engine.

use std::io;
use std::str::FromStr;

use blunders_engine;
use blunders_engine::fen::Fen;
use blunders_engine::search;
use blunders_engine::transposition::TranspositionTable;
use blunders_engine::uci::{self, UciCommand, UciOption, UciOptions, UciResponse};
use blunders_engine::Position;

fn main() -> io::Result<()> {
    println!("Blunders 0.1.0 by Paulo L");

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
    let mut tt = TranspositionTable::with_mb(uci_options["Hash"].spin().value());
    // Position to search.
    let mut position = Position::start_position();
    // If set to true, allow debugging strings to be printed.
    let mut debug = uci_options["Debug"].check().value;

    loop {
        // Wait to receive a line of input.
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        // Attempt to parse an input string into a UciCommand for processing.
        let command = if let Ok(comm) = UciCommand::from_str(&input) {
            comm
        } else {
            uci::error(&format!("{} could not be parsed", input.escape_debug()))?;
            continue;
        };

        match command {
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
                tt.clear();
                uci::debug(debug, "transposition table cleared")?;
            }

            // GUI commands engine to immediately stop any active search.
            UciCommand::Stop => {}

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
                        tt = TranspositionTable::with_mb(mb);
                        uci::debug(
                            debug,
                            &format!("tt mb: {}, capacity: {}", mb, tt.capacity()),
                        )?;

                    // Button was pressed to clear the hash table.
                    } else if option.name == "Clear Hash" {
                        tt.clear();
                        option.button_mut().pressed = false;
                        uci::debug(debug, "hash table cleared")?;

                    // Engine was informed if pondering is possible or not.
                    } else if option.name == "Ponder" {
                        uci::debug(
                            debug,
                            &format!("setoption Ponder: {}", option.check().value),
                        )?;

                    // Engine was given the number of threads it can use.
                    } else if option.name == "Threads" {
                        uci::debug(
                            debug,
                            &format!("setoption Threads: {}", option.spin().value),
                        )?;

                    // Engine debug mode was set.
                    } else if option.name == "Debug" {
                        let new_debug_value = option.check().value;
                        uci::debug(
                            debug | new_debug_value,
                            &format!("setoption Debug {}", new_debug_value),
                        )?;
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

            // Begin a search with provided parameters.
            UciCommand::Go(_search_ctrl) => {
                let depth = 7;
                let result = search::search_with_tt(position.clone(), depth, &mut tt);
                let score = result.score * position.player().sign();
                println!(
                    "info depth {} score cp {} time {} nodes {} nps {} pv {}",
                    depth,
                    score,
                    result.elapsed.as_millis(),
                    result.nodes,
                    (result.nodes as f64 / result.elapsed.as_secs_f64()).round(),
                    result.pv_line,
                );

                UciResponse::new_best_move(result.best_move).send()?;
            }
        }
    }

    Ok(())
}
