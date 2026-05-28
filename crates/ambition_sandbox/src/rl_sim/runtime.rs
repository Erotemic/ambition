use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

use ambition_engine as ae;

use crate::app::{SandboxSimulationPlugin, StartRoomOverride};
use crate::game_mode::GameMode;
use crate::input::ControlFrame;
use crate::ldtk_world;
use crate::rooms::RoomSet;

use super::action::AgentAction;
use super::observation::AgentObservation;
use super::options::{SandboxSimOptions, TimestepMode};

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
        Self::new_with_options(SandboxSimOptions::default())
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
    /// The first `app.update()` is run inside `new()` so the player entity
    /// is spawned before the caller sees an observation. This makes
    /// `sim.observation()` immediately valid.
    pub fn new_with_timestep(timestep: TimestepMode) -> Result<Self, String> {
        Self::new_with_options(SandboxSimOptions {
            timestep,
            start_room: None,
        })
    }

    /// Build a new simulation with full options control. RL training loops that
    /// want to focus on a specific room (e.g. only train on `goblin_encounter`) construct
    /// via this entry point with a `start_room` override. The override matches
    /// the visible binary's `--start-room` flag semantics.
    pub fn new_with_options(options: SandboxSimOptions) -> Result<Self, String> {
        let project = ldtk_world::LdtkProject::load_default_for_dev()?;
        let report = project.validate();
        if !report.is_ok() {
            report.print_to_stderr();
            return Err(format!(
                "sandbox LDtk validation failed: {} error(s)",
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

        // Programmatic start-room override: insert before SandboxSimulationPlugin
        // builds (which calls init_sandbox_resources and consumes the override).
        if let Some(room_id) = options.start_room.clone() {
            app.insert_resource(StartRoomOverride(room_id));
        }
        app.add_plugins(SandboxSimulationPlugin);

        // Bind the local in same name the rest of the function uses.
        let timestep = options.timestep;

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
        // First tick runs Startup so the player entity exists before
        // the caller's first `observation()` reads it.
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
    pub fn observation(&mut self) -> AgentObservation {
        // Build per-entity cluster queries. Each one re-uses the
        // `query_filtered::<&...>` shape; once Phase 3 collapses the
        // bridge this can switch to a single `Query<PlayerClusterQueryData>`
        // through a one-shot system.
        let mut kinematics_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerKinematics, With<crate::player::PlayerEntity>>(
            );
        let mut ground_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerGroundState, With<crate::player::PlayerEntity>>(
            );
        let mut wall_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerWallState, With<crate::player::PlayerEntity>>();
        let mut jump_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerJumpState, With<crate::player::PlayerEntity>>();
        let mut dash_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerDashState, With<crate::player::PlayerEntity>>();
        let mut flight_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerFlightState, With<crate::player::PlayerEntity>>(
            );
        let mut blink_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerBlinkState, With<crate::player::PlayerEntity>>(
            );
        let mut body_mode_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerBodyModeState, With<crate::player::PlayerEntity>>(
            );
        let mut env_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerEnvironmentContact, With<crate::player::PlayerEntity>>(
            );
        let mut mana_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerMana, With<crate::player::PlayerEntity>>();
        let mut offense_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerOffense, With<crate::player::PlayerEntity>>();
        let mut lifetime_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerLifetime, With<crate::player::PlayerEntity>>();
        let mut combat_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerCombatState, With<crate::player::PlayerEntity>>(
            );
        let mut health_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerHealth, With<crate::player::PlayerEntity>>();
        let mut safety_query = self
            .app
            .world_mut()
            .query_filtered::<&crate::player::PlayerSafetyState, With<crate::player::PlayerEntity>>(
            );

        let world = self.app.world();
        let kin = kinematics_query.single(world).ok();
        let ground = ground_query.single(world).ok();
        let wall = wall_query.single(world).ok();
        let jump = jump_query.single(world).ok();
        let dash = dash_query.single(world).ok();
        let flight = flight_query.single(world).ok();
        let blink = blink_query.single(world).ok();
        let body_mode = body_mode_query.single(world).ok();
        let env_contact = env_query.single(world).ok();
        let mana = mana_query.single(world).ok();
        let offense = offense_query.single(world).ok();
        let lifetime = lifetime_query.single(world).ok();
        let health = health_query
            .single(world)
            .map(|h| h.health)
            .unwrap_or_else(|_| crate::actor::Health::new(20));
        let room = world.resource::<RoomSet>().active_spec();
        let combat = combat_query.single(world).ok();
        let recently_damaged = combat.is_some_and(|c| c.damage_invuln_timer > 0.0);
        let in_hitstun = combat.is_some_and(|c| c.hitstun_timer > 0.0);
        let last_safe_pos = safety_query
            .single(world)
            .map(|s| s.last_safe_pos)
            .unwrap_or(ae::Vec2::ZERO);

        let zero = ae::Vec2::ZERO;
        let default_body = ae::default_player_body_size();
        let pos = kin.map(|k| k.pos).unwrap_or(zero);
        let vel = kin.map(|k| k.vel).unwrap_or(zero);
        let size = kin.map(|k| k.size).unwrap_or(default_body);
        let facing = kin.map(|k| k.facing).unwrap_or(1.0);
        let water = env_contact.and_then(|e| e.water);
        let climbable = env_contact.and_then(|e| e.climbable);
        AgentObservation {
            tick: self.tick,
            player_pos: (pos.x, pos.y),
            player_vel: (vel.x, vel.y),
            player_size: (size.x, size.y),
            on_ground: ground.is_some_and(|g| g.on_ground),
            on_wall: wall.is_some_and(|w| w.on_wall),
            wall_clinging: wall.is_some_and(|w| w.wall_clinging),
            wall_climbing: wall.is_some_and(|w| w.wall_climbing),
            facing,
            fast_falling: flight.is_some_and(|f| f.fast_falling),
            fly_enabled: flight.is_some_and(|f| f.fly_enabled),
            gliding: flight.is_some_and(|f| f.gliding),
            dash_charges: dash.map(|d| d.charges_available).unwrap_or(0),
            air_jumps: jump.map(|j| j.air_jumps_available).unwrap_or(0),
            blink_aiming: blink.is_some_and(|b| b.aiming),
            hp: health.current,
            hp_max: health.max,
            mana: mana.map(|m| m.meter.current as i32).unwrap_or(0),
            mana_max: mana.map(|m| m.meter.max as i32).unwrap_or(0),
            time_alive: lifetime.map(|l| l.time_alive).unwrap_or(0.0),
            resets: lifetime.map(|l| l.resets).unwrap_or(0),
            body_mode: format!(
                "{:?}",
                body_mode.map(|b| b.body_mode).unwrap_or(ae::BodyMode::Standing)
            ),
            active_room: room.id.clone(),
            world_size: (room.world.size.x, room.world.size.y),
            world_spawn: (room.world.spawn.x, room.world.spawn.y),
            last_safe_pos: (last_safe_pos.x, last_safe_pos.y),
            recently_damaged,
            in_hitstun,
            invincible: offense.is_some_and(|o| o.invincible),
            in_water: water.is_some(),
            water_kind: water.map(|c| format!("{:?}", c.kind)),
            water_submersion: water.map(|c| c.submersion).unwrap_or(0.0),
            on_climbable: climbable.is_some(),
            climbable_kind: climbable.map(|c| format!("{:?}", c.kind)),
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

    /// Returns the list of room ids the LDtk project compiled to.
    /// Useful for smoke tests that want to walk every room
    /// (`rl_smoke` binary) or RL training loops that pick a fresh
    /// room per episode.
    pub fn room_ids(&self) -> Vec<String> {
        self.app
            .world()
            .resource::<RoomSet>()
            .rooms
            .iter()
            .map(|r| r.id.clone())
            .collect()
    }
}
