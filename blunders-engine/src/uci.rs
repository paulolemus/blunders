//! Universal Chess Interface
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

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt::{self, Display, Write};
use std::hash::{Hash, Hasher};
use std::io;
use std::ops::Deref;
use std::ops::{Index, IndexMut};
use std::str::{FromStr, SplitWhitespace};

use crate::coretypes::Move;
use crate::fen::Fen;
use crate::Position;

const UCI_ID_NAME: &str = "Blunders 0.1";
const UCI_ID_AUTHOR: &str = "Paulo L";

/// UciCommands commands from an external program sent to this chess engine.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UciCommand {
    Uci,
    Debug(bool),
    IsReady,
    SetOption(RawOption),
    UciNewGame,
    Pos(Position),
    Go(SearchControls),
    Stop,
    PonderHit,
    Quit,
}

impl UciCommand {
    /// Parse a single input line into a UciCommand if possible.
    pub fn parse_command(input_str: &str) -> Result<Self, &'static str> {
        let mut input = input_str.split_whitespace();
        let head = input.next().ok_or("Empty Command")?;

        match head {
            "uci" => Ok(UciCommand::Uci),
            "debug" => Self::parse_debug(input),
            "isready" => Ok(UciCommand::IsReady),
            "setoption" => Self::parse_setoption(input),
            "ucinewgame" => Ok(UciCommand::UciNewGame),
            "position" => Self::parse_pos(input),
            "go" => Self::parse_go(input),
            "stop" => Ok(UciCommand::Stop),
            "ponderhit" => Ok(UciCommand::PonderHit),
            "quit" => Ok(UciCommand::Quit),
            _ => Err("Command unknown"),
        }
    }

    /// Extract a `debug` command if possible.
    /// command: `debug [on | off]`
    fn parse_debug(mut input: SplitWhitespace) -> Result<Self, &'static str> {
        let debug_mode_str = input.next().ok_or("debug missing mode [on | off]")?;

        match debug_mode_str {
            "on" => Ok(Self::Debug(true)),
            "off" => Ok(Self::Debug(false)),
            _ => Err("debug mode invalid argument"),
        }
    }

    /// Extract a `setoption` command if possible.
    ///command: `setoption name [id] (value x)`
    fn parse_setoption(mut input: SplitWhitespace) -> Result<Self, &'static str> {
        let name = input.next().ok_or("setoption missing name")?;
        (name == "name")
            .then(|| ())
            .ok_or("setoption not followed by name")?;

        let mut name = String::new();
        let mut value = String::new();
        let mut had_value = false;

        // the id following `name` consists of the input string until the token
        // `value` or end of input is encountered.
        while let Some(token) = input.next() {
            if token == "value" {
                had_value = true;
                break;
            } else {
                name.push_str(token);
                name.push(' ');
            }
        }
        name.pop(); // Remove trailing space.
        (name.len() > 0)
            .then(|| ())
            .ok_or("setoption name not followed by id")?;

        // input iterator is either empty, or "value" has been parsed and the rest
        // of input is the contents of value string.
        if had_value {
            for token in input {
                value.push_str(token);
                value.push(' ');
            }
            value.pop(); // Remove trailing space.
            (value.len() > 0)
                .then(|| ())
                .ok_or("setoption value not followed by string")?;
        }

        Ok(UciCommand::SetOption(RawOption {
            name: name.as_str().into(),
            value,
        }))
    }

    /// Extract a `position` command if possible.
    /// command: `position [fen fen_str | startpos] (moves move_list ...)`
    fn parse_pos(mut input: SplitWhitespace) -> Result<Self, &'static str> {
        let position_input = input
            .next()
            .ok_or("position missing description [fen | startpos]")?;

        // Parse a valid position from startpos or FEN, or return an Err(_).
        let mut position = match position_input {
            "startpos" => Ok(Position::start_position()),
            "fen" => {
                let mut fen_str = String::new();
                let err_str = "position fen malformed";
                for _ in 0..6 {
                    fen_str.push_str(input.next().ok_or(err_str)?);
                    fen_str.push(' ');
                }
                Position::parse_fen(&fen_str).map_err(|_| err_str)
            }
            _ => Err("position description type invalid"),
        }?;

        // Check if there is a sequence of moves to apply to the position.
        if let Some("moves") = input.next() {
            for move_str in input {
                let move_ = Move::from_str(move_str)?;
                position
                    .is_legal_move(move_)
                    .then(|| ())
                    .ok_or("position moves provided illegal move")?;

                position.do_move(move_);
            }
        }

        Ok(Self::Pos(position))
    }

    /// Extract a `go` command if possible.
    /// command: `go [wtime | btime | winc | binc | depth | nodes | mate | movetime | infinite]*`
    fn parse_go(mut input: SplitWhitespace) -> Result<Self, &'static str> {
        // The following options have no arguments:
        // ponder, infinite
        // The following options must be followed with an integer value:
        // wtime, btime, winc, binc, depth, nodes, mate, movetime, movestogo
        const HAS_U32_ARG: [&'static str; 8] = [
            "wtime",
            "btime",
            "winc",
            "binc",
            "depth",
            "movestogo",
            "mate",
            "movetime",
        ];

        let mut controls = SearchControls::new();

        while let Some(input_str) = input.next() {
            // Attempt to parse all options with a u32 argument type.
            if HAS_U32_ARG.contains(&input_str) {
                let argument: u32 = input
                    .next()
                    .ok_or("go no argument provided")?
                    .parse()
                    .map_err(|_| "go failed to parse integer")?;

                match input_str {
                    "wtime" => controls.wtime = Some(argument),
                    "btime" => controls.btime = Some(argument),
                    "winc" => controls.winc = Some(argument),
                    "binc" => controls.binc = Some(argument),
                    "depth" => controls.depth = Some(argument),
                    "movestogo" => controls.moves_to_go = Some(argument),
                    "mate" => controls.mate = Some(argument),
                    "movetime" => controls.move_time = Some(argument),
                    _ => return Err("go invalid option"),
                };
            } else if input_str == "nodes" {
                let argument: u64 = input
                    .next()
                    .ok_or("go no argument provided")?
                    .parse()
                    .map_err(|_| "go failed to parse integer")?;
                controls.nodes = Some(argument);
            } else if input_str == "infinite" {
                controls.infinite = true;
            } else {
                return Err("go invalid option");
            }
        }

        Ok(UciCommand::Go(controls))
    }
}

