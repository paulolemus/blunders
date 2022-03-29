//! Game structure.

use crate::error::{self, ErrorKind};
use crate::movelist::MoveHistory;
use crate::position::Position;

/// Game contains information for an in progress game:
/// The base position the game started from, the sequence of moves that were
/// played, and the current position.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Game {
    pub base_position: Position,
    pub moves: MoveHistory,
    pub position: Position,
}

impl Game {
    /// Create a new Game from a base position and a sequence of moves.
    /// This generates the current position by applying the sequence of moves to the base.
    /// If a move in the move history was illegal, Err is returned.
    pub fn new(base_position: Position, moves: MoveHistory) -> error::Result<Self> {
        let mut position = base_position;

        for move_ in &moves {
            let maybe_move_info = position.do_legal_move(*move_);
            maybe_move_info.ok_or(ErrorKind::GameIllegalMove)?;
        }

        Ok(Self {
            base_position,
            moves,
            position,
        })
    }

    /// Create a new game in the standard chess start position.
    pub fn start_position() -> Self {
        Self::from(Position::start_position())
    }
}

/// Convert a position to a Game with no past moves.
impl From<Position> for Game {
    fn from(position: Position) -> Self {
        Self::new(position, MoveHistory::new()).unwrap()
    }
}
