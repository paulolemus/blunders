//! Transposition Table.

use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::Mutex;

use crate::coretypes::{Cp, Move, MoveInfo};
use crate::position::{Cache, Position};
use crate::zobrist::{HashKind, ZobristTable};

/// The type of a node in a search tree.
/// See [Node Types](https://www.chessprogramming.org/Node_Types).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum NodeKind {
    All, // An All node has had all of its children searched.
    Cut, // A Cut node, or a node that was pruned because it caused a beta-cutoff.
    Pv,  // A principal variation node from a previous search.
}

/// Entry contains information about a previously searched position.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Entry {
    pub hash: HashKind,      // Full hash value for a position.
    pub node_kind: NodeKind, // Type of Node this position has in search tree.
    pub key_move: Move,      // Best move or refutation move.
    pub ply: u32,            // The depth searched to in this Position's subtree.
    pub score: Cp,           // Score in centipawns for hashed position.
}

impl Entry {
    /// Returns new Entry from provided information.
    pub fn new(hash: HashKind, node_kind: NodeKind, key_move: Move, ply: u32, score: Cp) -> Self {
        Self {
            hash,
            node_kind,
            key_move,
            ply,
            score,
        }
    }

    /// Returns a new Entry with illegal information.
    pub fn illegal() -> Self {
        Self {
            hash: 0,
            node_kind: NodeKind::All,
            key_move: Move::illegal(),
            ply: 0,
            score: Cp(0),
        }
    }
}

impl Hash for Entry {
    fn hash<H: Hasher>(&self, h: &mut H) {
        h.write_u64(self.hash)
    }
}

/// Fill a Vector to capacity.
fn fill_with_illegal(v: &mut Vec<TtEntry>) {
    let capacity = v.capacity();
    while v.len() < capacity {
        v.push(Mutex::new(Entry::illegal()));
    }
    debug_assert_eq!(v.len(), capacity);
    debug_assert_eq!(v.capacity(), capacity);
}

/// Converts a size in Megabytes to a capacity.
fn mb_to_capacity(mb: usize) -> usize {
    (mb * 1_000_000) / mem::size_of::<TtEntry>()
}

/// Type alias for inner type of TranspositionTable.
type TtEntry = Mutex<Entry>;

/// A Transposition Table (tt) with a fixed size, memoizing previously evaluated
/// chess positions. The table is safely sharable between threads as immutable.
/// Slots may be updated from an immutable reference as each slot has its own lock.
///
/// Example:
/// ```rust
/// # use std::sync::Arc;
/// # use blunders_engine::transposition::TranspositionTable;
/// # use blunders_engine::transposition::Entry;
/// let tt = Arc::new(TranspositionTable::with_capacity(100));
/// let tt_entry = Entry::illegal();
///
/// tt.replace(tt_entry);
/// ```
/// There are some notable differences in behavior between TranspositionTable
/// and std::collections::{HashMap, HashSet}.
/// TT only cares about the hash value. It does check for equivalence of provided position.
/// Index collisions are avoided, Key collisions are not.
///
/// Fields:
/// * ztable -> The random number table which is used for generating and updating hashes.
/// * transpositions -> The vector which contains position history.
pub struct TranspositionTable {
    max_capacity: usize,
    ztable: ZobristTable,
    transpositions: Vec<TtEntry>,
}

impl TranspositionTable {
    const DEFAULT_MAX_CAPACITY: usize = 100_000;

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// and a pre-allocated default max capacity.
    pub fn new() -> Self {
        let max_capacity = Self::DEFAULT_MAX_CAPACITY;
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_table(max_capacity, ztable)
    }

