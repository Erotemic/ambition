# Simulation tick-rate compatibility audit

**2026-10-26 engine composition scope:** Configurable deterministic fixed-step execution is the
intended non-rollback default, and GGRS is a separately selected synchronization policy. This
audit is still a **plan**: the current runtime still has 60 Hz assumptions, and mechanically
equivalent 30/40/120 Hz compositions have not been established by this document. Cross-platform
P2P determinism is an additional independent test; see [ADR
0036](../../adr/0036-synchronization-is-an-optional-game-capability.md) and [architecture
reassessment](architecture-reassessment-and-composable-multiplayer.md).
**State:** PLANNED. This document is an audit and a change plan. It does not claim that a rate other than 60 Hz works today.

**Source review:** 2026-10-10. The full text search used the supplied `1d668f4a` source archive. Key host, timing, collision, navigation, combat, and presentation sites were checked again against `Erotemic/ambition` `main` at `9c6318bc`. The latter branch has more work than the archive. Before implementation, check each named site at the then-current commit.

**Owner:** Simulation host and its game composition. Each capability owns its own rate-dependent mechanics. This page records the cross-domain plan. `docs/planning/queue.md` remains the execution queue.

## Goal and scope

Ambition must not require a 60 Hz simulation to be correct. It must support a lower rate on devices where simulation CPU cost limits play. It must also allow a higher rate in the future. An offline device may run a different rate from another offline device. Different rates may produce different trajectories, timings within one tick, or game feel. They must not cause broken collisions, missing events, incorrect elapsed-time behavior, or mismatched internal clocks.

**Initial product target:** a 40 Hz fixed-step, non-GGRS, offline Android mode. Compare it with 30 Hz and the existing 60 Hz mode. Treat 120 Hz as a design and test target, not an Android performance goal. Do not promise 60 Hz movement parity at other rates.

**Network rule:** two instances that use different simulation rates may **not** participate in one session. Reject a mismatch *before* they simulate a shared frame. Do not implement mixed-rate rollback, input resampling, or cross-rate determinism in this work. Same-rate netplay retains its existing rollback rules.

**Do not** change the active rate while a gameplay session runs. Do not adapt the rate automatically when frames are slow. A frame-rate cap, sprite quality tier, and simulation rate are separate choices.

### Rate policy

Use an integer-valued immutable rate, such as `SimulationTickRate(NonZeroU32)`. Convert it to `f64` for Bevy's fixed clock and to `f32` only where a physics or authoring API needs that precision. Reject zero and unsupported or absurd values at construction. Keep one owner of the effective period. Do not create a second collection of time conversions in game crates.

The **type can accept an integer**, while the shipping UI and build profiles offer only tested values. Start with `30`, `40`, and `60`; include `120` in engine verification. A rate such as `44` should not need a new enum variant or another code path. Add it to the shipping list only after it passes the same acceptance runs. Default remains 60.

Make the configuration a **construction input** for the engine host. It can come from a platform/build profile or a startup setting, but it must be resolved before plugins register the simulation schedule. Fix rate and host selection together for the session. Prefer a generic `FixedStep` host name over the current `Fixed60Hz` name. A `Rollback` host also has a rate, but it needs no mixed-rate protocol.

Define shared conversion helpers or methods with clear rounding laws:

