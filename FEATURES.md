# Ambition FEATURES

> Catalog of landed gameplay/system features. Pair with `TODO.md` (what's
> outstanding) and `docs/CURRENT_STATE.md` (architectural snapshot).
>
> Source of truth: working tree + git history. When a feature lands, add it
> here in the same commit as its docs.
>
> Last regenerated: 2026-05-07 (full re-audit against `git log --all`)

## How to read this
Each entry: feature name — what it does — where it lives — landing commits.
Status badges:
- `[stable]` — feature-complete, has tests, used in sandbox
- `[playable]` — works end-to-end but rough edges
- `[scaffolded]` — module + types exist; not yet expressive

---

## Sim → presentation message channels

- **ADR 0012 events refactor — Slices 1-5** `[stable]` — `sandbox_update` and helpers contain ZERO direct `play_sound` / `spawn_burst` / `spawn_dust` / `spawn_impact` / `spawn_blink_effects` / `spawn_slash_preview` / `spawn_reset_effects` / `physics::spawn_debris_burst` calls. They emit typed messages (`SfxMessage` / `VfxMessage` / `DebrisBurstMessage`) which presentation-side systems (`audio_play_sfx_messages`, `vfx_spawn_messages`, `physics_spawn_debris_messages`) consume. `bin/headless` runs `sandbox_update` end-to-end via `run_headless`. Multi-frame `tests/scripted_gameplay.rs` integration test pins the seam under MinimalPlugins. See `docs/events_refactor_plan.md`.
- **`PlayerDiedMessage` (Bevy 0.18 Message API)** `[stable]` — replaces the previous `SandboxRuntime::player_died_pending` bool. `death_respawn_player` pushes into `FrameFeedback.died`; `flush_feedback` drains into `MessageWriter`; `update_encounters_from_world` reads `MessageReader<PlayerDiedMessage>`. Keeps the runtime resource a pure state store. [lib.rs:67](crates/ambition_sandbox/src/lib.rs#L67), [app.rs:2172](crates/ambition_sandbox/src/app.rs#L2172).
- **`SfxMessage` / `VfxMessage` / `DebrisBurstMessage`** `[stable]` — Vec-collector → MessageWriter pattern documented in `docs/events_refactor_plan.md`. [audio.rs:53](crates/ambition_sandbox/src/audio.rs#L53), [fx.rs](crates/ambition_sandbox/src/fx.rs).

## Engine primitives (source-agnostic)

- **`World::water_at` + `WaterContact`** `[stable]` — single query API for "is the player in water?"; movement caches `Player.water_contact` once per tick. Source-agnostic: backend is unchanged whether the water came from LDtk IntGrid, an LDtk entity, or generated content. [world.rs](crates/ambition_engine/src/world.rs).
- **`World::climbable_at` + `ClimbableContact` + `Player.climbable_contact`** `[stable]` — mirror of the water pattern for ladders / walls / vines. `ClimbableKind::{Ladder, Wall, Vine}` for sprite/sfx branching, `ClimbableSpec { climb_speed, strafe_factor }` for per-region tuning, default speed 180 px/sec (slower than walk so climbing reads deliberate). Engine-side primitive plus `Player.climbable_contact` populated once per tick. [world.rs](crates/ambition_engine/src/world.rs), [movement.rs](crates/ambition_engine/src/movement.rs).
- **Ladders pass through overlapping solid blocks** `[stable]` — engine rule: while `body_mode == Climbing` AND `climbable_contact.is_some()`, blocks whose aabb overlaps the active climbable region are passable. Hazards stay dangerous. Generalizes the room-author trick of "carve a gap in the platform where the ladder ends" so future ladder authors don't have to remember it. [movement.rs](crates/ambition_engine/src/movement.rs) `block_passable_during_climb`.
- **Ladder traversal end-to-end (BodyMode::Climbing + integrate_climb + sandbox driver)** `[playable]` — engine consumes the contact in `integrate_climb` (gravity suspended, vel.y = axis_y * climb_speed, vel.x = axis_x * climb_speed * strafe_factor). Sandbox-side body-mode driver enters Climbing on Up + climbable_contact (Up grounded works as "step onto ladder from below"; Down only enters from airborne so grounded crouch isn't stolen). Exits on Jump (push off) or losing contact. [body_mode.rs](crates/ambition_sandbox/src/body_mode.rs).
- **LDtk `Climbable` IntGrid layer** `[stable]` — authoring path: paint cells on a layer named "Climbable" with values 1=Ladder, 2=Vine, 3=Wall. Runtime lowers them via `emit_climbable_regions_from_intgrid` into the same source-agnostic `World::climbable_regions` Vec the engine queries. Cell runs merge into rectangles via the existing `merge_intgrid_rects` helper (shared with the Water IntGrid path), so a 1×N or M×1 painted ladder becomes one region rather than N separate cells. Layer is optional — rooms without ladders just skip the layer. Four unit tests pin the value→kind mapping, the unknown-value error, and the empty-layer no-op. [ldtk_world.rs:483](crates/ambition_sandbox/src/ldtk_world.rs#L483).

## Engine — physics & player state

- **Wall-jump OOB fix via `body_is_side_contact`** `[stable]` — guards `sweep_player_y` from snap-direction tunneling on edge-touching walls. [movement.rs:1321](crates/ambition_engine/src/movement.rs#L1321). Commit `4002b4d` (Catch edge-touching wall side-contact in y-sweep).
- **`BodyMode` state machine** `[stable]` — Standing / Crouching / Crawling / Sliding / MorphBall drivers with per-mode `BodyShape`. [player_state.rs:145](crates/ambition_engine/src/player_state.rs#L145). Commits `dafa5a4`, `5899c2c`.
- **`ResourceMeter`** `[stable]` — generic stamina/mana/ammo/charge meter; used by projectile fireball mana. [player_state.rs:330](crates/ambition_engine/src/player_state.rs#L330).
- **Ledge grab + swim as opt-in abilities** `[playable]` — `Ability` flags gate behavior; sandbox-side state still being promoted. Commit `70fc567`.
- **Glide / slow-fall ability** `[stable]` — held-jump while airborne caps fall speed at `glide_fall_speed` (220, vs `max_fall_speed` 1040) and boosts air accel to `glide_air_accel` (6200, vs `air_accel` 4700). Cancels on land / fast-fall / blink-hang / water. `Player::gliding` flag drives sandbox sprite/sfx hooks. [movement.rs](crates/ambition_engine/src/movement.rs).
- **`evaluate_character_ai` vocabulary** `[stable]` — engine fn returning `CharacterAiMode`; per-brain knobs (`chase_speed`, `aggro_radius`) on archetypes. [character_ai.rs:98](crates/ambition_engine/src/character_ai.rs#L98), [features.rs:1233](crates/ambition_sandbox/src/features.rs#L1233). Commit `93b4e05`.
- **`Player::damage_multiplier` + `Player::invincible`** `[stable]` — promoted from `SandboxRuntime` to engine-side per-player state. Outgoing slash damage scales by `damage_multiplier`; incoming damage drops if `invincible`. F3 stats editor writes these directly on the player. Survives reset (settings preserved per tester intent). [movement.rs](crates/ambition_engine/src/movement.rs).
- **`Player::mana: ResourceMeter`** `[stable]` — promoted from sandbox `SandboxRuntime::mana_current/max` (i32) to the existing engine `ResourceMeter` type (f32 with regen/decay/try_spend). F3 inspector still surfaces i32; conversion happens at the editor boundary. Reset refills via `refill_full`. [movement.rs](crates/ambition_engine/src/movement.rs), [player_state.rs:330](crates/ambition_engine/src/player_state.rs#L330).

## Boss patterns & traversal

- **`BossPatternSchedule` + `BossAttackKind` + `BossPatternStep`** `[stable]` — reviewable timed-attack data: phase id, seed, list of (telegraph / active / recover) steps. `gradient_sentinel_phase1` and `_phase2` ship as authored examples; `is_valid()` + `total_time()` + `summary()` complete the API. [boss_patterns.rs](crates/ambition_engine/src/boss_patterns.rs).
- **`BossMovementKind` + `ArenaAnchor` + `gradient_sentinel_phase3_traversal`** `[stable]` — adds traversal choreography to the boss schedule data. `BossMovementKind::{Hold, Dash, Reposition, Orbit}` pairs with each attack step via `BossPatternStep::with_movement(...)`. `ArenaAnchor::{Center, LeftWall, RightWall, TopLeft, TopRight, BottomLeft, BottomRight}` keeps reposition steps arena-agnostic — the controller resolves anchors against the live arena's authored coordinates so the same pattern works in small and large boss rooms. Phase 3 showcase pattern uses all four movement kinds. Bevy-side controller interpretation is a separate follow-up. [boss_patterns.rs](crates/ambition_engine/src/boss_patterns.rs).

## Combat & projectiles

- **Player damage flow** `[stable]` — HP drain, entity-hazard knockback, max HP 20. Commits `bc85740`, `cc208ee`.
- **Charge fireballs** `[stable]` — hold-to-charge, multi-tier release. Commit `e71afd8`.
- **Hadouken motion-input** `[stable]` — quarter-circle + fire upgrades fireball; grace-input window; super Hadouken via full motion. Commits `ba1ed49`, `e71afd8`, `577aaa8`.
- **Unified damage path + projectile bounce + projectile-actor damage** `[stable]` — single damage funnel for melee/projectile/hazards. Commit `11ae11d`.
- **Fireball split off attack on gamepad** `[stable]` — separate inputs. Commit `643fd63`.

## Enemies, NPCs, AI

- **Patrolling NPCs with gravity** `[playable]` — uses shared `character_ai` vocabulary. Commit `93b4e05`.
- **Hostile NPCs** `[playable]` — NPCs convertible to enemies; death persisted; semantic boss IDs. Commits `75ebfcb`, `70fc567`, `f8a75b6`.
- **NPCs flip hostile on N strikes** `[stable]` — `NPC_HOSTILE_STRIKE_THRESHOLD = 3` (configurable per-NPC future hook). `NpcRuntime::strikes` accumulates per hit; crossing the threshold sets `hostile = true` + writes `npc_<id>_hostile` save flag. `apply_save` then replaces the `NpcRuntime` with an `EnemyRuntime` carrying the same id, so AI inheritance comes for free. Already-hostile NPCs die at 2× the threshold. [features.rs:2413](crates/ambition_sandbox/src/features.rs#L2413).
- **Enemy archetype S/M/L × aggression matrix (partial)** `[playable]` — `EnemyArchetype` enum carries 7 combat archetypes (Combatant baseline + Small{Skitter, Lurker} + Medium{Striker} + Large{Brute, Colossus} + AggressiveSeeker) plus 2 sandbag training variants. Per-archetype tuning matches (max_health / patrol_speed / chase_speed / aggro_radius / attack_range / contact_strength / damage_amount) live in [features.rs:1196](crates/ambition_sandbox/src/features.rs#L1196). Cross-archetype invariants tested: HP grows with size, aggro grows with aggression, damage scales with size. LDtk authors select via `EnemySpawn::brain` (e.g. `small_lurker`, `large_colossus`).
- **Boss phase machine** `[stable]` — `BossPhase` enum (intro / phase1 / phase2 / Stagger / Enraged); stagger threshold + window; per-phase tunables. [boss_encounter.rs:35](crates/ambition_engine/src/boss_encounter.rs#L35). Commit `06ea438`.
- **Boss encounter integration test** `[stable]`. Commit `07df47b`.

## Hazards / breakables

- **`BreakableTrigger::{OnHit, OnStand, Either}`** `[stable]` — typed authored knob replaces an earlier magic-string check. Stand-to-crumble accumulates a `stand_timer` per breakable; threshold is `BREAK_ON_STAND_SECONDS = 0.85`. Decays at 2× rate when the player isn't standing on it. [interaction.rs:128](crates/ambition_engine/src/interaction.rs#L128), [features.rs:577](crates/ambition_sandbox/src/features.rs#L577).

## Encounter / wave system (mob_lab)

- **mob_lab room** `[playable]` — LDtk area with EncounterTrigger / Switch defs; hallway threshold → lock-wall slam → camera zoom → music swap → 3 waves with delayed sub-spawns → switch toggles green/red on victory. [sandbox.ldtk](crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk). Commits `701e144`, `c6d390c`, `2c5eafd`, `138e575`, `da30fc9`, `3745ed4`, `3699c6e`.
- **Encounter reward chest** `[stable]` — ground placement, sprite, switch reset on encounter reset. Commit `e21abc3`.
- **Encounter resets on death + waves wait for kills** `[stable]`. Commits `1a49239`, `3745ed4`.

## Movement primitives & rooms

- **Swim mechanic + water_world room** `[playable]` — Mario-style swim, source-agnostic IntGrid `BlockKind::Water`, two-pool lab with overlapping-zone fix. Commits `8b3bec6`, `fe7cef5`.
- **Crouch / morph-ball as `BodyMode`-driven mechanics** `[stable]` — double-tap-down edge routes through `SandboxRuntime` to fire morph ball. Commits `dafa5a4`, `5899c2c`, `da1151a`.
- **One-way platforms + hazard tiles (32×16)** `[stable]` — fixes vertical squish; HazardBlock entities migrated to IntGrid. Commits `b59529a`, `8c8cf30`.
- **Smooth camera ease + victory chest** `[stable]`. Commit `1a49239`.
- **`CameraEaseTuning` resource** `[stable]` — replaces the hardcoded `CAMERA_ZOOM_{IN,OUT}_RATE` consts with an editable Bevy resource (`zoom_out_rate`, `zoom_in_rate`, `snap_epsilon`). Defaults match prior values; sandbox / tests can override without recompiling. [lib.rs](crates/ambition_sandbox/src/lib.rs).

## LDtk authoring (runtime spine)

- **OneWayPlatform + DamageVolume promotion** `[stable]` — typed LDtk runtime-spine components with parity overlay; JSON authority retained until parity tests pass. Commit `d5e0f52`.
- **LDtk biome metadata → runtime resource** `[stable]` — populates on every embedded active area. Commits `3ca2e2f`, `08afafb`.
- **Room music driven by biome metadata seam** `[stable]` — replaces ad-hoc per-room overrides. Commits `fdbbb02`, `458d902`.
- **Programmatic LDtk authoring + room builder + layout lints** `[stable]` — agent-friendlier authoring path. Commits `9db48f7`, `f8a75b6`, `fcda12d`.
- **`list_ldtk_metadata.py` tool + body_mode reset tests** `[stable]`. Commit `7ae5dd6`.
- **LDtk validator: blank `activeArea` rejection** `[stable]`. [ldtk_world.rs](crates/ambition_sandbox/src/ldtk_world.rs).
- **LDtk × audio cross-validation: unknown `music_track` warnings** `[stable]` — `LdtkProject::music_track_warnings` returns one warning per (level, unknown_id); `init_sandbox_resources` runs it at startup against `SandboxDataSpec::audio.music_tracks`. Regression-pinned by `embedded_ldtk_music_tracks_match_audio_catalog`. [ldtk_world.rs](crates/ambition_sandbox/src/ldtk_world.rs).
- **`RoomSet::layout_warnings` branch tests** `[stable]`. [rooms.rs:351](crates/ambition_sandbox/src/rooms.rs#L351).

## Cutscenes & quests

- **Engine + sandbox cutscene spine** `[playable]` — room-triggered cutscenes, lazy boss specs. Commits `06ea438`, `55dc1a5`.
- **Hold-Reset cutscene skip with HUD progress bar** `[stable]`. Commit `322d9f2`.
- **Per-cutscene "always skip if seen" flag** `[stable]` — `CutsceneScript::seen_flag: Option<String>` is set in save data when a cutscene completes (or is skipped via hold-Reset); future triggers consult `SandboxSaveData::flag` and skip silently. Cutscenes without a `seen_flag` always replay (intentional for test rooms / dev cycles). [cutscene.rs:62](crates/ambition_engine/src/cutscene.rs#L62), [cutscene.rs:180](crates/ambition_sandbox/src/cutscene.rs#L180).
- **Quest system + quest hooks** `[playable]` — engine + sandbox modules. Commits `06ea438`, `55dc1a5`.

## UI / menus

- **Map UI panel + minimap** `[playable]` — full-screen map + toggleable corner minimap; visited tracking. [map_menu.rs](crates/ambition_sandbox/src/map_menu.rs). Commit `75ebfcb`.
- **Map zoom controls + room-name labels** `[stable]` — full map shows full room ids (e.g. `central_hub_complex`) when boxes are wide enough, falls back to 2-letter initialisms when narrow. `+` / `−` adjust zoom (`MAP_ZOOM_STEP=1.25`, clamped to `[0.5, 4.0]`); `0` resets. Zoomed maps recenter on the active room. Status line shows current zoom + binding hint. 6 unit tests pin the clamp / reset / round-trip / label behavior. [map_menu.rs](crates/ambition_sandbox/src/map_menu.rs).
- **Reset Sandbox pause-menu item + reset processor** `[stable]`. Commit `45631d4`.
- **HUD wiring** `[playable]`. Commit `55dc1a5`.
- **Dedicated quest panel** `[playable]` — `QuestPanelText` UI surface anchored top-right, written by `update_quest_panel`. Decouples the quest log from the debug HUD's stats dump; collapses (empty string) when no active quests. [app.rs](crates/ambition_sandbox/src/app.rs).
- **F3 stats editor** `[stable]`. Commit `70fc567`.

## Settings (full system)

- **Settings architecture + menu input + persistence** `[playable]` — `settings/` module. Commits `00b2536`, `239b138`.
- **Per-controller-profile filter defaults** `[stable]` — `ControllerProfileId::filter_defaults()` returns calibrated deadzone + trigger-threshold values per pad. Xbox 360 widens deadzones (0.27/0.30 vs 0.18/0.20) and the trigger hysteresis band (0.20–0.65 vs 0.30–0.55) to compensate for known stick / trigger drift. PlayStation tightens deadzones for its sharper sticks. `ControlSettings::apply_profile_defaults` stomps the active settings with the profile's calibrated values. [settings/controls.rs](crates/ambition_sandbox/src/settings/controls.rs).
  - Controls subsection: deadzone (left + right stick), trigger thresholds, hysteresis, dash repeat. [settings/controls.rs](crates/ambition_sandbox/src/settings/controls.rs).
  - Gameplay subsection: difficulty (Easy/Medium/Hard with damage multipliers). [settings/gameplay.rs](crates/ambition_sandbox/src/settings/gameplay.rs).
  - Audio: volume + difficulty/assist multipliers wired from `UserSettings`. Commit `3be81f3`.

## Save / persistence

- **Sandbox save game + encounter state vocabulary** `[playable]`. Commit `239b138`.
- **Hostile-NPC death persisted across save** `[playable]`. Commit `75ebfcb`.

## Audio / music

- **Music backend split: production renderer (pretty-midi) + fallback** `[stable]` — large refactor pass. Commits `619d08f`, `1d7bd3f`, `7843010`, `d7459c6`.
- **Renderer overhaul** `[stable]` — compressor, Schroeder reverb, constraints, humanize, bends. Commit `b8d1a18`.
- **Adaptive music cues** `[playable]` — fast_paced_violin_boss, dinosaur_liberators (Freebird-length 9:08), violin_boss_relentless, crooked_ascent_boss (5/4 klezmer/Bartok), 3 faction-area themes. Commits `dddf505`, `fdfd4a5`, `a03c1d7`, `d4b25ca`, `207294e`, `ec5a3d1`.
- **Audio validation tests + `music_track_report` binary** `[stable]`. Commit `fb175f5`.
- **LFO automation curves + per-cue dynamics tuning** `[stable]`. Commits `a03c1d7`, `533c47d`.

## Input

- **`SandboxAction::Interact`** `[playable]` — bound to gamepad RightTrigger; covers every action on keyboard + gamepad. [input.rs:41](crates/ambition_sandbox/src/input.rs#L41). Commit `e503c0d`.
- **Double-tap-up Interact gesture** `[stable]` — `SandboxRuntime::register_up_tap` accumulates Up presses inside `feel.up_double_tap_window` and returns true on the second tap. The gesture is OR'd into `controls.interact_pressed` BEFORE doors / NPCs / chests see it, so single-press Up never fires interact via any current call site. [lib.rs:282](crates/ambition_sandbox/src/lib.rs#L282), [app.rs:1519](crates/ambition_sandbox/src/app.rs#L1519).
- **`SandboxAction::MenuSelect`** `[stable]` — Enter / NumpadEnter / Space / GamepadButton::South / configured Jump key all confirm in menus. [input.rs:342](crates/ambition_sandbox/src/input.rs#L342). Pause menu reads `actions.just_pressed(&SandboxAction::MenuSelect)`.
- **`SandboxAction::MenuNavigate{Up,Down,Left,Right}`** `[stable]` — D-pad + arrow keys + analog stick navigate menus. Toggleable via `controls.dpad_menu_navigation`. [input.rs:331](crates/ambition_sandbox/src/input.rs#L331).
- **Dash trigger hysteresis** `[stable]` — analog right trigger goes through `update_trigger_edge` with configurable release / press thresholds (defaults 0.30 / 0.55). Three input modes: Trigger (analog only) / Button (RB/R1) / Both. Prevents jitter spam on aged controllers. [input.rs:488](crates/ambition_sandbox/src/input.rs#L488), [settings/controls.rs:202](crates/ambition_sandbox/src/settings/controls.rs#L202).

## Mobile / Android touch input

- **`mobile_touch` feature flag** `[playable]` — default-enabled; gates the `mobile_input::bevy_plugin` module + `virtual_joystick = "2.7.2"` dep. Pure helper (`fold_touch_into_control_frame`) is always-built. Disable for distribution / console / minimal builds via `--no-default-features --features visible`.
- **`mobile_input::TouchInputState` + pure folder** `[stable]` — captures one frame of stick + button state and folds it into the engine's `ControlFrame`. Ten unit tests pin deadzone / sign / threshold / button edge semantics. RL agents can construct `TouchInputState` directly to drive the same seam without the Bevy plugin. [mobile_input.rs](crates/ambition_sandbox/src/mobile_input.rs).
- **`MobileTouchPlugin` (Bevy)** `[playable]` — gated behind `mobile_touch`. Adds `VirtualJoystickPlugin<MobileStick::{Move,Aim}>`, spawns a right-anchored column of action buttons (Jump/Attack/Dash/Blink/Interact/Projectile/Start/Reset), reads stick + button state into `MobileTouchState`, then writes `ControlFrame` via the pure folder. On-screen joystick spawn (with Knob.png art) is the remaining polish; today the plugin is loaded but the joysticks aren't visualized. Documented at [docs/mobile_touch_controls.md](docs/mobile_touch_controls.md).

## Headless / RL adapter

- **`rl` feature flag** `[stable]` — default-enabled; gates the `crate::rl` module (SandboxSim, AgentAction, AgentObservation, SandboxSimOptions, TimestepMode) plus the four RL/replay binaries (`headless`, `rl_random_walker`, `rl_smoke`, `trace_replay`) all of which `required-features = ["rl"]`. Disable for stripped builds via `--no-default-features --features visible`.
- **`run_visible` graceful headless fallback** `[stable]` — when no `DISPLAY` / `WAYLAND_DISPLAY` env var is set on Linux, or when the user passes `--headless` on the CLI, `run_visible` prints a one-line diagnostic and routes to `run_headless` instead of letting `bevy_winit` panic during event-loop creation. Tick count override via `--headless-ticks N` (default 120). [app.rs:185](crates/ambition_sandbox/src/app.rs#L185).
- **`SandboxSim` programmatic step API** `[stable]` — public Rust API at [rl.rs](crates/ambition_sandbox/src/rl.rs) wrapping the headless `App`. `SandboxSim::new()` builds the simulation; `sim.step(action)` writes a converted `ControlFrame` and ticks `Update` once, returning an `AgentObservation`. `AgentAction` covers movement / jump / dash / blink / attack / interact / projectile / fly-toggle / reset / aim. `AgentObservation` exposes player pos/vel/size/HP/dash/airjumps/body-mode/active-room plus per-tick flags (recently_damaged, in_hitstun). Foundation for RL agents, fuzz harnesses, replay drivers, and a future PyO3 binding. Re-exported as `ambition_sandbox::{SandboxSim, AgentAction, AgentObservation}`.
- **`SandboxSim` deterministic timestep** `[stable]` — `TimestepMode::{WallClock, Fixed { dt }}` knobs configure how `Time` advances per step. `Fixed` mode installs Bevy's `TimeUpdateStrategy::ManualDuration` so every step is bit-exact reproducible across runs. Helper constructors `fixed_60hz()` / `fixed_144hz()` match the visible binary's nominal framerate and the engine repro test's high-refresh path. Determinism test pins (action_seq, initial_state) → trajectory equality. [rl.rs](crates/ambition_sandbox/src/rl.rs).
- **`rl_random_walker` binary** `[stable]` — concrete demonstration that hooking up an RL agent is no harder than hooking up an input controller. Drives `SandboxSim` with a small LCG-seeded random policy, prints per-100-step heartbeats + end-of-run summary (jumps/dashes/blinks/attacks counts, max distance from spawn, room transitions). Doubles as a fuzz harness — 2000 frames in `central_hub_complex` complete without panic / OOB / health drain. [bin/rl_random_walker.rs](crates/ambition_sandbox/src/bin/rl_random_walker.rs).
- **`trace_replay` binary** `[stable]` — reads `frames[*].controls` + `frames[*].player.pos` from a recorded `GameplayTraceBuffer` JSON dump, drives a fresh `SandboxSim` at fixed-60Hz, and reports max-divergence + first-divergence frame. Use cases: bug repro from production traces, determinism validation after refactors, future CI guardrails (in-tree fixture trace as a regression test). Invocation: `cargo run --bin trace_replay -- path.json [--tolerance VAL]`. [bin/trace_replay.rs](crates/ambition_sandbox/src/bin/trace_replay.rs).
- **`rl_smoke` binary** `[stable]` — visits every room in the LDtk project via `SandboxSim::with_start_room`, drives a deterministic random walker for N steps, asserts HP within `[0, hp_max]` and finite player position. Catches regressions where a specific room panics on construction (boss/encounter init, IntGrid parsing, water/climbable lowering) or under any random input combination — bugs a single-room smoke test would miss. Invocation: `cargo run --bin rl_smoke -- [STEPS] [SEED]`. [bin/rl_smoke.rs](crates/ambition_sandbox/src/bin/rl_smoke.rs).
- **`SandboxSim::room_ids` + `SandboxSimOptions::with_start_room`** `[stable]` — RL training loops that focus on a specific area (e.g. only train on water_world or only on mob_lab waves) construct via `SandboxSimOptions::default().with_start_room("mob_lab")`. Resolves through the new `StartRoomOverride` resource that `init_sandbox_resources` consumes — same semantics as the visible binary's `--start-room` flag, no env::args manipulation needed. `room_ids()` enumerates all rooms in the loaded project for "walk every room" use cases.

## Sandbox showcase rooms (basement-reachable)

Per Jon's "every new room reachable from a door in the basement"
rule, these rooms live as leaf entries off `central_hub_basement`'s
door corridor and demonstrate one mechanic each.

- **`crawl_lab`** `[stable]` — Crouching showcase. 1024x384 corridor
  with a low ceiling that forces `BodyMode::Crouching`. Door from
  basement at x=240. [tools/examples/ldtk_specs/crawl_lab.yaml](tools/examples/ldtk_specs/crawl_lab.yaml).
- **`morph_lab`** `[stable]` — MorphBall showcase. 1024x384 with a
  16-px tunnel that ONLY MorphBall (15.4×15.4) fits — Crouching /
  Crawling / Sliding all bounce off. Door from basement at x=296.
  Reward chest on the far side. [tools/examples/ldtk_specs/morph_lab.yaml](tools/examples/ldtk_specs/morph_lab.yaml).
- **`ladder_lab`** `[stable]` — Ladder primitive showcase. 768x1024
  vertical room with a floor-to-ceiling ladder column on the right
  and an upper platform with a reward chest. Demonstrates the
  end-to-end ladder primitive (Climbable IntGrid cells → engine
  ClimbableRegion → BodyMode::Climbing → integrate_climb). Door from
  basement at x=420. [tools/examples/ldtk_specs/ladder_lab.yaml](tools/examples/ldtk_specs/ladder_lab.yaml).
- **`cutscene_lab`** `[stable]` — Auto-trigger cutscene proof room.
  768x384 with a `cutscene_lab_intro` script bound via `RoomCutsceneBindings`.
  First entry fires the cutscene; the seen-flag prevents re-runs;
  the hold-Reset skip flow still works. Door from basement at x=496.
  [tools/examples/ldtk_specs/cutscene_lab.yaml](tools/examples/ldtk_specs/cutscene_lab.yaml).
- **`quest_lab`** `[stable]` — Quest progression proof room. 768x384
  with a two-step `quest_lab_visit` quest: step 1 fires on
  `RoomEntered("quest_lab")`, step 2 fires on returning to
  `central_hub_complex`. Quest auto-starts at boot; completion
  persists via the existing save system. Door from basement at
  x=1620. [tools/examples/ldtk_specs/quest_lab.yaml](tools/examples/ldtk_specs/quest_lab.yaml).
- **`switch_lab`** `[stable]` — Switch / save persistence proof room.
  768x384 with a single Switch entity that fires the
  `test_switch_toggled` save flag on toggle + advances the existing
  `test_switch_quest`. Demonstrates the full Switch + save round-trip.
  Door from basement at x=1139. [tools/examples/ldtk_specs/switch_lab.yaml](tools/examples/ldtk_specs/switch_lab.yaml).

## Tooling / generators

- **Sprite passes** `[stable]` — tight-crop entity art, tile sprites, IntGrid-blocked tiles, 11 entity sprites via `gen2d`, procedural morph-ball sprite. Commits `e634d39`, `bd95097`, `efdb8fb`.
- **Programmatic LDtk authoring tool + smoke test** `[stable]`. Commit `9db48f7`.
- **Sprite generator + component sheet tooling** `[stable]`. Commits `4016e2e`, `95f7fd9`, `553186a`, `e3ca24e`.
- **AI-driven enemy update tool** `[stable]`. Commit `f8a75b6`.

## Continuous integration

- **GitHub Actions test workflow** `[stable]` — `.github/workflows/test.yml` runs three jobs on push / PR: (1) `cargo test -p ambition_engine` (engine library + proptest fuzzers), (2) `cargo test -p ambition_sandbox --lib` + `repro_walls` integration test + a `cargo run --bin headless -- 60` smoke pass, (3) `cargo fmt --check` + `cargo clippy` (informational, non-blocking).

## Tests / observability

- **Wall-jump repro test** `[stable]` — [tests/repro_walls.rs](crates/ambition_sandbox/tests/repro_walls.rs).
- **`RoomSet::layout_warnings` branch tests** `[stable]`.
- **Boss encounter integration test** `[stable]`.
- **Body-mode reset tests** `[stable]`.
- **`BodyShape::fits_at` proptest** `[stable]` — three proptest properties pin the geometric contract: empty world fits any finite shape at any finite center; one-block world fits iff the shape AABB and block AABB don't strictly intersect; MorphBall fits wherever Standing fits (strict shape shrink). [body_shape_fits_at.rs](crates/ambition_engine/tests/body_shape_fits_at.rs).
- **Wall-jump start-position fuzz** `[stable]` — proptest randomizes (x_offset, y_in_wall, vel_y) along the square_arena left wall and asserts no >100 px y-snap and no out-of-world / through-wall teleport on a single wall-jump frame. Regression guard around the `body_is_side_contact` predicate that closes the historical OOB-teleport bug class. [wall_jump_fuzz.rs](crates/ambition_engine/tests/wall_jump_fuzz.rs).
- **Wall-cling start-position fuzz** `[stable]` — proptest companion to wall-jump fuzz: covers the cling-steady-state path against two scenarios (square arena + the historical mob_lab lock-wall layout). Random (x_offset, y_in_wall, vel_y) + cling input must not produce a >100 px y-snap, through-wall x penetration, or out-of-world position. The lock-wall scenario adds a stronger assertion that y never snaps below 100 (the historical bug clamped to the arena ceiling at y≈23). [wall_cling_fuzz.rs](crates/ambition_engine/tests/wall_cling_fuzz.rs).
- **`ResourceMeter` envelope proptest** `[stable]` — five property tests pin meter-current ∈ [0, max] under random spend/refill/tick sequences, `try_spend` exact-success-iff-enough behavior, refill_full reaches max, regen monotonic up, decay monotonic down. [resource_meter_props.rs](crates/ambition_engine/tests/resource_meter_props.rs).
- **mob_lab lock-wall teleport regression** `[stable]` — pins the geometry from `docs/tech_debt_log.md` HIGH. Currently passes thanks to the `body_is_side_contact` predicate. [repro_walls.rs](crates/ambition_sandbox/tests/repro_walls.rs).
- **`scripted_gameplay` multi-frame integration test** `[stable]` — three scenarios under MinimalPlugins: 30 idle frames emit no lifecycle events; Reset press emits Reset message; heterogeneous Reset/Jump/move sequence runs to completion. Pins the sim → presentation message seam end-to-end. [scripted_gameplay.rs](crates/ambition_sandbox/tests/scripted_gameplay.rs).
- **`fuzz_random_walker` integration test** `[stable]` — five LCG-seeded `SandboxSim` random-walk runs (200 steps each, fixed-60Hz) assert no panic, finite player position, HP within `[0, hp_max]`. Catches "this random input combination panics" regressions in pure Rust. Stable seeds (1, 42, 99, 2026, 31337) make any failure reproducible via `cargo run --bin rl_random_walker -- <STEPS> <SEED>`. [fuzz_random_walker.rs](crates/ambition_sandbox/tests/fuzz_random_walker.rs).
- **Audio validation tests** `[stable]`.
- **`SandboxFeelTuning` invariant tests** `[stable]` — three default-value invariants: `time_ramp_up_rate > time_ramp_down_rate` (recovery snappier than entry); transition flash >= cooldown (no double-trigger); attack-active windows >= one 60fps frame (hittable). [feel.rs:104](crates/ambition_sandbox/src/feel.rs#L104).
- **`MechanicsRegistry` invariants** `[stable]` — duplicate id detection, blank text-field check, category/maturity label collision detection. [mechanics.rs:283](crates/ambition_sandbox/src/mechanics.rs#L283).
- **`PhysicsSandboxSettings` defaults + `PhysicsDebrisCue` distinctness** `[stable]` — debris settings sane and debris-cue variants compare distinct (silent fall-through guard for the `debris_recipe` match). [physics.rs](crates/ambition_sandbox/src/physics.rs).
- **`GameAssetConfig` CLI parsing tests** `[stable]` — extracted `from_arg_slice` so the CLI parser is unit-testable without `env::args`; six tests cover defaults, `--no-assets`, `--sprite-folder` (with/without value), unknown-flag tolerance, and `entity_sprite_for_kind` exhaustiveness. [game_assets.rs](crates/ambition_sandbox/src/game_assets.rs).
- **`BOSS_SHEET` / `BossSheetSpec` / `is_boss_kind` tests** `[stable]` — seven tests pin the boss spritesheet's pure-function surface (row-count vs enum variants, `frame_count`, `flat_index` row-end-to-end + clamp-on-overshoot, `render_size` aspect-ratio + minimum extent). [boss_sprites.rs](crates/ambition_sandbox/src/boss_sprites.rs).
- **`Motif` invariant tests** `[stable]` — `TANGENT_MOTIF`'s scale_degrees / rhythm_units have matching length and rhythm units are positive. Cheap insurance for the future arrangement engine. [music.rs](crates/ambition_engine/src/music.rs).
- **`--start-room` CLI parser tests** `[stable]` — extracted `parse_start_room_arg` so the parser is unit-testable without `env::args`; five tests pin space-form, equals-form, first-match-wins, no-flag, and trailing-flag-without-value. [app.rs:224](crates/ambition_sandbox/src/app.rs#L224).
- **Ledge-grab climb-up + drop-off tests** `[stable]` — pin the two intentional player-driven exits from a held ledge: Up + Jump snaps to climb_target and sets on_ground; Down clears the latched state and wall-cling flags. [ledge_grab.rs](crates/ambition_sandbox/src/ledge_grab.rs).
- **Character spritesheet flat_index clamp + frame_duration tests** `[stable]` — overshoot-clamping and per-row positive-duration invariants for ROBOT_SHEET. Mirror of the boss-spritesheet invariants. [character_sprites.rs](crates/ambition_sandbox/src/character_sprites.rs).
- **`Chest::new` defaults test** `[stable]` — pins state=Closed, persistent=true, reward propagates as-given. [interaction.rs](crates/ambition_engine/src/interaction.rs).
- **`World::water_at` submersion-math tests** `[stable]` — body inside region clamps to submersion=1; body straddling surface reports submersion=0. [world.rs](crates/ambition_engine/src/world.rs).
- **562+ workspace test count baseline** — engine + sandbox library tests, integration tests, doc tests, snapshot tests. Run via `cargo test --workspace`.

---

## Notes
- This catalog supersedes piecemeal "what's done" claims scattered in handoff docs. When closing a `TODO.md` item, add the corresponding entry here in the same commit.
- For a continuous narrative of what changed when, `git log --oneline` is authoritative — this file is the "current capabilities" view.
- "Recently completed" entries in `TODO.md` should be migrated here once they've stabilized (~2 weeks).
