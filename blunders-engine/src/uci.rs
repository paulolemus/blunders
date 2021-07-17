//! Universal Chess Interface
//!
//! TODO:
//! Need to figure out how to write a main function that
//! allows for input to be processed at any time,
//! and also for a search to be interrupted OR return at any time.
//!
//! One thought for input:
//! Have a unique thread for input. It will have a condvar?
//! Input thread will always block, listening for input.
//! When input is received, it parses it into a command, then sends it to the main thread.
//! It can send over a channel?
//!
//! Main thread:
//! Start input loop, process commands, delegate searches.
//! Main thread cannot block on any large task,
//! so searching and input need to be on different threads.
//! data structures like transposition table, curr_position.
//!
//! Main thread goes to sleep, and wakes up if:
//! 1. Input is received
//! 2. A search finishes
//!
//! Main thread may set values in search WHILE SEARCHING:
//! 1. stop=use most recent SearchResult from IDS. Active search returns.
//!
//!
//!

use crate::coretypes::Move;
use crate::Position;

const UCI_ID_NAME: &str = "Blunders 0.1";
const UCI_ID_AUTHOR: &str = "Paulo L.";

// Need to figure out what goes in variant.
pub struct Placeholder {}

/// UciCommands commands from an external program sent to this chess engine.
pub enum UciCommand {
    Uci,
    Debug(bool),
    IsReady,
    SetOption(Placeholder),
    UciNewGame,
    Pos(Position),
    Go(Placeholder),
    Stop,
    PonderHit,
    Quit,
}

/// Engine to external program communication.
pub enum UciResponse {
    Id,
    UciOk,
    ReadyOk,
    BestMove(Move),
    Info(UciInfo),
}

pub struct UciInfo {}

pub struct UciOption {}

impl UciCommand {
    fn parse_command(s: &str) -> Result<Self, &'static str> {
        let mut parts = s.trim().split_whitespace();
        let head = parts.next().ok_or("Empty Command.")?;

        match head {
            "uci" => Ok(UciCommand::Uci),
            "debug" => unimplemented!(),
            "isready" => Ok(UciCommand::IsReady),
            "setoption" => unimplemented!(),
            "ucinewgame" => Ok(UciCommand::UciNewGame),
            "position" => unimplemented!(),
            "go" => unimplemented!(),
            "stop" => Ok(UciCommand::Stop),
            "ponderhit" => Ok(UciCommand::PonderHit),
            "quit" => Ok(UciCommand::Quit),
            _ => Err("Command unknown."),
        }
    }
}
