use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const COLUMNS: usize = 30;
pub const ROWS: usize = 30;
pub const COUNT: usize = COLUMNS * ROWS;
pub const CELL_SIZE: f32 = 24.0;
pub const GRID_SIZE: f32 = 720.0;
pub const PARK: (f32, f32) = (740.0, 740.0);
pub const ORIGIN: (f32, f32) = (0.0, 0.0);
pub const BACKGROUND: (u8, u8, u8) = (255, 255, 255);
pub const NORMAL_COLOR: (u8, u8, u8) = (134, 142, 150);
pub const HOVERED_COLOR: (u8, u8, u8) = (47, 158, 68);
pub const MOTIONS_PER_SECOND: u64 = 60;
pub const CYCLE_SECONDS: u64 = 15;
pub const MAX_LATENESS: Duration = Duration::from_millis(5);
pub const DRAIN_TIMEOUT: Duration = Duration::from_millis(250);
pub const MAX_OBSERVATIONS: usize = 300 * MOTIONS_PER_SECOND as usize + 1;

pub fn validate_viewport(
    backend: &str,
    scale_factor: f64,
    logical_width: f32,
    logical_height: f32,
) -> Result<(), &'static str> {
    if backend.trim().is_empty()
        || scale_factor != 1.0
        || !logical_width.is_finite()
        || !logical_height.is_finite()
        || logical_width <= PARK.0
        || logical_height <= PARK.1
    {
        return Err(
            "Hover requires a native backend name or unspecified, actual scale1 and finite logical dimensions >740 for the grid and outside-grid park",
        );
    }
    Ok(())
}

pub fn target(motion: u64) -> usize {
    let position = (motion % COUNT as u64) as usize;
    let row = position / COLUMNS;
    let column = position % COLUMNS;
    row * COLUMNS
        + if row.is_multiple_of(2) {
            column
        } else {
            COLUMNS - 1 - column
        }
}

pub fn center(index: usize) -> Option<(i16, i16)> {
    (index < COUNT).then(|| {
        let cell_size = CELL_SIZE as i16;
        (
            (index % COLUMNS) as i16 * cell_size + cell_size / 2,
            (index / COLUMNS) as i16 * cell_size + cell_size / 2,
        )
    })
}

pub fn deadline(motion: u64) -> Duration {
    Duration::from_secs(motion / MOTIONS_PER_SECOND)
        + Duration::from_nanos(motion % MOTIONS_PER_SECOND * 1_000_000_000 / MOTIONS_PER_SECOND)
}

pub fn motion_count(duration: Duration) -> u64 {
    (duration.as_nanos() * u128::from(MOTIONS_PER_SECOND)).div_ceil(1_000_000_000) as u64
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observations {
    pub cells: Vec<usize>,
    pub invalid: bool,
}

impl Observations {
    pub fn verify(&self, submitted_motions: u64) -> Result<(), &'static str> {
        if self.invalid || self.cells.len() as u64 != submitted_motions {
            return Err("Native hover observation count is invalid");
        }
        if self
            .cells
            .iter()
            .enumerate()
            .any(|(motion, cell)| *cell != target(motion as u64))
        {
            return Err("Native hover observation order differs from submitted trajectory");
        }
        Ok(())
    }
}

/// Acquisition span relative to received Started, including negative baseline times.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceSampleTiming {
    pub started_ms: f64,
    pub completed_ms: f64,
    pub process_count: usize,
}

#[derive(Default)]
struct ObserverState {
    active: bool,
    current: Option<usize>,
    observations: Observations,
}

/// Report only native per-cell callbacks, never trajectory coordinates or scheduled ticks.
/// Cell indices are row-major. Clones may be captured by toolkit event closures.
#[derive(Clone, Default)]
pub struct HoverObserver(Arc<Mutex<ObserverState>>);

impl HoverObserver {
    pub fn enter(&self, index: usize) {
        self.current(Some(index));
    }

    /// A leave for a previously hovered cell must not clear a newer cell.
    pub fn leave(&self, index: usize) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.current == Some(index) {
            state.current = None;
        }
    }

    /// Report native cell state only, ignoring repeated callbacks for the same cell.
    pub fn current(&self, index: Option<usize>) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if !state.active || state.current == index {
            return;
        }
        state.current = index;
        if let Some(index) = index {
            if index >= COUNT || state.observations.cells.len() >= MAX_OBSERVATIONS {
                state.observations.invalid = true;
            } else {
                state.observations.cells.push(index);
            }
        }
    }

    pub(crate) fn reset(&self, active: bool) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.active = active;
        state.current = None;
        state.observations = Observations::default();
    }

    pub(crate) fn snapshot(&self) -> Observations {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .observations
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::hover::*;

    #[test]
    fn trajectory_is_serpentine_centers_and_wraps_after_fifteen_seconds() {
        let cells: Vec<_> = (0..COUNT as u64).map(target).collect();
        assert_eq!(&cells[..3], &[0, 1, 2]);
        assert_eq!(&cells[29..32], &[29, 59, 58]);
        assert_eq!(target(899), 870);
        assert_eq!(target(900), 0);
        let mut sorted = cells.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..COUNT).collect::<Vec<_>>());
        for cell in cells {
            let (x, y) = center(cell).unwrap();
            assert!((12..720).contains(&x) && (12..720).contains(&y));
        }
        assert_eq!(center(900), None);
        assert_eq!(deadline(900), Duration::from_secs(15));
        assert_eq!(motion_count(Duration::from_secs(10)), 600);
        assert_eq!(motion_count(Duration::from_millis(1)), 1);
    }

    #[test]
    fn observer_records_only_active_native_transitions_and_resets_each_phase() {
        let observer = HoverObserver::default();
        observer.enter(12);
        assert!(observer.snapshot().cells.is_empty());
        observer.reset(true);
        let callback = observer.clone();
        callback.enter(0);
        callback.current(Some(0));
        callback.enter(1);
        callback.leave(0);
        callback.enter(1);
        assert_eq!(observer.snapshot().cells, vec![0, 1]);
        observer.snapshot().verify(2).unwrap();
        callback.leave(1);
        callback.enter(1);
        assert!(observer.snapshot().verify(3).is_err());
        observer.reset(true);
        callback.enter(0);
        observer.snapshot().verify(1).unwrap();
        observer.reset(false);
        callback.enter(2);
        assert!(observer.snapshot().cells.is_empty());
    }

    #[test]
    fn verification_rejects_missing_reordered_and_invalid_observations() {
        for observations in [
            Observations {
                cells: vec![0],
                invalid: false,
            },
            Observations {
                cells: vec![1, 0],
                invalid: false,
            },
            Observations {
                cells: vec![0, 1],
                invalid: true,
            },
        ] {
            assert!(observations.verify(2).is_err());
        }
        let observer = HoverObserver::default();
        observer.reset(true);
        observer.enter(COUNT);
        assert!(observer.snapshot().invalid);
    }
}
