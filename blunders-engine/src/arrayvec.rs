//! Generic, fixed capacity, Vector on stack.

use std::array;
use std::cmp::Ordering;
use std::iter::{ExactSizeIterator, FusedIterator};

/// ArrayVec hold all items of a generic type on the stack with a fixed capacity.
/// Guarantees:
///
/// * The pushed items currently in ArrayVec are contiguous, starting from internal array's 0th index.
///
/// "MaybeUninit<T> is guaranteed to have the same size, alignment, and ABI as T:"
///
/// "Arrays are laid out so that the nth element of the array is offset from the
/// start of the array by n * the size of the type bytes.
/// An array of [T; n] has a size of size_of::<T>() * n and the same alignment of T."
///
/// From the above, the arrays [T; CAP] and [MaybeUninit<T>; CAP] are guaranteed to have
/// the same layout (size, alignment).
///
/// Idea:
///
/// Do not cast &[MaybeUninit<T>; CAP] to &[T; CAP] because it is unsound.
/// It is UB because all T's in array are not initialized,
/// like how `let x: usize = MaybeUninit::uninit().assume_init();` is immediately
/// UB, even if x is never accessed.
/// unsafe { &*(slice as *const [MaybeUninit<usize>] as *const [usize]) }
///
/// Todo:
/// * Change from [Option<T>; CAP] to [MaybeUninit<T>; CAP].
/// * impl Deref<Target=[T]>.
#[derive(Debug, Copy, Clone)]
pub struct ArrayVec<T: Copy + Clone, const CAPACITY: usize> {
    items: [Option<T>; CAPACITY],
    size: usize,
}

// Implementation details:
// The first size items in array will be the values in the array.
// size points to the element after the last item, so to junk data.
impl<T: Copy + Clone, const CAPACITY: usize> ArrayVec<T, CAPACITY> {
    // Associated constant to get capacity of structure at compile time.
    pub const CAP: usize = CAPACITY;

    pub fn new() -> Self {
        Self {
            items: [None; CAPACITY],
            size: 0,
        }
    }

    /// Returns number of items in container.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns true if container has no items, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if container is completely full, false otherwise.
    pub fn is_full(&self) -> bool {
        self.size == CAPACITY
    }

    /// Returns true if container contains the given item.
    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.items[0..self.size].contains(&Some(*item))
    }

    /// Appends an item to the back of the container. If the container is full, panic.
    /// push does not change the order of any items in the container before the appended item.
    pub fn push(&mut self, item: T) {
        // Guard against full array.
        if !self.is_full() {
            // size points to element after last valid data, so push into size then increment.
            self.items[self.size] = Some(item);
            self.size += 1;
        } else {
            panic!("Exceeded max capacity of array.");
        }
    }

    /// Copy all items of other into self. Panics if capacity is exceeded.
    pub fn append(&mut self, other: ArrayVec<T, CAPACITY>) {
        for item in other {
            self.push(item);
        }
    }

    /// Removes and returns last item from container, or None if empty.
    pub fn pop(&mut self) -> Option<T> {
        // Only process pop if container has items.
        if !self.is_empty() {
            self.size -= 1;
            self.items[self.size]
        } else {
            None
        }
    }

    /// Returns reference to element at position `index` or None if out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len() {
            self.items[index].as_ref()
        } else {
            None
        }
    }

    /// Removes all items in container, setting len to 0.
    pub fn clear(&mut self) {
        for item in &mut self.items[0..self.size] {
            *item = None;
        }
        self.size = 0;
    }

    /// Allow for sorting by &T instead of by &Option<T>,
    /// until underlying data structure is converted to MaybeUninit.
    pub fn sort_unstable_by<F>(&mut self, mut compare: F)
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let len = self.len();
        self.items[0..len].sort_unstable_by(|left, right| {
            compare(left.as_ref().unwrap(), right.as_ref().unwrap())
        });
    }
}

