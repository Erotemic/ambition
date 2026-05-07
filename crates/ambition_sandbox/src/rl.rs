//! Programmatic-input / RL-agent adapter for the sandbox simulation.
//!
//! The sandbox already separates simulation from presentation: the gameplay
//! systems read `Res<ControlFrame>`, and the visible binary's input pipeline
//! is the only thing that writes to it. Headless tests already drive the
//! sim by writing `ControlFrame` directly between `app.update()` calls.
//!
//! `SandboxSim` packages that stepping pattern into a small public API so
//! external drivers — RL agents, fuzz harnesses, scripted-replay tools,
//! Python bindings via PyO3 in the future — can build on top of one
//! shared seam instead of each rolling their own minimal-plugin App
//! boilerplate.
//!
//! Usage from Rust:
//!
//! ```no_run
//! use ambition_sandbox::rl::{AgentAction, SandboxSim};
//!
//! let mut sim = SandboxSim::new().expect("sim builds");
//! let mut action = AgentAction::default();
//! action.move_x = 1.0;
//! action.jump = true;
//! let obs = sim.step(action);
//! println!("after one tick: pos {:?}, on_ground {}", obs.player_pos, obs.on_ground);
//! ```
//!
//! Action / observation shape matches the simulation's existing
//! `ControlFrame` and engine `Player` aggregate so the conversion is
//! lossless and the seam stays narrow. Adding a new action knob means
//! adding a `ControlFrame` field; adding a new observation field means
//! reading another piece of `SandboxRuntime` / engine state out.

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use crate::app::{add_simulation_plugins, init_sandbox_resources};
use crate::game_mode::GameMode;
use crate::input::ControlFrame;
use crate::ldtk_world;
use crate::rooms::RoomSet;
use crate::SandboxRuntime;

/// Action emitted by an RL agent / scripted driver every tick.
///
/// Fields mirror the engine-relevant subset of `ControlFrame` — held vs
/// pressed flags are kept because the sandbox uses both edges (a held
/// jump glides; a pressed jump kicks off the buffered jump path). The
/// `aim_x` / `aim_y` knobs feed precision-blink aim when blink is held.
///
/// Defaults are all-zero / all-false: a `do nothing` action. Constructed
/// fields can be set individually since most agent policies emit a
/// sparse per-frame intent (e.g. just `move_x = 1.0` for "walk right").
#[derive(Clone, Copy, Debug, Default)]
pub struct AgentAction {
    pub move_x: f32,
    pub move_y: f32,
    pub jump: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash: bool,
    pub attack: bool,
    pub blink: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    pub pogo: bool,
    pub interact: bool,
    pub projectile: bool,
    pub projectile_held: bool,
    pub projectile_released: bool,
    pub fly_toggle: bool,
    pub reset: bool,
    pub start: bool,
    pub aim_x: f32,
    pub aim_y: f32,
}

impl AgentAction {
    /// Convenience constructor for tests / agent policies that only set
    /// the horizontal axis.
    pub fn move_x(value: f32) -> Self {
        Self {
            move_x: value,
            ..Self::default()
        }
    }

    /// Convenience: a pressed-this-frame jump with held kept on.
    pub fn jump() -> Self {
        Self {
            jump: true,
            jump_held: true,
            ..Self::default()
        }
    }

    /// Convenience: pressed-this-frame reset.
    pub fn reset() -> Self {
        Self {
            reset: true,
            ..Self::default()
        }
    }
}

impl From<AgentAction> for ControlFrame {
    fn from(a: AgentAction) -> Self {
        ControlFrame {
            axis_x: a.move_x,
            axis_y: a.move_y,
            jump_pressed: a.jump,
            jump_held: a.jump_held,
            jump_released: a.jump_released,
            dash_pressed: a.dash,
            up_pressed: a.move_y < -0.5,
            down_pressed: a.move_y > 0.5,
            fast_fall_pressed: false,
            blink_pressed: a.blink,
            blink_held: a.blink_held,
            blink_released: a.blink_released,
            attack_pressed: a.attack,
            pogo_pressed: a.pogo,
            fly_toggle_pressed: a.fly_toggle,
            interact_pressed: a.interact,
            reset_pressed: a.reset,
            start_pressed: a.start,
            projectile_pressed: a.projectile,
            projectile_held: a.projectile_held,
            projectile_released: a.projectile_released,
            aim_x: a.aim_x,
            aim_y: a.aim_y,
        }
    }
}

