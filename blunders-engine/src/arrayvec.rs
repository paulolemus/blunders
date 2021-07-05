//! Generic, fixed capacity, Vector on stack.

use std::array;
use std::iter::{ExactSizeIterator, FusedIterator};

/// ArrayVec hold all items of a generic type on the stack with a fixed capacity.
/// Guarantees:
///
/// * The pushed items currently in ArrayVec are contiguous, starting from internal array's 0th index.
///
/// Todo:
/// * Convert from Option<T> to MaybeUninit<T> for performance.
/// * impl Deref<Target=[T]>.
#[derive(Debug, Copy, Clone)]
pub struct ArrayVec<T, const CAPACITY: usize> {
    items: [Option<T>; CAPACITY],
    size: usize,
}

// Implementation details:
// The first size items in array will be the values in the array.
// size points to the element after the last item, so to junk data.
impl<T: Copy + Clone, const CAPACITY: usize> ArrayVec<T, CAPACITY> {
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
            // size points to element after last valid data, so
            // push into size then increment.
            self.items[self.size] = Some(item);
            self.size += 1;
        } else {
            panic!("Exceeded max capacity of array.");
        }
    }

    /// Inserts an item into the front of the container. If the container is full, panic.
    /// push_front slides all existing items in array to the right by one position.
    pub fn push_front(&mut self, item: T) {
        // Guard against full array.
        if !self.is_full() {
            // Shift all existing items in array to the right by 1 index.
            // There is guaranteed to available capacity.
            // Insert new item into front of array.
            let len = self.len();
            self.items.copy_within(0..len, 1);
            self.items[0] = Some(item);
            self.size += 1;
        } else {
            panic!("Exceeded max capacity of array, cannot push_front.");
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
        for item in &mut self.items {
            *item = None;
        }
        self.size = 0;
    }
}

impl<T, const CAPACITY: usize> IntoIterator for ArrayVec<T, CAPACITY> {
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

impl<T, const CAPACITY: usize> ArrayVecIterator<T, CAPACITY> {
    pub fn new(array_vec: ArrayVec<T, CAPACITY>) -> Self {
        assert!(array_vec.size < CAPACITY);
        let it = std::array::IntoIter::new(array_vec.items);
        let size = array_vec.size;
        Self { it, size }
    }
}

impl<T, const CAPACITY: usize> Iterator for ArrayVecIterator<T, CAPACITY> {
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

impl<T, const CAPACITY: usize> ExactSizeIterator for ArrayVecIterator<T, CAPACITY> {}
impl<T, const CAPACITY: usize> FusedIterator for ArrayVecIterator<T, CAPACITY> {}

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
}