impl FromStr for UciCommand {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_command(s)
    }
}

/// Engine to external program communication.
#[derive(Debug, Clone)]
pub enum UciResponse {
    Id,
    UciOk,
    ReadyOk,
    Opt(UciOption),
    BestMove(Move),
    Info(UciInfo),
}

impl UciResponse {
    pub fn new_option(uci_opt: UciOption) -> Self {
        Self::Opt(uci_opt)
    }

    pub fn new_best_move(move_: Move) -> Self {
        Self::BestMove(move_)
    }

    pub fn new_info(uci_info: UciInfo) -> Self {
        Self::Info(uci_info)
    }

    /// Send this UciResponse over stdout.
    /// TODO: Allow for writing to files or stdout.
    pub fn send(&self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        <io::StdoutLock as io::Write>::write_all(&mut handle, self.to_string().as_ref())?;
        <io::StdoutLock as io::Write>::flush(&mut handle)
    }
}

impl Display for UciResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Id => {
                f.write_str("id name ")?;
                f.write_str(UCI_ID_NAME)?;
                f.write_char('\n')?;
                f.write_str("id author ")?;
                f.write_str(UCI_ID_AUTHOR)?;
                f.write_char('\n')
            }
            Self::UciOk => f.write_str("uciok\n"),
            Self::ReadyOk => f.write_str("readyok\n"),
            Self::BestMove(move_) => {
                f.write_str("bestmove ")?;
                move_.fmt(f)?;
                f.write_char('\n')
            }
            Self::Opt(uci_opt) => {
                write!(f, "{}\n", uci_opt)
            }
            Self::Info(_info) => {
                // TODO
                f.write_str("info string todo\n")
            }
        }
    }
}