    /// Returns a reference to the zobrist table.
    pub fn zobrist_table(&self) -> &ZobristTable {
        &self.ztable
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// with given capacity pre-allocated.
    pub fn with_capacity(max_capacity: usize) -> Self {
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_table(max_capacity, ztable)
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// with capacity calculated to fill given Megabytes.
    pub fn with_mb(mb: usize) -> Self {
        assert!(mb > 0);
        let max_capacity = mb_to_capacity(mb);
        assert!(max_capacity > 0, "max capacity is not greater than 0");

        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_table(max_capacity, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable
    /// with pre-allocated default max capacity.
    pub fn with_zobrist_table(ztable: ZobristTable) -> Self {
        let max_capacity = Self::DEFAULT_MAX_CAPACITY;
        Self::with_capacity_and_zobrist_table(max_capacity, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable
    /// and capacity pre-allocated.
    pub fn with_capacity_and_zobrist_table(max_capacity: usize, ztable: ZobristTable) -> Self {
        let mut transpositions = Vec::with_capacity(max_capacity);
        fill_with_illegal(&mut transpositions);

        Self {
            max_capacity,
            ztable,
            transpositions,
        }
    }

    /// Returns the capacity of the TranspositionTable.
    pub fn capacity(&self) -> usize {
        assert_eq!(self.max_capacity, self.transpositions.capacity());
        self.transpositions.capacity()
    }

    /// Removes all items from TranspositionTable.
    /// Since the TT uniquely holds its inner vector, this operation is safely guarded
    /// by its signature `&mut self`, as it cannot be held by any other thread.
    pub fn clear(&mut self) {
        for slot in &mut self.transpositions {
            *slot = Mutex::new(Entry::illegal());
        }
        debug_assert_eq!(self.max_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.max_capacity, self.transpositions.len());
    }

    /// Drops original table and allocates a new table of size `new_mb`.
    /// Entries in the original table are not preserved.
    /// Returns the table's new capacity.
    pub fn set_mb(&mut self, new_mb: usize) -> usize {
        let max_capacity = mb_to_capacity(new_mb);
        let ztable = self.ztable.clone();
        *self = Self::with_capacity_and_zobrist_table(max_capacity, ztable);
        self.capacity()
    }

    /// Generate a hash for a Position with context to this TranspositionTable.
    /// Hashes used for this table must be generated from it's context, because a hash for
    /// any position are likely to be different between different TranspositionTables.
    pub fn generate_hash(&self, position: &Position) -> HashKind {
        self.ztable.generate_hash(position.into())
    }

    /// Update hash for the application of a Move on Position.
    pub fn update_hash(
        &self,
        hash: &mut HashKind,
        position: &Position,
        move_info: MoveInfo,
        cache: Cache,
    ) {
        self.ztable
            .update_hash(hash, position.into(), move_info, cache);
    }

    /// Generate a new hash from a Move applied to an existing Hash and Position.
    pub fn update_from_hash(
        &self,
        mut hash: HashKind,
        position: &Position,
        move_info: MoveInfo,
        cache: Cache,
    ) -> HashKind {
        self.ztable
            .update_hash(&mut hash, position.into(), move_info, cache);
        hash
    }

    /// Convert a full hash to an index for this TranspositionTable.
    pub fn hash_to_index(&self, hash: HashKind) -> usize {
        (hash % self.max_capacity as HashKind) as usize
    }

    /// Inserts an item into the TranspositionTable without increasing capacity.
    /// It unconditionally replaces any item that already exists at the hash index.
    pub fn replace(&self, tt_entry: Entry) {
        let index = self.hash_to_index(tt_entry.hash);
        {
            *self.transpositions[index].lock().unwrap() = tt_entry;
        }
        debug_assert_eq!(self.max_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.max_capacity, self.transpositions.len());
    }

    // TODO:
    // FIGURE OUT REPLACEMENT STRATEGY
    // One option would be to include a marker saying if node is exact, pv, or cut,
    // and using that and depth to determine if should replace.

    /// Attempt to insert an item into the tt depending on a replacement strategy.
    /// tt_entry is inserted if the hash index is empty.
    /// Otherwise, it inserts using the provided closure returns true.
    ///
    /// Closure signature: should_replace(&replacing_item, &slotted_item) -> bool.
    ///
    /// ## Example:
    /// ```rust
    /// # use blunders_engine::transposition::TranspositionTable;
    /// # use blunders_engine::transposition::Entry;
    /// # use blunders_engine::transposition::NodeKind;
    /// # use blunders_engine::coretypes::Cp;
    /// # use blunders_engine::coretypes::{Move, Square::*};
    /// # let mut tt = TranspositionTable::new();
    /// let hash = 0;
    /// let tt_entry = Entry::new(hash, NodeKind::All, Move::new(D2, D4, None), 3, Cp(1));
    ///
    /// let mut tt_entry_ignored = tt_entry.clone();
    /// tt_entry_ignored.score = Cp(0);
    /// let mut tt_entry_replaced = tt_entry.clone();
    /// tt_entry_replaced.score = Cp(10);
    ///
    /// // Hash slot starts empty, so tt_entry is inserted.
    /// tt.replace_by(tt_entry, |replacing, slotted| replacing.score >= slotted.score);
    /// assert_eq!(tt.get(hash).unwrap(), tt_entry);
    ///
    /// // Hash slot is full, and closure does not return true, so item is not replaced.
    /// tt.replace_by(tt_entry_ignored, |replacing, slotted| replacing.score >= slotted.score);
    /// assert_eq!(tt.get(hash).unwrap(), tt_entry);
    /// assert_ne!(tt.get(hash).unwrap(), tt_entry_ignored);
    ///
    /// // Hash slot is full, and closure does returns true, so item is replaced.
    /// tt.replace_by(tt_entry_replaced, |replacing, slotted| replacing.score >= slotted.score);
    /// assert_ne!(tt.get(hash).unwrap(), tt_entry);
    /// assert_eq!(tt.get(hash).unwrap(), tt_entry_replaced);
    ///
    pub fn replace_by<F>(&self, tt_entry: Entry, should_replace: F)
    where
        F: FnOnce(&Entry, &Entry) -> bool,
    {
        let index = self.hash_to_index(tt_entry.hash);

        {
            let mut lock = self.transpositions[index].lock().unwrap();
            if should_replace(&tt_entry, &lock) {
                *lock = tt_entry;
            }
        }
        debug_assert_eq!(self.max_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.max_capacity, self.transpositions.len());
    }

    /// Returns true if TranspositionTable contains a given hash. This does not
    /// cover Key collisions resulting in identical hashes from the same Position.
    pub fn contains(&self, hash: HashKind) -> bool {
        let index = self.hash_to_index(hash);
        {
            self.transpositions[index].lock().unwrap().hash == hash
        }
    }

    /// Returns Entry if hash exists in container, None otherwise.
    pub fn get(&self, hash: HashKind) -> Option<Entry> {
        let index = self.hash_to_index(hash);
        let entry = { *self.transpositions[index].lock().unwrap() };

        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::Square::*;

    #[test]
    fn new_tt_no_panic() {
        let hash: HashKind = 100;
        let tt = TranspositionTable::new();
        let tt_entry = Entry {
            hash,
            node_kind: NodeKind::All,
            key_move: Move::new(A2, A3, None),
            ply: 3,
            score: Cp(100),
        };

        tt.replace(tt_entry);
        assert!(tt.contains(hash));
    }

    #[test]
    fn tt_single_capacity_replaces() {
        let tt = TranspositionTable::with_capacity(1);
        let tt_entry1 = Entry {
            hash: 100,
            node_kind: NodeKind::All,
            key_move: Move::new(A2, A3, None),
            ply: 3,
            score: Cp(100),
        };
        let tt_entry2 = Entry {
            hash: 200,
            node_kind: NodeKind::All,
            key_move: Move::new(B5, B3, None),
            ply: 4,
            score: Cp(-200),
        };

        // Starts empty.
        assert!(!tt.contains(tt_entry1.hash));
        assert!(!tt.contains(tt_entry2.hash));
        assert_eq!(tt.get(tt_entry1.hash), None);
        assert_eq!(tt.get(tt_entry2.hash), None);

        // Inserts one item correctly.
        tt.replace(tt_entry1);
        assert!(tt.contains(tt_entry1.hash));
        assert!(!tt.contains(tt_entry2.hash));
        assert_eq!(tt.get(tt_entry1.hash), Some(tt_entry1));
        assert_eq!(tt.get(tt_entry2.hash), None);

        // Replaces previous item in index.
        tt.replace(tt_entry2);
        assert!(!tt.contains(tt_entry1.hash));
        assert!(tt.contains(tt_entry2.hash));
        assert_eq!(tt.get(tt_entry1.hash), None);
        assert_eq!(tt.get(tt_entry2.hash), Some(tt_entry2));
    }

    #[test]
    fn tt_start_position() {
        let tt = TranspositionTable::with_capacity(10000);
        let pos = Position::start_position();
        let hash = tt.generate_hash(&pos);
        let tt_entry = Entry {
            hash,
            node_kind: NodeKind::All,
            key_move: Move::new(D2, D4, None),
            ply: 5,
            score: Cp(0),
        };

        // Starts without Entry.
        assert!(!tt.contains(hash));
        assert_eq!(tt.get(hash), None);

        // Finds correct Entry from large table.
        tt.replace(tt_entry);
        assert!(tt.contains(hash));
        assert_eq!(tt.get(hash), Some(tt_entry));
    }
}