/// Per-tick observation surfaced to an RL agent or scripted driver.
///
/// All fields are simple owned types so this struct can be cheaply moved
/// across language boundaries (PyO3, FFI) without lifetime entanglements.
/// Strings (`body_mode`, `active_room`) are owned `String` for the same
/// reason. Add fields here when the agent needs more state — the cost is
/// one or two `world.resource()` reads per tick, which is negligible.
#[derive(Clone, Debug)]
pub struct AgentObservation {
    /// Number of `app.update()` calls since `SandboxSim::new`. The first
    /// observation (after `new()`) returns `tick = 0`; the first `step`
    /// returns `tick = 1`.
    pub tick: u64,
    pub player_pos: (f32, f32),
    pub player_vel: (f32, f32),
    pub player_size: (f32, f32),
    pub on_ground: bool,
    pub on_wall: bool,
    pub wall_clinging: bool,
    pub wall_climbing: bool,
    pub facing: f32,
    pub fast_falling: bool,
    pub fly_enabled: bool,
    pub gliding: bool,
    pub dash_charges: u8,
    pub air_jumps: u8,
    pub blink_aiming: bool,
    pub hp: i32,
    pub hp_max: i32,
    pub mana: i32,
    pub mana_max: i32,
    pub time_alive: f32,
    pub resets: u32,
    pub body_mode: String,
    pub active_room: String,
    pub world_size: (f32, f32),
    pub world_spawn: (f32, f32),
    pub last_safe_pos: (f32, f32),
    /// True if `damage_invuln_timer` is positive — the player took damage
    /// recently. Useful as a sparse negative-reward signal.
    pub recently_damaged: bool,
    /// True while the player is in hitstun. Movement input is reduced
    /// during this window.
    pub in_hitstun: bool,
    /// True if invincibility is on (debug toggle / future invuln frames).
    pub invincible: bool,
    /// True if the player AABB overlaps a water region this frame.
    /// `water_kind` carries `Some("Clear")` / `Some("Murky")` only when
    /// `in_water` is true; cheap one-bit-plus-label encoding lets RL
    /// policies condition on water without a full struct copy.
    pub in_water: bool,
    pub water_kind: Option<String>,
    /// `[0, 1]` how submerged the player is. 0 when not in water.
    pub water_submersion: f32,
    /// True if the player AABB overlaps a climbable region (ladder /
    /// wall / vine) this frame.
    pub on_climbable: bool,
    pub climbable_kind: Option<String>,
}

impl AgentObservation {
    /// Player health fraction in `[0.0, 1.0]`. Returns 0.0 when `hp_max`
    /// is zero (defensive against a future schema change).
    pub fn hp_fraction(&self) -> f32 {
        if self.hp_max <= 0 {
            0.0
        } else {
            (self.hp as f32 / self.hp_max as f32).clamp(0.0, 1.0)
        }
    }

