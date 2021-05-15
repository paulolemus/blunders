//! Performance Test
//!
//! [Perft](https://www.chessprogramming.org/Perft)
//!
//! A simple debugging and testing function used to count
//! the number of nodes at a specific depth.

use std::ops::{Add, AddAssign};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::coretypes::Move;
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
pub fn perft(position: Position, ply: u32, threads: usize) -> PerftInfo {
    // Guard easy to calculate inputs.
    if ply == 0 {
        // Ever only 1 position at 0 ply.
        return PerftInfo::new(1);
    } else if ply <= 2 || threads <= 1 {
        // Simple enough to not require threads, or single threaded.
        return perft_recurse(position, ply);
    }
    debug_assert!(ply >= 3);
    debug_assert!(threads >= 2);

    let legal_moves = position.get_legal_moves();
    let mut thread_moves_list: Vec<Vec<Move>>;

    // Need to figure out how to partition moves for each thread.
    // If no moves, return early.
    // If there aren't enough moves to go around, partition by 1.
    // Otherwise, separate into n parts where n is number of threads.
    if legal_moves.len() == 0 {
        return PerftInfo::new(0);
    } else if legal_moves.len() < threads {
        thread_moves_list = legal_moves.chunks(1).map(|slice| slice.to_vec()).collect();
    } else {
        // Separate legal moves into n parts where n is the number of threads.
        let moves_per_thread = legal_moves.len() / threads;
        thread_moves_list = legal_moves
            .chunks(moves_per_thread)
            .map(|slice| slice.to_vec())
            .collect();
        // Shorten thread_moves_list to equal number of threads by
        // adding excess lists to first list.
        while thread_moves_list.len() > threads {
            let extra_list = thread_moves_list.pop().unwrap();
            thread_moves_list[0].extend(extra_list);
        }
    }
    // Pass each move list to a different thread for searching.
    debug_assert!(thread_moves_list.len() > 0);
    debug_assert!(thread_moves_list.len() <= threads);

    let mut thread_moves_list_iter = thread_moves_list.into_iter();
    let local_legal_moves = thread_moves_list_iter.next().unwrap();
    let perft_info_mutex = Arc::new(Mutex::new(PerftInfo::new(0)));
    let mut handles = Vec::new();

    // Create threads to process partitioned moves.
    for thread_legal_moves in thread_moves_list_iter {
        let perft_info_lock = Arc::clone(&perft_info_mutex);

        let handle = thread::spawn(move || {
            let mut perft_info = PerftInfo::new(0);
            for legal_move in thread_legal_moves {
                let child_position = position.make_move(legal_move);
                perft_info += perft_recurse(child_position, ply - 1);
            }
            *perft_info_lock.lock().unwrap() += perft_info;
        });

        handles.push(handle);
    }

    // Process on local thread.
    let mut local_perft_info = PerftInfo::new(0);
    for legal_move in local_legal_moves {
        let child_position = position.make_move(legal_move);
        local_perft_info += perft_recurse(child_position, ply - 1);
    }

    // Wait for all handles to finish.
    for handle in handles {
        handle.join().unwrap();
    }

    let total_perft_info = *perft_info_mutex.lock().unwrap();
    total_perft_info + local_perft_info
}

/// Ply must be non-zero.
fn perft_recurse(position: Position, ply: u32) -> PerftInfo {
    debug_assert_ne!(ply, 0);
    if ply == 1 {
        // If we reach the depth before the end,
        // return the count of legal moves.
        PerftInfo::new(position.get_legal_moves().len() as u64)
    } else {
        let legal_moves = position.get_legal_moves();
        let mut perft_info = PerftInfo::new(0);
        for legal_move in legal_moves {
            let child_position = position.make_move(legal_move);
            perft_info += perft_recurse(child_position, ply - 1);
        }
        perft_info
    }
}
