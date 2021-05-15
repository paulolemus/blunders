//! Performance Test
//!
//! [Perft](https://www.chessprogramming.org/Perft)
//!
//! A simple debugging and testing function used to count
//! the number of nodes at a specific depth.

use std::ops::{Add, AddAssign};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::position::Position;

/// Debugging information about results of perft test.
/// nodes: Number of nodes at lowest depth of perft.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PerftInfo {
    pub nodes: u64,
}

impl PerftInfo {
    fn new(nodes: u64) -> Self {
        PerftInfo { nodes }
    }
}

impl Add for PerftInfo {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        PerftInfo {
            nodes: self.nodes + rhs.nodes,
        }
    }
}

impl AddAssign for PerftInfo {
    fn add_assign(&mut self, rhs: Self) {
        self.nodes += rhs.nodes;
    }
}

// Count the number of nodes at a certain depth.
// This ignores higher terminal nodes.
// In other words, it counts the number of paths to the given depth.
pub fn perft(mut position: Position, ply: u32, threads: usize) -> PerftInfo {
    // Guard easy to calculate inputs.
    if ply == 0 {
        // Ever only 1 position at 0 ply.
        return PerftInfo::new(1);
    } else if ply <= 2 || threads <= 1 {
        // Simple enough to not require threads, or single threaded.
        return perft_recurse(&mut position, ply);
    }
    debug_assert!(ply >= 3);
    debug_assert!(threads >= 2);

    let legal_moves = Arc::new(Mutex::new(position.get_legal_moves()));
    {
        let legal_moves = legal_moves.lock().unwrap();
        if legal_moves.len() == 0 {
            return PerftInfo::new(0);
        }
    }
    let total_perft_info = Arc::new(Mutex::new(PerftInfo::new(0)));
    let num_new_threads = threads - 1;
    let mut handles = Vec::new();

    // Create threads to process partitioned moves.
    for _ in 0..num_new_threads {
        // Arcs
        let legal_moves = legal_moves.clone();
        let total_perft_info = total_perft_info.clone();

        let handle = thread::spawn(move || {
            let mut perft_info = PerftInfo::new(0);
            let mut maybe_move = { legal_moves.lock().unwrap().pop() };
            while let Some(legal_move) = maybe_move {
                let move_info = position.do_move(legal_move);
                perft_info += perft_recurse(&mut position, ply - 1);
                position.undo_move(move_info);
                maybe_move = legal_moves.lock().unwrap().pop();
            }
            *total_perft_info.lock().unwrap() += perft_info;
        });

        handles.push(handle);
    }

    // Process on local thread.
    let mut perft_info = PerftInfo::new(0);
    let mut maybe_move = { legal_moves.lock().unwrap().pop() };
    while let Some(legal_move) = maybe_move {
        let move_info = position.do_move(legal_move);
        perft_info += perft_recurse(&mut position, ply - 1);
        position.undo_move(move_info);
        maybe_move = legal_moves.lock().unwrap().pop();
    }

    // Wait for all handles to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    let total_perft_info = *total_perft_info.lock().unwrap();
    total_perft_info + perft_info
}

/// Ply must be non-zero.
fn perft_recurse(position: &mut Position, ply: u32) -> PerftInfo {
    debug_assert_ne!(ply, 0);
    if ply == 1 {
        // If we reach the depth before the end,
        // return the count of legal moves.
        PerftInfo::new(position.get_legal_moves().len() as u64)
    } else {
        let legal_moves = position.get_legal_moves();
        let mut perft_info = PerftInfo::new(0);
        for legal_move in legal_moves {
            let move_info = position.do_move(legal_move);
            perft_info += perft_recurse(position, ply - 1);
            position.undo_move(move_info);
        }
        perft_info
    }
}
