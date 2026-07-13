use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition::engine_core as ae;

use crate::app::{SandboxSimulationPlugin, StartRoomOverride};
use ambition::actors::ldtk_world;
use ambition::actors::rooms::RoomSet;
use ambition::input::ControlFrame;

use super::action::AgentAction;
use super::observation::{AgentObservation, EnemyObs, PickupObs};
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
            ..SandboxSimOptions::default()
        })
    }

    /// Build a new simulation with full options control. RL training loops that
    /// want to focus on a specific room (e.g. only train on `goblin_encounter`) construct
    /// via this entry point with a `start_room` override. The override matches
    /// the visible binary's `--start-room` flag semantics.
    pub fn new_with_options(options: SandboxSimOptions) -> Result<Self, String> {
        // Sim-entry choke point: install the game's content data (character
        // catalog, worlds) before the catalog build / world load reads them.
        // First-install-wins, same as install_boss_roster. Audio is App-local:
        // `SandboxSimulationPlugin`'s `init_sandbox_resources` registers the
        // audio fragment and reads it from the `AudioCatalogRegistry` resource.
        ambition_content::character_catalog::install();
        ambition_content::worlds::install();
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
        // The shared engine foundation — one definition in ambition::runtime.
        ambition::runtime::add_headless_foundation(&mut app);

        // Netcode N0.1: choose the sim schedule BEFORE the first sim plugin
        // builds. `SandboxSimulationPlugin` adds CONTENT ahead of the engine
        // group, and content registers into `SimSchedule` too — a late choice
        // would split the sim graph across two schedules, which
        // `set_sim_schedule` panics on rather than allow.
        if options.fixed_tick {
            use ambition::platformer::schedule::SimScheduleExt as _;
            app.set_sim_schedule(bevy::app::FixedUpdate);
        }

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
        //
        // Under `fixed_tick` the frame dt must equal the `Time<Fixed>` timestep
        // EXACTLY (same `Duration`, so integer nanos, so no drift): the
        // accumulator then expends precisely once per `app.update()` and one
        // `step()` is one tick — forever, not just for the first few thousand.
        if let TimestepMode::Fixed { dt } = timestep {
            let frame_dt = if options.fixed_tick {
                app.world()
                    .resource::<bevy::time::Time<bevy::time::Fixed>>()
                    .timestep()
            } else {
                std::time::Duration::from_secs_f32(dt)
            };
            app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_dt));
        }
        // First tick runs Startup so the player entity exists before
        // the caller's first `observation()` reads it.
        app.update();
        // Bevy's first frame has `dt == 0`, so under `fixed_tick` the fixed
        // accumulator expends nothing and that Startup frame ran no sim step.
        // One more frame puts a fixed-tick sim in the same state a
        // frame-stepped one reaches at construction: exactly one step executed.
        // Without this, every parameterized suite would be off by one step in
        // one of the two modes.
        if options.fixed_tick {
            app.update();
        }

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
        self.step_frame(action.into())
    }

    /// Step one tick driven by a raw [`ControlFrame`] — the unit an
    /// [`InputStream`](ambition::engine_core::InputStream) records (netcode
    /// N0.2).
    ///
    /// `step` is this plus an `AgentAction → ControlFrame` conversion. A REPLAY
    /// drives this directly: the recorded stream already IS control frames, and
    /// routing them back through `AgentAction` would silently drop every field
    /// that type does not carry.
    pub fn step_frame(&mut self, frame: ControlFrame) -> AgentObservation {
        *self.app.world_mut().resource_mut::<ControlFrame>() = frame;
        self.app.update();
        self.tick = self.tick.saturating_add(1);
        self.observation()
    }

    /// Step one frame and return the post-step observation paired with
    /// the example shaped reward ([`super::reward::default_shaped`])
    /// computed over the pre→post transition. Convenience for RL loops
    /// that want the canonical example reward without threading the
    /// previous observation themselves; a task-specific harness should
    /// compute its own reward from the returned observations instead.
    pub fn step_with_reward(&mut self, action: AgentAction) -> (AgentObservation, f32) {
        let prev = self.observation();
        let cur = self.step(action);
        let reward = super::reward::default_shaped(&prev, &cur);
        (cur, reward)
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
        // Single `BodyClusterQueryData` query covers the 12 cluster
        // components the observation reads. Three sandbox-side
        // components (`BodyCombat`, `BodyHealth`,
        // `PlayerSafetyState`) live outside the engine's cluster
        // bundle and stay on their own queries.
        let mut cluster_query = self
            .app
            .world_mut()
            .query_filtered::<ambition::engine_core::BodyClusterQueryData, ambition::actors::actor::PrimaryPlayerOnly>();
        let mut combat_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition::characters::actor::BodyCombat, ambition::actors::actor::PrimaryPlayerOnly>(
            );
        let mut health_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition::characters::actor::BodyHealth, ambition::actors::actor::PrimaryPlayerOnly>();
        let mut safety_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition::actors::avatar::PlayerSafetyState, ambition::actors::actor::PrimaryPlayerOnly>(
            );
        // World-side observability (enemies, pickups) for combat /
        // collection assertions. Read once per tick; cheap.
        let mut enemy_query = self.app.world_mut().query::<(
            &ambition::actors::actor::BodyKinematics,
            &ambition::characters::actor::BodyHealth,
        )>();
        let mut pickup_query = self
            .app
            .world_mut()
            .query::<&ambition::actors::items::pickup::GroundItem>();

        let world = self.app.world();
        let gravity_dir = world
            .get_resource::<ambition::actors::physics::GravityField>()
            .map(|g| (g.dir.x, g.dir.y))
            .unwrap_or((0.0, 1.0));
        let enemies: Vec<EnemyObs> = enemy_query
            .iter(world)
            .map(|(kin, health)| EnemyObs {
                pos: (kin.pos.x, kin.pos.y),
                hp: health.current(),
                hp_max: health.max(),
                alive: health.alive(),
            })
            .collect();
        let pickups: Vec<PickupObs> = pickup_query
            .iter(world)
            .map(|g| PickupObs {
                pos: (g.pos.x, g.pos.y),
                id: g.spec.id.clone(),
            })
            .collect();
        let cluster = cluster_query.single(world).ok();
        let health = health_query
            .single(world)
            .map(|h| h.health)
            .unwrap_or_else(|_| ambition::characters::actor::Health::new(20));
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
        let pos = cluster.as_ref().map(|c| c.kinematics.pos).unwrap_or(zero);
        let vel = cluster.as_ref().map(|c| c.kinematics.vel).unwrap_or(zero);
        let size = cluster
            .as_ref()
            .map(|c| c.kinematics.size)
            .unwrap_or(default_body);
        let facing = cluster.as_ref().map(|c| c.kinematics.facing).unwrap_or(1.0);
        let water = cluster.as_ref().and_then(|c| c.env_contact.water);
        let climbable = cluster.as_ref().and_then(|c| c.env_contact.climbable);
        let ground = cluster.as_ref().map(|c| &*c.ground);
        let wall = cluster.as_ref().map(|c| &*c.wall);
        let jump = cluster.as_ref().map(|c| &*c.jump);
        let dash = cluster.as_ref().map(|c| &*c.dash);
        let flight = cluster.as_ref().map(|c| &*c.flight);
        let blink = cluster.as_ref().map(|c| &*c.blink);
        let body_mode = cluster.as_ref().map(|c| &*c.body_mode);
        let mana = cluster.as_ref().map(|c| &*c.mana);
        let offense = cluster.as_ref().map(|c| &*c.offense);
        let lifetime = cluster.as_ref().map(|c| &*c.lifetime);
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
                body_mode
                    .map(|b| b.body_mode)
                    .unwrap_or(ae::BodyMode::Standing)
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
            gravity_dir,
            enemies,
            pickups,
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

    /// Set the room's ambient gravity direction (unit vector). `(0, 1)`
    /// is default down; `(0, -1)` inverts to up. Writes [`BaseGravity`],
    /// which `resolve_active_gravity` copies into the live `GravityField`
    /// each frame (so it is the durable, frame-stable invert — poking
    /// `GravityField` directly gets overwritten next tick). Test-only
    /// scaffolding for gravity-symmetry checks.
    pub fn set_base_gravity_dir(&mut self, dir: (f32, f32)) {
        let mut base = self
            .app
            .world_mut()
            .resource_mut::<ambition::actors::physics::BaseGravity>();
        base.dir = ae::Vec2::new(dir.0, dir.1);
    }

    /// Set the active input-frame mapping mode for scripted control.
    ///
    /// `AgentAction` fields are raw input axes. Symmetry/regression tests that
    /// want to drive controlled-body-local directions can set
    /// [`InputFrameMode::BodyRelativeStrict`], making `move_x` / `move_y` mean local
    /// side/down directly. Other tests can select the user-facing modes and
    /// convert local intent through `AccelerationFrame::raw_axis_for_resolved_input`.
    pub fn set_movement_frame_mode(&mut self, mode: ae::InputFrameMode) {
        let mut settings = self
            .app
            .world_mut()
            .resource_mut::<ambition::persistence::settings::UserSettings>();
        settings.gameplay.movement_frame_mode = mode;
    }

    /// Teleport the player to `pos` and zero its velocity. Test setup — still a
    /// discrete TRANSIT (ADR 0024 authority): contacts and attachment reconcile
    /// so a scenario cannot start with stale departure facts.
    pub fn teleport_player(&mut self, pos: (f32, f32)) {
        let mut q = self.app.world_mut().query_filtered::<(
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
        ), ambition::actors::actor::PrimaryPlayerOnly>();
        if let Ok((mut cluster_item, mut motion_model)) = q.single_mut(self.app.world_mut()) {
            let mut clusters = cluster_item.as_clusters_mut();
            ae::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                ae::Vec2::new(pos.0, pos.1),
                ae::movement::TransitVelocity::Zero,
            );
        }
    }

    /// Grant the player the pogo (down-attack bounce) ability. Test setup.
    pub fn grant_pogo_ability(&mut self) {
        let mut q = self
            .app
            .world_mut()
            .query_filtered::<&mut ambition::actors::actor::BodyAbilities, ambition::actors::actor::PrimaryPlayerOnly>();
        if let Ok(mut abilities) = q.single_mut(self.app.world_mut()) {
            abilities.abilities.pogo = true;
        }
    }

    /// Grant the player flight and turn it on. Test / RL setup — the sibling of
    /// [`Self::grant_pogo_ability`]. Free flight needs BOTH the ability flag and
    /// the live `fly_enabled` toggle (see `integrate_flight_clusters`), so this
    /// sets both; nothing in the sim disables `fly_enabled` except the player's
    /// own fly-toggle input, so it persists across steps.
    pub fn grant_flight(&mut self) {
        let mut q = self.app.world_mut().query_filtered::<(
            &mut ambition::actors::actor::BodyAbilities,
            &mut ambition::actors::actor::BodyFlightState,
        ), ambition::actors::actor::PrimaryPlayerOnly>();
        if let Ok((mut abilities, mut flight)) = q.single_mut(self.app.world_mut()) {
            abilities.abilities.fly = true;
            flight.fly_enabled = true;
        }
    }

    /// Spawn a boss into the live sim at `pos` via [`SpawnActorRequest`], then
    /// step one frame so the spawn command flushes and the entity exists. The
    /// programmatic counterpart to a room `BossSpawn` — scene setup for scenario
    /// tests / RL without authoring an LDtk room.
    ///
    /// `half_size` seeds the kinematic body; a boss whose profile defines
    /// `combat_size` (e.g. the mockingbird) overrides it for the contact box.
    /// `brain` resolves the behavior profile (`BossBrain::PhaseScript { script_id }`
    /// pins it; `Dormant` / `Custom` fall back to `name`).
    pub fn spawn_boss_at(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: ambition::entity_catalog::placements::BossBrain,
    ) {
        self.spawn_boss_at_with(
            id,
            name,
            pos,
            half_size,
            brain,
            ambition::actors::features::BossOverrides::default(),
        );
    }

    /// Like [`Self::spawn_boss_at`] but applies per-spawn "tweaks Z"
    /// ([`BossOverrides`](ambition::actors::features::BossOverrides)): hp /
    /// combat size / phase triggers / encounter opt-out. The refactor's headline
    /// "spawn boss X with tweaks Z at Y and it just works" seam.
    pub fn spawn_boss_at_with(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: ambition::entity_catalog::placements::BossBrain,
        overrides: ambition::actors::features::BossOverrides,
    ) {
        self.app
            .world_mut()
            .write_message(ambition::actors::features::SpawnActorRequest {
                id: id.into(),
                name: name.into(),
                pos: ae::Vec2::new(pos.0, pos.1),
                half_size: ae::Vec2::new(half_size.0, half_size.1),
                // Ignored for the Boss kind (always faction Boss); set for completeness.
                faction: ambition::actors::features::ActorFaction::Boss,
                grudge_against: None,
                kind: ambition::actors::features::SpawnActorKind::Boss { brain, overrides },
            });
        self.app.update();
    }

    /// Spawn a normal hostile ENEMY into the live sim at `pos` via the same
    /// [`SpawnActorRequest`] seam room load uses — `ActorFaction::Enemy`,
    /// `hostile_to_player` aggression, `Hostile` disposition. The enemy archetype
    /// + its brain/ActionSet resolve from `brain` (e.g.
    /// `CharacterBrain::Custom("cellular_automaton_fighter")`). Steps one frame so the
    /// spawn command flushes and the entity exists. Counterpart to
    /// [`Self::spawn_boss_at`] for the actor (non-boss) path.
    pub fn spawn_enemy_at(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: ambition::entity_catalog::placements::CharacterBrain,
    ) {
        self.app
            .world_mut()
            .write_message(ambition::actors::features::SpawnActorRequest {
                id: id.into(),
                name: name.into(),
                pos: ae::Vec2::new(pos.0, pos.1),
                half_size: ae::Vec2::new(half_size.0, half_size.1),
                faction: ambition::actors::features::ActorFaction::Enemy,
                grudge_against: None,
                kind: ambition::actors::features::SpawnActorKind::Enemy { brain },
            });
        self.app.update();
    }

    /// Inject a block into the live sim world (a pogo orb, one-way
    /// platform, solid, …). Used by symmetry tests to place a known
    /// target without authoring a room. Build with `ae::Block::pogo_orb`
    /// / `ae::Block::one_way` / etc.
    pub fn add_block(&mut self, block: ae::Block) {
        self.app
            .world_mut()
            .resource_mut::<ambition::engine_core::RoomGeometry>()
            .0
            .blocks
            .push(block);
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
