//! Main CLI interface to Blunders engine.

use std::io::{self, Write};
use std::time;

use blunders_engine;
use blunders_engine::coretypes::Move;
use blunders_engine::search::alpha_beta;
use blunders_engine::Position;

fn main() -> io::Result<()> {
    println!("Blunders CLI 0.1.0\n");

    let mut input = String::new();
    let mut position = Position::start_position();
    loop {
        println!("{}", position);
        print!("> ");
        io::stdout().flush().unwrap();

        // Handle non move input.
        input.clear();
        io::stdin().read_line(&mut input)?;
        match input.trim() {
            "exit" => break,
            "newgame" | "ng" => {
                position = Position::start_position();
                println!("Starting new game...");
                continue;
            }
            "help" => {
                println!("Commands:\nnewgame | ng => begin a new game,\nexit => end CLI.");
                continue;
            }
            _ => (),
        }

        let maybe_move: Result<Move, _> = input.trim().parse();

        if let Err(_) = maybe_move {
            println!("What was that?");
            continue;
        }

        // Have a move, have engine play after.
        if let Ok(move_) = maybe_move {
            let was_legal = position.do_legal_move(move_);

            if !was_legal {
                println!("That move was illegal! No action taken.");
                continue;
            }
            // Check if human player check or stalemated.
            if position.is_checkmate() {
                println!("Congrats!! You won by CHECKMATE. Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                continue;
            }
            if position.is_stalemate() {
                println!("The game is DRAWN via STALEMATE. Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                continue;
            }

            // Have computer play its response.
            println!("{}\nthinking...", position);
            let now = time::Instant::now();
            let (_cp, best_move) = alpha_beta(position, 6);
            let timed = now.elapsed();
            print!("Blunders played move {}, ", best_move);
            println!(
                "and thought for {}.{} seconds.",
                timed.as_secs(),
                timed.subsec_millis()
            );
            position.do_move(best_move);

            // Check if engine check or stalemated.
            if position.is_checkmate() {
                print!("Oh no!! Blunders engine was won by CHECKMATE. ");
                println!("Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                continue;
            }
            if position.is_stalemate() {
                println!("The game is DRAWN via STALEMATE. Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                continue;
            }
        }
    }
    Ok(())
}
