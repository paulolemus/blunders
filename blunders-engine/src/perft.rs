//! Performance Test
//!
//! [Perft](https://www.chessprogramming.org/Perft)
//!
//! A simple debugging and testing function used to count
//! the number of nodes at a specific depth.

use crate::position::Position;
use std::ops::{Add, AddAssign};

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
pub fn perft(position: Position, ply: u32) -> PerftInfo {
    perft_recurse(position, ply)
}

fn perft_recurse(position: Position, ply: u32) -> PerftInfo {
    if ply == 0 {
        // If we reach the max depth of search, return 1 to count current node.
        PerftInfo::new(1)
    } else if ply == 1 {
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
