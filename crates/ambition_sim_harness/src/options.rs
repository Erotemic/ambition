//! Construction, timestep, and GGRS rollback options for `Platformer2dSimHarness`.

/// Construction options for `Platformer2dSimHarness`. Builder-style so future
/// knobs (RNG seed, ability set override, debug overlays) drop in
/// without breaking callers that take `Platformer2dSimHarnessOptions::default()`.
#[derive(Clone, Debug, Default)]
pub struct Platformer2dSimHarnessOptions {
    pub timestep: TimestepMode,
    /// Optional starting room id (matches the visible binary's
    /// `--start-room` flag). When `Some`, looked up against
    /// `RoomSet::room_index_by_id`; if not found, a warning is printed
    /// and the LDtk-authored start room stays active.
    pub start_room: Option<String>,
    /// Run the SIM in `FixedUpdate` on `Time<Fixed>` (netcode N0.1) instead of
    /// frame-stepped in `Update`.
    ///
    /// The schedule GRAPH is identical either way — every sim plugin registers
    /// into `SimSchedule` rather than naming a literal — so this flag exists to
    /// prove exactly that: a suite parameterized over both modes is N0.1's exit
    /// check.
    ///
    /// When set, each `Platformer2dSimHarness::step` advances exactly one sim tick: the
    /// frame dt handed to Bevy is pinned to the `Time<Fixed>` timestep, so the
    /// accumulator expends once and only once.
    pub fixed_tick: bool,
    /// Drive the authoritative simulation through GGRS. `SyncTest` is the
    /// deterministic harness mode: GGRS repeatedly saves, rewinds, and
    /// resimulates the real game schedule while comparing checksums.
    pub rollback: RollbackMode,
}

impl Platformer2dSimHarnessOptions {
    /// Builder: set the timestep mode.
    pub fn with_timestep(mut self, timestep: TimestepMode) -> Self {
        self.timestep = timestep;
        self
    }

    /// Builder: set the starting room id.
    pub fn with_start_room(mut self, room_id: impl Into<String>) -> Self {
        self.start_room = Some(room_id.into());
        self
    }

    /// Builder: host the sim in `FixedUpdate` (see [`Self::fixed_tick`]).
    pub fn with_fixed_tick(mut self, fixed_tick: bool) -> Self {
        self.fixed_tick = fixed_tick;
        self
    }

    /// Builder: drive the sim through a GGRS sync-test session.
    pub fn with_sync_test_rollback(mut self) -> Self {
        self.rollback = RollbackMode::SyncTest {
            check_distance: 7,
            max_prediction_window: 12,
            players: 1,
        };
        self.fixed_tick = false;
        self.timestep = TimestepMode::fixed_60hz();
        self
    }

    /// Builder: configure the GGRS sync-test rollback window explicitly.
    pub fn with_sync_test_rollback_settings(
        mut self,
        check_distance: usize,
        max_prediction_window: usize,
    ) -> Self {
        self.rollback = RollbackMode::SyncTest {
            check_distance,
            max_prediction_window,
            players: 1,
        };
        self.fixed_tick = false;
        self.timestep = TimestepMode::fixed_60hz();
        self
    }

    /// Builder: how many SEATS the sync-test session carries. (queue Y1)
    ///
    /// Only meaningful alongside a sync-test mode; on a disabled rollback it is
    /// a no-op rather than an error, because "how many seats would rewind" is
    /// not a question a non-rewinding harness has an answer to.
    pub fn with_rollback_players(mut self, count: usize) -> Self {
        if let RollbackMode::SyncTest { players, .. } = &mut self.rollback {
            *players = count;
        }
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RollbackMode {
    #[default]
    Disabled,
    SyncTest {
        check_distance: usize,
        max_prediction_window: usize,
        /// Seats in the session. (queue Y1)
        ///
        /// One until 2026-07-28, which meant the rollback oracle proved
        /// determinism for ONE input stream while the shipped game seated four.
        /// A second stream is not a bigger version of the first: it is the only
        /// way a desync in seat two's input handling has anywhere to show up.
        players: usize,
    },
}

impl RollbackMode {
    pub fn enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// Per-tick simulation timestep policy.
///
/// `WallClock` is the default — `app.update()` reads whatever wall dt
/// elapsed since the previous update, matching the visible binary's
/// real-time behavior. This is fine for "drive the sim at human pace"
/// use cases (random walker, scripted demo).
///
/// `Fixed { dt }` advances `Time` by exactly `dt` seconds per step
/// before running `Update`. This is what RL training and replay
/// debugging want: identical (action_seq, initial_state) tuples produce
/// identical trajectories regardless of how fast the host machine
/// runs the loop. The default fixed dt of `1.0 / 60.0` matches the
/// visible binary's nominal 60 Hz target.
#[derive(Clone, Copy, Debug, Default)]
pub enum TimestepMode {
    #[default]
    WallClock,
    Fixed {
        dt: f32,
    },
}

impl TimestepMode {
    /// 60 Hz fixed timestep — matches the sandbox's nominal frame rate.
    pub fn fixed_60hz() -> Self {
        TimestepMode::Fixed { dt: 1.0 / 60.0 }
    }

    /// 144 Hz fixed timestep — matches the high-refresh path the
    /// engine repro tests use (`control_dt: 1.0 / 144.0`).
    pub fn fixed_144hz() -> Self {
        TimestepMode::Fixed { dt: 1.0 / 144.0 }
    }
}
