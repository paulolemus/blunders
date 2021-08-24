//! Shared Transposition Table.

use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::Mutex;

use crate::coretypes::{Cp, Move, MoveInfo, PlyKind};
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

/// Entry contains information about a single previously searched position.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Entry {
    /// Full hash value for a position.
    pub hash: HashKind,
    /// Type of Node this position has in search tree.
    pub node_kind: NodeKind,
    /// Best move or refutation move of position.
    pub key_move: Move,
    /// The ply/depth that was searched to in this position's subtree.
    pub ply: PlyKind,
    /// The Score in centipawns for the position.
    pub score: Cp,
}

impl Entry {
    /// Returns new Entry from provided information.
    pub fn new(
        hash: HashKind,
        node_kind: NodeKind,
        key_move: Move,
        ply: PlyKind,
        score: Cp,
    ) -> Self {
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

impl Default for Entry {
    fn default() -> Self {
        Self::illegal()
    }
}

/// Bucket holds all items that correspond to an index in the Transposition Table.
/// This bucket holds two Entries in order to allow the best of both worlds for replacement schemes:
/// 1. Replace on condition, and 2. Always replace.
///
/// A replacement scheme is provided when attempting to replace an entry in this bucket.
/// `scheme_entry` is always checked first.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Bucket {
    /// Age of `scheme_entry`. Useful for custom replacement schemes.
    pub age: u8,
    /// Entry that gets updated only if a replacement scheme is passed.
    pub scheme_entry: Entry,
    /// Entry that always gets replaced if the replacement scheme fails for `scheme_entry`.
    pub always_entry: Entry,
}

impl Bucket {
    /// Number of entries in this bucket.
    const fn len() -> usize {
        2
    }

    /// Illegal initial value.
    fn illegal() -> Self {
        Self {
            age: 0,
            scheme_entry: Entry::illegal(),
            always_entry: Entry::illegal(),
        }
    }

    /// Returns an entry if it exists in this Bucket.
    /// The scheme entry is checked before the always entry.
    /// If they both have the same hash, scheme entry is returned first.
    #[inline]
    fn get(&self, hash: HashKind) -> Option<Entry> {
        if self.scheme_entry.hash == hash {
            Some(self.scheme_entry)
        } else if self.always_entry.hash == hash {
            Some(self.always_entry)
        } else {
            None
        }
    }

    /// Returns true if any entry in this bucket contains the given hash.
    fn contains(&self, hash: HashKind) -> bool {
        self.scheme_entry.hash == hash || self.always_entry.hash == hash
    }

    /// Unconditionally replaces the `always_entry` slot.
    /// This does not affect age or scheme entry.
    #[inline]
    fn store(&mut self, always_entry: Entry) {
        self.always_entry = always_entry;
    }

    /// Unconditionally replaces the `scheme_entry` slot of this bucket,
    /// and move the old priority entry into the always slot.
    #[inline]
    fn replace(&mut self, scheme_entry: Entry, age: u8) {
        self.age = age;
        self.always_entry = self.scheme_entry;
        self.scheme_entry = scheme_entry;
    }

    /// Replaces the `scheme_entry` slot if `should_replace` returns true,
    /// otherwise the `always_entry` slot is replaced.
    ///
    /// FnOnce signature:
    ///
    /// should_replace(&new_entry, new_age, &existing_scheme_entry, existing_age) -> bool
    #[inline]
    fn replace_by<F>(&mut self, entry: Entry, age: u8, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        match should_replace(&entry, age, &self.scheme_entry, self.age) {
            true => self.replace(entry, age),
            false => self.store(entry),
        }
    }
}

impl Default for Bucket {
    fn default() -> Self {
        Self::illegal()
    }
}

/// Fill a Vector to capacity.
fn fill_with_default(v: &mut Vec<TtEntry>) {
    let capacity = v.capacity();
    while v.len() < capacity {
        v.push(Mutex::default());
    }
    debug_assert_eq!(v.len(), capacity);
    debug_assert_eq!(v.capacity(), capacity);
}

/// Converts a size in Megabytes to a capacity of inner vector.
fn mb_to_inner_capacity(mb: usize) -> usize {
    (mb * 1_000_000) / mem::size_of::<TtEntry>()
}

/// Type alias for inner type of TranspositionTable.
type TtEntry = Mutex<Bucket>;

/// A Transposition Table (tt) with a fixed size, memoizing previously evaluated
/// chess positions. The table is safely sharable between threads as immutable.
/// Slots may be updated from an immutable reference as each slot has its own lock.
///
/// The table uses a two layer system which ensures that new entries are always inserted
/// into the table while also allowing important entries to remain for as long as they need.
///
/// The first layer is the replacement scheme layer, which allows the user to decide
/// when to replace the entry based on a conditional test.
///
/// The second layer is the always replace layer, which gets replaced if the first layer does not.
///
/// Example:
/// ```rust
/// # use std::sync::Arc;
/// # use blunders_engine::transposition::{Entry, NodeKind, TranspositionTable};
/// # use blunders_engine::coretypes::{Move, Cp, Square::*};
/// let tt = Arc::new(TranspositionTable::with_capacity(100));
/// let age = 1;
/// let hash = 100;
/// let entry = Entry::new(hash, NodeKind::Pv, Move::new(D2, D4, None), 5, Cp(3));
///
/// tt.replace(entry, age);
/// assert_eq!(tt.get(hash), Some(entry));
/// ```
pub struct TranspositionTable {
    /// Number of buckets in transpositions vector.
    bucket_capacity: usize,
    /// ZobristTable used to unify all entry hashes to the same hash generator.
    ztable: ZobristTable,
    /// Bucketed vector of transpositions.
    transpositions: Vec<TtEntry>,
}

impl TranspositionTable {
    /// Number of entries table holds by default.
    const DEFAULT_MAX_ENTRIES: usize = 100_000;

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// and a pre-allocated default max entry capacity.
    pub fn new() -> Self {
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_table(Self::DEFAULT_MAX_ENTRIES, ztable)
    }

