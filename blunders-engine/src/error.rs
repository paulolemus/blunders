//! Blunders Engine error type.

use std::error;
use std::fmt::{self, Display};
use std::result;

use crate::fen::ParseFenError;

/// Blunders Engine generic result type.
pub type Result<T> = result::Result<T, Error>;

/// A list specifying general errors for Blunders engine.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An argument was expected following a string key, but none was provided.
    UciNoArgument,
    /// Uci failed to parse an integer type.
    UciCannotParseInt,
    /// Uci received an unsupported option.
    UciInvalidOption,
    /// Uci received an unknown command.
    UciUnknownCommand,
    /// Uci received no command string.
    UciNoCommand,
    /// Uci debug missing mode.
    UciDebugNoMode,
    /// Uci debug illegal mode.
    UciDebugIllegalMode,
    /// No name provided for Uci setoption command.
    UciSetOptionNoName,
    /// Uci position command malformed.
    UciPositionMalformed,
    /// Uci position command given illegal move.
    UciPositionIllegalMove,
    /// Uci Option fails to update.
    UciOptionCannotUpdate,
    /// Fen error kinds.
    Fen,

    /// Square parse string malformed.
    ParseSquareMalformed,
    /// File parse string malformed.
    ParseFileMalformed,
    /// Rank parse string malformed.
    ParseRankMalformed,
    /// Color parse string malformed.
    ParseColorMalformed,
    /// Piece parse string malformed.
    ParsePieceMalformed,
    /// Piece parse string malformed.
    ParseCastlingMalformed,

    /// Time Management Mode cannot be created, missing fields.
    ModeNotSatisfied,

    /// The engine can only play games with a finite static number of moves.
    /// That limit has been exceeded.
    MoveHistoryExceeded,

    /// Engine's transposition table is being referenced from another thread.
    EngineTranspositionTableInUse,
    /// Engine is currently searching, so another search cannot be started.
    EngineAlreadySearching,

    // An illegal move was provided, and could not be applied to some base position.
    GameIllegalMove,
}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::UciNoArgument => "uci no argument",
            ErrorKind::UciCannotParseInt => "uci cannot parse integer",
            ErrorKind::UciInvalidOption => "uci invalid option",
            ErrorKind::UciUnknownCommand => "uci unknown command",
            ErrorKind::UciNoCommand => "uci no command",
            ErrorKind::UciDebugNoMode => "uci debug no mode",
            ErrorKind::UciDebugIllegalMode => "uci debug illegal mode",
            ErrorKind::UciSetOptionNoName => "uci setoption no name",
            ErrorKind::UciPositionMalformed => "uci position malformed",
            ErrorKind::UciPositionIllegalMove => "uci position illegal move",
            ErrorKind::UciOptionCannotUpdate => "uci option cannot update",
            ErrorKind::Fen => "fen",

            ErrorKind::ParseSquareMalformed => "parse square malformed",
            ErrorKind::ParseFileMalformed => "parse file malformed",
            ErrorKind::ParseRankMalformed => "parse rank malformed",
            ErrorKind::ParseColorMalformed => "parse color malformed",
            ErrorKind::ParsePieceMalformed => "parse piece malformed",
            ErrorKind::ParseCastlingMalformed => "parse castling malformed",

            ErrorKind::ModeNotSatisfied => "mode not satisfied",

            ErrorKind::MoveHistoryExceeded => "move history exceeded",

            ErrorKind::EngineTranspositionTableInUse => "engine transposition table in use",
            ErrorKind::EngineAlreadySearching => "engine already searching",

            ErrorKind::GameIllegalMove => "position history illegal move",
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The primary and general error type for the Blunders Engine.
#[derive(Debug)]
pub enum Error {
    Simple(ErrorKind),
    Message(ErrorKind, String),
    Custom(ErrorKind, Box<dyn error::Error + Send + Sync>),
}

impl Error {
    pub fn new<E>(error_kind: ErrorKind, inner_error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::Custom(error_kind, inner_error.into())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Simple(error_kind) => {
                write!(f, "{error_kind}")
            }
            Error::Message(error_kind, string) => {
                write!(f, "{error_kind}: {string}")
            }
            Error::Custom(error_kind, ref box_error) => {
                write!(f, "{error_kind}, error: {}", *box_error)
            }
        }
    }
}

impl error::Error for Error {}

impl From<ErrorKind> for Error {
    fn from(error_kind: ErrorKind) -> Self {
        Self::Simple(error_kind)
    }
}

impl From<ParseFenError> for Error {
    fn from(error: ParseFenError) -> Self {
        Self::Custom(ErrorKind::Fen, error.into())
    }
}

impl<S: ToString> From<(ErrorKind, S)> for Error {
    fn from((error_kind, stringable): (ErrorKind, S)) -> Self {
        Self::Message(error_kind, stringable.to_string())
    }
}
