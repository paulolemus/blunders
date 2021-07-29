//! Array Vector Types
//!
//! Eventually Blunders would like to use its own implementation for ArrayVec
//! for experimental purposes but until they are stable arrayvec library is used.

// mod arrayvec;
// mod opt_arrayvec;

// ArrayVec Implementation used in engine:
pub use ::arrayvec::ArrayVec;

use std::fmt::Display;

/// Returns a string with the displayed string format of an ArrayVec.
/// This is a temporary work-around until internal ArrayVec is stable,
/// as Display cannot be implemented on external types.
pub fn display<T: Display, const CAP: usize>(arrayvec: &ArrayVec<T, CAP>) -> String {
    let mut displayed = String::new();
    for item in arrayvec.iter() {
        displayed.push_str(&item.to_string());
        displayed.push(' ');
    }
    displayed.pop();

    displayed
}

/// Appends all items of other to the ArrayVec.
pub fn append<T, const CAP: usize>(vec: &mut ArrayVec<T, CAP>, other: ArrayVec<T, CAP>) {
    for item in other {
        vec.push(item);
    }
}
