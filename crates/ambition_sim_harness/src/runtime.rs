//! Programmatic Ambition simulation runtime, including direct and GGRS-driven stepping.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition::engine_core as ae;

use ambition::actors::rooms::RoomSet;
use ambition::input::ControlFrame;

use crate::action::AgentAction;
use crate::observation::{AgentObservation, EnemyObs, PickupObs};
use crate::options::{RollbackMode, SandboxSimOptions, TimestepMode};

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
    rollback: RollbackMode,
}

impl SandboxSim {
    /// Build a new simulation, composing a caller-supplied game.
    ///
    /// The harness owns the *engine* half: it builds the `App`, adds the shared
    /// headless foundation (`add_headless_foundation`), and — when `fixed_tick`
    /// is set — chooses the sim schedule **before any sim plugin builds** (a
    /// content plugin registers into `SimSchedule` too, so a late choice would
    /// split the sim graph across two schedules, which `set_sim_schedule` panics
    /// on). It then hands the App to `compose`, which installs *that game's*
    /// content + sim plugins (validating the world, inserting any start-room
    /// override, adding the sim assembly), returning an `Err` string on invalid
    /// content — matching the policy that a bad content/world file is a hard
    /// error rather than a silent default.
    ///
    /// This is how the harness composes below the product shell: `ambition_app`
    /// passes its own composition (see `ambition_app::rl_sim::AmbitionSim`); a
    /// demo/test passes a minimal or provider-specific one — neither requires the
    /// harness to know about any particular game.
    ///
    /// The first `app.update()` runs inside `build` so the player entity exists
    /// before the caller's first `observation()` reads it (and a second under
    /// `fixed_tick`, so both timestep modes reach the same one-step-executed state
    /// at construction).
    pub fn build(
        options: SandboxSimOptions,
        compose: impl FnOnce(&mut App, &SandboxSimOptions) -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut app = App::new();
        // The shared engine foundation — one definition in ambition::runtime.
        ambition::runtime::add_headless_foundation(&mut app);

        // Netcode N0.1: choose the sim schedule BEFORE the first sim plugin
        // builds (see the doc note above).
        {
            use ambition::runtime::SimulationHostAppExt as _;
            let host = if options.rollback.enabled() {
                ambition::runtime::SimulationHost::Ggrs
            } else if options.fixed_tick {
                ambition::runtime::SimulationHost::Fixed60Hz
            } else {
                ambition::runtime::SimulationHost::RenderFrame
            };
            app.set_simulation_host(host);
        }

        // Caller-supplied composition: content install + world validation +
        // start-room override + the game's sim plugin(s). A content/world error
        // propagates out as the constructor's `Err`.
        compose(&mut app, &options)?;

        // GGRS owns the simulation cadence. The exact integer-nanosecond period
        // matches bevy_ggrs's accumulator, so one harness update requests one
        // new GGRS frame before any forced resimulation work.
        let rollback = options.rollback;
        let timestep = if rollback.enabled() {
            TimestepMode::fixed_60hz()
        } else {
            options.timestep
        };

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
        if rollback.enabled() {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_nanos(
                    1_000_000_000u64 / ambition::runtime::SIM_TICK_HZ as u64,
                ),
            ));
        } else if let TimestepMode::Fixed { dt } = timestep {
            let frame_dt = if options.fixed_tick {
                app.world()
                    .resource::<bevy::time::Time<bevy::time::Fixed>>()
                    .timestep()
            } else {
                std::time::Duration::from_secs_f32(dt)
            };
            app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_dt));
        }
        // First update runs Startup. In rollback mode there is deliberately no
        // Session yet, so no simulation frame can advance before the canonical
        // session root and exact content identity exist.
        app.update();

        if let RollbackMode::SyncTest {
            check_distance,
            max_prediction_window,
        } = rollback
        {
            ambition::runtime::rollback::start_sync_test_session(
                app.world_mut(),
                ambition::runtime::rollback::SyncTestSettings {
                    check_distance,
                    max_prediction_window,
                },
            )
            .map_err(|error| format!("failed to start GGRS sync-test session: {error}"))?;
            app.update();
        } else if options.fixed_tick {
            // Bevy's first frame has `dt == 0`, so the fixed accumulator needs
            // one additional update to execute the same initial simulation tick.
            app.update();
        }

        Ok(Self {
            app,
            tick: 0,
            timestep,
            rollback,
        })
    }

    /// Configure the timestep policy after construction. Useful for
    /// tests that build a sim, capture an observation, then switch to
    /// fixed-timestep before exercising determinism-sensitive code.
    /// Installs / removes the `TimeUpdateStrategy::ManualDuration`
    /// resource accordingly.
    pub fn set_timestep(&mut self, timestep: TimestepMode) {
        if self.rollback.enabled() {
            self.timestep = TimestepMode::fixed_60hz();
            self.app.insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_nanos(
                    1_000_000_000u64 / ambition::runtime::SIM_TICK_HZ as u64,
                ),
            ));
            return;
        }
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
        if self.rollback.enabled() {
            self.app
                .world_mut()
                .resource_mut::<ambition::runtime::rollback::PendingLocalInput>()
                .0 = frame;
        } else {
            *self.app.world_mut().resource_mut::<ControlFrame>() = frame;
        }
        self.app.update();
        self.tick = self.tick.saturating_add(1);
        self.observation()
    }

    /// Step one frame and return the post-step observation paired with
    /// the example shaped reward ([`crate::reward::default_shaped`])
    /// computed over the pre→post transition. Convenience for RL loops
    /// that want the canonical example reward without threading the
    /// previous observation themselves; a task-specific harness should
    /// compute its own reward from the returned observations instead.
    pub fn step_with_reward(&mut self, action: AgentAction) -> (AgentObservation, f32) {
        let prev = self.observation();
        let cur = self.step(action);
        let reward = crate::reward::default_shaped(&prev, &cur);
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
        // The published maneuver projection (ADR 0024): the observation's
        // cling/glide/blink flags are semantic facts, not policy internals.
        let mut facts_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition::engine_core::BodyMotionFacts, ambition::actors::actor::PrimaryPlayerOnly>();
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
        let facts = facts_query.single(world).ok();
        let health = health_query
            .single(world)
            .map(|h| h.health)
            .unwrap_or_else(|_| ambition::characters::actor::Health::new(20));
        let room = ambition::platformer::lifecycle::session_world_component::<RoomSet>(world)
            .expect("active session RoomSet")
            .active_spec();
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
            wall_clinging: facts.is_some_and(|f| f.wall_clinging),
            wall_climbing: facts.is_some_and(|f| f.wall_climbing),
            facing,
            fast_falling: facts.is_some_and(|f| f.fast_falling),
            fly_enabled: flight.is_some_and(|f| f.fly_enabled),
            gliding: facts.is_some_and(|f| f.gliding),
            dash_charges: dash.map(|d| d.charges_available).unwrap_or(0),
            air_jumps: jump.map(|j| j.air_jumps_available).unwrap_or(0),
            blink_aiming: facts.is_some_and(|f| f.blink_aiming),
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

    fn sync_test_settings(&self) -> Option<ambition::runtime::rollback::SyncTestSettings> {
        match self.rollback {
            RollbackMode::Disabled => None,
            RollbackMode::SyncTest {
                check_distance,
                max_prediction_window,
            } => Some(ambition::runtime::rollback::SyncTestSettings {
                check_distance,
                max_prediction_window,
            }),
        }
    }

    /// Discard rollback history and make the current live world the frame-zero
    /// baseline for the next GGRS step.
    ///
    /// Harness callers must use this after mutating authoritative state through
    /// [`Self::world_mut`]. The typed setup helpers call it automatically. A
    /// mutation that is not represented in GGRS input cannot remain behind the
    /// rollback cursor: resimulating an older frame would correctly omit it.
    pub fn rebase_rollback_history(&mut self) -> Result<(), String> {
        let Some(settings) = self.sync_test_settings() else {
            return Ok(());
        };
        ambition::runtime::rollback::stop_session(self.app.world_mut());
        ambition::runtime::rollback::start_sync_test_session(self.app.world_mut(), settings)
            .map_err(|error| format!("failed to rebase GGRS sync-test history: {error}"))
    }

    /// Execute one setup-only simulation frame without retaining rollback
    /// history, then install a fresh full SyncTest session over the resulting
    /// world. This is for message-driven setup seams such as SpawnActorRequest:
    /// the request is external harness input, while the spawned entity becomes
    /// part of the new frame-zero baseline.
    fn run_rollback_setup_frame(&mut self) -> Result<(), String> {
        let Some(settings) = self.sync_test_settings() else {
            self.app.update();
            return Ok(());
        };

        ambition::runtime::rollback::stop_session(self.app.world_mut());
        ambition::runtime::rollback::start_sync_test_session(
            self.app.world_mut(),
            ambition::runtime::rollback::SyncTestSettings {
                check_distance: 0,
                max_prediction_window: settings.max_prediction_window,
            },
        )
        .map_err(|error| format!("failed to start GGRS setup frame: {error}"))?;
        self.app.update();
        ambition::runtime::rollback::session_health(self.app.world())
            .map_err(|error| format!("GGRS setup frame failed: {error}"))?;
        self.rebase_rollback_history()
    }

    fn rebase_after_direct_setup_mutation(&mut self) {
        self.rebase_rollback_history()
            .expect("valid GGRS settings rebase after harness setup mutation");
    }

    /// True when this harness is driven by an active GGRS session.
    pub fn rollback_enabled(&self) -> bool {
        self.rollback.enabled()
    }

    /// Non-rollback diagnostic counters proving that GGRS performed actual
    /// save/load/resimulation work beneath a harness step.
    pub fn rollback_execution_stats(
        &self,
    ) -> Option<ambition::runtime::rollback::RollbackExecutionStats> {
        self.app
            .world()
            .get_resource::<ambition::runtime::rollback::RollbackExecutionStats>()
            .copied()
    }

    pub fn rollback_status(&self) -> Option<&ambition::runtime::rollback::RollbackSessionStatus> {
        self.app
            .world()
            .get_resource::<ambition::runtime::rollback::RollbackSessionStatus>()
    }

    /// Return an actionable error if the active GGRS session invalidated its
    /// content/schema contract or the sync-test detected divergent resimulation.
    pub fn rollback_health(&self) -> Result<(), String> {
        ambition::runtime::rollback::session_health(self.app.world())
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
    /// invariants the simulation relies on. When rollback is enabled, call
    /// [`Self::rebase_rollback_history`] after any authoritative mutation and
    /// before the next [`Self::step`].
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
        drop(base);
        self.rebase_after_direct_setup_mutation();
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
        drop(settings);
        self.rebase_after_direct_setup_mutation();
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
        self.rebase_after_direct_setup_mutation();
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
        self.rebase_after_direct_setup_mutation();
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
        self.rebase_after_direct_setup_mutation();
    }

    /// Spawn a boss into the live sim at `pos` via [`SpawnActorRequest`], then
    /// step one frame so the spawn command flushes and the entity exists. The
    /// programmatic counterpart to a room `BossSpawn` — scene setup for scenario
    /// tests / RL without authoring an LDtk room.
    ///
    /// `half_size` seeds the kinematic body; a boss whose profile defines
    /// `combat_size` (e.g. the mockingbird) overrides it for the contact box.
    /// `brain` resolves the behavior profile (`BossBrain::PhaseScript { script_id }`
    /// pins it; `Dormant` / `Custom` fall back to `name`). When rollback is
    /// enabled, the setup frame is excluded from history and the spawned world
    /// becomes the next session's frame-zero baseline.
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
        self.run_rollback_setup_frame()
            .expect("boss setup frame establishes a fresh GGRS rollback baseline");
    }

    /// Spawn a normal hostile ENEMY into the live sim at `pos` via the same
    /// [`SpawnActorRequest`] seam room load uses — `ActorFaction::Enemy`,
    /// `hostile_to_player` aggression, `Hostile` disposition. The enemy archetype
    /// + its brain/ActionSet resolve from `brain` (e.g.
    /// `CharacterBrain::Custom("cellular_automaton_fighter")`). Steps one frame so the
    /// spawn command flushes and the entity exists. Counterpart to
    /// [`Self::spawn_boss_at`] for the actor (non-boss) path. Rollback mode
    /// rebases history after this external setup request is materialized.
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
        self.run_rollback_setup_frame()
            .expect("enemy setup frame establishes a fresh GGRS rollback baseline");
    }

    /// Inject a block into the live sim world (a pogo orb, one-way
    /// platform, solid, …). Used by symmetry tests to place a known
    /// target without authoring a room. Build with `ae::Block::pogo_orb`
    /// / `ae::Block::one_way` / etc.
    pub fn add_block(&mut self, block: ae::Block) {
        ambition::platformer::lifecycle::session_world_component_mut::<
            ambition::engine_core::RoomGeometry,
        >(self.app.world_mut())
        .expect("active session RoomGeometry")
        .0
        .blocks
        .push(block);
        self.rebase_after_direct_setup_mutation();
    }

    /// Returns the list of room ids the LDtk project compiled to.
    /// Useful for smoke tests that want to walk every room
    /// (`rl_smoke` binary) or RL training loops that pick a fresh
    /// room per episode.
    pub fn room_ids(&self) -> Vec<String> {
        ambition::platformer::lifecycle::session_world_component::<RoomSet>(self.app.world())
            .expect("active session RoomSet")
            .rooms
            .iter()
            .map(|r| r.id.clone())
            .collect()
    }
}