    /// Returns a reference to the zobrist table.
    pub fn zobrist_table(&self) -> &ZobristTable {
        &self.ztable
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// with given capacity pre-allocated, where capacity is the number of entries in table.
    pub fn with_capacity(entry_capacity: usize) -> Self {
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_table(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// with capacity calculated to fill given Megabytes.
    pub fn with_mb(mb: usize) -> Self {
        assert!(mb > 0);
        let bucket_capacity = mb_to_inner_capacity(mb);
        assert!(bucket_capacity > 0, "max capacity is not greater than 0");
        let entry_capacity = bucket_capacity * Bucket::len();

        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_table(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable
    /// with pre-allocated default max capacity.
    pub fn with_zobrist_table(ztable: ZobristTable) -> Self {
        let entry_capacity = Self::DEFAULT_MAX_ENTRIES;
        Self::with_capacity_and_zobrist_table(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable
    /// and capacity in entries pre-allocated.
    pub fn with_capacity_and_zobrist_table(entry_capacity: usize, ztable: ZobristTable) -> Self {
        // Add Bucket::len - 1 to guarantee minimum capacity due to integer division floor.
        let bucket_capacity = (entry_capacity + Bucket::len() - 1) / Bucket::len();

        let mut transpositions = Vec::with_capacity(bucket_capacity);
        fill_with_default(&mut transpositions);

        assert_eq!(bucket_capacity, transpositions.capacity());
        assert_eq!(bucket_capacity, transpositions.len());
        Self {
            bucket_capacity,
            ztable,
            transpositions,
        }
    }

    /// Returns the capacity of entries of the TranspositionTable.
    pub fn capacity(&self) -> usize {
        assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        self.transpositions.capacity() * Bucket::len()
    }

    /// Returns the capacity of buckets in this TranspositionTable.
    pub fn bucket_capacity(&self) -> usize {
        assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        self.bucket_capacity
    }

    /// Removes all items from TranspositionTable.
    /// Since the TT uniquely holds its inner vector, this operation is safely guarded
    /// by its signature `&mut self`, as it cannot be held by any other thread.
    pub fn clear(&mut self) {
        for slot in &mut self.transpositions {
            *slot = Mutex::default();
        }
        debug_assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.bucket_capacity, self.transpositions.len());
    }

    /// Drops original table and allocates a new table of size `new_mb`.
    /// Entries in the original table are not preserved.
    /// Returns the table's new entry capacity.
    pub fn set_mb(&mut self, new_mb: usize) -> usize {
        let bucket_capacity = mb_to_inner_capacity(new_mb);
        let entry_capacity = bucket_capacity * Bucket::len();
        let ztable = self.ztable.clone();
        *self = Self::with_capacity_and_zobrist_table(entry_capacity, ztable);
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
        (hash % self.bucket_capacity as HashKind) as usize
    }

    /// Unconditionally replace an existing item in the TranspositionTable
    /// where replace_by true would place it.
    /// Capacity of the table remains unchanged.
    pub fn replace(&self, entry: Entry, age: u8) {
        let index = self.hash_to_index(entry.hash);
        {
            self.transpositions[index]
                .lock()
                .unwrap()
                .replace(entry, age);
        }
        debug_assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.bucket_capacity, self.transpositions.len());
    }

    /// Attempt to insert an item into the tt depending on a replacement scheme.
    /// If the replacement scheme evaluates to true, the entry replaces the bucket scheme_entry.
    /// Otherwise, it is inserted into the always_entry.
    ///
    /// Closure signature: should_replace(&replacing_entry, age, &existing_entry, existing_age) -> bool.
    ///
    /// ## Example:
    /// ```rust
    /// # use blunders_engine::transposition::{Entry, NodeKind, TranspositionTable};
    /// # use blunders_engine::coretypes::{Cp, Move, Square::*};
    /// # let node_kind = NodeKind::All;
    /// # let best_move = Move::new(D2, D4, None);
    /// # let score = Cp(1);
    /// let mut tt = TranspositionTable::with_capacity(2);
    /// assert_eq!(tt.bucket_capacity(), 1); // All hashes index same bucket.
    /// let age = 1;
    ///
    /// let deep_hash = 0;
    /// let deep_ply = 10;
    /// let deep_entry = Entry::new(deep_hash, node_kind, best_move, deep_ply, score);
    ///
    /// let shallow_hash = 8;
    /// let shallow_ply = 2;
    /// let shallow_entry = Entry::new(shallow_hash, node_kind, best_move, shallow_ply, score);
    ///
    /// fn replacement_scheme(entry: &Entry, age: u8, existing: &Entry, existing_age: u8) -> bool {
    ///     age != existing_age || entry.ply > existing.ply
    /// }
    ///
    /// // Hash slot starts empty, so tt_entry replaces priority slot.
    /// tt.replace_by(deep_entry, age, replacement_scheme);
    /// assert_eq!(tt.get(deep_hash).unwrap(), deep_entry);
    ///
    /// // Shallow entry does not pass replacement test, so it is placed in always slot.
    /// tt.replace_by(shallow_entry, age, replacement_scheme);
    /// assert_eq!(tt.get(deep_hash).unwrap(), deep_entry);
    /// assert_eq!(tt.get(shallow_hash).unwrap(), shallow_entry);
    ///
    /// let other_hash = 101;
    /// let other_ply = 1;
    /// let other_entry = Entry::new(other_hash, node_kind, best_move, other_ply, score);
    ///
    /// // Other entry does not pass test for priority, so it replaces the always slot.
    /// tt.replace_by(other_entry, age, replacement_scheme);
    /// assert_eq!(tt.get(shallow_hash), None);
    /// assert_eq!(tt.get(deep_hash).unwrap(), deep_entry);
    /// assert_eq!(tt.get(other_hash).unwrap(), other_entry);
    ///
    pub fn replace_by<F>(&self, entry: Entry, age: u8, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let index = self.hash_to_index(entry.hash);

        {
            self.transpositions[index]
                .lock()
                .unwrap()
                .replace_by(entry, age, should_replace);
        }
        debug_assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.bucket_capacity, self.transpositions.len());
    }

    /// Store the entry into the index bucket's always replace slot, without changing age or scheme slot.
    pub fn store(&self, entry: Entry) {
        let index = self.hash_to_index(entry.hash);
        {
            self.transpositions[index].lock().unwrap().store(entry);
        }
    }

    /// Returns true if a TranspositionTable bucket contains an entry with the given hash.
    /// Key collisions are expected to be rare but possible,
    /// so care should be taken with the return value.
    pub fn contains(&self, hash: HashKind) -> bool {
        let index = self.hash_to_index(hash);
        { *self.transpositions[index].lock().unwrap() }.contains(hash)
    }

    /// Returns Entry if hash exists in the indexed bucket, None otherwise.
    pub fn get(&self, hash: HashKind) -> Option<Entry> {
        let index = self.hash_to_index(hash);
        let bucket: Bucket = { *self.transpositions[index].lock().unwrap() };
        bucket.get(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::Square::*;

    #[test]
    fn size_of_requirements() {
        // Want a single entry to fit into L1 cache line?
        // Need to verify that this is how this works, not sure since Mutex is used.
        use std::mem::size_of;
        let size = size_of::<TtEntry>();
        println!("size_of::<TtEntry>() = {}", size);
        //assert!(size <= 64);
    }

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

        tt.store(tt_entry);
        assert!(tt.contains(hash));
    }

    #[test]
    fn tt_single_capacity_replaces() {
        let tt = TranspositionTable::with_capacity(1);
        let age = 1;
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
        tt.replace(tt_entry1, age);
        assert!(tt.contains(tt_entry1.hash));
        assert!(!tt.contains(tt_entry2.hash));
        assert_eq!(tt.get(tt_entry1.hash), Some(tt_entry1));
        assert_eq!(tt.get(tt_entry2.hash), None);

        // Replaces previous item in index priority slot, should move to always slot.
        tt.replace(tt_entry2, age);
        assert!(tt.contains(tt_entry1.hash));
        assert!(tt.contains(tt_entry2.hash));
        assert_eq!(tt.get(tt_entry1.hash), Some(tt_entry1));
        assert_eq!(tt.get(tt_entry2.hash), Some(tt_entry2));
    }

    #[test]
    fn tt_start_position() {
        let tt = TranspositionTable::with_capacity(10000);
        let pos = Position::start_position();
        let hash = tt.generate_hash(&pos);
        let age = 1;
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
        tt.replace(tt_entry, age);
        assert!(tt.contains(hash));
        assert_eq!(tt.get(hash), Some(tt_entry));
    }
}