/// Send a debug info string over UCI.
/// TODO: This is a temporary function until UciInfo and UciResponse are worked out.
pub fn debug(can_debug: bool, s: &str) -> io::Result<()> {
    if can_debug {
        let mut debug_str = String::from("info string debug ");
        debug_str.push_str(s);
        debug_str.push('\n');

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        <io::StdoutLock as io::Write>::write_all(&mut handle, debug_str.as_ref())?;
        <io::StdoutLock as io::Write>::flush(&mut handle)
    } else {
        Ok(())
    }
}

/// Send an error info string over UCI.
/// TODO: This is a temporary function until UciInfo and UciResponse are worked out.
pub fn error(s: &str) -> io::Result<()> {
    let mut error_str = String::from("info string error ");
    error_str.push_str(s);
    error_str.push('\n');

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    <io::StdoutLock as io::Write>::write_all(&mut handle, error_str.as_ref())?;
    <io::StdoutLock as io::Write>::flush(&mut handle)
}

#[derive(Debug, Clone)]
pub struct UciInfo {}

/// Type parsed from a Uci `setoption` command.
/// The value is stringly typed, because it can be a string, bool, integer, or nothing.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawOption {
    name: CaselessString,
    value: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Check {
    pub value: bool,
    pub default: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Spin {
    pub value: i64,
    pub default: i64,
    pub min: i64,
    pub max: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Combo {
    pub value: String,
    pub default: String,
    pub choices: HashSet<String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Button {
    pub pressed: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UciOptionString {
    pub value: String,
    pub default: String,
}

impl Spin {
    /// Spin uses an i64 as its value type because it must cover any sort of numeric input.
    /// Spin::value<T> allows the value to be converted automatically to the intended type.
    /// This panics if the type cannot convert.
    pub fn value<T: TryFrom<i64>>(&self) -> T {
        match T::try_from(self.value) {
            Ok(converted) => converted,
            _ => panic!("spin value TryFrom<i64> conversion failed"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UciOptionType {
    Check(Check),
    Spin(Spin),
    Combo(Combo),
    Button(Button),
    String(UciOptionString),
}

impl Display for UciOptionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UciOptionType::Check(Check { default, .. }) => {
                write!(f, "type check default {}", default)
            }
            UciOptionType::Spin(Spin {
                default, min, max, ..
            }) => {
                write!(f, "type spin default {} min {} max {}", default, min, max)
            }
            UciOptionType::Combo(Combo {
                default, choices, ..
            }) => {
                write!(f, "type combo default {}", default)?;
                for choice in choices {
                    write!(f, " var {}", choice)?;
                }
                Ok(())
            }
            UciOptionType::Button(_) => f.write_str("type button"),
            UciOptionType::String(UciOptionString { default, .. }) => {
                write!(f, "type string default {}", default)
            }
        }
    }
}

/// Options to allow:
/// option name Hash type spin default 1 min 1 max 16000
/// option name Clear Hash type button
/// option name Ponder type check default false
/// option name Threads type spin default 1 min 1 max 32
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UciOption {
    pub name: CaselessString,
    pub option_type: UciOptionType,
}

impl UciOption {
    /// Create a new UciOption of type check, with a default value.
    pub fn new_check(name: &str, default: bool) -> Self {
        Self {
            name: name.into(),
            option_type: UciOptionType::Check(Check {
                value: default,
                default,
            }),
        }
    }

    /// Create a new UciOption of type spin with a default value, and a min and max.
    pub fn new_spin(name: &str, default: i64, min: i64, max: i64) -> Self {
        assert!(min < max, "Illegal spin, min >= max");
        assert!(default >= min, "Illegal spin, default < min");
        assert!(default <= max, "Illegal spin, default > max");

        Self {
            name: name.into(),
            option_type: UciOptionType::Spin(Spin {
                value: default,
                default,
                min,
                max,
            }),
        }
    }

    /// Create a new UciOption of type combo with a default value and a list of choices.
    /// Default value must be a member of choices, including capitalization, but
    /// ignoring whitespace.
    pub fn new_combo(name: &str, default: &str, choices: &[&str]) -> Self {
        let default = default.trim().to_string();
        let choices: HashSet<String> = choices.iter().map(|s| s.trim().to_string()).collect();

        // Assert that default is a legal choice in a case insensitive comparison.
        assert!(matches!(
            choices
                .iter()
                .find(|item| item.to_lowercase() == default.to_lowercase()),
            Some(_)
        ));

        Self {
            name: name.into(),
            option_type: UciOptionType::Combo(Combo {
                value: default.clone(),
                default,
                choices,
            }),
        }
    }

    /// Create a new UciOption of type button with a default state of pressed or not pressed.
    pub fn new_button(name: &str, pressed: bool) -> Self {
        Self {
            name: name.into(),
            option_type: UciOptionType::Button(Button { pressed }),
        }
    }

    /// Create a new UciOption of type string with a default value.
    pub fn new_string(name: &str, default: &str) -> Self {
        Self {
            name: name.into(),
            option_type: UciOptionType::String(UciOptionString {
                value: default.trim().to_string(),
                default: default.trim().to_string(),
            }),
        }
    }

    /// Assume that a UciOption is of type Check, and return reference to inner Check struct.
    /// Panics if UciOption is not Check.
    pub fn check(&self) -> &Check {
        match self.option_type {
            UciOptionType::Check(ref check) => check,
            _ => panic!("option type is not check"),
        }
    }
    /// Assume that a UciOption is of type Spin, and return reference to inner Spin struct.
    /// Panics if UciOption is not Spin.
    pub fn spin(&self) -> &Spin {
        match self.option_type {
            UciOptionType::Spin(ref spin) => spin,
            _ => panic!("option type is not spin"),
        }
    }
    /// Assume that a UciOption is of type Combo, and return reference to inner Combo struct.
    /// Panics if UciOption is not Combo.
    pub fn combo(&self) -> &Combo {
        match self.option_type {
            UciOptionType::Combo(ref combo) => combo,
            _ => panic!("option type is not combo"),
        }
    }
    /// Assume that a UciOption is of type Button, and return reference to inner Button struct.
    /// Panics if UciOption is not Button.
    pub fn button(&self) -> &Button {
        match self.option_type {
            UciOptionType::Button(ref button) => button,
            _ => panic!("option type is not button"),
        }
    }
    /// Assume that a UciOption is of type String, and return reference to inner String struct.
    /// Panics if UciOption is not String.
    pub fn string(&self) -> &UciOptionString {
        match self.option_type {
            UciOptionType::String(ref s) => s,
            _ => panic!("option type is not String"),
        }
    }

    /// Assume that a UciOption is of type Check, and return reference to inner Check struct.
    /// Panics if UciOption is not Check.
    pub fn check_mut(&mut self) -> &mut Check {
        match self.option_type {
            UciOptionType::Check(ref mut check) => check,
            _ => panic!("option type is not check"),
        }
    }
    /// Assume that a UciOption is of type Spin, and return reference to inner Spin struct.
    /// Panics if UciOption is not Spin.
    pub fn spin_mut(&mut self) -> &mut Spin {
        match self.option_type {
            UciOptionType::Spin(ref mut spin) => spin,
            _ => panic!("option type is not spin"),
        }
    }
    /// Assume that a UciOption is of type Combo, and return reference to inner Combo struct.
    /// Panics if UciOption is not Combo.
    pub fn combo_mut(&mut self) -> &mut Combo {
        match self.option_type {
            UciOptionType::Combo(ref mut combo) => combo,
            _ => panic!("option type is not combo"),
        }
    }
    /// Assume that a UciOption is of type Button, and return reference to inner Button struct.
    /// Panics if UciOption is not Button.
    pub fn button_mut(&mut self) -> &mut Button {
        match self.option_type {
            UciOptionType::Button(ref mut button) => button,
            _ => panic!("option type is not button"),
        }
    }
    /// Assume that a UciOption is of type String, and return reference to inner String struct.
    /// Panics if UciOption is not String.
    pub fn string_mut(&mut self) -> &mut UciOptionString {
        match self.option_type {
            UciOptionType::String(ref mut s) => s,
            _ => panic!("option type is not String"),
        }
    }

    /// Given a RawOption, try to extract a typed value from it's stringly-typed value.
    /// The type of the parsed value must match the value of this UciOptionType value.
    /// This returns a mutable reference to self on successful update.
    pub fn try_update(&mut self, raw_opt: &RawOption) -> Result<&mut Self, &'static str> {
        (self.name == raw_opt.name)
            .then(|| ())
            .ok_or("names do not match")?;

        match self.option_type {
            UciOptionType::Check(Check { ref mut value, .. }) => {
                *value = bool::from_str(&raw_opt.value).map_err(|_| "raw value not a bool")?;
            }
            UciOptionType::Spin(Spin {
                ref mut value,
                min,
                max,
                ..
            }) => {
                let new_value =
                    i64::from_str_radix(&raw_opt.value, 10).map_err(|_| "raw value not an int")?;
                (min..=max)
                    .contains(&new_value)
                    .then(|| ())
                    .ok_or("value out of range")?;
                *value = new_value;
            }
            UciOptionType::Combo(Combo {
                ref mut value,
                ref choices,
                ..
            }) => {
                choices
                    .contains(&raw_opt.value)
                    .then(|| ())
                    .ok_or("value not a valid choice")?;
                *value = raw_opt.value.clone();
            }
            UciOptionType::Button(Button { ref mut pressed }) => *pressed = true,
            UciOptionType::String(UciOptionString { ref mut value, .. }) => {
                *value = raw_opt.value.clone()
            }
        };

        Ok(self)
    }
}

impl Display for UciOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "option name {} {}", self.name.0, self.option_type)
    }
}

/// CaselessString is a String wrapper that compares and hashes a string with
/// ignored casing and leading/trailing whitespace.
/// It retains casing for printing, and removes leading/trailing whitespace.
#[derive(Debug, Clone)]
pub struct CaselessString(String);

impl PartialEq for CaselessString {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}
impl Eq for CaselessString {}

impl Hash for CaselessString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state);
    }
}

