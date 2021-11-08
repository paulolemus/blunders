//! Shared Transposition Table.

use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::coretypes::{Cp, Move, MoveInfo, PieceKind::*, PlyKind, Square};
use crate::position::{Cache, Position};
use crate::zobrist::{HashKind, ZobristTable};

/// The type of a node in a search tree.
/// See [Node Types](https://www.chessprogramming.org/Node_Types).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum NodeKind {
    /// An All node has had all of its children searched.
    All,
    /// A Cut node, or a node that was pruned because it caused a beta-cutoff.
    Cut,
    /// A principal variation node from a previous search.
    Pv,
}

impl TryFrom<u8> for NodeKind {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const ALL: u8 = NodeKind::All as u8;
        const CUT: u8 = NodeKind::Cut as u8;
        const PV: u8 = NodeKind::Pv as u8;

        match value {
            ALL => Ok(NodeKind::All),
            CUT => Ok(NodeKind::Cut),
            PV => Ok(NodeKind::Pv),
            _ => Err(()),
        }
    }
}

/// Entry contains information about a single previously searched position.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Entry {
    /// Full hash value for a position.
    pub hash: HashKind,
    /// Best move or refutation move of position.
    pub key_move: Move,
    /// The Score in centipawns for the position.
    pub score: Cp,
    /// The ply/depth that was searched to in this position's subtree.
    pub ply: PlyKind,
    /// Type of Node this position has in search tree.
    pub node_kind: NodeKind,
}

impl Entry {
    /// Returns new Entry from provided information.
    pub fn new(
        hash: HashKind,
        key_move: Move,
        score: Cp,
        ply: PlyKind,
        node_kind: NodeKind,
    ) -> Self {
        Self {
            hash,
            key_move,
            score,
            ply,
            node_kind,
        }
    }

