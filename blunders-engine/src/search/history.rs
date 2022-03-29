//! History structure used within search.

use arrayvec::ArrayVec;

use crate::coretypes::MAX_HISTORY;
use crate::position::Game;
use crate::zobrist::{HashKind, ZobristTable};

type HashHistory = ArrayVec<HashKind, MAX_HISTORY>;
type Unrepeatables = ArrayVec<usize, MAX_HISTORY>;

/// History primary use is for tracking repeated moves to prevent threefold repetition.
/// It is stateful, in that functions assume the next interaction comes from the next
/// possible move in a played game.
///
/// It contains the hashes of all previously visited positions,
/// and the indices of positions which cannot be repeated in future positions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct History {
    hash_history: HashHistory,    // All visited position hashes in order.
    unrepeatables: Unrepeatables, // Stack of unrepeatable position indices.
    head: usize,                  // Most recent unrepeatable position.
}

impl History {
    /// Create a new empty History.
    pub fn empty() -> Self {
        Self {
            hash_history: HashHistory::new(),
            unrepeatables: Unrepeatables::new(),
            head: 0,
        }
    }
    /// Create a new History from a game and a Zobrist Table.
    pub fn new(game: &Game, ztable: &ZobristTable) -> Self {
        let mut history = Self::empty();
        let mut position = game.base_position;

        // Only push a move when it is in the past (original hash after a move is applied).
        // The final (current) move is not added to history because it is active.
        for move_ in &game.moves {
            let hash = ztable.generate_hash((&position).into());
            let move_info = position.do_legal_move(*move_).expect("move not legal");

            history.push(hash, move_info.is_unrepeatable());
        }

        debug_assert_eq!(position, game.position);
        history
    }

    /// Pushes a new position into the hash history, and updates the most recent unrepeatable
    /// index if applicable.
    pub fn push(&mut self, hash: HashKind, is_unrepeatable: bool) {
        self.hash_history.push(hash);

        if is_unrepeatable {
            self.unrepeatables.push(self.head);
            self.head = self.hash_history.len().saturating_sub(1);
        }
    }

    /// Pops a position from history stack. If the popped item was the most recent unrepeatable,
    /// then replace it with the previous unrepeatable index.
    pub fn pop(&mut self) {
        self.hash_history.pop();

        // If the current head exceeds the limit, replace it with the previous unrepeatable index.
        if self.head >= self.hash_history.len() {
            self.head = self.unrepeatables.pop().unwrap_or(0);
        }
    }

    /// Returns true if the position occurs at least once in history.
    /// This is done by only checking the history from the last unrepeatable index to the most recent entry.
    /// All positions before the index cannot reoccur in the next sequence.
    pub fn contains(&self, hash: HashKind) -> bool {
        self.contains_n(hash, 1)
    }

    /// Returns true if the position occurs in history at least `n` times,
    /// assuming the position to check may be the next move in this game's history.
    pub fn contains_n(&self, hash: HashKind, count: usize) -> bool {
        let mut counter = 0;
        self.hash_history[self.head..].iter().rev().any(|old_hash| {
            if *old_hash == hash {
                counter += 1;
                if counter >= count {
                    return true;
                }
            }
            false
        })
    }

    /// Returns true if the position occurs twice in history, indicating that the given
    /// position is the second repetition (position occurs total of three times).
    pub fn is_threefold_repetition(&self, hash: HashKind) -> bool {
        self.contains_n(hash, 2)
    }

    /// Returns true if the position occurs once in history, indicating that
    /// the given position is the first repetition (position occurs total of two times).
    pub fn is_twofold_repetition(&self, hash: HashKind) -> bool {
        self.contains(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;

    #[test]
    fn position_with_no_history() {
        let ztable = ZobristTable::new();
        let game = Game::from(Position::start_position());

        let history = History::new(&game, &ztable);

        assert_eq!(history.head, 0);
        assert_eq!(history.hash_history.len(), 0);
        assert_eq!(history.unrepeatables.len(), 0);
    }
}