    /// True iff the player is alive (hp > 0). Cheap accessor for reward
    /// shaping.
    pub fn alive(&self) -> bool {
        self.hp > 0
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
#[derive(Clone, Copy, Debug)]
pub enum TimestepMode {
    WallClock,
    Fixed { dt: f32 },
}

impl Default for TimestepMode {
    fn default() -> Self {
        TimestepMode::WallClock
    }
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

/// A self-contained sandbox simulation, ready to be stepped programmatically.
///
/// Internally this owns a Bevy `App` configured with the same simulation
/// plugins the headless binary uses. Stepping the sim is just writing
/// the converted `ControlFrame` into the resource and calling
/// `app.update()` once.
///
/// `SandboxSim` is `Send` because the inner `App` is, but it is not
/// `Sync` — multi-thread RL agents should keep one `SandboxSim` per
/// worker thread (or wrap with a Mutex).
pub struct SandboxSim {
    app: App,
    tick: u64,
    timestep: TimestepMode,
}

impl SandboxSim {
    /// Build a new simulation with the embedded LDtk world and the
    /// default wall-clock timestep. See `new_with_timestep` for fixed-
    /// timestep determinism.
    pub fn new() -> Result<Self, String> {
        Self::new_with_timestep(TimestepMode::default())
    }

    /// Build a new simulation with the embedded LDtk world. Returns an
    /// error string if the LDtk world fails validation — this matches
    /// the policy that an invalid sandbox file is a hard error rather
    /// than a silent default.
    ///
    /// `timestep` controls how `Time` advances between `step` calls.
    /// `WallClock` (default) lets Bevy pick up wall dt; `Fixed { dt }`
    /// pins each step to exactly `dt` seconds for deterministic
    /// trajectories.
    ///
    /// The first `app.update()` is run inside `new()` so the player and
    /// `SandboxRuntime` are spawned before the caller sees an
    /// observation. This makes `sim.observation()` immediately valid.
    pub fn new_with_timestep(timestep: TimestepMode) -> Result<Self, String> {
        let project = ldtk_world::LdtkProject::load_embedded();
        let report = project.validate();
        if !report.is_ok() {
            report.print_to_stderr();
            return Err(format!(
                "embedded LDtk validation failed: {} error(s)",
                report.errors.len()
            ));
        }
        if let Err(errors) = project.to_room_set() {
            return Err(errors.join("; "));
        }

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(ImagePlugin::default());
        app.add_plugins(TransformPlugin);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameMode>();

        init_sandbox_resources(&mut app);
        add_simulation_plugins(&mut app);

        // In Fixed mode, install Bevy's `TimeUpdateStrategy::ManualDuration`
        // BEFORE the first Startup tick. This is what tells Bevy's
        // `time_system` to ignore wall-clock time and advance Time by
        // exactly `dt` per `App::update`. Without this, the Startup tick
        // pulls in the variable wall dt accumulated while
        // `init_sandbox_resources` ran, breaking the determinism
        // contract on tick 0. `Time::advance_by` does not survive
        // Bevy's First-schedule time_system run; the strategy resource
        // is the documented seam for headless / deterministic stepping.
        if let TimestepMode::Fixed { dt } = timestep {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(dt),
            ));
        }
        // First tick runs Startup and primes SandboxRuntime so the
        // caller's first `observation()` returns a populated snapshot.
        app.update();

        Ok(Self {
            app,
            tick: 0,
            timestep,
        })
    }

    /// Configure the timestep policy after construction. Useful for
    /// tests that build a sim, capture an observation, then switch to
    /// fixed-timestep before exercising determinism-sensitive code.
    /// Installs / removes the `TimeUpdateStrategy::ManualDuration`
    /// resource accordingly.
    pub fn set_timestep(&mut self, timestep: TimestepMode) {
        self.timestep = timestep;
        match timestep {
            TimestepMode::Fixed { dt } => {
                self.app.insert_resource(TimeUpdateStrategy::ManualDuration(
                    std::time::Duration::from_secs_f32(dt),
                ));
            }
            TimestepMode::WallClock => {
                self.app.insert_resource(TimeUpdateStrategy::Automatic);
            }
        }
    }

    /// Returns the current timestep policy.
    pub fn timestep(&self) -> TimestepMode {
        self.timestep
    }

    /// Step the simulation forward one frame with the given action.
    /// Returns the post-step observation.
    ///
    /// In `Fixed { dt }` mode, the `TimeUpdateStrategy::ManualDuration`
    /// resource installed in `new_with_timestep` makes Bevy advance
    /// Time by exactly `dt` per `app.update()`. In `WallClock` mode the
    /// strategy resource was never installed, so Bevy's default
    /// `Automatic` reads wall-clock dt.
    pub fn step(&mut self, action: AgentAction) -> AgentObservation {
        *self.app.world_mut().resource_mut::<ControlFrame>() = action.into();
        self.app.update();
        self.tick = self.tick.saturating_add(1);
        self.observation()
    }

    /// Step the simulation `n` times with the same action. Convenience
    /// for "hold this action for N frames" without the caller writing
    /// the loop. Returns the final observation.
    pub fn step_n(&mut self, action: AgentAction, n: u32) -> AgentObservation {
        let mut obs = self.observation();
        for _ in 0..n {
            obs = self.step(action);
        }
        obs
    }

    /// Returns the current observation without advancing the simulation.
    /// Useful for inspecting state mid-episode without burning a tick.
    pub fn observation(&self) -> AgentObservation {
        let world = self.app.world();
        let runtime = world.resource::<SandboxRuntime>();
        let player = &runtime.player;
        let health = &runtime.player_health;
        let room = world.resource::<RoomSet>().active_spec();

        AgentObservation {
            tick: self.tick,
            player_pos: (player.pos.x, player.pos.y),
            player_vel: (player.vel.x, player.vel.y),
            player_size: (player.size.x, player.size.y),
            on_ground: player.on_ground,
            on_wall: player.on_wall,
            wall_clinging: player.wall_clinging,
            wall_climbing: player.wall_climbing,
            facing: player.facing,
            fast_falling: player.fast_falling,
            fly_enabled: player.fly_enabled,
            gliding: player.gliding,
            dash_charges: player.dash_charges_available,
            air_jumps: player.air_jumps_available,
            blink_aiming: player.blink_aiming,
            hp: health.current,
            hp_max: health.max,
            mana: player.mana.current as i32,
            mana_max: player.mana.max as i32,
            time_alive: player.time_alive,
            resets: player.resets,
            body_mode: format!("{:?}", player.body_mode),
            active_room: room.id.clone(),
            world_size: (room.world.size.x, room.world.size.y),
            world_spawn: (room.world.spawn.x, room.world.spawn.y),
            last_safe_pos: (
                runtime.last_safe_player_pos.x,
                runtime.last_safe_player_pos.y,
            ),
            recently_damaged: runtime.damage_invuln_timer > 0.0,
            in_hitstun: runtime.hitstun_timer > 0.0,
            invincible: player.invincible,
            in_water: player.water_contact.is_some(),
            water_kind: player.water_contact.map(|c| format!("{:?}", c.kind)),
            water_submersion: player.water_contact.map(|c| c.submersion).unwrap_or(0.0),
            on_climbable: player.climbable_contact.is_some(),
            climbable_kind: player.climbable_contact.map(|c| format!("{:?}", c.kind)),
        }
    }

    /// Press the in-sim Reset edge for one frame, drains it for the
    /// following frame, and returns the resulting observation. The
    /// existing reset machinery handles teardown of room transitions /
    /// hazards / encounters cleanly; an RL "episode reset" should
    /// usually go through this path rather than rebuilding the App.
    pub fn reset_episode(&mut self) -> AgentObservation {
        self.step(AgentAction::reset());
        self.step(AgentAction::default())
    }

    /// Tick count: number of `step` calls executed.
    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    /// Read-only access to the inner Bevy world for advanced consumers
    /// (custom observation extractors, state assertions). Most agents
    /// should stick to `observation()`; this escape hatch exists so
    /// research code doesn't have to fork the crate to inspect a new
    /// field.
    pub fn world(&self) -> &World {
        self.app.world()
    }

    /// Mutable world access. Useful for test setup ("teleport the
    /// player to position X then step"). Use with care — writing to
    /// gameplay-critical resources mid-episode can violate the
    /// invariants the simulation relies on.
    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_constructs_and_returns_initial_observation() {
        let sim = SandboxSim::new().expect("sim builds");
        let obs = sim.observation();
        assert_eq!(obs.tick, 0, "fresh sim is at tick 0");
        assert!(obs.alive(), "spawned player is alive");
        assert!(obs.hp_max > 0, "max hp populated from data");
        assert!(!obs.active_room.is_empty(), "active room id populated");
    }

    #[test]
    fn idle_step_advances_tick_without_panicking() {
        let mut sim = SandboxSim::new().expect("sim builds");
        let obs = sim.step(AgentAction::default());
        assert_eq!(obs.tick, 1);
    }

    #[test]
    fn step_n_holds_action_across_frames() {
        let mut sim = SandboxSim::new().expect("sim builds");
        let obs = sim.step_n(AgentAction::default(), 30);
        assert_eq!(obs.tick, 30);
        // 30 idle frames should not have killed the player.
        assert!(obs.alive(), "30 idle frames don't kill the player");
    }

    #[test]
    fn move_action_translates_to_horizontal_velocity() {
        let mut sim = SandboxSim::new().expect("sim builds");
        // 10 frames of "walk right". Velocity should pick up positive x;
        // exact magnitude depends on movement tuning so we only assert
        // the sign.
        let obs = sim.step_n(AgentAction::move_x(1.0), 10);
        assert!(
            obs.player_vel.0 > 0.0,
            "after 10 frames of walk-right, vel.x should be positive (got {})",
            obs.player_vel.0
        );
    }

    #[test]
    fn agent_action_to_control_frame_preserves_axes() {
        let action = AgentAction {
            move_x: 0.7,
            move_y: -0.3,
            jump: true,
            ..AgentAction::default()
        };
        let frame: ControlFrame = action.into();
        assert!((frame.axis_x - 0.7).abs() < f32::EPSILON);
        assert!((frame.axis_y + 0.3).abs() < f32::EPSILON);
        assert!(frame.jump_pressed);
        assert!(!frame.jump_held);
    }

    #[test]
    fn fixed_timestep_produces_deterministic_trajectory() {
        // Two sims, same fixed timestep, same action sequence: their
        // player positions must match exactly at every step. This is
        // the foundation for replay debugging and RL training.
        let actions = [
            AgentAction::move_x(1.0),
            AgentAction::jump(),
            AgentAction::move_x(1.0),
            AgentAction::move_x(1.0),
            AgentAction::default(),
            AgentAction::move_x(-1.0),
            AgentAction::move_x(-1.0),
            AgentAction {
                dash: true,
                move_x: -1.0,
                ..AgentAction::default()
            },
            AgentAction::default(),
            AgentAction::default(),
        ];

        let mut sim_a = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).unwrap();
        let mut sim_b = SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).unwrap();
        for (i, action) in actions.iter().enumerate() {
            let a = sim_a.step(*action);
            let b = sim_b.step(*action);
            assert_eq!(
                a.player_pos, b.player_pos,
                "tick {i}: positions diverged ({:?} vs {:?})",
                a.player_pos, b.player_pos
            );
            assert_eq!(
                a.player_vel, b.player_vel,
                "tick {i}: velocities diverged ({:?} vs {:?})",
                a.player_vel, b.player_vel
            );
            assert_eq!(
                a.hp, b.hp,
                "tick {i}: HP diverged ({} vs {})",
                a.hp, b.hp
            );
        }
    }

    #[test]
    fn timestep_setter_round_trips() {
        let mut sim = SandboxSim::new().unwrap();
        assert!(matches!(sim.timestep(), TimestepMode::WallClock));
        sim.set_timestep(TimestepMode::fixed_144hz());
        assert!(matches!(
            sim.timestep(),
            TimestepMode::Fixed { dt } if (dt - 1.0 / 144.0).abs() < 1e-6
        ));
    }

    #[test]
    fn observation_hp_fraction_handles_default() {
        let obs = AgentObservation {
            tick: 0,
            player_pos: (0.0, 0.0),
            player_vel: (0.0, 0.0),
            player_size: (16.0, 32.0),
            on_ground: false,
            on_wall: false,
            wall_clinging: false,
            wall_climbing: false,
            facing: 1.0,
            fast_falling: false,
            fly_enabled: false,
            gliding: false,
            dash_charges: 0,
            air_jumps: 0,
            blink_aiming: false,
            hp: 10,
            hp_max: 20,
            mana: 0,
            mana_max: 100,
            time_alive: 0.0,
            resets: 0,
            body_mode: "Standing".to_string(),
            active_room: "test".to_string(),
            world_size: (256.0, 256.0),
            world_spawn: (0.0, 0.0),
            last_safe_pos: (0.0, 0.0),
            recently_damaged: false,
            in_hitstun: false,
            invincible: false,
            in_water: false,
            water_kind: None,
            water_submersion: 0.0,
            on_climbable: false,
            climbable_kind: None,
        };
        assert!((obs.hp_fraction() - 0.5).abs() < f32::EPSILON);
        assert!(obs.alive());
    }

    #[test]
    fn observation_reports_no_water_no_climbable_in_default_spawn() {
        let sim = SandboxSim::new().expect("sim builds");
        let obs = sim.observation();
        // central_hub_complex spawn has neither water nor climbables.
        assert!(!obs.in_water, "default spawn should not be in water");
        assert_eq!(obs.water_submersion, 0.0);
        assert!(obs.water_kind.is_none());
        assert!(!obs.on_climbable);
        assert!(obs.climbable_kind.is_none());
    }
}
