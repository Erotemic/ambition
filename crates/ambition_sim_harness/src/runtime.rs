//! Programmatic Ambition simulation runtime, including direct and GGRS-driven stepping.

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_platformer2d::engine_core as ae;

use ambition_platformer2d::actors::rooms::RoomSet;
use ambition_platformer2d::input::ControlFrame;

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
        options: Platformer2dSimHarnessOptions,
        compose: impl FnOnce(&mut App, &Platformer2dSimHarnessOptions) -> Result<(), String>,
    ) -> Result<Self, String> {
        let mut app = App::new();
        // The shared engine foundation — one definition in ambition_platformer2d::runtime.
        ambition_platformer2d::runtime::add_headless_foundation(&mut app);

        // Netcode N0.1: choose the sim schedule BEFORE the first sim plugin
        // builds (see the doc note above).
        {
            use ambition_platformer2d::runtime::SimulationHostAppExt as _;
            let host = if options.rollback.enabled() {
                ambition_platformer2d::runtime::SimulationHost::Ggrs
            } else if options.fixed_tick {
                ambition_platformer2d::runtime::SimulationHost::Fixed60Hz
            } else {
                ambition_platformer2d::runtime::SimulationHost::RenderFrame
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
                    1_000_000_000u64 / ambition_platformer2d::runtime::SIM_TICK_HZ as u64,
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

        // ⚠ **the SUBJECT, not just the world.** Every caller of this
        // constructor drives a body on the next line; a world without one is a
        // world they cannot use, and the desync canary reported exactly that as
        // `"the sandbox session has a controlled subject"` the first time the
        // harness met a shell-routed host.
        if let Err(budget) =
            ambition_platformer2d::platformer::lifecycle::settle_until_controlled_subject(
                &mut app,
                ambition_platformer2d::platformer::lifecycle::SESSION_SETTLE_FRAMES,
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
            // ⭐ **DECLARE THE SEAT COUNT BEFORE THE SESSION, not after.**
            //
            // `with_rollback_players(n)` is a statement about how many people are
            // playing, and it has to reach the seat topology or nothing else
            // agrees with it. The host that composes this harness freezes a
            // topology from LIVE DEVICES — of which a headless test has none — so
            // it froze one player while the session carried two handles, and
            // seat two's authored frames were written into a `PendingSeatInputs`
            // the topology said did not exist. The symptom was silent: seat two
            // authored forty frames of right and its fighter moved 0.00px.
            //
            // ⚠ **`capture_for_roster`, not `capture`.** The separate entry point
            // exists precisely so that "nobody declared a seat count" cannot look
            // like a decision somebody made — and here somebody did decide, in
            // the harness options.
            //
            // ⛔ this is queue G2's stronger endpoint and NOT its cheap one:
            // *"publish the decided roster topology BEFORE installation, so no
            // mismatch exists to detect."* The cheap remedy — compare the running
            // settings and restart on a mismatch — was probed and removed,
            // because restarting a live session loses the seat→handle binding.
            {
                use ambition_platformer2d::input::{
                    LocalChannelPlan, LocalDeviceOrder, LocalInputSource, LocalSeatTopology,
                };
                let world = app.world_mut();
                let order = LocalDeviceOrder::from_devices(
                    world
                        .get_resource::<LocalDeviceOrder>()
                        .map(|order| order.devices().to_vec())
                        .unwrap_or_default(),
                );
                if let Some(mut topology) = world.get_resource_mut::<LocalSeatTopology>() {
                    // ⚠ **the harness declares the IDENTITY mapping**: seat `n`
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
            ambition_platformer2d::runtime::rollback::start_sync_test_session(
                app.world_mut(),
                ambition_platformer2d::runtime::rollback::SyncTestSettings {
                    check_distance,
                    max_prediction_window,
                    players,
                },
            )
            .map_err(|error| format!("failed to start GGRS sync-test session: {error}"))?;
            app.update();
        } else if options.fixed_tick {
            // Bevy's first frame has `dt == 0`, so the fixed accumulator needs
            // one additional update to execute the same initial simulation tick.
            app.update();
        }

        // ⭐ **K2b.1: settle before handing the harness back.** Every caller of
        // this constructor then reads the world immediately — `room_ids()`, an
        // observation, a `RoomSet` — which works today only because direct entry
        // spawns its root at PLUGIN-BUILD time. A shell-composed host activates
        // asynchronously, so the same read would find nothing until the load
        // barrier reaches `Ready`.
        //
        // ⚠ **best-effort, deliberately, while the build-time root still
        // exists.** It returns `Ok(0)` on every path today, so this changes
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
                    1_000_000_000u64 / ambition_platformer2d::runtime::SIM_TICK_HZ as u64,
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
    /// [`InputStream`](ambition_platformer2d::engine_core::InputStream) records (netcode
    /// N0.2).
    ///
    /// `step` is this plus an `AgentAction → ControlFrame` conversion. A REPLAY
    /// drives this directly: the recorded stream already IS control frames, and
    /// routing them back through `AgentAction` would silently drop every field
    /// that type does not carry.
    /// Author a SECONDARY seat's input for the next step. (queue Y1)
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
        ambition_platformer2d::runtime::rollback::drive_seat_frame(
            self.app.world_mut(),
            ambition_platformer2d::characters::brain::PlayerSlot(slot),
            frame,
        );
    }

    pub fn step_frame(&mut self, frame: ControlFrame) -> AgentObservation {
        // ONE seam, whichever host this harness was built with. The branch that
        // used to be here is the engine's now
        // (`rollback::drive_control_frame`), because every driver that grew its
        // own copy grew the same bug: writing the wrong resource is silently
        // ignored, and the sim simply never moves.
        ambition_platformer2d::runtime::rollback::drive_control_frame(self.app.world_mut(), frame);
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
            .query_filtered::<ambition_platformer2d::engine_core::BodyClusterQueryData, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
        // The published maneuver projection (ADR 0024): the observation's
        // cling/glide/blink flags are semantic facts, not policy internals.
        let mut facts_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::engine_core::BodyMotionFacts, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
        let mut combat_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::characters::actor::BodyCombat, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>(
            );
        let mut health_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::characters::actor::BodyHealth, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
        let mut safety_query = self
            .app
            .world_mut()
            .query_filtered::<&ambition_platformer2d::actors::avatar::PlayerSafetyState, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>(
            );
        // World-side observability (enemies, pickups) for combat /
        // collection assertions. Read once per tick; cheap.
        let mut enemy_query = self.app.world_mut().query::<(
            &ambition_platformer2d::actors::actor::BodyKinematics,
            &ambition_platformer2d::characters::actor::BodyHealth,
        )>();
        // ⚠ IN-WORLD items only. A picked-up item keeps its entity now (it
        // records custody instead of being despawned), so an unfiltered query
        // would report the axe in the agent's own hand as an axe lying on the
        // floor — an instrument agreeing with a state that does not exist.
        let mut pickup_query = self.app.world_mut().query::<(
            &ambition_platformer2d::actors::items::pickup::GroundItem,
            &ambition_platformer2d::actors::items::pickup::ItemCustody,
        )>();

        let world = self.app.world();
        let gravity_dir = world
            .get_resource::<ambition_platformer2d::actors::physics::GravityField>()
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
            .unwrap_or_else(|_| ambition_platformer2d::characters::actor::Health::new(20));
        let room =
            ambition_platformer2d::platformer::lifecycle::session_world_component::<RoomSet>(world)
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
    /// ⛔ **this is the IN-PLACE room reset, not a new game.** The host turns the
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

    fn sync_test_settings(
        &self,
    ) -> Option<ambition_platformer2d::runtime::rollback::SyncTestSettings> {
        match self.rollback {
            RollbackMode::Disabled => None,
            RollbackMode::SyncTest {
                check_distance,
                max_prediction_window,
                players,
            } => Some(ambition_platformer2d::runtime::rollback::SyncTestSettings {
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
        ambition_platformer2d::runtime::rollback::stop_session(self.app.world_mut());
        ambition_platformer2d::runtime::rollback::start_sync_test_session(
            self.app.world_mut(),
            settings,
        )
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

        ambition_platformer2d::runtime::rollback::stop_session(self.app.world_mut());
        ambition_platformer2d::runtime::rollback::start_sync_test_session(
            self.app.world_mut(),
            ambition_platformer2d::runtime::rollback::SyncTestSettings {
                check_distance: 0,
                max_prediction_window: settings.max_prediction_window,
                ..ambition_platformer2d::runtime::rollback::SyncTestSettings::for_players(1)
            },
        )
        .map_err(|error| format!("failed to start GGRS setup frame: {error}"))?;
        self.app.update();
        ambition_platformer2d::runtime::rollback::session_health(self.app.world())
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
    ) -> Option<ambition_platformer2d::runtime::rollback::RollbackExecutionStats> {
        self.app
            .world()
            .get_resource::<ambition_platformer2d::runtime::rollback::RollbackExecutionStats>()
            .copied()
    }

    pub fn rollback_status(
        &self,
    ) -> Option<&ambition_platformer2d::runtime::rollback::RollbackSessionStatus> {
        self.app
            .world()
            .get_resource::<ambition_platformer2d::runtime::rollback::RollbackSessionStatus>()
    }

    /// Return an actionable error if the active GGRS session invalidated its
    /// content/schema contract or the sync-test detected divergent resimulation.
    pub fn rollback_health(&self) -> Result<(), String> {
        ambition_platformer2d::runtime::rollback::session_health(self.app.world())
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

    /// Mutable access to the whole App, so a fixture can INSTALL SYSTEMS on top
    /// of a harness rather than only poke its world.
    ///
    /// ⛔ this exists because the two halves of the couch-multiplayer question
    /// could not be asked in one place. The harness carries a rollback session;
    /// the host carries the device→seat layer (`LocalDeviceOrder`,
    /// `assign_local_seat_devices`, `SeatDeviceOwnership`). With only
    /// [`Self::world_mut`], a test could author seat FRAMES but there were no
    /// `InputParticipant` entities to own a pad — so a disconnect-under-rollback
    /// probe ran green over an empty query and proved nothing at all (`[seat-probe] []`).
    ///
    /// The composition stays in the TEST, not here: this crate must not learn
    /// which host systems a fixture wants, or every consumer inherits that
    /// opinion. `world_mut()` for state, this for wiring.
    ///
    /// ⚠ add systems BEFORE the first [`Self::step`]. `step` runs a full
    /// `app.update()`, and a system added mid-episode starts running against a
    /// world whose rollback baseline was taken without it — which desyncs on the
    /// next rewind rather than failing where you added it.
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
            .resource_mut::<ambition_platformer2d::actors::physics::BaseGravity>();
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
        let mut settings =
            self.app
                .world_mut()
                .resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>();
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
            &mut ambition_platformer2d::actors::features::MotionModel,
        ), ambition_platformer2d::actors::actor::PrimaryPlayerOnly>(
        );
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
            .query_filtered::<&mut ambition_platformer2d::actors::actor::BodyAbilities, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
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
            &mut ambition_platformer2d::actors::actor::BodyAbilities,
            &mut ambition_platformer2d::actors::actor::BodyFlightState,
        ), ambition_platformer2d::actors::actor::PrimaryPlayerOnly>(
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
        brain: ambition_platformer2d::entity_catalog::placements::BossBrain,
    ) {
        self.spawn_boss_at_with(
            id,
            name,
            pos,
            half_size,
            brain,
            ambition_platformer2d::actors::features::BossOverrides::default(),
        );
    }

    /// Like [`Self::spawn_boss_at`] but applies per-spawn "tweaks Z"
    /// ([`BossOverrides`](ambition_platformer2d::actors::features::BossOverrides)): hp /
    /// combat size / phase triggers / encounter opt-out. The refactor's headline
    /// "spawn boss X with tweaks Z at Y and it just works" seam.
    pub fn spawn_boss_at_with(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: ambition_platformer2d::entity_catalog::placements::BossBrain,
        overrides: ambition_platformer2d::actors::features::BossOverrides,
    ) {
        self.app.world_mut().write_message(
            ambition_platformer2d::actors::features::SpawnActorRequest {
                id: id.into(),
                name: name.into(),
                pos: ae::Vec2::new(pos.0, pos.1),
                half_size: ae::Vec2::new(half_size.0, half_size.1),
                // Ignored for the Boss kind (always faction Boss); set for completeness.
                faction: ambition_platformer2d::actors::features::ActorFaction::Boss,
                grudge_against: None,
                kind: ambition_platformer2d::actors::features::SpawnActorKind::Boss {
                    brain,
                    overrides,
                },
            },
        );
        self.run_rollback_setup_frame()
            .expect("boss setup frame establishes a fresh GGRS rollback baseline");
    }

    // ⛔ **`spawn_enemy_at` DELETED 2026-08-14.** Its only difference from
    // `spawn_enemy_character_at` was passing `None` for the character, and
    // `SpawnActorKind::Enemy::character` is required now — a staged request
    // that names no creature is not expressible, so an entry point whose whole
    // purpose was to make one had nothing left to do. It had zero callers.

    pub fn spawn_enemy_character_at(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        pos: (f32, f32),
        half_size: (f32, f32),
        brain: ambition_platformer2d::entity_catalog::placements::CharacterBrain,
        character: &str,
    ) {
        self.app.world_mut().write_message(
            ambition_platformer2d::actors::features::SpawnActorRequest {
                id: id.into(),
                name: name.into(),
                pos: ae::Vec2::new(pos.0, pos.1),
                half_size: ae::Vec2::new(half_size.0, half_size.1),
                faction: ambition_platformer2d::actors::features::ActorFaction::Enemy,
                grudge_against: None,
                kind: ambition_platformer2d::actors::features::SpawnActorKind::Enemy {
                    brain,
                    character: ambition_platformer2d::entity_catalog::CharacterId::from(character),
                },
            },
        );
        self.run_rollback_setup_frame()
            .expect("enemy setup frame establishes a fresh GGRS rollback baseline");
    }

    /// Inject a block into the live sim world (a pogo orb, one-way
    /// platform, solid, …). Used by symmetry tests to place a known
    /// target without authoring a room. Build with `ae::Block::pogo_orb`
    /// / `ae::Block::one_way` / etc.
    pub fn add_block(&mut self, block: ae::Block) {
        ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<
            ambition_platformer2d::engine_core::RoomGeometry,
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
        ambition_platformer2d::platformer::lifecycle::session_world_component::<RoomSet>(
            self.app.world(),
        )
        .expect("active session RoomSet")
        .rooms
        .iter()
        .map(|r| r.id.clone())
        .collect()
    }
}
