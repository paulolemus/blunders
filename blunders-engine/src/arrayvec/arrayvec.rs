//! Array Vector
//! A Generic, fixed capacity Vector on stack.
//!
//! Notes:
//! "MaybeUninit<T> is guaranteed to have the same size, alignment, and ABI as T:"
//!
//! "Arrays are laid out so that the nth element of the array is offset from the
//! start of the array by n * the size of the type bytes.
//! An array of [T; n] has a size of size_of::<T>() * n and the same alignment of T."
//!
//! From the above, the arrays [T; CAP] and [MaybeUninit<T>; CAP] are guaranteed to have
//! the same layout (size, alignment).
//!
//! Do not cast &[MaybeUninit<T>; CAP] to &[T; CAP] because it is unsound.
//! It is UB because all T's in array are not initialized,
//! like how `let x: usize = MaybeUninit::uninit().assume_init();` is immediately
//! UB, even if x is never accessed.
//! unsafe { &*(slice as *const [MaybeUninit<usize>] as *const [usize]) }

use std::fmt::{self, Display};
use std::iter::{ExactSizeIterator, FusedIterator};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::slice;

/// ArrayVec is a stack-based, fixed capacity, contiguous container.
/// It guarantees that its first `size` items starting from index 0
/// are valid items.
pub struct ArrayVec<T, const CAPACITY: usize> {
    items: [MaybeUninit<T>; CAPACITY],
    size: usize,
}

impl<T, const CAPACITY: usize> ArrayVec<T, CAPACITY> {
    /// Returns an empty instance of ArrayVec with fixed capacity.
    pub fn new() -> Self {
        let items: [MaybeUninit<T>; CAPACITY] = unsafe {
            // Safe, Legal to assume array of MaybeUninit is initialized;
            // It doesn't require init.  This is from std docs.
            MaybeUninit::uninit().assume_init()
        };
        let size = 0;

        Self { items, size }
    }

    /// Returns number of items in container.
    pub const fn len(&self) -> usize {
        self.size
    }

    /// Manually sets the length of this ArrayVec.
    /// This operation is unsafe because manually changing the length of valid items
    /// invalidates the invariant that all items within the length are initialized.
    ///
    /// This also changes what happens when ArrayVec is dropped.
    /// If the length is set to 0 from 5, those 5 items will not be dropped.
    pub unsafe fn set_len(&mut self, new_size: usize) {
        self.size = new_size;
    }

    /// Returns true if container holds no items.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if container is at capacity, and can hold no more items.
    pub const fn is_full(&self) -> bool {
        self.len() == CAPACITY
    }

    /// Returns the capacity of this ArrayVec.
    pub const fn capacity(&self) -> usize {
        CAPACITY
    }

    /// Appends an item to the back of this ArrayVec.
    /// Panics if the container is full before pushing.
    pub fn push(&mut self, item: T) {
        // Notes:
        // `size` points to the item after the last valid item,
        // so it points to either uninitialized memory, or to a value.
        // Values in the array must not be dropped.

        // Guard against full container.
        if !self.is_full() {
            let size = self.size;

            unsafe {
                // Item is moved into MaybeUninit<T>. Neither item nor
                // the original value of MaybeUninit are dropped.
                self.items[size].as_mut_ptr().write(item);
            }

            self.size += 1;
        } else {
            panic!("exceeded max capacity of ArrayVec");
        }
    }

    /// Removes and returns last item from container, or None if it is empty.
    pub fn pop(&mut self) -> Option<T> {
        // Only mutate if container has items.
        if !self.is_empty() {
            // size points to item after last item in container.
            // Decrement to get to last item.
            self.size -= 1;
            let size = self.size;

            unsafe {
                Some(
                    // Bitwise copies MaybeUninit<T> into T, which can be dropped externally.
                    // The value in the array is logically dropped, because it is now treated
                    // as junk data.
                    self.items[size].as_ptr().read(),
                )
            }
        } else {
            None
        }
    }

    /// Removes all items in container, setting size to 0.
    pub fn clear(&mut self) {
        // Pop ensures drop is called on each item in
        // container by extracting it as a value.
        while !self.is_empty() {
            self.pop();
        }
    }

    /// Returns a slice of valid items.
    pub fn as_slice<'a>(&'a self) -> &'a [T] {
        // Slice of T must never extend into Uninit territory, ever.
        unsafe {
            // Take a pointer to the head of items.
            // *MaybeUninit<T> can be cast to *T because the layout guaranteed to be
            // the same.
            let valid_ptr: *const T = {
                let uninit_ptr: *const MaybeUninit<T> = self.items.as_ptr();
                uninit_ptr.cast()
            };

            // Create a slice from the pointer to the head of the array,
            // and the number of elements, which is equal to the size.
            slice::from_raw_parts(valid_ptr, self.size)
        }
    }

    /// Return a mutable slice of items in ArrayVec.
    pub fn as_slice_mut<'a>(&'a mut self) -> &'a mut [T] {
        // Slice of T must never extend into Uninit territory, ever.
        unsafe {
            // Take a pointer to the head of items.
            // *MaybeUninit<T> can be cast to *T because the layout guaranteed to be
            // the same.
            let valid_ptr: *mut T = {
                let uninit_ptr: *mut MaybeUninit<T> = self.items.as_mut_ptr();
                uninit_ptr.cast()
            };

            // Create a slice from the pointer to the head of the array,
            // and the number of elements, which is equal to the size.
            slice::from_raw_parts_mut(valid_ptr, self.size)
        }
    }
}