    /// Returns a new Entry with illegal information.
    pub fn illegal() -> Self {
        Self {
            hash: 0,
            key_move: Move::illegal(),
            score: Cp(0),
            ply: 0,
            node_kind: NodeKind::All,
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

/// Transposition Table Bucket that holds 2 entries,
/// consisting of a priority slot and a general slot.
pub trait TwoBucket: Debug + Default + Sync {
    /// The number of entries held by this bucket.
    fn len() -> usize {
        2
    }

    /// Returns an entry if its corresponding hash exists in this bucket.
    /// If no entry's hash matches the given hash, returns None.
    fn get(&self, hash: HashKind) -> Option<Entry>;

    /// Returns true if this bucket has any entry which contains the given hash.
    fn contains(&self, hash: HashKind) -> bool;

    /// Unconditionally store the entry in the general slot, without updating age.
    fn store(&self, general_entry: Entry);

    /// Unconditionally place the entry in the priority slot and update age.
    fn replace(&self, priority_entry: Entry, age: u8);

    /// Move the existing priority entry to the general slot,
    /// then place the new priority entry into the priority slot and update age.
    fn swap_replace(&self, priority_entry: Entry, age: u8);

    /// Replaces the `priority` slot if `should_replace` returns true,
    /// otherwise the `general` slot is replaced.
    ///
    /// # Example:
    /// if should_replace {
    ///     priority := entry
    /// } else {
    ///     general := entry
    /// }
    ///
    /// FnOnce signature:
    ///
    /// should_replace(&new_entry, new_age, &existing_priority_entry, existing_age) -> bool
    fn replace_by<F>(&self, entry: Entry, age: u8, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool;

    /// If should_replace returns true, then swap_replace with the given entry.
    ///
    /// Example:
    ///
    /// if should_replace {
    ///     general := priority
    ///     priority := entry
    /// } else {
    ///     general := entry
    /// }
    ///
    /// FnOnce signature:
    ///
    /// should_replace(&new_entry, new_age, &existing_priority_entry, existing_age) -> bool
    fn swap_replace_by<F>(&self, entry: Entry, age: u8, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool;
}

/// Dummy Bucket holds no data.
/// This is useful for running a search effectively without a transposition table.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct DummyBucket;

impl TwoBucket for DummyBucket {
    fn get(&self, _hash: HashKind) -> Option<Entry> {
        None
    }
    fn contains(&self, _hash: HashKind) -> bool {
        false
    }
    fn store(&self, _general_entry: Entry) {}
    fn replace(&self, _priority_entry: Entry, _age: u8) {}
    fn swap_replace(&self, _priority_entry: Entry, _age: u8) {}
    fn replace_by<F>(&self, _entry: Entry, _age: u8, _should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
    }
    fn swap_replace_by<F>(&self, _entry: Entry, _age: u8, _should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
    }
}

/// Age type alias used for age of a Priority Entry.
pub type AgeKind = u8;

/// Bucket holds all items that correspond to an index in the Transposition Table.
/// This bucket holds two Entries in order to allow the best of both worlds for replacement schemes:
/// 1. Replace on condition, and 2. Always replace.
///
/// A replacement scheme is provided when attempting to replace an entry in this bucket.
/// `scheme_entry` is always checked first.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct LockInner {
    /// Entry that gets updated only if a replacement scheme is passed.
    pub priority: Entry,
    /// Entry that always gets replaced if the replacement scheme fails for `scheme_entry`.
    pub general: Entry,
    /// Age of `scheme_entry`. Useful for custom replacement schemes.
    pub age: AgeKind,
}

impl LockInner {
    /// Replace priority slot with new priority entry and update age.
    #[inline]
    fn inner_replace(&mut self, priority_entry: Entry, age: AgeKind) {
        self.priority = priority_entry;
        self.age = age;
    }

    #[inline]
    fn inner_swap_replace(&mut self, priority_entry: Entry, age: AgeKind) {
        self.general = mem::replace(&mut self.priority, priority_entry);
        self.age = age;
    }

    #[inline]
    fn inner_store(&mut self, general_entry: Entry) {
        self.general = general_entry;
    }
}

/// Bucket implemented with a Mutex for sync and lock.
#[derive(Debug)]
pub struct LockBucket {
    mu: Mutex<LockInner>,
}

impl LockBucket {
    /// Illegal initial value.
    fn illegal() -> Self {
        LockBucket {
            mu: Mutex::new(LockInner {
                age: 0,
                priority: Entry::illegal(),
                general: Entry::illegal(),
            }),
        }
    }
}

impl Default for LockBucket {
    fn default() -> Self {
        Self::illegal()
    }
}

impl TwoBucket for LockBucket {
    #[inline]
    fn get(&self, hash: HashKind) -> Option<Entry> {
        let inner: LockInner = { *self.mu.lock().unwrap() };

        if inner.priority.hash == hash {
            Some(inner.priority)
        } else if inner.general.hash == hash {
            Some(inner.general)
        } else {
            None
        }
    }

    #[inline]
    fn contains(&self, hash: HashKind) -> bool {
        let (priority_hash, general_hash) = {
            let lock = self.mu.lock().unwrap();
            (lock.priority.hash, lock.general.hash)
        };
        priority_hash == hash || general_hash == hash
    }

    #[inline]
    fn store(&self, general_entry: Entry) {
        let mut lock = self.mu.lock().unwrap();
        lock.inner_store(general_entry);
    }

    #[inline]
    fn replace(&self, priority_entry: Entry, age: AgeKind) {
        let mut lock = self.mu.lock().unwrap();
        lock.inner_replace(priority_entry, age);
    }

    #[inline]
    fn swap_replace(&self, priority_entry: Entry, age: AgeKind) {
        let mut lock = self.mu.lock().unwrap();
        lock.inner_swap_replace(priority_entry, age);
    }

    #[inline]
    fn replace_by<F>(&self, entry: Entry, age: AgeKind, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let mut lock = self.mu.lock().unwrap();
        match should_replace(&entry, age, &lock.priority, lock.age) {
            true => lock.inner_replace(entry, age),
            false => lock.inner_store(entry),
        };
    }

    #[inline]
    fn swap_replace_by<F>(&self, entry: Entry, age: AgeKind, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let mut lock = self.mu.lock().unwrap();
        match should_replace(&entry, age, &lock.priority, lock.age) {
            true => lock.inner_swap_replace(entry, age),
            false => lock.inner_store(entry),
        };
    }
}

/// Aligned Packed Data Format for AtomicEntry.
///
/// # Current Data Format Sizes
/// key_move: u24, 3/8
/// score: i16, 2/8
/// ply: u8, 1/8
/// node_kind: u8, 1/8
/// optional_age: u8, 1/8
///
/// # Move Serialization
/// from: u8, 1/8
/// to: u8, 1/8
/// promotion: match u8, 1/8
///
/// # Current Data Format Packed U64
/// Hex F = 0b1111
/// u64 hex: FFFFFFFFFFFFFFFF
///
/// age <- node_kind <- ply <- score <- key_move =
/// age <- node_kind <- ply <- score <- promotion, to, from
#[rustfmt::skip]
#[allow(dead_code)] // For assertion bytes.
mod adf {
    pub const FROM_MASK: u64      = 0x00000000000000FF;
    pub const TO_MASK: u64        = 0x000000000000FF00;
    pub const PROMOTION_MASK: u64 = 0x0000000000FF0000;
    pub const SCORE_MASK: u64     = 0x000000FFFF000000;
    pub const PLY_MASK: u64       = 0x0000FF0000000000;
    pub const NODE_KIND_MASK: u64 = 0x00FF000000000000;
    pub const AGE_MASK: u64       = 0xFF00000000000000;

    pub const FROM_SHIFT: u8      = 0;
    pub const TO_SHIFT: u8        = 8;
    pub const PROMOTION_SHIFT: u8 = 16;
    pub const SCORE_SHIFT: u8     = 24;
    pub const PLY_SHIFT: u8       = 40;
    pub const NODE_KIND_SHIFT: u8 = 48;
    pub const AGE_SHIFT: u8       = 56;

    pub const FROM_BYTES: usize           = 1;
    pub const TO_BYTES: usize             = 1;
    pub const PROMOTION_BYTES: usize      = 1;
    pub const SCORE_BYTES: usize          = 2;
    pub const PLY_BYTES: usize            = 1;
    pub const NODE_KIND_BYTES: usize      = 1;
    pub const AGE_BYTES: usize            = 1;
    pub const OPT_PIECE_KIND_BYTES: usize = 1;
    pub const SQUARE_BYTES: usize         = 1;
}

/// AtomicEntry holds an Entry without an age in a unique format: As 2 AtomicU64 integers.
/// Importantly, the only data that can be corrupted from an entry is its hash.
#[derive(Debug)]
pub struct AtomicEntry {
    /// All the data from an Entry excluding its hash, packed into a single u64.
    data: AtomicU64,
    /// The hash of an Entry, XORed with the u64 representation of the rest of its data.
    hash_xor_data: AtomicU64,
}

impl AtomicEntry {
    /// Atomically load (read from) all fields of AtomicEntry.
    fn load(&self, ordering: Ordering) -> LoadedAtomicEntry {
        LoadedAtomicEntry {
            data: self.data.load(ordering),
            hash_xor_data: self.hash_xor_data.load(ordering),
        }
    }

    /// Atomically store (write to) all fields of AtomicEntry.
    fn store(&self, loaded_entry: LoadedAtomicEntry, ordering: Ordering) {
        self.data.store(loaded_entry.data, ordering);
        self.hash_xor_data
            .store(loaded_entry.hash_xor_data, ordering);
    }
}

impl From<LoadedAtomicEntry> for AtomicEntry {
    fn from(loaded_entry: LoadedAtomicEntry) -> Self {
        Self {
            data: AtomicU64::new(loaded_entry.data),
            hash_xor_data: AtomicU64::new(loaded_entry.hash_xor_data),
        }
    }
}

impl Default for AtomicEntry {
    fn default() -> Self {
        Self::from(LoadedAtomicEntry::default())
    }
}

/// Exactly the same as AtomicEntry but with u64.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct LoadedAtomicEntry {
    data: u64,
    hash_xor_data: u64,
}

impl LoadedAtomicEntry {
    /// Returns the hash value of this entry.
    const fn hash(&self) -> HashKind {
        self.data ^ self.hash_xor_data
    }

    /// Returns the unpacked Entry of this LoadedAtomicEntry.
    fn entry(&self) -> Entry {
        self.unpack().0
    }

    #[inline]
    fn pack_move(move_: Move) -> u64 {
        let from_u8 = move_.from as u8;
        let to_u8 = move_.to as u8;
        let promotion_u8 = match move_.promotion {
            None => 0,
            Some(King) => 1,
            Some(Pawn) => 2,
            Some(Knight) => 3,
            Some(Rook) => 4,
            Some(Queen) => 5,
            Some(Bishop) => 6,
        };

        let from_pack = Self::pack_u8(from_u8, adf::FROM_SHIFT);
        let to_pack = Self::pack_u8(to_u8, adf::TO_SHIFT);
        let promotion_pack = Self::pack_u8(promotion_u8, adf::PROMOTION_SHIFT);
        from_pack | to_pack | promotion_pack
    }

    /// Promotion unpacking must mirror the packing in Self::pack_move(..).
    fn unpack_move(packed: u64) -> Move {
        let from_u8 = Self::unpack_u8(packed, adf::FROM_SHIFT, adf::FROM_MASK);
        let to_u8 = Self::unpack_u8(packed, adf::TO_SHIFT, adf::TO_MASK);
        let promo_u8 = Self::unpack_u8(packed, adf::PROMOTION_SHIFT, adf::PROMOTION_MASK);

        let from = Square::try_from(from_u8).unwrap();
        let to = Square::try_from(to_u8).unwrap();
        let promotion = match promo_u8 {
            0 => None,
            1 => Some(King),
            2 => Some(Pawn),
            3 => Some(Knight),
            4 => Some(Rook),
            5 => Some(Queen),
            6 => Some(Bishop),
            _ => None,
        };

        Move::new(from, to, promotion)
    }

    #[inline]
    const fn pack_i16(value: i16, shift: u8, mask: u64) -> u64 {
        ((value as u64) << shift) & mask
    }

    /// Extract an i16 from the containing u64 packed data.
    #[inline]
    const fn unpack_i16(packed_data: u64, shift: u8, mask: u64) -> i16 {
        ((packed_data & mask) >> shift) as i16
    }

    #[inline]
    const fn pack_u8(value: u8, shift: u8) -> u64 {
        (value as u64) << shift
    }

    #[inline]
    const fn unpack_u8(packed_data: u64, shift: u8, mask: u64) -> u8 {
        ((packed_data & mask) >> shift) as u8
    }

    /// Pack an entry and an age into a single u64 integer.
    fn pack(&mut self, entry: Entry, age: u8) {
        let hash = entry.hash;
        let mut data: u64 = 0;

        data |= Self::pack_move(entry.key_move);
        data |= Self::pack_i16(entry.score.0, adf::SCORE_SHIFT, adf::SCORE_MASK);
        data |= Self::pack_u8(entry.ply, adf::PLY_SHIFT);
        data |= Self::pack_u8(entry.node_kind as u8, adf::NODE_KIND_SHIFT);
        data |= Self::pack_u8(age, adf::AGE_SHIFT);
        self.data = data;
        self.hash_xor_data = hash ^ data;
    }

    /// Unpack requires that AtomicEntry packed data was packed from a valid Entry.
    fn unpack(&self) -> (Entry, AgeKind) {
        let data = self.data;
        let hash_xor_data = self.hash_xor_data;
        let hash: u64 = data ^ hash_xor_data;

        let key_move = Self::unpack_move(data);
        let score = Cp(Self::unpack_i16(data, adf::SCORE_SHIFT, adf::SCORE_MASK));
        let ply: PlyKind = Self::unpack_u8(data, adf::PLY_SHIFT, adf::PLY_MASK);
        let node_kind = NodeKind::try_from(Self::unpack_u8(
            data,
            adf::NODE_KIND_SHIFT,
            adf::NODE_KIND_MASK,
        ))
        .unwrap();

        let age: AgeKind = Self::unpack_u8(data, adf::AGE_SHIFT, adf::AGE_MASK);
        let entry = Entry::new(hash, key_move, score, ply, node_kind);
        (entry, age)
    }
}

impl Default for LoadedAtomicEntry {
    fn default() -> Self {
        Self::from(Entry::illegal())
    }
}

impl From<Entry> for LoadedAtomicEntry {
    fn from(entry: Entry) -> Self {
        let mut loaded_atomic_entry = LoadedAtomicEntry {
            data: 0,
            hash_xor_data: 0,
        };
        loaded_atomic_entry.pack(entry, 0);
        loaded_atomic_entry
    }
}

impl From<(Entry, AgeKind)> for LoadedAtomicEntry {
    fn from((entry, age): (Entry, AgeKind)) -> Self {
        let mut loaded_atomic_entry = LoadedAtomicEntry {
            data: 0,
            hash_xor_data: 0,
        };
        loaded_atomic_entry.pack(entry, age);
        loaded_atomic_entry
    }
}

/// Bucket implemented with an XOR atomic trick for sync.
#[derive(Debug, Default)]
pub struct AtomicBucket {
    priority: AtomicEntry,
    general: AtomicEntry,
}

impl TwoBucket for AtomicBucket {
    fn get(&self, hash: HashKind) -> Option<Entry> {
        let loaded_priority = self.priority.load(Ordering::Acquire);
        let loaded_general = self.general.load(Ordering::Acquire);

        if hash == loaded_priority.hash() {
            Some(loaded_priority.entry())
        } else if hash == loaded_general.hash() {
            Some(loaded_general.entry())
        } else {
            None
        }
    }

    /// Returns true if this bucket has any entry which contains the given hash.
    fn contains(&self, hash: HashKind) -> bool {
        let loaded_priority = self.priority.load(Ordering::Acquire);
        let loaded_general = self.general.load(Ordering::Acquire);
        hash == loaded_priority.hash() || hash == loaded_general.hash()
    }

    /// Unconditionally store the entry in the general slot, without updating age.
    fn store(&self, general_entry: Entry) {
        self.general.store(general_entry.into(), Ordering::Release);
    }

    /// Unconditionally place the entry in the priority slot and update age.
    fn replace(&self, priority_entry: Entry, age: u8) {
        self.priority
            .store((priority_entry, age).into(), Ordering::Release);
    }

    /// Move the existing priority entry to the general slot,
    /// then place the new priority entry into the priority slot and update age.
    fn swap_replace(&self, priority_entry: Entry, age: u8) {
        let new_general = self.priority.load(Ordering::Acquire);
        self.replace(priority_entry, age);
        self.general.store(new_general, Ordering::Release);
    }

    /// Replaces the `priority` slot if `should_replace` returns true,
    /// otherwise the `general` slot is replaced.
    ///
    /// # Example:
    /// if should_replace {
    ///     priority := entry
    /// } else {
    ///     general := entry
    /// }
    ///
    /// FnOnce signature:
    ///
    /// should_replace(&new_entry, new_age, &existing_priority_entry, existing_age) -> bool
    fn replace_by<F>(&self, entry: Entry, age: u8, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let priority = self.priority.load(Ordering::Acquire);
        let (existing_entry, existing_age) = priority.unpack();

        match should_replace(&entry, age, &existing_entry, existing_age) {
            true => self.replace(entry, age),
            false => self.store(entry),
        }
    }

    /// If should_replace returns true, then swap_replace with the given entry.
    ///
    /// Example:
    ///
    /// if should_replace {
    ///     general := priority
    ///     priority := entry
    /// } else {
    ///     general := entry
    /// }
    ///
    /// FnOnce signature:
    ///
    /// should_replace(&new_entry, new_age, &existing_priority_entry, existing_age) -> bool
    fn swap_replace_by<F>(&self, entry: Entry, age: u8, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let priority = self.priority.load(Ordering::Acquire);
        let (existing_entry, existing_age) = priority.unpack();

        if should_replace(&entry, age, &existing_entry, existing_age) {
            self.replace(entry, age);
            self.general.store(priority, Ordering::Release);
        } else {
            self.store(entry);
        }
    }
}

/// Fill a Vector to capacity.
fn fill_with_default<Bucket: TwoBucket>(v: &mut Vec<Bucket>) {
    let capacity = v.capacity();
    while v.len() < capacity {
        v.push(Bucket::default());
    }
    debug_assert_eq!(v.len(), capacity);
    debug_assert_eq!(v.capacity(), capacity);
}

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
/// let entry = Entry::new(hash, Move::new(D2, D4, None), Cp(3), 5, NodeKind::Pv);
///
/// tt.replace(entry, age);
/// assert_eq!(tt.get(hash), Some(entry));
/// ```
pub struct TranspositionTable<Bucket: TwoBucket = AtomicBucket> {
    /// Number of buckets in transpositions vector.
    bucket_capacity: usize,
    /// ZobristTable used to unify all entry hashes to the same hash generator.
    ztable: ZobristTable,
    /// Bucketed vector of transpositions.
    transpositions: Vec<Bucket>,
}

/// Transposition Table functions that use the default generic parameter bucket.
impl TranspositionTable {
    /// Returns a new Transposition Table using the default bucket type.
    pub fn new() -> Self {
        Self::new_in()
    }

    /// Returns a new Transposition Table that holds `entry_capacity` entries
    /// using the default bucket type.
    pub fn with_capacity(entry_capacity: usize) -> Self {
        Self::with_capacity_in(entry_capacity)
    }

    /// Returns a new Transposition Table with a capacity that fills given Megabytes
    /// using the default bucket type.
    pub fn with_mb(mb: usize) -> Self {
        Self::with_mb_in(mb)
    }

    /// Returns a new TranspositionTable with provided ZobristTable with pre-allocated
    /// default max capacity and default bucket type.
    pub fn with_zobrist(ztable: ZobristTable) -> Self {
        Self::with_zobrist_in(ztable)
    }

    /// Returns a new TranspositionTable with capacity in Megabytes and a given ZobristTable
    /// using the default bucket type.
    pub fn with_mb_and_zobrist(mb: usize, ztable: ZobristTable) -> Self {
        Self::with_mb_and_zobrist_in(mb, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable and capacity in entries pre-allocated
    /// using the default bucket type.
    pub fn with_capacity_and_zobrist(entry_capacity: usize, ztable: ZobristTable) -> Self {
        Self::with_capacity_and_zobrist_in(entry_capacity, ztable)
    }
}

/// Generic Transposition Table functions.
impl<Bucket: TwoBucket> TranspositionTable<Bucket> {
    /// Number of entries table holds by default.
    const DEFAULT_MAX_ENTRIES: usize = 100_000;

    /// Converts a size in Megabytes to a capacity of inner vector.
    fn mb_to_bucket_capacity(mb: usize) -> usize {
        assert!(mb > 0, "mb cannot be 0");
        (mb * 1_000_000) / mem::size_of::<Bucket>()
    }

    fn mb_to_entry_capacity(mb: usize) -> usize {
        assert!(mb > 0, "mb cannot be 0");
        let bucket_capacity = Self::mb_to_bucket_capacity(mb);
        bucket_capacity * Bucket::len()
    }

    /// Returns a reference to the zobrist table.
    pub fn zobrist_table(&self) -> &ZobristTable {
        &self.ztable
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// and a pre-allocated default max entry capacity.
    pub fn new_in() -> Self {
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_in(Self::DEFAULT_MAX_ENTRIES, ztable)
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// with given capacity pre-allocated, where capacity is the number of entries in table.
    pub fn with_capacity_in(entry_capacity: usize) -> Self {
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_in(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with a randomly generated ZobristTable
    /// with capacity calculated to fill given Megabytes.
    pub fn with_mb_in(mb: usize) -> Self {
        let entry_capacity = Self::mb_to_entry_capacity(mb);
        let ztable = ZobristTable::new();
        Self::with_capacity_and_zobrist_in(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable
    /// with pre-allocated default max capacity.
    pub fn with_zobrist_in(ztable: ZobristTable) -> Self {
        let entry_capacity = Self::DEFAULT_MAX_ENTRIES;
        Self::with_capacity_and_zobrist_in(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with capacity in Megabytes and a given ZobristTable.
    pub fn with_mb_and_zobrist_in(mb: usize, ztable: ZobristTable) -> Self {
        let entry_capacity = Self::mb_to_entry_capacity(mb);
        Self::with_capacity_and_zobrist_in(entry_capacity, ztable)
    }

    /// Returns a new TranspositionTable with provided ZobristTable
    /// and capacity in entries pre-allocated.
    pub fn with_capacity_and_zobrist_in(entry_capacity: usize, ztable: ZobristTable) -> Self {
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
        for bucket in &mut self.transpositions {
            *bucket = Bucket::default();
        }
        debug_assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.bucket_capacity, self.transpositions.len());
    }

    /// Drops original table and allocates a new table of size `new_mb`.
    /// Entries in the original table are not preserved.
    /// Returns the table's new entry capacity.
    pub fn set_mb(&mut self, new_mb: usize) -> usize {
        let entry_capacity = Self::mb_to_entry_capacity(new_mb);
        let ztable = self.ztable.clone();
        *self = Self::with_capacity_and_zobrist_in(entry_capacity, ztable);
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

    /// Returns true if a TranspositionTable bucket contains an entry with the given hash.
    /// Key collisions are expected to be rare but possible,
    /// so care should be taken with the return value.
    pub fn contains(&self, hash: HashKind) -> bool {
        let index = self.hash_to_index(hash);
        self.transpositions[index].contains(hash)
    }

    /// Returns Entry if hash exists in the indexed bucket, None otherwise.
    pub fn get(&self, hash: HashKind) -> Option<Entry> {
        let index = self.hash_to_index(hash);
        self.transpositions[index].get(hash)
    }

    /// Unconditionally replace an existing item in the TranspositionTable
    /// where replace_by true would place it.
    /// Capacity of the table remains unchanged.
    pub fn replace(&self, priority_entry: Entry, age: AgeKind) {
        let index = self.hash_to_index(priority_entry.hash);
        self.transpositions[index].replace(priority_entry, age);

        debug_assert_eq!(self.bucket_capacity, self.transpositions.capacity());
        debug_assert_eq!(self.bucket_capacity, self.transpositions.len());
    }

    /// Move entry in priority slot to general slot then place priority_entry into priority slot.
    pub fn swap_replace(&self, priority_entry: Entry, age: AgeKind) {
        let index = self.hash_to_index(priority_entry.hash);
        self.transpositions[index].swap_replace(priority_entry, age);
    }

    /// Store the entry into the index bucket's general slot, without changing age or scheme slot.
    pub fn store(&self, general_entry: Entry) {
        let index = self.hash_to_index(general_entry.hash);
        self.transpositions[index].store(general_entry);
    }

    /// Attempt to insert an item into the tt depending on a replacement scheme.
    /// If the replacement scheme evaluates to true, the entry replaces the bucket priority slot.
    /// Otherwise, it is inserted into the general slot.
    ///
    /// Closure signature: should_replace(&replacing_entry, age, &existing_priority_entry, existing_age) -> bool.
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
    /// let deep_entry = Entry::new(deep_hash, best_move, score, deep_ply, node_kind);
    ///
    /// let shallow_hash = 8;
    /// let shallow_ply = 2;
    /// let shallow_entry = Entry::new(shallow_hash, best_move, score, shallow_ply, node_kind);
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
    /// let other_entry = Entry::new(other_hash, best_move, score, other_ply, node_kind);
    ///
    /// // Other entry does not pass test for priority, so it replaces the always slot.
    /// tt.replace_by(other_entry, age, replacement_scheme);
    /// assert_eq!(tt.get(shallow_hash), None);
    /// assert_eq!(tt.get(deep_hash).unwrap(), deep_entry);
    /// assert_eq!(tt.get(other_hash).unwrap(), other_entry);
    pub fn replace_by<F>(&self, entry: Entry, age: AgeKind, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let index = self.hash_to_index(entry.hash);
        self.transpositions[index].replace_by(entry, age, should_replace);
    }

    /// If entry passes the should_replace test, then the existing entry in the priority slot
    /// is moved to the general slot and new entry gets placed in the priority slot.
    /// Otherwise, the new entry is placed in the general slot.
    pub fn swap_replace_by<F>(&self, entry: Entry, age: AgeKind, should_replace: F)
    where
        F: FnOnce(&Entry, u8, &Entry, u8) -> bool,
    {
        let index = self.hash_to_index(entry.hash);
        self.transpositions[index].swap_replace_by(entry, age, should_replace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coretypes::{PieceKind, Square::*};
    use std::mem::size_of;

    #[test]
    fn atomic_pack_sizes() {
        //! AtomicEntry requires an exact data layout for struct that it packs.
        //! This test ensures that if there is a change in the data layout externally,
        //! it does not result in unexpected behavior as the test fails.
        assert_eq!(adf::FROM_BYTES, size_of::<Square>());
        assert_eq!(adf::TO_BYTES, size_of::<Square>());
        assert_eq!(adf::PROMOTION_BYTES, size_of::<Option<PieceKind>>());
        assert_eq!(adf::SCORE_BYTES, size_of::<Cp>());
        assert_eq!(adf::PLY_BYTES, size_of::<PlyKind>());
        assert_eq!(adf::NODE_KIND_BYTES, size_of::<NodeKind>());
        assert_eq!(adf::AGE_BYTES, size_of::<AgeKind>());
    }

    #[test]
    fn loaded_atomic_entry() {
        {
            // Illegal entry test.
            let entry = Entry::illegal();
            let loaded = LoadedAtomicEntry::from(entry);
            assert_eq!(entry.hash, loaded.hash());
            assert_eq!(entry, loaded.entry());
        }
        {
            // Random entry test.
            let entry = Entry::new(500, Move::new(D2, D4, None), Cp(5000), 5, NodeKind::Pv);
            let age: AgeKind = 7;

            let loaded = LoadedAtomicEntry::from((entry, age));
            let (loaded_entry, loaded_age) = loaded.unpack();
            assert_eq!(entry, loaded_entry);
            assert_eq!(age, loaded_age);
        }
        {
            // Random entry test with negative.
            let entry = Entry::new(
                0xAAFFEE,
                Move::new(H7, H8, Some(Knight)),
                Cp(-51),
                10,
                NodeKind::Cut,
            );
            let age: AgeKind = 7;

            let loaded = LoadedAtomicEntry::from((entry, age));
            let (loaded_entry, loaded_age) = loaded.unpack();
            assert_eq!(entry, loaded_entry);
            assert_eq!(age, loaded_age);
        }
    }

    // TODO
    //#[test]
    //fn size_of_requirements() {
    //    // Want a single entry to fit into L1 cache line?
    //    // Need to verify that this is how this works, not sure since Mutex is used.
    //    use std::mem::size_of;
    //    let size = size_of::<TtEntry>();
    //    println!("size_of::<TtEntry>() = {}", size);
    //    assert!(size <= 64);
    //}

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
