//! Programmatic Ambition simulation runtime, including direct and GGRS-driven stepping.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_platformer2d::actor::{
    default_body_size, ActorFaction, BodyAbilities, BodyClusterQueryData, BodyCombat,
    BodyFlightState, BodyHealth, BodyKinematics, BodyMode, BodyMotionFacts, BodySafetyState,
    BossBrain, BossOverrides, Health, MotionModel, PrimaryPlayerOnly, SpawnActorKind,
    SpawnActorRequest, TransitVelocity, transit_body,
};
use ambition_platformer2d::character::{CharacterBrain, CharacterId};
use ambition_platformer2d::engine::{
    add_headless_foundation, SimulationHost, SimulationHostAppExt as _, SIM_TICK_HZ,
};
use ambition_platformer2d::item::{GroundItem, ItemCustody};
use ambition_platformer2d::participant::{
    LocalChannelPlan, LocalDeviceOrder, LocalInputSource, LocalSeatTopology,
};
use ambition_platformer2d::session::{
    session_world_component, session_world_component_mut, settle_until_controlled_subject,
    SESSION_SETTLE_FRAMES,
};
use ambition_platformer2d::settings::UserSettings;
use ambition_platformer2d::sim::{ControlFrame, InputFrameMode, PlayerSlot};
use ambition_platformer2d::world::{
    prelude::{Block, RoomGeometry},
    rooms::RoomSet,
    BaseGravity, GravityField,
};

use crate::action::AgentAction;
use crate::observation::{AgentObservation, EnemyObs, PickupObs};
use crate::options::{Platformer2dSimHarnessOptions, RollbackMode, TimestepMode};

/// A self-contained sandbox simulation, ready to be stepped programmatically.
///
/// Internally this owns a Bevy `App` configured with the same simulation
/// plugins the headless binary uses. Stepping the sim is just writing
/// the converted `ControlFrame` into the resource and calling
/// `app.update()` once.
///
/// `Platformer2dSimHarness` is `Send` because the inner `App` is, but it is not
/// `Sync` — multi-thread RL agents should keep one `Platformer2dSimHarness` per
/// worker thread (or wrap with a Mutex).
pub struct Platformer2dSimHarness {
    app: App,
    tick: u64,
    timestep: TimestepMode,
    rollback: RollbackMode,
}

