//! How `GameState` owns a run's seed (`crate::seed`, `docs/variety.md`): the source the seeds come
//! from, the seed of the run in progress, and the debug switch that fixes it.
//!
//! `start_run` takes the next seed and the run keeps it for its life, through the result screen.
//! The hub, a game with no hub and a test room (a sandbox) have none. `main.rs` reads the system
//! clock once and passes a `SeedSource` in; nothing else reads a clock, so a test is never
//! random. The seed is not saved in the profile.
use super::GameState;
use crate::seed::SeedSource;

/// The seeds a game hands out and the one the last run was started with.
#[derive(Debug, Default)]
pub(super) struct SeedState {
    source: SeedSource,
    /// While set, every run starts with exactly this seed (F4's "Fixed seed").
    fixed: Option<u64>,
    /// The seed of the run in progress, and of a run whose result screen is showing.
    pub(super) current: Option<u64>,
    /// The seed of the last run started, kept after it is over, for F4's read-only label.
    last: Option<u64>,
}

impl SeedState {
    /// The seed the next run takes and the source as it is after taking it. A fixed seed does not
    /// move the source. Nothing is used up until `begin`, so a run that fails to start leaves the
    /// next seed where it was.
    pub(super) fn next_seed(&self) -> (u64, SeedSource) {
        let mut source = self.source.clone();
        let seed = self.fixed.unwrap_or_else(|| source.next());
        (seed, source)
    }

    /// A run started with `seed`; `source` is what `next_seed` returned with it.
    pub(super) fn begin(&mut self, seed: u64, source: SeedSource) {
        self.source = source;
        self.current = Some(seed);
        self.last = Some(seed);
    }
}

impl GameState {
    /// Uses `source` for the seeds of the runs that follow. `main.rs` starts it from the system
    /// clock; a game that is never given one uses `SeedSource::default()`, a fixed start.
    pub fn use_seed_source(&mut self, source: SeedSource) {
        self.run.seeds.source = source;
    }

    /// The seed of the run in progress, or of the one whose result screen is showing. `None` in
    /// the hub, in a game with no hub and in a test room, which is a sandbox and takes none.
    pub fn run_seed(&self) -> Option<u64> {
        let running = self.run_in_progress();
        let result = self.test_run.is_none() && self.showing_result();
        if running || result {
            self.run.seeds.current
        } else {
            None
        }
    }

    /// The seed of the last run started, even after it is over; `None` before the first run.
    pub fn last_run_seed(&self) -> Option<u64> {
        self.run.seeds.last
    }

    /// While `Some`, every run starts with exactly this seed. `None` goes back to the source. A
    /// run already going keeps the seed it started with.
    pub fn set_fixed_seed(&mut self, seed: Option<u64>) {
        self.run.seeds.fixed = seed;
    }

    /// The seed every run starts with, while one is fixed.
    pub fn fixed_seed(&self) -> Option<u64> {
        self.run.seeds.fixed
    }
}