impl PartialEq<&str> for CaselessString {
    fn eq(&self, other: &&str) -> bool {
        *self == Self::from(*other)
    }
}

impl Deref for CaselessString {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for CaselessString {
    fn from(s: &str) -> Self {
        Self(s.trim().to_string())
    }
}

/// Underlying type for UciOptions.
type OptionsMap = HashMap<CaselessString, UciOption>;

/// A HashMap wrapper for UciOption that has extra functionality for UciOption.
/// An option can only be updated with an option of equivalent type.
pub struct UciOptions(OptionsMap);

impl UciOptions {
    /// Create a new UciOptions using underlying HashMap::new().
    pub fn new() -> Self {
        Self(OptionsMap::new())
    }

    /// Insert stores a UciOption using it's name as the key and the full item as the value.
    /// It always replaces what is located in the container completely.
    /// If an item existed in the container, the item is removed and returned.
    pub fn insert(&mut self, uci_opt: UciOption) -> Option<UciOption> {
        let key = uci_opt.name.clone();
        // Remove key before inserting ensures Key capitalization is updated.
        let old_value = self.0.remove(&key);
        self.0.insert(key, uci_opt);
        old_value
    }

    /// UciOptions are uniquely defined by their name. Returns true if a key exists.
    pub fn contains<K: Into<CaselessString>>(&self, key: K) -> bool {
        let key: CaselessString = key.into();
        self.0.contains_key(&key)
    }

