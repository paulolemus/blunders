//! Time Management

use std::convert::TryFrom;
use std::time::Instant;

use crate::coretypes::Color;
use crate::error::{self, ErrorKind};
use crate::uci::SearchControls;

const TIME_RATIO: u32 = 15; // Use 1/15th of remaining time per timed move.
const OVERHEAD: u128 = 10; // Expected amount of time loss in ms.

/// There are 4 supported search modes currently, Infinite, Standard, Depth, and MoveTime.  
/// Infinite mode: do not stop searching. Search must be signaled externally to stop.  
/// Standard mode: standard chess time controls with time per side.  
/// Depth mode: search to a given depth.  
/// MoveTime mode: search for a specified time per move.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Mode {
    Infinite,           // Search until told to stop. Requires `infinite`.
    Standard(Standard), // Each player has a time limit. Requires `wtime`, `btime`.
    Depth(Depth),       // Search to a given depth. Requires `depth`.
    MoveTime(MoveTime), // Search for a specified amount of time. Requires `movetime`.
}

impl Mode {
    /// Returns true if a search should be stopped.
    pub fn stop(&self, root_player: Color, ply: u32) -> bool {
        match self {
            Mode::Infinite => Infinite::stop(),
            Mode::Depth(depth_mode) => depth_mode.stop(ply),
            Mode::MoveTime(movetime_mode) => movetime_mode.stop(ply),
            Mode::Standard(standard_mode) => standard_mode.stop(root_player, ply),
        }
    }

    /// Returns a new Infinite Mode.
    pub fn infinite() -> Self {
        Self::Infinite
    }

    /// Returns a new Depth Mode.
    pub fn depth(ply: u32, movetime: Option<u32>) -> Self {
        Self::Depth(Depth {
            depth: ply,
            instant: Instant::now(),
            movetime,
        })
    }

    /// Returns a new MoveTime mode.
    pub fn movetime(movetime: u32, ply: Option<u32>) -> Self {
        Self::MoveTime(MoveTime {
            movetime,
            instant: Instant::now(),
            depth: ply,
        })
    }

    pub fn standard(
        wtime: i32,
        btime: i32,
        winc: Option<u32>,
        binc: Option<u32>,
        moves_to_go: Option<u32>,
        ply: Option<u32>,
    ) -> Self {
        Self::Standard(Standard {
            wtime,
            btime,
            winc,
            binc,
            moves_to_go,
            depth: ply,
            instant: Instant::now(),
        })
    }
}

impl TryFrom<SearchControls> for Mode {
    type Error = error::Error;
    fn try_from(controls: SearchControls) -> error::Result<Self> {
        if Infinite::satisfied(&controls) {
            Ok(Mode::Infinite)
        } else if Standard::satisfied(&controls) {
            Ok(Mode::standard(
                controls.wtime.unwrap(),
                controls.btime.unwrap(),
                controls.winc,
                controls.binc,
                controls.moves_to_go,
                controls.depth,
            ))
        } else if MoveTime::satisfied(&controls) {
            Ok(Mode::movetime(controls.move_time.unwrap(), controls.depth))
        } else if Depth::satisfied(&controls) {
            Ok(Mode::depth(controls.depth.unwrap(), controls.move_time))
        } else {
            Err(ErrorKind::ModeNotSatisfied.into())
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Infinite;

impl Infinite {
    fn stop() -> bool {
        false
    }
    /// Returns true if search controls has all required fields for Infinite mode.
    fn satisfied(search_controls: &SearchControls) -> bool {
        search_controls.infinite
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Depth {
    pub depth: u32,
    instant: Instant,
    movetime: Option<u32>,
}

impl Depth {
    /// Depth mode stops when its depth limit is passed, or optionally if movetime is met.
    fn stop(&self, ply: u32) -> bool {
        if ply > self.depth {
            return true;
        }

        if let Some(movetime) = self.movetime {
            let elapsed_ms = self.instant.elapsed().as_millis();
            if elapsed_ms >= (movetime as u128).saturating_sub(OVERHEAD) {
                return true;
            }
        }

        return false;
    }

    /// Returns true if search controls has all required fields for Depth mode.
    fn satisfied(search_controls: &SearchControls) -> bool {
        search_controls.depth.is_some()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MoveTime {
    movetime: u32,
    instant: Instant,
    depth: Option<u32>,
}

impl MoveTime {
    /// MoveTime mode stops after a given time has passed, or optionally if its depth is passed.
    fn stop(&self, ply: u32) -> bool {
        let elapsed_ms = self.instant.elapsed().as_millis();
        if elapsed_ms >= (self.movetime as u128).saturating_sub(OVERHEAD) {
            return true;
        }

        if let Some(depth) = self.depth {
            if ply > depth {
                return true;
            }
        }

        return false;
    }

    /// Returns true if search controls has all required fields for MoveTime mode.
    fn satisfied(search_controls: &SearchControls) -> bool {
        search_controls.move_time.is_some()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Standard {
    instant: Instant,
    wtime: i32,
    btime: i32,
    winc: Option<u32>,
    binc: Option<u32>,
    moves_to_go: Option<u32>,
    depth: Option<u32>,
}

impl Standard {
    /// Standard stops after using some heuristic to determine how much of remaining time to use.
    /// Optionally, stops when a depth is passed.
    fn stop(&self, root_player: Color, ply: u32) -> bool {
        let target_elapsed = self.target_elapsed_ms(root_player);
        let elapsed_ms = self.instant.elapsed().as_millis();

        if elapsed_ms >= target_elapsed {
            return true;
        }

        // Optional depth
        if let Some(depth) = self.depth {
            if ply > depth {
                return true;
            }
        }

        false
    }

    fn target_elapsed_ms(&self, root_player: Color) -> u128 {
        let remaining_time = match root_player {
            Color::White => self.wtime,
            Color::Black => self.btime,
        };

        // Clamp to lower bound of 0.
        let remaining_time: u128 = if remaining_time.is_negative() {
            0
        } else {
            remaining_time as u128
        };

        (remaining_time / TIME_RATIO as u128).saturating_sub(OVERHEAD)
    }

    /// Returns true if search controls has all required fields for Standard Mode.
    fn satisfied(search_controls: &SearchControls) -> bool {
        search_controls.wtime.is_some() && search_controls.btime.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard() {
        let mut controls = SearchControls::default();
        controls.wtime = Some(5000);
        controls.btime = Some(5000);

        let mode = Mode::try_from(controls);

        assert!(mode.is_ok());
        let mode = mode.unwrap();
        assert!(matches!(mode, Mode::Standard(_)));
    }
}
