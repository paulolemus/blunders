//! Main CLI interface to Blunders engine.

use std::io;
use std::str::FromStr;

use blunders_engine;
use blunders_engine::search;
use blunders_engine::transposition::TranspositionTable;
use blunders_engine::uci::{UciCommand, UciOption, UciOptions, UciResponse};
use blunders_engine::Position;

fn main() -> io::Result<()> {
    println!("Blunders 0.1.0 by Paulo L");

    let mut tt = TranspositionTable::with_capacity(100_000);
    let mut position = Position::start_position();
    let mut uci_options = UciOptions::new();

    // option name Hash type spin default 1 min 1 max 16000
    // option name Clear Hash type button
    // option name Ponder type check default false
    // option name Threads type spin default 1 min 1 max 32
    let option_hash = UciOption::new_spin("Hash", 1, 1, 16000);
    let option_clear_hash = UciOption::new_button("Clear Hash", false);
    let option_ponder = UciOption::new_check("Ponder", false);
    let option_threads = UciOption::new_spin("Threads", 1, 1, 32);

    uci_options.insert(option_hash);
    uci_options.insert(option_clear_hash);
    uci_options.insert(option_ponder);
    uci_options.insert(option_threads);

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        // Get next valid command.
        let command = if let Ok(comm) = UciCommand::from_str(&input) {
            comm
        } else {
            println!("info error {} could not be parsed", input);
            continue;
        };

        match command {
            UciCommand::Uci => {
                UciResponse::Id.send()?;
                for uci_opt in uci_options.values() {
                    UciResponse::new_option(uci_opt.clone()).send()?;
                }
                UciResponse::UciOk.send()?;
            }
            UciCommand::IsReady => {
                UciResponse::ReadyOk.send()?;
            }
            UciCommand::UciNewGame => {
                tt.clear();
            }
            UciCommand::Stop => {}
            UciCommand::PonderHit => {}
            UciCommand::Quit => break,
            UciCommand::Debug(_value) => {}
            UciCommand::SetOption(raw_opt) => match uci_options.update_from_raw(&raw_opt) {
                Ok(_) => println!("info setoption updated successfully"),
                Err(s) => {
                    print!("info setoption error: ");
                    println!("{}", s);
                }
            },
            UciCommand::Pos(new_position) => {
                position = new_position;
            }
            UciCommand::Go(_search_ctrl) => {
                let result = search::search_with_tt(position.clone(), 7, &mut tt);
                println!(
                    "info depth 7 score cp {} time {} nodes {} nps {} pv {}",
                    result.score,
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