    /// Attempts to update a stored UciOption with the value in a RawOption.
    /// This will not create a new UciOption entry.
    /// This returns a mutable reference to the updated value in the table on successful update.
    pub fn update(&mut self, raw_opt: &RawOption) -> Result<&mut UciOption, &'static str> {
        self.0
            .get_mut(&raw_opt.name)
            .ok_or("RawOption name not a valid UciOption")?
            .try_update(&raw_opt)
    }
}

impl<K: Into<CaselessString>> Index<K> for UciOptions {
    type Output = UciOption;
    fn index(&self, key: K) -> &Self::Output {
        let key: CaselessString = key.into();
        &self.0[&key]
    }
}

impl<K: Into<CaselessString>> IndexMut<K> for UciOptions {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        let key: CaselessString = key.into();
        self.0.get_mut(&key).expect("key not present")
    }
}

impl Deref for UciOptions {
    type Target = OptionsMap;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SearchControls {
    pub wtime: Option<u32>,
    pub btime: Option<u32>,
    pub winc: Option<u32>,
    pub binc: Option<u32>,
    pub moves_to_go: Option<u32>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub mate: Option<u32>,
    pub move_time: Option<u32>,
    pub infinite: bool,
}

impl SearchControls {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for SearchControls {
    fn default() -> Self {
        Self {
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            moves_to_go: None,
            depth: None,
            nodes: None,
            mate: None,
            move_time: None,
            infinite: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::Square::*;

    /// Tests commands: uci, isready, ucinewgame, stop, ponderhit, quit
    #[test]
    fn parse_command_singles() {
        {
            let input = "uci";
            let command = UciCommand::parse_command(&input);
            assert_eq!(UciCommand::Uci, command.unwrap());
        }
        {
            let input = "isready\n";
            let command = UciCommand::parse_command(&input);
            assert_eq!(UciCommand::IsReady, command.unwrap());
        }
        {
            let input = "ucinewgame";
            let command = UciCommand::parse_command(&input);
            assert_eq!(UciCommand::UciNewGame, command.unwrap());
        }
        {
            let input = "stop";
            let command = UciCommand::parse_command(&input);
            assert_eq!(UciCommand::Stop, command.unwrap());
        }
        {
            let input = "ponderhit";
            let command = UciCommand::parse_command(&input);
            assert_eq!(UciCommand::PonderHit, command.unwrap());
        }
        {
            let input = "quit";
            let command = UciCommand::parse_command(&input);
            assert_eq!(UciCommand::Quit, command.unwrap());
        }
    }

    #[test]
    fn parse_command_debug() {
        let on = "debug on";
        let off = "debug off";
        let command_on = UciCommand::parse_command(on);
        let command_off = UciCommand::parse_command(off);
        assert_eq!(UciCommand::Debug(true), command_on.unwrap());
        assert_eq!(UciCommand::Debug(false), command_off.unwrap());
    }

    #[test]
    fn parse_command_setoption() {
        {
            let input = "setoption name Hash value 100\n";
            let command = UciCommand::parse_command(input);
            let raw_opt = RawOption {
                name: "hash".into(),
                value: String::from("100"),
            };
            assert_eq!(UciCommand::SetOption(raw_opt), command.unwrap());
        }
        {
            let input = "setoption name Multi Word Name value this is a test string.c";
            let command = UciCommand::parse_command(input);
            let raw_opt = RawOption {
                name: "Multi Word Name".into(),
                value: String::from("this is a test string.c"),
            };
            assert_eq!(UciCommand::SetOption(raw_opt), command.unwrap());
        }
        {
            let input = "setoption name Clear Hash \n";
            let command = UciCommand::parse_command(input);
            let raw_opt = RawOption {
                name: "Clear Hash".into(),
                value: String::from(""),
            };
            assert_eq!(UciCommand::SetOption(raw_opt), command.unwrap());
        }
    }

    #[test]
    fn parse_command_pos() {
        {
            // Simple start position.
            let start_position = Position::start_position();
            let command_start_str = "position startpos";
            let command_start1 = UciCommand::parse_command(command_start_str).unwrap();
            assert_eq!(UciCommand::Pos(start_position), command_start1);
        }

        {
            // Derived from applying moves to start position.
            let moves = vec![Move::new(D2, D4, None), Move::new(D7, D5, None)];
            let mut pos = Position::start_position();
            moves.into_iter().for_each(|move_| {
                pos.do_move(move_);
            });
            let command_start_moves_str = "position startpos moves d2d4 d7d5";
            let command = UciCommand::parse_command(command_start_moves_str).unwrap();
            assert_eq!(UciCommand::Pos(pos), command);
        }

        {
            // Positions derived from a fen.
            let pos_fen_str = "rnbqkbnr/pppp1ppp/8/4P3/8/8/PPP1PPPP/RNBQKBNR b KQkq - 0 2";
            let command_str =
                "position fen rnbqkbnr/pppp1ppp/8/4P3/8/8/PPP1PPPP/RNBQKBNR b KQkq - 0 2";
            let pos = Position::parse_fen(pos_fen_str).unwrap();
            let command = UciCommand::parse_command(command_str).unwrap();
            assert_eq!(UciCommand::Pos(pos), command);
        }

        {
            // Derive from a fen string with moves applied.
            let _base_fen_str = "rnbqkbnr/pppp1ppp/8/4P3/8/8/PPP1PPPP/RNBQKBNR b KQkq - 0 2";
            let post_fen_str = "rnbqkbnr/ppp2ppp/3P4/8/8/8/PPP1PPPP/RNBQKBNR b KQkq - 0 3";
            let command_str = "position fen rnbqkbnr/pppp1ppp/8/4P3/8/8/PPP1PPPP/RNBQKBNR b KQkq - 0 2 moves d7d6 e5d6";
            let pos_post = Position::parse_fen(post_fen_str).unwrap();
            let command = UciCommand::parse_command(command_str).unwrap();
            println!("pos: {}", pos_post);

            if let UciCommand::Pos(pos) = command {
                println!("com: {}", pos);
            };
            assert_eq!(UciCommand::Pos(pos_post), command);
        }
    }

    #[test]
    fn parse_command_go() {
        {
            let input = "go depth 10 wtime 40000 \n";
            let command = UciCommand::parse_command(input).unwrap();
            let mut search_ctrl = SearchControls::new();
            search_ctrl.depth = Some(10);
            search_ctrl.wtime = Some(40000);
            assert_eq!(UciCommand::Go(search_ctrl), command);
        }
    }

    #[test]
    fn ucioptions_insert_update_contains() {
        // option name Hash type spin default 1 min 1 max 16000
        // option name Clear Hash type button
        // option name Ponder type check default false
        // option name Threads type spin default 1 min 1 max 32
        let option_hash = UciOption::new_spin("Hash", 1, 1, 16000);
        let option_clear_hash = UciOption::new_button("Clear Hash", false);
        let option_ponder = UciOption::new_check("Ponder", false);
        let option_threads = UciOption::new_spin("Threads", 1, 1, 32);

        let mut uci_options = UciOptions::new();

        assert_eq!(uci_options.len(), 0);
        assert_eq!(uci_options.insert(option_hash.clone()), None);
        assert_eq!(uci_options.insert(option_clear_hash.clone()), None);
        assert_eq!(uci_options.insert(option_ponder.clone()), None);
        assert_eq!(uci_options.insert(option_threads.clone()), None);
        assert_eq!(uci_options.len(), 4);

        let raw_hash = RawOption {
            name: "hash".into(),
            value: "14".into(),
        };
        assert!(matches!(uci_options.update(&raw_hash), Ok(_)));

        assert_eq!(
            option_clear_hash,
            *uci_options.get(&"clear hash".into()).unwrap()
        );
        assert_eq!(option_ponder, *uci_options.get(&"ponder".into()).unwrap());
        assert_eq!(option_threads, *uci_options.get(&"threads".into()).unwrap());
        assert_ne!(option_hash, *uci_options.get(&"hash".into()).unwrap());
    }
}