/// Drop MUST be implemented for ArrayVec since Dropping MaybeUninit is a no-op.
/// If this container holds any non-Copy type, their values must be dropped manually
/// or cause a memory leak.
impl<T, const CAPACITY: usize> Drop for ArrayVec<T, CAPACITY> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const CAPACITY: usize> Deref for ArrayVec<T, CAPACITY> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
impl<T, const CAPACITY: usize> DerefMut for ArrayVec<T, CAPACITY> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<T, const CAPACITY: usize> AsRef<[T]> for ArrayVec<T, CAPACITY> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const CAPACITY: usize> AsMut<[T]> for ArrayVec<T, CAPACITY> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_slice_mut()
    }
}

// When T is both Copy and Clone, MaybeUninit<T> is Clone, and [MaybeUninit<T>] is Clone.
// Cloning the entire array [MaybeUninit<T>] is only safe if T is Copy.
impl<T: Copy + Clone, const CAPACITY: usize> Clone for ArrayVec<T, CAPACITY> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            size: self.size.clone(),
        }
    }
}

impl<T: Display, const CAPACITY: usize> Display for ArrayVec<T, CAPACITY> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut displayed = String::new();

        for item in self.iter() {
            displayed.push_str(&item.to_string());
            displayed.push(' ');
        }
        displayed.pop();

        f.write_str(&displayed)
    }
}

// IntoIterator for T, &T, and &mut T
impl<T, const CAPACITY: usize> IntoIterator for ArrayVec<T, CAPACITY> {
    type Item = T;
    type IntoIter = IntoIter<T, CAPACITY>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self)
    }
}
impl<'a, T, const CAPACITY: usize> IntoIterator for &'a ArrayVec<T, CAPACITY> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
impl<'a, T, const CAPACITY: usize> IntoIterator for &'a mut ArrayVec<T, CAPACITY> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice_mut().iter_mut()
    }
}

/// IntoIterator target for ArrayVec.
/// TODO:
/// Need to figure out how to properly handle memory between
/// ArrayVec and IntoIter.
/// Normally, when ArrayVec gets dropped it drops all valid items in
/// its container by assuming that the first `size` elements are initialized and owned.
/// However, this IntoIter needs to iterate the array starting from the front,
/// and if it were to get dropped, it need only drop those items that have not yet been reached.
///
/// mem::ManuallyDrop?
pub struct IntoIter<T, const CAPACITY: usize> {
    vec: ArrayVec<T, CAPACITY>,
    size: usize,
    idx: usize,
}

impl<T, const CAPACITY: usize> IntoIter<T, CAPACITY> {
    /// Create a new IntoIter from an ArrayVec.
    fn new(array_vec: ArrayVec<T, CAPACITY>) -> Self {
        todo!();
        let size = array_vec.size;

        // Remove the vec's ability to drop items that remain inside itself.
        // IntoIter will handle dropping those values now.
        unsafe {
            array_vec.set_len(0);
        }

        Self {
            vec: array_vec,
            size,
            idx: 0,
        }
    }
}

impl<T, const CAPACITY: usize> Iterator for IntoIter<T, CAPACITY> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
        // TODO:
        // if self.size > 0 {
        //     self.size -= 1;

        //     // The number of initialized items was known as size.
        //     // Until size is 0, each item is guaranteed to be valid.
        //     // assume_init also moves T out of container, so it now can
        //     // be dropped externally.
        //     unsafe { Some(self.it.next().unwrap().assume_init()) }
        // } else {
        //     None
        // }
    }

    /// Exact remaining length of iterator is always known.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}
impl<T, const CAPACITY: usize> ExactSizeIterator for IntoIter<T, CAPACITY> {}
impl<T, const CAPACITY: usize> FusedIterator for IntoIter<T, CAPACITY> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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
    fn deref_t_slice() {
        let mut vec: ArrayVec<u32, 10> = ArrayVec::new();

        assert_eq!(vec.contains(&500), false);
        vec.push(500);
        assert_eq!(vec.contains(&500), true);

        assert_eq!(vec.get(0), Some(&500));
        assert_eq!(vec.get(1), None);

        // Mutate
        *vec.get_mut(0).unwrap() = 10;
        assert_eq!(vec.contains(&500), false);
        assert_eq!(vec.contains(&10), true);
        assert_eq!(vec.get(0), Some(&10));
        assert_eq!(vec.get(1), None);
        assert_eq!(vec.pop(), Some(10));

        // iterate

        let mut items = HashSet::new();
        items.insert(100);
        items.insert(200);
        items.insert(300);
        let mut vec: ArrayVec<u32, 10> = ArrayVec::new();
        for item in &items {
            vec.push(*item);
        }
        for item in &vec {
            assert!(items.contains(item));
        }
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

        let mut iter = arrayvec.iter();
        assert_eq!(-10, *iter.next().unwrap());
        assert_eq!(0, *iter.next().unwrap());
        assert_eq!(40, *iter.next().unwrap());
        assert_eq!(300, *iter.next().unwrap());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn clears() {
        let mut arrayvec = ArrayVec::<i32, 100>::new();
        assert_eq!(arrayvec.len(), 0);

        arrayvec.push(100);
        arrayvec.push(500);
        assert_eq!(arrayvec.len(), 2);

        arrayvec.clear();
        assert_eq!(arrayvec.len(), 0);
        assert_eq!(arrayvec.pop(), None);
    }
}
