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
    /// Does [`Self::start_room`] have to EXIST?
    ///
    ///  `false` is not an oversight — it is a PROMISE, and there is a test
    /// named after it (`unknown_start_room_does_not_panic_or_error`). A library
    /// caller may legitimately name a room outside the composition it is
    /// building, and falling back to the authored start is the tolerant, correct
    /// answer for that caller. The CLI flag is already strict for the opposite
    /// reason: somebody typed it, just now, meaning that room.
    ///
    /// so this is the CALLER stating which of the two it is, rather than the verb changing
    /// meaning underneath both.
    pub start_room_must_resolve: bool,
    /// The schedule GRAPH is identical either way — every sim plugin registers
    /// into `SimSchedule` rather than naming a literal — so this flag exists to
    /// prove exactly that: a suite parameterized over both modes is N0.1's exit
    /// check.
    pub fixed_tick: bool,
    /// Drive the authoritative simulation through GGRS. `SyncTest` is the
    /// deterministic harness mode: GGRS repeatedly saves, rewinds, and
    /// resimulates the real game schedule while comparing checksums.
    pub rollback: RollbackMode,
    /// Compose the session the way a MATCH experience is composed: with no
    /// home avatar.
    ///
    /// The default (`false`) is an ordinary exploration session, which lowers
    /// the experience's home body. Set it when the harness is going to publish a
    /// `MatchParticipantRoster` with a LOCAL seat in it: the match owns its whole
    /// cast, and a home avatar would be a second claimant on the session's
    /// control channel — which match preparation refuses by name.
    ///
    ///  the composition, not the roster, has to say this. By the time a roster
    /// is published the avatar has already been built.
    pub seats_a_match: bool,
    /// Boot this harness WITH a save file already loaded, the way the binary
    /// does.
    ///
    /// ⭐⭐ THE ORDERING IS THE WHOLE POINT. In the shipped game
    /// `load_save_at_startup` runs in `Startup`, so the file's bytes are in the
    /// world BEFORE the session activates and builds its first room. A test that
    /// writes the save into a running session instead can only ever measure the
    /// correction road; it cannot reach the question of what the first
    /// construction knew. This option inserts `AmbitionGameSave` before the
    /// harness's first `update`, which is the same pair of facts the startup
    /// loader produces: bytes present, `SaveRestored` false.
    pub save: Option<ambition_platformer2d::session::AmbitionGameSaveData>,
}

impl Platformer2dSimHarnessOptions {
    /// Builder: set the timestep mode.
    pub fn with_timestep(mut self, timestep: TimestepMode) -> Self {
        self.timestep = timestep;
        self
    }

    /// Builder: set the starting room id, TOLERANTLY — an id this composition
    /// does not have falls back to the authored start room with a warning.
    /// See [`Self::with_required_start_room`] when you mean it must be there.
    pub fn with_start_room(mut self, room_id: impl Into<String>) -> Self {
        self.start_room = Some(room_id.into());
        self
    }

    /// Builder: require the named starting room to resolve or refuse to boot.
    pub fn with_required_start_room(mut self, room_id: impl Into<String>) -> Self {
        self.start_room = Some(room_id.into());
        self.start_room_must_resolve = true;
        self
    }

    /// Builder: host the sim in `FixedUpdate` (see [`Self::fixed_tick`]).
    pub fn with_fixed_tick(mut self, fixed_tick: bool) -> Self {
        self.fixed_tick = fixed_tick;
        self
    }

    /// Builder: compose the session with no home avatar, because this harness
    /// is going to seat a match into it (see [`Self::seats_a_match`]).
    pub fn seating_a_match(mut self) -> Self {
        self.seats_a_match = true;
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

    /// Builder: boot with a save file already loaded (see [`Self::save`]).
    pub fn with_save(mut self, save: ambition_platformer2d::session::AmbitionGameSaveData) -> Self {
        self.save = Some(save);
        self
    }

    /// Builder: number of seats in a sync-test session. No-op when rollback is
    /// disabled.
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
        /// Number of input seats represented in the sync-test session.
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
/// This is what RL training and replay debugging want: identical (action_seq, initial_state)
/// tuples produce identical trajectories regardless of how fast the host machine runs the loop.
#[derive(Clone, Copy, Debug, Default)]
pub enum TimestepMode {
    #[default]
    WallClock,
    Fixed {
        dt: f32,
    },
}

impl TimestepMode {
    pub fn fixed_60hz() -> Self {
        TimestepMode::Fixed { dt: 1.0 / 60.0 }
    }

    /// 144 Hz fixed timestep — matches the high-refresh path the
    /// engine repro tests use (`control_dt: 1.0 / 144.0`).
    pub fn fixed_144hz() -> Self {
        TimestepMode::Fixed { dt: 1.0 / 144.0 }
    }
}