impl<T: Copy + Clone, const CAPACITY: usize> IntoIterator for ArrayVec<T, CAPACITY> {
    type Item = T;
    type IntoIter = ArrayVecIterator<T, CAPACITY>;
    fn into_iter(self) -> Self::IntoIter {
        ArrayVecIterator::<T, CAPACITY>::new(self)
    }
}

/// Into Iterator type for ArrayVec. This Iterator only iterates the items currently
/// in the consumed ArrayVec, and ignores all items beyond ArrayVec's size.
pub struct ArrayVecIterator<T, const CAPACITY: usize> {
    it: array::IntoIter<Option<T>, CAPACITY>,
    size: usize,
}

impl<T: Copy + Clone, const CAPACITY: usize> ArrayVecIterator<T, CAPACITY> {
    pub fn new(array_vec: ArrayVec<T, CAPACITY>) -> Self {
        assert!(array_vec.size < CAPACITY);
        let it = std::array::IntoIter::new(array_vec.items);
        let size = array_vec.size;
        Self { it, size }
    }
}

impl<T: Copy + Clone, const CAPACITY: usize> Iterator for ArrayVecIterator<T, CAPACITY> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.size > 0 {
            self.size -= 1;
            self.it.next().unwrap()
        } else {
            None
        }
    }

    // Size is guaranteed from the consumed ArrayVec.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}

impl<T: Copy + Clone, const CAPACITY: usize> ExactSizeIterator for ArrayVecIterator<T, CAPACITY> {}
impl<T: Copy + Clone, const CAPACITY: usize> FusedIterator for ArrayVecIterator<T, CAPACITY> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_push_pop() {
        const CAP: usize = 10;
        let mut arr_vec = ArrayVec::<i32, CAP>::new();
        arr_vec.push(10);
        assert_eq!(arr_vec.len(), 1);
        assert!(!arr_vec.is_empty());

        let item = arr_vec.pop();
        assert_eq!(item.unwrap(), 10);
        assert_eq!(arr_vec.len(), 0);
        assert!(arr_vec.is_empty());

        let item = arr_vec.pop();
        assert!(item.is_none());
        assert_eq!(arr_vec.len(), 0);
        assert!(arr_vec.is_empty());

        let item = arr_vec.pop();
        assert!(item.is_none());
        assert_eq!(arr_vec.len(), 0);
        assert!(arr_vec.is_empty());

        arr_vec.push(20);
        assert_eq!(arr_vec.len(), 1);
        assert!(!arr_vec.is_empty());

        let item = arr_vec.pop();
        assert_eq!(item.unwrap(), 20);
        assert_eq!(arr_vec.len(), 0);
        assert!(arr_vec.is_empty());
    }

    #[test]
    #[should_panic]
    fn panics_when_capacity_exceeded() {
        const CAP: usize = 2;
        let mut list = ArrayVec::<i32, CAP>::new();
        list.push(100);
        list.push(500);

        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
        list.push(1000);
    }

    #[test]
    fn sorting() {
        let mut arrayvec = ArrayVec::<i32, 100>::new();
        arrayvec.push(40);
        arrayvec.push(300);
        arrayvec.push(-10);
        arrayvec.push(0);
        assert_eq!(4, arrayvec.len());
        arrayvec.sort_unstable_by(|a, b| a.cmp(b));
        assert_eq!(4, arrayvec.len());

        let mut iter = arrayvec.into_iter();
        assert_eq!(-10, iter.next().unwrap());
        assert_eq!(0, iter.next().unwrap());
        assert_eq!(40, iter.next().unwrap());
        assert_eq!(300, iter.next().unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn clears() {
        let mut arrayvec = ArrayVec::<i32, 100>::new();
        for item in &arrayvec.items {
            assert_eq!(*item, None);
        }

        arrayvec.push(100);
        arrayvec.push(500);
        assert_eq!(arrayvec.len(), 2);

        arrayvec.clear();
        assert_eq!(arrayvec.len(), 0);
        for item in &arrayvec.items {
            assert_eq!(*item, None);
        }
    }
}
