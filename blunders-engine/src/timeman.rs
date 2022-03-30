//! Time Management

use std::time::{Duration, Instant};

use crate::coretypes::{Color, PlyKind};
use crate::error::{self, ErrorKind};
use crate::uci::SearchControls;

const TIME_RATIO: u32 = 15; // Use 1/15th of remaining time per timed move.
const OVERHEAD: Duration = Duration::from_millis(10); // Expected amount of time loss in ms.

// Returns true if the duration since the start of search is gte to the provided time to move.
fn is_out_of_time(start_time: Instant, move_time: Duration) -> bool {
    start_time.elapsed() + OVERHEAD >= move_time
}

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
    pub fn stop(&self, root_player: Color, ply: PlyKind, start_time: Instant) -> bool {
        match self {
            Mode::Infinite => Infinite::stop(),
            Mode::Depth(depth_mode) => depth_mode.stop(ply, start_time),
            Mode::MoveTime(movetime_mode) => movetime_mode.stop(ply, start_time),
            Mode::Standard(standard_mode) => standard_mode.stop(root_player, ply, start_time),
        }
    }

    /// Returns a new Infinite Mode.
    pub fn infinite() -> Self {
        Self::Infinite
    }

    /// Returns a new Depth Mode.
    pub fn depth(ply: PlyKind, movetime: Option<Duration>) -> Self {
        Self::Depth(Depth {
            depth: ply,
            movetime,
        })
    }

    /// Returns a new MoveTime mode.
    pub fn movetime(movetime: Duration, ply: Option<PlyKind>) -> Self {
        Self::MoveTime(MoveTime {
            movetime,
            depth: ply,
        })
    }

    pub fn standard(
        wtime: Duration,
        btime: Duration,
        winc: Option<Duration>,
        binc: Option<Duration>,
        moves_to_go: Option<u32>,
        ply: Option<PlyKind>,
    ) -> Self {
        Self::Standard(Standard {
            wtime,
            btime,
            winc,
            binc,
            moves_to_go,
            depth: ply,
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
    pub depth: PlyKind,
    movetime: Option<Duration>,
}

impl Depth {
    /// Depth mode stops when its depth limit is passed, or optionally if movetime is met.
    fn stop(&self, ply: PlyKind, start_time: Instant) -> bool {
        if ply > self.depth {
            return true;
        }

        if let Some(movetime) = self.movetime {
            if is_out_of_time(start_time, movetime) {
                return true;
            }
        }

        false
    }

    /// Returns true if search controls has all required fields for Depth mode.
    fn satisfied(search_controls: &SearchControls) -> bool {
        search_controls.depth.is_some()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MoveTime {
    movetime: Duration,
    depth: Option<PlyKind>,
}

impl MoveTime {
    /// MoveTime mode stops after a given time has passed, or optionally if its depth is passed.
    fn stop(&self, ply: PlyKind, start_time: Instant) -> bool {
        if is_out_of_time(start_time, self.movetime) {
            return true;
        }
        if let Some(depth) = self.depth {
            if ply > depth {
                return true;
            }
        }

        false
    }

    /// Returns true if search controls has all required fields for MoveTime mode.
    fn satisfied(search_controls: &SearchControls) -> bool {
        search_controls.move_time.is_some()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Standard {
    wtime: Duration,
    btime: Duration,
    winc: Option<Duration>,
    binc: Option<Duration>,
    moves_to_go: Option<u32>,
    depth: Option<PlyKind>,
}

impl Standard {
    /// Standard stops after using some heuristic to determine how much of remaining time to use.
    /// Optionally, stops when a depth is passed.
    fn stop(&self, root_player: Color, ply: PlyKind, start_time: Instant) -> bool {
        if is_out_of_time(start_time, self.player_movetime(root_player)) {
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

    /// Return the target movetime for a player.
    fn player_movetime(&self, root_player: Color) -> Duration {
        let player_time = match root_player {
            Color::White => self.wtime,
            Color::Black => self.btime,
        };
        player_time / TIME_RATIO
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
        let controls = SearchControls {
            wtime: Some(Duration::from_millis(5000)),
            btime: Some(Duration::from_millis(5000)),
            ..Default::default()
        };
        let mode = Mode::try_from(controls);

        assert!(mode.is_ok());
        let mode = mode.unwrap();
        assert!(matches!(mode, Mode::Standard(_)));
    }
}