- **Elapsed gameplay time:** count `WorldTime::sim_dt()` (already scaled by the game's time controls). This is **not** `SimTick / Hz` during pause, bullet time, or hitstop.
- **Unscaled simulation duration:** use the chosen fixed period and `SimTick` if the timer must advance while scaled simulation time is zero. Convert authored seconds to ticks with a documented rounding rule; never use an implicit 60.
- **One-shot gameplay windows:** decide whether to use `ceil(seconds * Hz)` and at least one eligible tick. Test boundaries at every supported rate. Do not shorten a hitbox to zero ticks by rounding down.
- **Input, rollback, and replay identity:** a tick is a whole authoritative decision step. Its duration comes from the session rate. It is not a render frame.
- **Authoring reference frames:** a move described as "six frames at the original 60 Hz design rate" may be stored as **0.1 seconds**. The `60` in that unit conversion is not an active simulation clock and must not be replaced mechanically.

An integer Hz is a rate, not a rational representation of the exact frame period. Use one computed `Duration` for the host's stepping and keep the expected nanosecond rounding explicit. Bevy `FixedUpdate` and GGRS can use different accumulator implementations. Do not assume that calling `app.update()` once always executes one simulation tick.

## Change inventory

**Priority definitions:** P0 blocks correct operation at a new rate or the planned Android host. P1 can change real gameplay in an unintended way or invalidate authoring. P2 is presentation, tooling, or follow-up calibration. These are work packets, **not** new rows in the execution queue.

### P0-1. Give the simulation host one rate authority

**Sites**

- `crates/ambition_platformer2d_runtime/src/lib.rs`: `SIM_TICK_HZ = 60.0`; `SimulationHost::Fixed60Hz`; `Platformer2dSimulationFoundationPlugin`; `PlatformerEnginePlugins::fixed_tick`.
- `crates/ambition_platformer2d/src/app.rs`: `engine_group`, `manual_step_period`, `host_step_period`.
- `crates/ambition_sim_schedule/src/lib.rs`: host/schedule selection and fixed-tick queries.
- `crates/ambition_sim_harness/src/runtime.rs` and `options.rs`: the independent `TimestepMode` and `fixed_tick` selection.
- `crates/ambition_platformer2d_runtime/src/room_transition/loading.rs` and `commit.rs`: matches on host kind and eager versus confirmed publication.
- `crates/ambition_conversation/src/dialog.rs`, `crates/ambition_platformer2d_host/src/lib.rs`, and the other `SimulationHost` consumers: inspect their matches when the enum changes.

**Problem:** the fixed host is named for 60 Hz and inserts `Time::<Fixed>::from_hz(SIM_TICK_HZ)`. Other code converts that same constant to a manual-step period. The harness can also specify a `Fixed { dt }`; that does not by itself prove that the *production fixed schedule* uses the requested rate.

**Change:** choose the fixed-step period once, before installing simulation plugins; pass or read that same immutable rate from host setup, harness construction, and manual stepping. Rename the host kind without adding a second fixed-step schedule. Ensure the ordinary simulation and rollback simulation both call the same registered gameplay systems. Update host-pattern matches without turning the rollback-only confirmation path into an eager path.

**Done when:** the full Ambition game boots through its fixed-step host at 30, 40, 60, and 120 Hz; a harness that requests a rate observes exactly that fixed period; the current 60 Hz configuration keeps its established time semantics.

### P0-2. Make non-GGRS offline Ambition a real product path

**Sites**

- `game/ambition_app/src/app/visible_composition.rs`: currently selects `SimulationHost::Rollback` unconditionally, including Android.
- `game/ambition_app/src/app/plugins.rs`: installs `AmbitionRollbackPlugin` and game-specific rollback declarations only for a rollback host.
- `crates/ambition_platformer2d_host/src/lib.rs`: frame-to-tick `SlotControlLatches` and the fixed-step drain.
- `crates/ambition_platformer2d_runtime/src/input.rs`: input publication for fixed, rollback, and frame-step hosts.
- `crates/ambition_platformer2d_runtime/src/room_transition/{loading,commit}.rs`: eager and confirmed room changes.
- `crates/ambition_platformer2d_runtime/src/external_effects.rs`: uses confirmation only when `ConfirmedFrameBoundary` exists.
- `game/ambition_app/src/host/platform/android.rs`: platform lifecycle and pause/resume.

**Problem:** a single-player session normally has `check_distance = 0`, but it still runs under the GGRS schedule and local-session machinery. A fixed-step engine path exists; the **shipped Ambition composition** has not been accepted on that path as an equal product mode.

**Change:** select `FixedStep` for an explicit offline Android profile. Do not install the GGRS backend, local sync-test owner, or rollback-only state registration on this road. Do not remove saving, checkpoints, session lifetime, item custody, or room transitions. These features must use the existing eager/ordinary host contract. The input latch must deliver one press or release event even when several render frames occur between simulation ticks. It must also avoid duplicating a press when several simulation ticks occur in one render frame.

**Done when:** **at 60 Hz first**, a real offline fixed-host run completes New Game, input, pause/resume, death/checkpoint, room crossing, save/load, session replacement, portal transit, and normal SFX/VFX without needing any GGRS session. Then repeat critical paths at 40 and 30 Hz. Compare Android CPU cost *at the same rate* with and without GGRS before attributing a gain to removing rollback.

### P0-3. Convert hitstop using the selected rate

**Sites**

- `crates/ambition_combat/src/impact_hitstop/mod.rs`: `TICKS_PER_SECOND = 60.0`, `ImpactHitstop::until_tick`, and `request_impact_hitstop_on_resolved_hits`.
- `crates/ambition_combat/src/impact_hitstop/tests.rs` and rollback registration/checksum of `ImpactHitstop`.

**Problem:** this timer uses **unscaled `SimTick`** because a hitstop must be able to end while `WorldTime::sim_dt()` is zero. It multiplies seconds by 60 to choose the expiry tick. At 40 Hz, 100 ms would still use six ticks and last about 150 ms.

**October 11 cross-check:** this remains present in the supplied
`ed0d1ccd` snapshot: `impact_hitstop/mod.rs:57-61` defines
`TICKS_PER_SECOND: f32 = 60.0`. Do **not** open a second hitstop-rate task
under the cosmic-form bug: one concerns conversion of authored seconds to
an expiry tick, the other whether a connect modifier changes the relevant
hitstop authority. They can share tests but not a duplicate rate source.

**Change:** inject the immutable session rate into the conversion. Retain tick-based expiry and the current rule for extending an existing freeze. Define rounding for very short positive hitlag. Keep the expiry fully deterministic under same-rate resimulation.

**Done when:** 100 ms, short positive freezes, overlapping hits, pause, and scaled-time ramp cases have bounded duration error of at most one chosen tick at all target rates. Existing 60 Hz behavior remains the baseline.

### P0-4. Fix rising one-way platform contact, not just its speed limit

**Sites**

- `crates/ambition_platformer2d_world/src/platforms/mod.rs`: `MAX_ONE_WAY_CLIMB = 420.0` px/s with the stated 60 Hz safety reason.
- `crates/ambition_platformer2d_core/src/collision_semantics.rs`: `ONE_WAY_CROSSING_SLOP = 8.0` px.
- `crates/ambition_platformer2d_core/src/movement/{integration.rs,surface_momentum/mod.rs}`: body attachment, support, and rider motion.
- Authored one-way surfaces from `crates/ambition_platformer2d_ldtk`.

**Problem:** a platform rising at 420 px/s travels **7 px at 60 Hz**, **10.5 px at 40 Hz**, and **14 px at 30 Hz**. The current one-way crossing test has 8 px of slack. A live rider may end below the new face and fall through. At 120 Hz, the same travel is 3.5 px.

**Change:** resolve platform/rider contact using the platform's motion over the actual tick, or another shared swept/relative-motion rule. Apply it to all bodies and gravity orientations. Do **not** solve this only by lowering the allowed rise speed on Android; that changes authored world behavior. Do **not** raise a global collision tolerance without checking new false landings from below.

**Done when:** bodies stay attached to ascending, descending, one-way, and ordinary moving supports at 30, 40, 60, and 120 Hz, including high authored speeds and gravity reversal. Tests must also show that a body jumping from below still passes through a one-way platform as designed.

### P0-5. Make discrete world-state detection safe at the longest tick

**Sites**

- `crates/ambition_platformer2d_core/src/world.rs`: `MAX_EXPECTED_BODY_SPEED = 1560.0`; `MIN_STATE_REGION_THICKNESS = speed / 60`; `thin_region_warnings`.
- Readers of `water_at` and `climbable_at` and LDtk geometry validation.
- `crates/ambition_platformer2d_core/src/movement` and abilities that move bodies at high speed.

**Problem:** water and climbable regions are sampled at discrete positions. The documented safe thickness is 26 px at 60 Hz. The same maximum body speed travels **39 px at 40 Hz** and **52 px at 30 Hz**. Some authored regions can be crossed without a sample inside them.

**Change:** validate authored region thickness against the **slowest supported simulation rate** of the content, or make the relevant region entry/exit checks swept. Decide the authoring policy once; it cannot depend on the rate of the developer's workstation. Include the displacement of a moving region if that class is supported. A speed bound is useful only if all admitted movement paths respect it.

**Done when:** a fast body cannot silently skip an admitted water/climbable region at the lowest supported rate, and the validator reports the relevant supported rate and threshold. Retain warnings only where a warning, rather than refusal or a sweep, is the intended policy.

### P0-6. Align navigation rollouts with the real simulation step

**Sites**

- `crates/ambition_platformer2d_world/src/navigation/envelope.rs`: `EnvelopeProbe::default().dt = 1.0 / 60.0`.
- `crates/ambition_platformer2d_world/src/navigation/graph.rs`: `NavGraph::build`, `MAX_LEG_STEPS = 360`, `STALL_STEPS = 15`, rollout costs.
- `crates/ambition_platformer2d_actor_monolith/src/features/ecs/navigation.rs`: `RoomNavigation`, `GraphKey`, and lazy graph construction.
- `crates/ambition_platformer2d_core/src/navigation.rs` and `crates/ambition_characters/src/brain/state_machine/nav_follower.rs`.

**Problem:** the graph uses the real movement kernel but predicts at a fixed 60 Hz step. A body moving at 30, 40, or 120 Hz does not follow exactly those arcs. The cache key records geometry, body, abilities, size, and motion frame, **not rate**. Step-count limits and stall limits also change their duration when rate changes.

**Change:** give graph construction and probes an explicit chosen fixed `dt`; include rate in graph cache identity and invalidation policy. Express rollout horizons and stall budgets as seconds, then derive tick bounds at the requested rate. Preserve a finite work bound to avoid expensive first-use graph builds; measure cost at every selected rate. Do not use the 60 Hz authoring probe as live 40 Hz navigation advice.

**Done when:** an NPC's planned jump/drop/climb/flight leg is validated at the same rate its follower executes. Check landing on the correct support, failed-leg replanning, and graph reuse at different rates. Measure the graph-build cost because 120 Hz can make step-based probing more expensive even when wall-clock horizons are equal.

### P1-1. Audit other predictive movement tools and authoring tests

**Sites**

- `crates/ambition_platformer2d_core/src/movement/recovery.rs`: default `RecoveryProbe` is 240 steps at `1/60` s.
- `crates/ambition_platformer2d_core/src/movement/containment.rs`: `ContainmentProbe::holding` uses 900 steps at `1/60` s.
- `crates/ambition_platformer2d_ldtk/src/conversion/entity_converters/surface_ramp_winding_oracle.rs`: fixed-step movement reference.
- `crates/ambition_world_items/src/containers.rs`: actual falling chest uses `sim_dt`; `settled_chest_center` uses a separate 60 Hz virtual rollout.
- `crates/ambition_combat/src/brain/smash/arena.rs` and other combat/recovery analysis probes.

**Change:** distinguish **pure reference tests** that intentionally specify 60 Hz from **authoring or runtime verdicts** that must predict behavior at the chosen rate. Make the latter accept rate and a horizon in seconds. A pre-settled chest must agree with its live falling path at the selected rate, or explicitly use a rate-independent resting-position solver. Avoid changing test fixtures whose aim is to retain a known 60 Hz reference.

### P1-2. Fix projectile and short-window hit detection where required

**Sites**

- `game/ambition_app/tests/projectile_speed_stays_under_the_swept_threshold.rs`.
- Projectile victim contact in `crates/ambition_projectiles` / `crates/ambition_combat`, and `ambition_geometry::CombatVolume::swept_aabb`.
- `crates/ambition_combat/src/feel.rs`: tests assume an active attack window need only exceed one **60 Hz** frame.
- Authored attack and hurt-volume data, including narrow volumes.

**Problem:** endpoint projectile overlap can miss a victim when per-tick travel exceeds the victim's effective damage width. The existing guard checks authored projectile speed against **28 px times the fixed 60 Hz rate**. At 640 px/s a shot moves 10.7 px at 60 Hz, 16 px at 40 Hz, and 21.3 px at 30 Hz. This does **not** prove every default shot tunnels at 30 Hz: the 28 px reference still exceeds 21.3 px. It does show that the safety margin shrinks, and the guard does not cover every pose-derived narrow hurt volume.

**Change:** run the authored projectile census against the selected/slowest supported rate and the smallest relevant damage geometry. Where it fails, use a proper swept hit test for that path, not a union of endpoint AABBs. Review brief active attack windows, and ensure a valid authored hit window cannot fall between simulation samples at 30/40 Hz. Do not re-time all seconds-based attack data without evidence.

### P1-3. Separate fighter AI time from fighter AI decision ticks

**Sites**

- `crates/ambition_characters/src/brain/fighter/data.rs`: `DEFAULT_TICK_HZ = 60.0`, `DEFAULT_DECISION_INTERVAL_TICKS = 5`, `FighterCfg::new`, `FighterState::new`.
- `crates/ambition_characters/src/perception.rs`: `DelayedPerception::from_reaction_ms(reaction_ms, tick_hz)` already accepts a rate.
- `crates/ambition_combat/src/brain/fighter/{decision,charge,rollout}.rs`: APM, charge, reaction, internal step predictions.
- `crates/ambition_combat/src/brain/fighter/charge.rs`: `FULL_HOLD_TICKS = 90`, `PARTIAL_HOLD_TICKS = 12`, `STRING_CARRY_TICKS = 20`.
- `crates/ambition_platformer2d_actor_spawn/src/actor_spawn/brain_builders/catalog.rs`, `crates/ambition_platformer2d_actor_spawn/src/actor_spawn/brain_builders.rs`: constructed fighter configurations.
- `crates/ambition_characters/src/actor/character_catalog/entry.rs`: default authored decision interval.

**Problem:** fighter reaction delay conversion already handles an input rate, but most constructors default it to 60. A five-tick decision interval means 83 ms at 60 Hz, 125 ms at 40 Hz, and 42 ms at 120 Hz. The hard-coded hold lengths change in the same way. The APM and rollout methods use the `FighterCfg` rate, so a stale value can make AI score against a different timeline than the live body.

**Change:** bind the rate when constructing the authoritative brain configuration, before the perception buffer and its snapshot state are built. Decide which authored fields are intentionally raw ticks and which represent seconds; use rate-derived ticks for the latter. Keep AI choices deterministic within one rate, not equal across rates. Preserve a test-only 60 Hz evaluation rig where it is a deliberate benchmark, but do not let that rig supply the live game's rate.

**Done when:** reaction delay and charge/decision cadence have the intended elapsed-time range at 30, 40, 60, and 120 Hz. AI navigation and attack selection remain able to operate at each rate.

### P1-4. Fix the remaining raw-tick gameplay ceremonies and gestures

**Sites**

- `game/ambition_app/src/app/versus_rules.rs`: `COUNTDOWN_TICKS = 90`, `FIGHT_CALL_TICKS = 30`, and `MatchPhase::Starting`.
- `game/ambition_demo_smash/assets/data/match_rules.ron`: `opening_countdown_ticks: 18` and `time_limit_ticks: 28800`, written in ticks at 60 Hz.
- `crates/ambition_match/src/prepared.rs`: `MatchRules::opening_countdown_ticks`, `opening_phase`, and time-limit rules.
- `crates/ambition_characters/src/actor/attack_gesture.rs`: `flick_window_ticks = 4`, `special_turn_window_ticks = 4`.
- `crates/ambition_combat/src/moveset/mod.rs`: raw-tick gesture and B-reverse windows.

**Problem:** the round/opening countdown is an elapsed time expressed as a count of live simulation ticks. It becomes slower at low rates and faster at high rates. Flick and special-turn windows change their timing too. Those gesture windows affect input feel and can also alter which move the resolver chooses.

**Change:** preserve the *authoritative tick boundary*, but derive its length from named authored seconds and the selected rate. For gestures, either publish rate-derived tick windows at session construction or store elapsed unscaled time with explicit semantics. Avoid storing a duplicate rate-independent and rate-dependent authority for the same window.

**Important exception:** `LiveMatchTicks` in `crates/ambition_match/src/live_match_clock.rs` already accumulates elapsed microseconds and projects them into **conceptual 60 Hz units**. The Smash time limit and match item cadence use that projection. A literal `60` there is a *reference unit*, **not** evidence that the runtime must execute at 60 Hz. Keep that distinction; verify at other rates before altering the match-clock format.

### P1-5. Make rate part of replay and session compatibility

**Sites**

- `crates/ambition_platformer2d_core/src/input_stream.rs`: `InputStream.tick_hz` is already recorded; validation currently checks nonzero and frame continuity.
- `crates/ambition_platformer2d_runtime/src/input_stream.rs`: recorder takes an explicit rate.
- `crates/ambition_platformer2d_rollback_ggrs/src/{lib,session,peer}.rs`: `RollbackFrameRate` and the session builders call `with_fps(SIM_TICK_HZ)`.
- `crates/ambition_platformer2d_runtime/src/rollback/authority.rs`: `RollbackTimelineContract` currently names prepared content and schema, not rate.
- `crates/ambition_sim_harness/src/{runtime,options}.rs`, replay and trace tools, `game/ambition_app/src/app/cli.rs` (`SHARED_HOST_HEADLESS_TICK_HZ = 60.0`).

**Change:** derive each recorder's `tick_hz` from the selected host. A replay must either construct a host at the stream's declared rate or reject the stream **before** simulation. It must never silently replay 60 Hz frames at 40 Hz. Same-rate rollback must configure the GGRS driver, session builder, and any manual stepper consistently. Include rate in the handshake or frozen session compatibility data checked **before frame zero**; incompatible instances must receive a clear error. Do not rely only on the fact that each endpoint independently called `with_fps`.

**Save/load policy:** inventory durable save fields for raw tick deadlines and frame ages. Do not reinterpret a 60 Hz expiry as a 40 Hz expiry. Either convert only well-defined duration fields at a clean save boundary or reject cross-rate resumptions until that conversion is designed. A new session may select a different rate only when its restored mechanics are well-defined. Keep the durable save system independent of ephemeral GGRS snapshots.

### P2-1. Correct presentation effects derived from simulation ticks

**Sites**

- `crates/ambition_sim_view/src/presented_pose.rs`: `NOMINAL_TICK_DT = 1.0 / 60.0` enters the movement-versus-teleport threshold. It must use the selected fixed period.
- `crates/ambition_render/src/rendering/hit_flash.rs`: `BLINK_PERIOD_TICKS = 10`, charge rates `2.0 / 60.0`, `10.0 / 60.0`, `CHARGE_PHASE_WRAP = 3600`.
- `crates/ambition_render/src/rendering/bubble_shield.rs`: `DANGER_PERIOD_TICKS = 8`.
- `crates/ambition_render/src/rendering/dizzy_stars.rs`: `ORBIT_TICKS = 48.0`.
- Presentation camera, effects, and debug overlays that derive phase or age from `SimTick`.

**Change:** use elapsed simulation seconds or a rate-derived phase when the visual frequency should be stable across rates. Preserve discontinuity/teleport suppression, and sample the right presentation phase for fixed-step and GGRS hosts. Visual clocks must not accidentally advance twice because two fixed ticks ran inside one rendered frame.

**Do not confuse clocks:** `COMPOSED_HOLD_FRAMES = 30` in `crates/ambition_render/src/rendering/actors/rigged.rs` counts **rendered frames** for atlas residency. It is a render-FPS issue, not a reason to substitute `SimulationTickRate`. Likewise `HARD_FALL_SHAKE_GAIN = 1/60` in `camera_ease.rs` is a calibrated gain, and `60 / bpm` in audio is seconds per minute. No automated rule should rewrite these.

### P2-2. Make tools, validation, and fixtures state their own clock

**Sites**

- `game/ambition_app/src/app/cli.rs`: `SHARED_HOST_HEADLESS_TICK_HZ` and a separate literal `1.0 / 60.0` in the acceptance runner.
- `crates/ambition_sim_harness/src/options.rs`: helpers for 60/144 Hz and the rollback harness default.
- `game/ambition_app_tools/src/bin/{officer_probe,room_census,fight_discovery}.rs`: explicit 60 Hz fixture periods.
- `game/ambition_demo_smash_app/src/tools/{ladder_probe,ladder_rig,capture_probe}.rs`: convert ticks to seconds with 60; the demo profile scripts assume 60.
- `crates/ambition_combat/src/brain/fighter/evaluation.rs`: intentionally fixed 60 Hz evaluator.
- `crates/ambition_boss_encounter/src/pattern/{validator,content_schema}.rs` and `game/ambition_content/assets/data/boss_validator_bands.ron`: an authored tick-rate conversion for fight validation.
- `scripts/moveset_report.py`, replay fixture names, capture/report metadata, and profiling exports.

**Change:** tools that run the **shipping host** must report the actual rate and derive seconds from it. Tools that intentionally model a 60 Hz reference must name that reference and must not claim to measure the 40 Hz shipping game. Decide whether boss validator bands are a fixed 60 Hz *design reference* or a validation of the target runtime rate; make that contract explicit. Keep existing 60 Hz golden traces as named reference fixtures. Add independent rate-specific evidence rather than overwriting them.

A 120 Hz game with a 30 FPS display will still have four simulation steps per rendered frame. Capture, frame-step, profiling, and moveset tools must label which count they report. No fixed number of outer `app.update()` calls may stand in for a verified count of simulation advances.

## Known safe or intentionally rate-dependent behavior

Do not rewrite a field just because a comment mentions 60 Hz.

- `ambition_time::WorldTime::sim_dt()`, `entity_dt()`, `player_dt()`, and the mirrored `SimDt` already express scaled elapsed time. Most cooldowns, cutscenes, boss phase timers, projectile lifetimes, and animation durations use seconds. Check their callers, not their spelling.
- `ambition_time::PresentationTime` uses the render clock. Do not retime audio, menus, and the outer UI to match a lower simulation rate.
- The existing input latch is intended to bridge render samples and simulation ticks. Its implementation still needs product acceptance at each rate.
- An authored `3.0 / 60.0` second jump-squat or tech window can be an intentional 50 ms/60 Hz design reference. Do not convert it to `3.0 / current_rate` unless the mechanic *explicitly* calls for three live ticks.
- Match clock `micros * 60 / 1_000_000` expresses **conceptual reference frames** and is designed to be independent of the number of steps that elapsed during slowed gameplay. It must not become `micros * current_rate / 1_000_000` without changing the corresponding match rule and item-cadence contract.
- Sprite-sheet `frame_count`, animation frames, `60`-minute music conversion, 60-pixel sizes, and FPS cap settings are not simulation-tick assumptions.
- A lower rate is allowed to produce different deterministic state. Its output must be self-consistent at that rate. Do not assert equal state hashes across 30/40/60/120 Hz.

## Implementation order and acceptance

### Phase A — host and product route, without changing game feel

1. Introduce the integer rate type and one fixed-period authority. Keep 60 Hz as default.
2. Select an offline `FixedStep` route in the actual Ambition visible composition, at **60 Hz**. Do not install GGRS on that route.
3. Run the real single-player lifecycle on both 60 Hz hosts and check input, session, portal, room transition, save, checkpoint, item, audio, and VFX behavior.
4. Profile CPU time and simulation advances on Android with the same scene and settings. This measures the **host** difference, before the rate difference.

### Phase B — mechanical correctness at other rates

1. Fix hitstop and match/versus tick-based durations.
2. Fix moving-platform rider contact and rate-specific state-region/authoring safety.
3. Align navigation and other predictive rollouts with the chosen step; fix graph cache identity and bounded work horizons.
4. Bind fighter AI reaction/decision/hold timing to the selected rate.
5. Verify short combat windows and projectile/hurtbox swept contact at the slowest rate. Correct only the unsafe paths.
6. Verify save/replay mismatch behavior and same-rate GGRS construction.

### Phase C — feel, presentation, and Android selection

1. Correct the presented-pose travel bound and rate-derived VFX pulses.
2. Compare 30, 40, and 60 Hz **with the same 30 FPS render cap and art tier**. Record simulation CPU ms per tick, simulation ticks per render frame, frame-time p50/p95/p99, and catch-up bursts.
3. Check actual game feel: jumping, moving platforms, attack timing, short taps, hits, shield, navigation, boss telegraphs, water/climbing, room crossings, portal transit, and pause/resume.
4. Offer only measured/accepted rates in the Android product profile. Keep 60 Hz as the default elsewhere; a future 120 Hz profile uses the same rate API.

### Minimum behavioral acceptance matrix

| Scenario | 30 Hz | 40 Hz | 60 Hz | 120 Hz |
|---|---|---|---|---|
| Fixed host steps at selected period; input tap consumed once | required | required | baseline | required |
| Rising one-way rider, pass-through from below, gravity reversal | required | required | baseline | required |
| High-speed entry into thin water/climbable region | required | required | baseline | required |
| Projectile versus minimum supported hurt volume; brief active hitbox | required | required | baseline | required |
| Hitstop duration and overlapping hitstop while `sim_dt = 0` | required | required | baseline | required |
| NPC graph predicts a real jump/landing and does not reuse another-rate graph | required | required | baseline | required |
| Fighter reaction/decision/charge duration | required | required | baseline | required |
| Match opening and versus countdown duration | required | required | baseline | required |
| Save/load + room crossing + checkpoint without GGRS | required | required | baseline | useful |
| Smooth pose interpolation and correct teleport suppression | required | required | baseline | required |
| Wrong-rate replay or peer session rejected before advance | required | required | baseline | required |

For every timer whose authoring unit is seconds, use a tolerance of at most **one selected simulation tick** where a discrete step must choose the boundary. For true collision and item/quest outcomes, require the stated semantic result, not a tolerance against the 60 Hz position. Run fixed rates in headless tests; use Android hardware for the performance and rendering verdicts. Do not add heavy pre-commit or pre-push hooks to this work.

### Performance decision

If work per simulation tick were constant, a change from 60 to 40 Hz would remove about one-third of simulation advances; 30 Hz would remove half. Those are **upper-level work-count estimates**, not verified CPU or FPS gains. Physics step cost, catch-up policy, scheduler overhead, navigation construction, render CPU work, and the GGRS host all affect the actual result. A 40 Hz sim at a 30 FPS render target has a noninteger ratio and can perform one or two sim ticks per rendered frame. The fixed-step accumulator and presentation phase must handle that without visible double movement or repeated input.

Measure 40 Hz first. Compare 30 Hz if 40 Hz does not make the device playable. Treat 44 Hz as another candidate, not a special clock architecture. Keep 120 Hz available for future feel experiments, but do not require a slow device to run it.

### Additional 2026-10-11 demo timing witnesses

The Mary-O enemies added after the original census use unscaled `SimTick`
modulo constants in `game/ambition_demo_mary_o/src/void_lizard.rs`
(`ROUND_TICKS = 420`, `THROW_TICKS`, `HOP_TICK`) and
`game/ambition_demo_mary_o/src/swallow.rs` (`DROP_EVERY_TICKS = 150`,
placement-byte-derived phase). Those are 7-second and 2.5-second rhythms
**only at 60 Hz**. `plant.rs` also uses a tick plus placement-derived offset.
Changing the host to 40 Hz without changing these contracts slows the attacks
and changes which actors act together. This does not block a 60 Hz launch,
but it belongs in the P1 gameplay-time conversion and the I4 custom-brain
port, not a separate Mary-O clock implementation.

**Design:** preserve intent in authored seconds or explicit 60 Hz design
frames; resolve the cadence against the one immutable session rate. If an
attack is a discrete once-per-cycle event, use a specified rounding rule and
stable per-occurrence phase. Use a stable occurrence key, not a visible name
that may repeat across live room instances. Do not assume scaled
`WorldTime::sim_dt()` if the attack must proceed through hitstop; state
whether the clock is scaled or unscaled. Module observations must expose the
same contract as native brains. Keep the 60 Hz sequence in the baseline.

**Acceptance:** equivalent elapsed-time attack cadence at 30/40/60/120 Hz,
explicit bounds of one chosen tick, two instances with independent phases,
controlled movement and a hitstop window. Poison: run the 420-tick modulo
unmodified at 40 Hz; the duration test must fail. The portal conductor's
own tick timing must be reviewed under the same rule before it is advertised
as a rate-independent module.

## Exit rule

This work is complete when:

1. No gameplay or authoring system **silently assumes that one authoritative tick is `1/60` second**.
2. The 60 Hz game still behaves as its current baseline.
3. The low-power Android profile uses a real non-GGRS fixed host and passes the core lifecycle and mechanical acceptance cases at its selected lower rate.
4. Replays, saves, and sessions either state a compatible rate or refuse unsupported cross-rate use clearly.
5. Rates are chosen once at construction, and differing-rate peers cannot start a common timeline.
6. The measured Android CPU and frame-time results justify shipping the chosen lower rate.

Do not attempt a global `60 -> configured_hz` replacement. Convert the **meaning** of each timing value: live ticks, elapsed simulation seconds, conceptual 60 Hz design frames, render frames, or plain numeric data.
