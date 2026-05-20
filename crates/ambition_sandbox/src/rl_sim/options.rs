/// Construction options for `SandboxSim`. Builder-style so future
/// knobs (RNG seed, ability set override, debug overlays) drop in
/// without breaking callers that take `SandboxSimOptions::default()`.
#[derive(Clone, Debug, Default)]
pub struct SandboxSimOptions {
    pub timestep: TimestepMode,
    /// Optional starting room id (matches the visible binary's
    /// `--start-room` flag). When `Some`, looked up against
    /// `RoomSet::room_index_by_id`; if not found, a warning is printed
    /// and the LDtk-authored start room stays active.
    pub start_room: Option<String>,
}

impl SandboxSimOptions {
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
    Fixed { dt: f32 },
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
