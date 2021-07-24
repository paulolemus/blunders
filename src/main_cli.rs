//! Main CLI interface to Blunders engine.

use std::io::{self, Write};

use blunders_engine;
use blunders_engine::coretypes::{Move, MoveInfo};
use blunders_engine::eval::evaluate_abs;
use blunders_engine::search;
use blunders_engine::transposition::TranspositionTable;
use blunders_engine::Position;

enum InputKind {
    Exit,
    Newgame,
    Help,
    Error,
    Undo,
    GameMove(Move),
}

impl From<&str> for InputKind {
    fn from(s: &str) -> Self {
        let maybe_move: Result<Move, _> = s.trim().parse();
        if let Ok(move_) = maybe_move {
            Self::GameMove(move_)
        } else {
            match s {
                "exit" => Self::Exit,
                "newgame" | "ng" => Self::Newgame,
                "help" => Self::Help,
                "undo" => Self::Undo,
                _ => Self::Error,
            }
        }
    }
}

fn main() -> io::Result<()> {
    println!("Blunders CLI 0.1.0\n");
    let mut tt = TranspositionTable::with_capacity(100_000);
    let mut input = String::new();
    let mut position = Position::start_position();
    let mut move_history: Vec<MoveInfo> = Vec::new();

    loop {
        // Wait for user input.
        {
            // Print evaluation of starting position.
            let static_cp = evaluate_abs(&position);
            println!("Current Static cp  : {}", static_cp);
        }
        println!("{}", position);
        print!("> ");
        io::stdout().flush().unwrap();
        input.clear();
        io::stdin().read_line(&mut input)?;

        let input_kind: InputKind = input.trim().into();

        match input_kind {
            InputKind::Exit => break,
            InputKind::Newgame => {
                position = Position::start_position();
                move_history.clear();
                println!("Starting new game...");
                continue;
            }
            InputKind::Help => {
                println!("Commands:");
                println!("newgame | ng => Begin a new game.");
                println!("undo => Undo the position to return to your last move.");
                println!("help => Print this help text.");
                println!("exit => end CLI.");
                println!("\nTo make a move, enter a move in algebraic coordinate form.");
                println!("Examples: d2d4 -> Move piece on D2 to D4.");
                continue;
            }
            InputKind::Undo => {
                // Undo both computer's move and player's last move.
                if let Some(our_move_info) = move_history.pop() {
                    position.undo_move(our_move_info);
                    println!("Undo move {}.", our_move_info.move_());
                }
                if let Some(their_move_info) = move_history.pop() {
                    position.undo_move(their_move_info);
                    println!("Undo move {}.", their_move_info.move_());
                }
                continue;
            }
            InputKind::Error => {
                println!("Invalid command: {}", input);
                continue;
            }
            _ => (),
        }

        // Process a player move, then process an engine move.
        if let InputKind::GameMove(move_) = input_kind {
            let (was_legal, maybe_move_info) = position.do_legal_move(move_);

            if !was_legal {
                println!("That move was illegal! No action taken.");
                continue;
            }
            move_history.push(maybe_move_info.unwrap());

            // Check if human player check or stalemated.
            if position.is_checkmate() {
                println!("{}", position);
                println!("Congrats!! You won by CHECKMATE. Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                move_history.clear();
                continue;
            }
            if position.is_stalemate() {
                println!("{}", position);
                println!("The game is DRAWN via STALEMATE. Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                move_history.clear();
                continue;
            }
            {
                // Print evaluation of position after player move.
                let static_cp = evaluate_abs(&position);
                println!("Current Static cp  : {}", static_cp);
            }

            // Have computer play its response.
            println!("{}\nthinking...", position);
            let result = search::search_with_tt(position, 8, &mut tt);
            move_history.push(position.do_move(result.best_move));

            // Check if engine check or stalemated.
            if position.is_checkmate() {
                println!("Oh no!! Blunders engine was won by CHECKMATE. ");
                println!("{}", position);
                println!("Press Enter to start a new game.");

                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                move_history.clear();
                continue;
            }
            if position.is_stalemate() {
                println!("The game is DRAWN via STALEMATE.");
                println!("{}", position);
                println!("Press Enter to start a new game.");
                io::stdin().read_line(&mut input)?;
                position = Position::start_position();
                move_history.clear();
                continue;
            }

            // Print diagnostic information.
            let static_cp = evaluate_abs(&position);
            println!("Blunders played move {}.", result.best_move);
            println!("{}", result);
            println!("Current Static cp: {}", static_cp);
        }
    }
    Ok(())
}