impl Platformer2dSimHarness {
    /// Build a headless simulation around a caller-supplied game composition.
    ///
    /// The harness installs the shared engine foundation and chooses any fixed
    /// simulation schedule before game plugins register systems. `compose` then
    /// installs game-specific content/simulation and may reject invalid content.
    /// Construction performs the initial update(s) needed before observations are
    /// available in either timestep mode.
    pub fn build(
        options: Platformer2dSimHarnessOptions,
        compose: impl FnOnce(&mut App, &Platformer2dSimHarnessOptions) -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut app = App::new();
        // The shared engine foundation — one definition in ambition_platformer2d::engine.
        add_headless_foundation(&mut app);

        // Netcode N0.1: choose the sim schedule BEFORE the first sim plugin
        // builds (see the doc note above).
        {
            let host = if options.rollback.enabled() {
                SimulationHost::Rollback
            } else if options.fixed_tick {
                SimulationHost::Fixed60Hz
            } else {
                SimulationHost::RenderFrame
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

        // This is what tells Bevy's `time_system` to ignore wall-clock time and advance Time by
        // exactly `dt` per `App::update`. Without this, the Startup tick pulls in the variable
        // wall dt accumulated while `init_sandbox_resources` ran, breaking the determinism
        // contract on tick 0. `Time::advance_by` does not survive Bevy's First-schedule
        // time_system run; the strategy resource is the documented seam for headless /
        // deterministic stepping.
        //
        // Under `fixed_tick` the frame dt must equal the `Time<Fixed>` timestep
        // EXACTLY (same `Duration`, so integer nanos, so no drift): the
        // accumulator then expends precisely once per `app.update()` and one
        // `step()` is one tick — forever, not just for the first few thousand.
        // ⭐⭐ THE CANONICAL PERIOD COMES FROM THE ENGINE, for the two hosts that
        // HAVE one. This spelled out the GGRS integer-nanosecond division and the
        // `Time<Fixed>` lookup itself, which made three places in the repo that
        // knew the same one-nanosecond distinction — and a fourth was written
        // before it was noticed. `ambition_platformer2d::app::manual_step_period`
        // reads `SimulationHost` and answers once.
        //
        // ⛔ THE ARBITRARY-dt PATH IS DELIBERATELY NOT THAT. A caller asking for
        // `TimestepMode::Fixed { dt }` without `fixed_tick` is asking for a
        // DIFFERENT clock on purpose, and folding it into the canonical helper
        // would silently ignore the number it passed.
        if rollback.enabled() || matches!(timestep, TimestepMode::Fixed { .. }) {
            let canonical = if rollback.enabled() || options.fixed_tick {
                ambition_platformer2d::sim::manual_step_period(&app)
            } else {
                None
            };
            let frame_dt = canonical.or_else(|| match timestep {
                TimestepMode::Fixed { dt } => Some(std::time::Duration::from_secs_f32(dt)),
                _ => None,
            });
            if let Some(frame_dt) = frame_dt {
                app.insert_resource(TimeUpdateStrategy::ManualDuration(frame_dt));
            }
        }
        // First update runs Startup. In rollback mode there is deliberately no
        // Session yet, so no simulation frame can advance before the canonical
        // session root and exact content identity exist.
        app.update();

        // the SUBJECT, not just the world. Every caller of this
        // constructor drives a body on the next line; a world without one is a
        // world they cannot use, and the desync canary reported exactly that as
        // `"the sandbox session has a controlled subject"` the first time the
        // harness met a shell-routed host.
        if let Err(budget) =
            settle_until_controlled_subject(
                &mut app,
                SESSION_SETTLE_FRAMES,
            )
        {
            bevy::log::debug!(
                "sim harness: no session world after {budget} frames; the caller's \
                 first world read will be the one that reports it"
            );
        }

        if let RollbackMode::SyncTest {
            check_distance,
            max_prediction_window,
            players,
        } = rollback
        {
            // DECLARE THE SEAT COUNT BEFORE THE SESSION, not after.
            //
            // `with_rollback_players(n)` is a statement about how many people are playing, and
            // it has to reach the seat topology or nothing else agrees with it.
            //
            // `capture_for_roster`, not `capture`. The separate entry point
            // exists precisely so that "nobody declared a seat count" cannot look
            // like a decision somebody made — and here somebody did decide, in
            // the harness options.
            {
                let world = app.world_mut();
                let order = LocalDeviceOrder::from_devices(
                    world
                        .get_resource::<LocalDeviceOrder>()
                        .map(|order| order.devices().to_vec())
                        .unwrap_or_default(),
                );
                if let Some(mut topology) = world.get_resource_mut::<LocalSeatTopology>() {
                    // the harness declares the IDENTITY mapping: seat `n`
                    // plays on pad `n`. A headless run has no devices at all and
                    // drives its seats through the latches, so any plan of the
                    // right SIZE would size the session correctly — but the plan
                    // is also what a seat's device assignment reads, and a
                    // harness that declared something else would be describing a
                    // couch nobody set up.
                    topology.capture_for_roster(
                        &order,
                        LocalChannelPlan::from_sources(
                            (0..players).map(|channel| LocalInputSource::Pad(channel as u8)),
                        ),
                    );
                }
            }
            ambition_platformer2d::rollback::start_sync_test_session(
                app.world_mut(),
                ambition_platformer2d::rollback::SyncTestSettings {
                    check_distance,
                    max_prediction_window,
                    players,
                },
            )
            .map_err(|error| format!("failed to start GGRS sync-test session: {error}"))?;
            app.update();
        } else if options.fixed_tick {
            app.update();
        }

        // A shell-composed host activates asynchronously, so the same read would find nothing
        // until the load barrier reaches `Ready`.
        //
        // best-effort, deliberately, while the build-time root still
        // exists. It returns `Ok(0)` on every path today, so this changes
        // nothing and cannot break a harness whose world genuinely arrives on a
        // later frame under rollback. It becomes a hard error in K2b.2, when the
        // build-time root is deleted and "no world" stops being a possible
        // steady state.
        Ok(Self {
            app,
            tick: 0,
            timestep,
            rollback,
        })
    }

    /// Configure the timestep policy after construction. Installs / removes the
    /// `TimeUpdateStrategy::ManualDuration` resource accordingly.
    pub fn set_timestep(&mut self, timestep: TimestepMode) {
        if self.rollback.enabled() {
            self.timestep = TimestepMode::fixed_60hz();
            self.app.insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_nanos(
                    1_000_000_000u64 / SIM_TICK_HZ as u64,
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
    /// In `WallClock` mode the strategy resource was never installed, so Bevy's default
    /// `Automatic` reads wall-clock dt.
    pub fn step(&mut self, action: AgentAction) -> AgentObservation {
        self.step_frame(action.into())
    }

    /// Step one tick driven by a raw [`ControlFrame`] — the unit an
    /// [`InputStream`](ambition_platformer2d::sim::InputStream) records (netcode
    /// N0.2).
    ///
    /// `step` is this plus an `AgentAction → ControlFrame` conversion. A REPLAY
    /// drives this directly: the recorded stream already IS control frames, and
    /// routing them back through `AgentAction` would silently drop every field
    /// that type does not carry.
    /// Author a SECONDARY seat's input for the next step.
    ///
    /// Seat zero is `step`/`step_frame`; this is every other pad. Call it before
    /// the step it should apply to — it accumulates into the seat's latch (or
    /// its pending input on a driver-authored host), and the step is what hands
    /// the whole set to GGRS.
    ///
    /// Only meaningful when the session actually carries that seat
    /// (`with_rollback_players`). A frame authored for a seat the session does
    /// not hold is written and never asked for, which is inert rather than
    /// wrong — the same as a pad plugged into a one-player game.
    pub fn drive_seat(&mut self, slot: u8, frame: ControlFrame) {
        ambition_platformer2d::rollback::drive_slot_frame(
            self.app.world_mut(),
            PlayerSlot(slot),
            frame,
        );
    }

    pub fn step_frame(&mut self, frame: ControlFrame) -> AgentObservation {
        // ONE seam, whichever host this harness was built with.
        ambition_platformer2d::rollback::drive_control_frame(self.app.world_mut(), frame);
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
            .query_filtered::<BodyClusterQueryData, PrimaryPlayerOnly>();
        // The published maneuver projection (ADR 0024): the observation's
        // cling/glide/blink flags are semantic facts, not policy internals.
        let mut facts_query = self
            .app
            .world_mut()
            .query_filtered::<&BodyMotionFacts, PrimaryPlayerOnly>();
        let mut combat_query = self
            .app
            .world_mut()
            .query_filtered::<&BodyCombat, PrimaryPlayerOnly>(
            );
        let mut health_query = self
            .app
            .world_mut()
            .query_filtered::<&BodyHealth, PrimaryPlayerOnly>();
        let mut safety_query = self
            .app
            .world_mut()
            .query_filtered::<&BodySafetyState, PrimaryPlayerOnly>(
            );
        // World-side observability (enemies, pickups) for combat /
        // collection assertions. Read once per tick; cheap.
        let mut enemy_query = self.app.world_mut().query::<(
            &BodyKinematics,
            &BodyHealth,
        )>();
        // IN-WORLD items only. A picked-up item keeps its entity now (it
        // records custody instead of being despawned), so an unfiltered query
        // would report the axe in the agent's own hand as an axe lying on the
        // floor — an instrument agreeing with a state that does not exist.
        let mut pickup_query = self.app.world_mut().query::<(
            &GroundItem,
            &ItemCustody,
        )>();

        let world = self.app.world();
        let gravity_dir = world
            .get_resource::<GravityField>()
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
            .filter(|(_, custody)| custody.in_world())
            .map(|(g, _)| PickupObs {
                pos: (g.pos.x, g.pos.y),
                id: g.spec.id.clone(),
            })
            .collect();
        let cluster = cluster_query.single(world).ok();
        let facts = facts_query.single(world).ok();
        let health = health_query
            .single(world)
            .map(|h| h.health)
            .unwrap_or_else(|_| Health::new(20));
        let room =
            session_world_component::<RoomSet>(world)
                .expect("active session RoomSet")
                .active_spec();
        let combat = combat_query.single(world).ok();
        let recently_damaged = combat.is_some_and(|c| c.damage_invuln_timer > 0.0);
        let in_hitstun = combat.is_some_and(|c| c.hitstun_timer > 0.0);
        let last_safe_pos = safety_query
            .single(world)
            .map(|s| s.last_safe_pos)
            .unwrap_or(Vec2::ZERO);

        let zero = Vec2::ZERO;
        let default_body = default_body_size();
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
                    .unwrap_or(BodyMode::Standing)
            ),
            active_room: room.id.clone(),
            world_size: (room.world.size.x, room.world.size.y),
            world_spawn: (room.world.spawn.x, room.world.spawn.y),
            last_safe_pos: (last_safe_pos.x, last_safe_pos.y),
            recently_damaged,
            in_hitstun,
            invincible: health.invulnerable.any(),
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
    ///
    /// this is the IN-PLACE room reset, not a new game. The host turns the
    /// pressed edge into `reset_sandbox` plus a room-feature reset: the body
    /// returns to spawn and the room's feature state is restored where it stands.
    /// It does NOT sweep room-scoped entities, empty a hand, wipe the save, or
    /// re-run authored room construction — that is `NewGameResetRequested`, a
    /// different product, requested by its own resource. A test that drives this
    /// and then asserts the room was rebuilt is measuring the wrong road.
    pub fn reset_episode(&mut self) -> AgentObservation {
        self.step(AgentAction::reset());
        self.step(AgentAction::default())
    }

    fn sync_test_settings(&self) -> Option<ambition_platformer2d::rollback::SyncTestSettings> {
        match self.rollback {
            RollbackMode::Disabled => None,
            RollbackMode::SyncTest {
                check_distance,
                max_prediction_window,
                players,
            } => Some(ambition_platformer2d::rollback::SyncTestSettings {
                check_distance,
                max_prediction_window,
                players,
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
        ambition_platformer2d::rollback::stop_session(self.app.world_mut());
        ambition_platformer2d::rollback::start_sync_test_session(self.app.world_mut(), settings)
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

        ambition_platformer2d::rollback::stop_session(self.app.world_mut());
        ambition_platformer2d::rollback::start_sync_test_session(
            self.app.world_mut(),
            ambition_platformer2d::rollback::SyncTestSettings {
                check_distance: 0,
                max_prediction_window: settings.max_prediction_window,
                ..ambition_platformer2d::rollback::SyncTestSettings::for_players(1)
            },
        )
        .map_err(|error| format!("failed to start GGRS setup frame: {error}"))?;
        self.app.update();
        ambition_platformer2d::rollback::session_health(self.app.world())
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
    ) -> Option<ambition_platformer2d::rollback::RollbackExecutionStats> {
        self.app
            .world()
            .get_resource::<ambition_platformer2d::rollback::RollbackExecutionStats>()
            .copied()
    }

    pub fn rollback_status(
        &self,
    ) -> Option<&ambition_platformer2d::rollback::RollbackSessionStatus> {
        self.app
            .world()
            .get_resource::<ambition_platformer2d::rollback::RollbackSessionStatus>()
    }

    /// Return an actionable error if the active GGRS session invalidated its
    /// content/schema contract or the sync-test detected divergent resimulation.
    pub fn rollback_health(&self) -> Result<(), String> {
        ambition_platformer2d::rollback::session_health(self.app.world())
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

    /// Mutable access to the App for fixture-specific system wiring.
    ///
    /// Keep host-specific composition in tests rather than this harness. Install
    /// systems before the first [`Self::step`] so the rollback baseline includes
    /// the complete schedule.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
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
            .resource_mut::<BaseGravity>();
        base.dir = Vec2::new(dir.0, dir.1);
        drop(base);
        self.rebase_after_direct_setup_mutation();
    }

    /// Set the active input-frame mapping mode for scripted control.
    pub fn set_movement_frame_mode(&mut self, mode: InputFrameMode) {
        let mut settings =
            self.app
                .world_mut()
                .resource_mut::<UserSettings>();
        settings.gameplay.movement_frame_mode = mode;
        drop(settings);
        self.rebase_after_direct_setup_mutation();
    }

    /// Teleport the player to `pos` and zero its velocity. Test setup — still a
    /// discrete TRANSIT (ADR 0024 authority): contacts and attachment reconcile
    /// so a scenario cannot start with stale departure facts.
    pub fn teleport_player(&mut self, pos: (f32, f32)) {
        let mut q = self.app.world_mut().query_filtered::<(
            BodyClusterQueryData,
            &mut MotionModel,
        ), PrimaryPlayerOnly>(
        );
        if let Ok((mut cluster_item, mut motion_model)) = q.single_mut(self.app.world_mut()) {
            let mut clusters = cluster_item.as_clusters_mut();
            transit_body(
                &mut motion_model,
                &mut clusters,
                Vec2::new(pos.0, pos.1),
                TransitVelocity::Zero,
            );
        }
        self.rebase_after_direct_setup_mutation();
    }

    /// Grant the player the pogo (down-attack bounce) ability. Test setup.
    pub fn grant_pogo_ability(&mut self) {
        let mut q = self
            .app
            .world_mut()
            .query_filtered::<&mut BodyAbilities, PrimaryPlayerOnly>();
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
            &mut BodyAbilities,
            &mut BodyFlightState,
        ), PrimaryPlayerOnly>(
        );
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
        brain: BossBrain,
    ) {
        self.spawn_boss_at_with(
            id,
            name,
            pos,
            half_size,
            brain,
            BossOverrides::default(),
        );
    }

    /// Like [`Self::spawn_boss_at`] but applies per-spawn "tweaks Z"
    /// ([`BossOverrides`](BossOverrides)): hp /
    /// combat size / phase triggers / encounter opt-out. The refactor's headline
    /// "spawn boss X with tweaks Z at Y and it just works" seam.
    pub fn spawn_boss_at_with(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: BossBrain,
        overrides: BossOverrides,
    ) {
        self.app.world_mut().write_message(
            SpawnActorRequest {
                id: id.into(),
                name: name.into(),
                pos: Vec2::new(pos.0, pos.1),
                half_size: Vec2::new(half_size.0, half_size.1),
                // Ignored for the Boss kind (always faction Boss); set for completeness.
                faction: ActorFaction::Boss,
                grudge_against: None,
                kind: SpawnActorKind::Boss {
                    brain,
                    overrides,
                },
            },
        );
        self.run_rollback_setup_frame()
            .expect("boss setup frame establishes a fresh GGRS rollback baseline");
    }

    // It had zero callers.

    pub fn spawn_enemy_character_at(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: CharacterBrain,
        character: &str,
    ) {
        self.app.world_mut().write_message(
            SpawnActorRequest {
                id: id.into(),
                name: name.into(),
                pos: Vec2::new(pos.0, pos.1),
                half_size: Vec2::new(half_size.0, half_size.1),
                faction: ActorFaction::Enemy,
                grudge_against: None,
                kind: SpawnActorKind::Enemy {
                    brain,
                    character: CharacterId::from(character),
                },
            },
        );
        self.run_rollback_setup_frame()
            .expect("enemy setup frame establishes a fresh GGRS rollback baseline");
    }

    /// Inject a block into the live sim world (a pogo orb, one-way
    /// platform, solid, …). Used by symmetry tests to place a known
    /// target without authoring a room. Build with `Block::pogo_orb`
    /// / `Block::one_way` / etc.
    pub fn add_block(&mut self, block: Block) {
        session_world_component_mut::<
            RoomGeometry,
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
        session_world_component::<RoomSet>(
            self.app.world(),
        )
        .expect("active session RoomSet")
        .rooms
        .iter()
        .map(|r| r.id.clone())
        .collect()
    }
}
