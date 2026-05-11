# Ambition TODO

> **Sandbox-first.** The sandbox *is* the vertical slice — every gameplay component
> assembled in a test-arranged form rather than a story-arranged one. Story-arranged
> slice content (hub → first zone → Gradient Sentinel) resumes once the sandbox
> bar is met.
>
> **Source-of-truth pact:** This file (outstanding work) and `FEATURES.md`
> (landed work) are authoritative. Keep them fresh — when an item lands, move
> it to `FEATURES.md` in the same commit. Re-grep before claiming "done."
>
> **Adding new ideas:** drop them in the `## Proposed` section at the bottom.
> Jon moves Proposed items into `## Accepted` (with a tier letter) or `## Rejected`.
>
> Last full re-audit: 2026-05-07 (against `git log --all`).
> Test count: 628+ across `cargo test --workspace` (210 engine + 386 sandbox lib + integration suites; +crouch_stability +replay_fixture +mobile_input + …).

## Status legend
- `[ ]` not started · `[~]` scaffolded but not feature-complete · `[x]` recently completed (kept here briefly so it doesn't get re-added)
- **`[V?/D?]`** value (1–5) / difficulty (1–5). V: 1=marginal, 5=critical. D: 1=≤30min, 2=1–3hr, 3=half day, 4=multi-day, 5=week+ or risky.
- NOTE: don't always trust difficulty ratings, don't be afraid to tackle something because it is difficult.

## Recently completed (will migrate to FEATURES.md as they age)
- [x] **`SandboxSim` programmatic step API + visible-binary headless fallback** — see FEATURES.md → "Headless / RL adapter". Visible binary no longer panics on display-less VMs (auto-falls-back, or via `--headless`). RL drivers can now call `SandboxSim::new()` then `sim.step(action)` to advance the sim with a typed `AgentAction`/`AgentObservation` pair. [rl.rs](crates/ambition_sandbox/src/rl.rs), [app.rs:185](crates/ambition_sandbox/src/app.rs#L185).
- [x] Wall-jump OOB fix via `body_is_side_contact` — see FEATURES.md → physics
- [x] mob_lab full pass (intro / lock-wall / waves / music swap) — see FEATURES.md → encounter
- [x] Settings system (controls / gameplay / persistence / deadzone) — see FEATURES.md → settings
- [x] Map UI + minimap toggle — see FEATURES.md → UI
- [x] Boss phase machine (Stagger / Enraged / threshold) — see FEATURES.md → enemies
- [x] Cutscene system + hold-Reset skip — see FEATURES.md → cutscenes
- [x] Quest system + room-triggered hooks — see FEATURES.md → cutscenes
- [x] Projectile + charge fireballs + Hadouken motion-input — see FEATURES.md → combat
- [x] Swim + water_world two-pool lab — see FEATURES.md → movement primitives
- [x] OneWayPlatform + DamageVolume LDtk promotion + parity overlay — see FEATURES.md → LDtk
- [x] Encounter reward chest (replaces hardcoded `Health{2}`) — see FEATURES.md → encounter
- [x] `BodyMode::{Crouch, MorphBall, Crawling, Sliding}` drivers — see FEATURES.md → physics
- [x] `evaluate_character_ai` engine fn + per-brain knobs — see FEATURES.md → AI
- [x] LDtk biome metadata → runtime resource → room music seam — see FEATURES.md → LDtk
- [x] `SandboxAction::Interact` (gamepad RT + keyboard) — see FEATURES.md → input
- [x] **Enter as menu-select in settings menu** — `MenuSelect` bound to Enter / NumpadEnter / Space / South / user's jump in [input.rs:342](crates/ambition_sandbox/src/input.rs#L342). Pause menu reads `actions.just_pressed(&SandboxAction::MenuSelect)`.
- [x] **D-pad navigation in settings menu** — `MenuNavigate{Up,Down,Left,Right}` bound to `GamepadButton::DPad{Up,Down,Left,Right}` in [input.rs:331](crates/ambition_sandbox/src/input.rs#L331). Toggleable via `controls.dpad_menu_navigation`.
- [x] **Controller dash hysteresis** — analog trigger goes through `update_trigger_edge` with release/press thresholds; trigger / button / both modes; per-input edge guards prevent jitter spam. [input.rs:488](crates/ambition_sandbox/src/input.rs#L488), [controls.rs:202](crates/ambition_sandbox/src/settings/controls.rs#L202).
- [x] **`PlayerDiedMessage`** — replaces `runtime.player_died_pending` bool with a Bevy 0.18 buffered `Message`. Producer pushes into `FrameFeedback.died` from `death_respawn_player`; encounter system reads `MessageReader<PlayerDiedMessage>`. [lib.rs:67](crates/ambition_sandbox/src/lib.rs#L67), [app.rs:2172](crates/ambition_sandbox/src/app.rs#L2172), [encounter.rs:1057](crates/ambition_sandbox/src/encounter.rs#L1057).

---

## S — Active sandbox blockers (do first)

- [~] **Wall-cling teleport on mob_lab lock wall** `[V5/D3]` — currently mitigated by `body_is_side_contact` predicate; pinned by `mob_lab_lock_wall_cling_does_not_teleport` regression test in `repro_walls.rs` AND by the new proptest fuzz in `wall_cling_fuzz.rs` (random positions across two scenarios). The historical y=434 → y=-23 snap is closed; the parry contact-normal fix (path_forward step D1) is the proper cleanup but no longer urgent. Source: `docs/tech_debt_log.md` (HIGH).
- [~] **Parry contact-normal in `sweep_player_x` / `sweep_player_y` (path_forward step D1)** `[V5/D3]` — Parry `ShapeCastHit::normal1` is now plumbed through `geometry`/`world`, but direct snap-direction consumption was reverted after full-world wall-cling repros teleported to `y=-23`. Keep the existing overlap/approach guards in `movement/collision.rs` until D1 has a targeted semantic test for side-wall vs floor/ceiling contacts.
  - Local validation gate: `cargo fmt --all`, `cargo test -p ambition_engine --lib`, `cargo test -p ambition_sandbox --test repro_walls`, and `cargo test -p ambition_sandbox --test fuzz_random_walker`.
  - Benchmark/lesson notes for this failed refactor live under `dev/benchmark-candidates/` and `dev/journals/` as additive files.
- [x] **Double-tap-up Interact trigger audit** — `register_up_tap` in [SandboxRuntime](crates/ambition_sandbox/src/lib.rs#L282) emits the `door_double_tap_up` gesture; [app.rs:1519](crates/ambition_sandbox/src/app.rs#L1519) ORs it into `controls.interact_pressed` before doors / NPCs / chests see it. Single-press Up never fires interact via any current site (chest_open / npc_talk / LoadingZone Door all read `interact_pressed` only). Save-point sites don't exist yet; when they land, route through `interact_pressed` to inherit the same gating.
- [ ] **Glitchy platform behavior (intermittent)** `[V4/D3]` — vague repro. Bug record/replay (see C) would catch this. Source: `tmp-todo-notes.txt`. _Diagnostic logging on riding-state transitions landed via `Player::was_riding_platform` + `tracing::debug!` (target=`ambition::platform`); enable with `RUST_LOG=ambition::platform=debug` to capture the contact transition trail._

## A — Sandbox expressiveness

### Mechanics & combat
- [x] **Ladder + climbable-zone primitive** `[V4/D3]` — fully landed 2026-05-07 (engine primitive + traversal + LDtk IntGrid authoring + ladder_lab showcase room reachable from basement). The `tools/author_ldtk_area.py` extension paints Climbable cells; `ladder_lab` demonstrates Up-into-ladder → climb → reward chest. Sprite wiring (ladder_tile.png) is the only follow-up — separate row in the Sprite wiring batch.
- [ ] **Ledge grab + climb-up promotion to engine** `[V4/D3]` — move `LedgeProbe`, `Ability::ledge_grab`, state branch into `ambition_engine::player_state` (currently sandbox-side). Add diagonal-corner probe test. Climb-up animation slot separate from grab.
- [x] **NPCs become enemies on hit (in certain circumstances)** — already wired. `NPC_HOSTILE_STRIKE_THRESHOLD = 3` flips NPCs hostile after three player strikes; `apply_save` then replaces the `NpcRuntime` with an `EnemyRuntime` carrying the same id. Hostility persists via `npc_<id>_hostile` save flag. Tested by `striking_npc_three_times_flips_them_hostile` + `apply_save_with_hostile_flag_replaces_npc_with_enemy`. [features.rs:2413](crates/ambition_sandbox/src/features.rs#L2413).
- [~] **More enemy varieties (S/M/L × aggression bands)** `[V4/D4]` — `EnemyArchetype` now has 7 combat archetypes (Combatant, SmallSkitter, SmallLurker, MediumStriker, LargeBrute, LargeColossus, AggressiveSeeker) plus 2 sandbag training variants. Cross-axis invariants tested (HP grows with size; aggro radius grows with aggression; damage scales). LDtk authors enable any of them with `EnemySpawn::brain` set to the brain id (e.g. `large_colossus`). Remaining for the full 9-cell matrix: dedicated low-aggression Medium and high-aggression Medium/Small variants. Authoring trivial — add to `EnemyArchetype::from_brain` and the per-archetype tuning matches.

### Test rooms (sandbox = component showcase)
- [x] **Crawl/morph proof room** — `crawl_lab` (Crouching) and `morph_lab` (MorphBall) both authored 2026-05-07 and reachable from the basement door corridor. See FEATURES.md → "Sandbox showcase rooms".
- [~] **Save-point lab + persisted-switch test room** `[V4/D3]` — `switch_lab` authored 2026-05-07 demonstrates Switch persistence end-to-end (toggle → flag → save). Remaining: dedicated save-point entity (a "save here" interactable distinct from regular Switch toggles), reset-switches sub-room, broader test-state schema (boss defeated, mob room cleared). Source: `tmp-todo-notes.txt`.
- [x] **Quest test room** `[V3/D3]` — `quest_lab` authored 2026-05-07 with `quest_lab_visit` two-step quest. Reachable from basement at x=672. See FEATURES.md → "Sandbox showcase rooms". A more elaborate fetch/talk quest is a follow-up but the room-driven progression is end-to-end here.
- [x] **Cutscene test room** `[V3/D2]` — `cutscene_lab` authored 2026-05-07 with `cutscene_lab_intro` script + binding. Reachable from basement at x=496. See FEATURES.md → "Sandbox showcase rooms".

### Sprite wiring batch
- [x] **MorphBall sprite** — procedurally generated 64×64 RGBA sphere at startup, no `morph_ball.png` asset needed. [body_mode.rs:162](crates/ambition_sandbox/src/body_mode.rs#L162). Wired through `MorphBallSprite::handle` resource.
- [ ] `switch_armed.png` / `switch_disabled.png` → Switch rendering `[V2/D1]` — blocked on art generation
- [ ] `lock_wall_tile.png` → runtime-inserted lock walls `[V2/D1]` — blocked on art generation
- [ ] `water_surface_tile.png` → overlay layer above water `[V2/D2]` — blocked on art generation
- [ ] `ladder_tile.png` → climbable zones `[V2/D1]` (gated on Ladder primitive) — blocked on art generation
- [ ] `acid_tile.png` / `lava_tile.png` → IntGrid value mappings `[V2/D1]` — blocked on art generation
- [ ] `bg_circuit_tile.png` → parallax in `central_hub_complex` `[V2/D2]` — blocked on art generation

### Architecture
- [ ] **Stitched (loading-zone-free) room transitions** `[V4/D4]` — user wanted basement reachable by "drop down" from hub, not via load. Source: `tmp-todo-notes.txt`.
  - Adjacency model in `RoomSet`
  - Camera + collision spans both rooms during traversal
  - Debug overlay zoom-out to view stitched layout
  - Decision: stitching vs single big room — prototype both

## B — Authoring ergonomics, validators, audio polish

- [ ] **Adaptive music phase transitions in boss room** `[V4/D3]` — stem-blend/layered tracks via existing [music.rs](crates/ambition_sandbox/src/music.rs) hooked to `BossPhase`. Cues already authored. Source: `tmp-todo-notes.txt`.
- [ ] **Music level / mixing debug tools** `[V3/D3]` — the goblin intro→loop volume hunt (commit `b8c0f84`) leaned on hand-running `ffmpeg loudnorm` against individual OGG files to find that the per-stem outputs measured -50 to -inf LUFS while the per-section full mixes were ~20 dB above. Tools to add: (a) a `music_level_report` binary that walks `assets/audio/music/generated/<cue>/` and prints integrated LUFS / true-peak / RMS per file in a diff-friendly table, with a parity column comparing each cue against a configurable target loudness (e.g. `-23 LUFS` to match lofi); (b) an in-engine HUD overlay (gated behind `dev_tools`) showing the live per-bank / per-slot gain values + post-master dB, so transition dips are visible while playing; (c) optionally, on-disk peak/RMS metadata in the renderer output that the in-engine debug HUD can compare against the live signal to flag mastering regressions. Source: 2026-05-08 goblin music regression hunt.
- [ ] **Equal-power crossfade in `update_gain_smoothing`** `[V2/D2]` — the current per-slot gain smoothing is a one-pole IIR (`1 - exp(-dt/STEM_GAIN_BLEND_SECONDS)`); two such ramps crossing at the same time-constant produce a -3 dB midpoint power dip on every section transition. Replace with a phase-driven sin²/cos² weighting when a transition is in flight. Source: deferred from commit `b8c0f84`.
- [ ] **Renderer: master per-stem outputs** `[V3/D3]` — the music renderer's compressor / reverb / limiter chain only runs on the per-section full mix, not on the individual stems. As a result the wave1 / wave2 / wave3 stems are -35 dB below the full mix and three of six are essentially silent. Fix lets `wave2_brute` (and other state-level stem-gain tweaks) become audible again; until then `first_goblin_tune_v2_spec` plays single mastered full mixes per section. Source: deferred from commit `b8c0f84`.

## C — Engine cleanups (compounding)

- [x] **ADR 0012 events refactor — main work** — Slices 1-5 (sfx / vfx / debris messages, setup split, headless `sandbox_update`) all landed. `app.rs` has zero direct `play_sound` / `spawn_burst` / `spawn_dust` / `spawn_impact` calls; `fx.rs` only calls them as the consumer of `VfxMessage`. Headless ticks `sandbox_update` cleanly via `run_headless`. Remaining cleanup ([ ] hardening only):
  - [ ] Tighten `SandboxRuntime` field visibility from `pub` to `pub(crate)` (deferred — risks breaking `bevy-inspector-egui` field reflection; revisit when the inspector wiring uses Reflect-only access patterns).
  - [x] Add `tests/scripted_gameplay.rs` integration test (3 scenarios: 30 idle frames, Reset press emits Reset message, heterogeneous Reset/Jump/move sequence runs to completion).
- [ ] **CharacterAi authoritative migration** `[V3/D4]` — convert one enemy archetype's movement to read evaluator output (currently observed-only); then one boss pattern; parity test. Source: `character_ai_refactor.md`.
- [x] **Bug record/replay ring buffer** `[V4/D4]` — last 600 frames of `ControlFrame + SimMessages + player snapshot`, F8 / auto-OOB dump, replay binary all landed. Trace recorder in [trace.rs](crates/ambition_sandbox/src/trace.rs) writes JSON+Markdown dumps; auto-OOB triggers via `detect_oob` + `request_dump(DumpReason::OobAuto)`. Manual F8 hotkey via `handle_trace_hotkey`. Replay binary at [bin/trace_replay.rs](crates/ambition_sandbox/src/bin/trace_replay.rs) drives a fresh `SandboxSim` from any trace JSON and reports divergence. Source: `path_forward.md` step F.
- [~] **`bevy_rl` integration for AI playtesting** `[V4/D4]` — RL agents that exercise the sandbox to surface bugs (and eventually for proper RL training). **Substantially landed 2026-05-07** as `SandboxSim` (`crates/ambition_sandbox/src/rl.rs`): step API, action/observation structs, deterministic fixed-timestep mode, `rl_random_walker` and `trace_replay` binaries, 8 unit tests. Remaining:
  - PyO3 binding so research code in Python can drive it
  - Reward shaping (currently observation-only; reward is the agent's job)
  - Evaluate `bevy_rl` crate vs continuing custom — `SandboxSim`'s shape is intentionally compatible

## D — Compile-time investments

- [x] Split `features.rs` into `features/` domain modules `[V4/D4]` — landed 2026-05-08. The old mega-file is now a facade plus `runtime`, `bus`, `events`, `hazards`, `enemies`, `bosses`, `breakables`, `pickups`, `chests`, `npcs`, `path_motion`, `world_overlay`, and focused tests.
- [~] Split `ldtk_world.rs` into modules per `path_forward.md` step C `[V3/D4]` — in progress. Extracted `bevy_runtime`, `hot_reload`, `intgrid`, `surfaces`, `fields`, and LDtk tests; remaining follow-up is to split the facade body into schema/validation/room-compiler modules.
- [ ] Promote `KinematicPath` to typed components + index `[V2/D3]`
- [ ] **Extract `ambition_game` crate** `[V4/D5]` — engine / game / sandbox 3-crate layout. Holds encounter, boss_encounter, quest, cutscene, save, ledge_grab, swim, map_menu, NPC AI, audio, rendering primitives. Source: `crate_split_plan.md`.

## E — UI / inventory / polish

- [ ] **N64 OOT/MM-style spinning-cube inventory** `[V3/D5]` — 4 faces (map / loadout / quests / system options); modernize contents but keep nostalgic cube spin + menu-change SFX
  - 3D cube widget (bevy_ui or world-space camera trick)
  - Per-face contents wired to existing systems (map_menu, quest, settings)

## F — Documentation / hygiene


## ♾ — Evergreen / perpetual

> These never "complete" — they describe ongoing investments. When you hit a
> hard task, ask: "is there a tool I could build that would make this and the
> next ten of these easier?" If yes, log a Proposed item or attack it inline.

- ♾ **Improve the programmatic LDtk map editor / authoring tools as needed.**
  Whenever a level-authoring task feels painful (manual JSON edits, repeated
  copy-paste, fragile coord math, validator surprises), pause and improve
  `tools/validate_ambition_ldtk.py`, `crates/ambition_sandbox/src/room_builder.rs`,
  programmatic LDtk authoring helpers, debug overlays, or the LDtk validator
  warnings. Tooling investment compounds.
- ♾ **Be on the lookout for tool-buildable pain points.** Any time a task
  takes >30min of mechanical fiddling that a script could automate, write
  the script. Log under `tools/` or `crates/ambition_sandbox/src/dev_tools.rs`.
- ♾ **Keep `TODO.md` / `FEATURES.md` / source docs in sync with reality.**
  Re-grep before claiming a TODO is "the bug". Many items here are stale.

## G — Story-arranged slice (resume after sandbox bar is met)

- [ ] Real central hub authoring `[V3/D4]`
- [ ] Basement / first zone authoring `[V3/D4]`
- [ ] Gradient Sentinel boss implementation `[V3/D4]`
- [~] Improved boss movement patterns — traversal choreography (boss dash, arena reposition) `[V4/D3]`. Engine schedule data shipped 2026-05-07: `BossMovementKind`, `ArenaAnchor`, `gradient_sentinel_phase3_traversal` showcase. Remaining: Bevy-side controller (`crates/ambition_sandbox/src/boss_encounter.rs`) that interprets `step.movement` into actual world transforms; new boss runtime fields (target_pos / movement_progress) to drive the dash + reposition. Source: `tmp-todo-notes.txt`.
- [ ] Intro cutscene polish — "Hey you, you're finally awake" beat `[V2/D3]`

---

## Known issues / unanswered questions (logged but not yet investigated)

- **Unified input abstraction (ControlFrame as single seam)** — Jon
  noted 2026-05-07: "We need an elegant structure and abstraction
  layer so different control methods are not finicky." Touch input
  bugs (joystick misread, missing pause button, race conditions)
  trace back to having TWO input seams: `ActionState<SandboxAction>`
  (Leafwing-driven, read directly by pause_menu_toggle / inventory /
  pause_menu_navigate / cutscene-related systems) and `ControlFrame`
  (derived from ActionState by `populate_control_frame_from_actions`,
  with touch OR-merged in via `mobile_input::fold_to_control_frame`).
  Touch reaches only the second; anything reading ActionState directly
  is invisible to touch input until that consumer is migrated.
  Quick patch in 2026-05-07: pause_menu_toggle reads BOTH ActionState
  and ControlFrame.start_pressed, with the touch fold ordered
  `.before(pause_menu_toggle)`. Real fix: make ControlFrame the only
  consumer-side seam; consumers that need menu-specific actions read
  ControlFrame fields like `controls.menu_select`. Migrate
  pause_menu_navigate (MenuNavigateUp/Down/Left/Right + MenuSelect +
  MenuBack) and inventory_input (Inventory press) and cutscene
  dialogue dismiss to read ControlFrame so touch works for them too.
  Then deprecate the direct ActionState reads.

- **Move all dev hotkeys into the settings menu** — Jon noted
  2026-05-07: "Also F2 is also bound to slowing the time and
  toggling the display. We should move all of these options into
  settings, so there is a canonical non-hotkey place for
  everything." F-keys currently bound: F1 (?), F2 (slowmo), F3
  (inspector), F4 (?), F5 (?), F6 (windowed), F7 (borderless), F8
  (trace dump), F9 (?), F10 (input preset), F11 (display mode), F12
  (?). The Touch HUD toggle in `mobile_input::TouchControlsVisible`
  is intentionally NOT bound to a hotkey -- it's accessed via the
  resource so the future settings menu can flip it canonically. All
  other dev hotkeys should follow the same pattern: keep the resource
  for code access, add a settings row, deprecate the hotkey.

- **Goblin music transition fades audibly cross-section** — Jon noted
  2026-05-07: "when [music] transitions it doesn't just blend in new
  layers, you hear the previous music fade out, and we don't want
  that. We just want new layers to fade in." Current music director
  in [music.rs](crates/ambition_sandbox/src/music.rs) uses bank-A /
  bank-B crossfade (LOOP_SECTION_CROSSFADE_SECONDS=1.7), so each
  section transition fades out the previous bank as it fades in the
  new bank. The user-friendly behavior would keep the previous
  section playing at unchanged gain and ONLY fade in the new
  section's stems on top -- functionally an "additive layer add"
  rather than a section swap. Requires music director refactor:
  treat sections that share an underlying composition (e.g. all
  goblin v2 sections) as the same continuous track, with stems
  fading in independently per section transition. Quick partial
  workaround in place: drop intro/outro full to 0.40 to reduce the
  loudness drop. Real fix is the architecture change.



- **Moving platforms not driven by LDtk authoring** `[V4/D4]` — Jon
  noted 2026-05-07: "Also fix the fact that the moving platforms
  don't seem to associated with the ldtk files. I'm not sure where
  they are coming from, but every entity in games must be associated
  with some ldtk placement. ldtk is the source of truth for the
  world layout."
  Root cause: `MovingPlatformState::time_reference(&world)` in
  [platforms.rs:31](crates/ambition_sandbox/src/platforms.rs#L31)
  procedurally constructs ONE platform per room based on world size
  -- it never reads any authoring data. There's a `KinematicPath`
  entity def in the LDtk file (see line 2012) but it's not parsed
  into MovingPlatformState by any system; the LDtk-authored paths
  are ignored at runtime. Fix:
  - Parse `KinematicPath` entities in ldtk_world.rs into a
    `KinematicPathSpec` list per room
  - Replace `runtime.moving_platform: MovingPlatformState` (single)
    with a `Vec<MovingPlatformState>` populated from the spec list
  - Update every read site (world_with_moving_platform,
    sync_moving_platform, integrate_velocity, encounter logic) to
    iterate the Vec
  - Add KinematicPath placements to existing rooms that already
    have the procedural platform (square_arena, etc.) so they
    keep working post-migration
  - Remove the procedural `time_reference` constructor (or mark it
    debug-only behind a `#[cfg(test)]`)
  Construct sites today: lib.rs:234, app.rs:2154, app.rs:2260,
  setup.rs:208 + 5 test sites. Each becomes a Vec construction.

## Accepted / In-flight (Jon-tagged)

- **Android demo touch controls via `virtual_joystick` + `ControlFrame` bridge** `[V3/D3]` — add an optional mobile input path that keeps Leafwing for keyboard/gamepad but translates Bevy touch joystick/buttons into the existing `ControlFrame` seam. Goal is a sideloadable Pixel 5 demo, not polished mobile UX. Source: Android demo discussion; `virtual_joystick` 2.7.x matches Bevy 0.18 and avoids hand-rolling virtual sticks.
  - Add optional `mobile_touch` feature on `ambition_sandbox` pulling `virtual_joystick = { version = "2.7.2", default-features = false }`.
  - Add `mobile_input.rs` with `MobileStick::{Move,Aim}` and systems that read `VirtualJoystickMessage<MobileStick>` into `ControlFrame::{axis_x,axis_y,aim_x,aim_y}`; preserve Ambition's +Y-down convention.
  - Add simple Bevy UI touch buttons/zones for `Jump`, `Attack`, `Dash`, `Blink`, `Interact`, `Projectile`, `Start`, and `Reset`, writing the corresponding `ControlFrame` edge/held fields.
  - Register the mobile systems only behind `mobile_touch` and only in the visible/presentation half, so `SandboxSim`, headless, keyboard, and gamepad paths remain unchanged.
  - Keep Leafwing as the canonical desktop/gamepad mapper; do not replace `SandboxAction` until the mobile demo proves the shape.
  - Add a tiny smoke/test seam: a pure helper that folds a synthetic joystick axis + button state into `ControlFrame`, with tests for deadzone/sign/edge semantics.
  - Document Android demo controls in a focused doc or `CURRENT_STATE.md` note, and move this entry to `FEATURES.md` if the APK boots and the sandbox can move/jump/dash on-device.

## Proposed (agent drop-zone — Jon triages into Accepted / Rejected)

> Agents may append new TODO directions here freely. Do not insert into the
> tier sections above without explicit acceptance. Format: one bullet, V/D
> guess, source/context, ~2 lines max.

- **PyO3 binding for `SandboxSim`** `[V3/D3]` — wraps `SandboxSim::{new, step, observation, reset_episode}` + `AgentAction` / `AgentObservation` as a Python module. Lets RL research code in Python drive the sim without writing Rust glue. Source: SandboxSim landed 2026-05-07 with deterministic stepping; the FFI shape is already designed (owned types, no lifetimes).
- **Boss music binding extension to `BossEncounterRegistry`** `[V4/D3]` — `MusicCueCatalog::encounter_bindings` currently only watches `EncounterRegistry` (mob waves). Extend to also bind `BossEncounterPhase` (Intro/Phase1/Transition/Phase2/Stagger/Enrage) to cue states. Currently `boss_encounter.rs` already publishes `MusicRequested { track }` per phase — we just need the cue side to resolve those into adaptive states. Authored audio assets are still needed for the actual audio change. Source: TODO B `[V4/D3]` audited 2026-05-07; engine wiring is straightforward, audio authoring is the gating cost.

- **Sandbox-side boss controller hook for `BossMovementKind`** `[V4/D3]` — engine schedule data + `evaluate_pattern()` already produce traversal beats; the sandbox `boss_encounter.rs` / `features.rs` boss runtime currently only consumes attack verbs. Wire `step.movement` (Dash / Reposition / Orbit) into actual world transforms so the boss feels mobile. Source: TODO G boss traversal landed 2026-05-07; this is the Bevy half.
- **`BossEncounterSpec.schedules: HashMap<BossEncounterPhase, BossPatternSchedule>`** `[V3/D3]` — replace the `match (spec.id, phase) -> schedule` lookup in `BossEncounterState::current_pattern_schedule` with a per-spec schedules map so future bosses can author their own without code changes. Source: TODO G traversal patterns landed 2026-05-07.
- **Compact LDtk JSON formatter** `[V2/D3]` — the repair script's `json.dumps(indent='\t')` can't reproduce LDtk's editor's inline-arrays-when-short style, producing huge diffs on first apply (200k+ lines). Write a smarter writer that matches LDtk's wrapping rules (or fork an existing JSON5 / LDtk-aware printer). Source: crawl_lab/morph_lab/ladder_lab applies in 2026-05-07 each produced large diffs that subsequent edits don't.
- **Ladders pass through solid blocks (engine flag)** `[V3/D3]` — alternative to authoring a gap in the upper platform whenever a ladder ends at a floor. Add an engine-side rule: while `Player::body_mode == Climbing`, the player's `aabb` ignores collision with `BlockKind::Solid` blocks that overlap the active `climbable_contact.region_aabb`. Generalizes the ladder_lab gap-carve fix and removes a foot-gun for future ladder authors. Source: ladder_lab fix 2026-05-07; Jon suggested either approach.
- **Generated tile sprites for IntGrid layers** `[V3/D3]` — Climbable currently renders as colored placeholder rectangles + rung stripes (`spawn_climbable_region`). Eventually replace with proper tileset textures: ladder_tile.png, vine_tile.png, climbable_wall_tile.png. Same path Water + Hazard + Solid block rendering will eventually take. Source: per Jon's "every tile needs some graphic, even just a placeholder" rule -- placeholder is in place, real art is the polish layer.

*(more ideas below)*

## Accepted

*(items moved here have been agreed; assign a tier letter and migrate up when convenient)*

## Rejected

*(keeps a record so the same idea doesn't get re-proposed)*

---

## Notes
- **Verify before claiming done.** Many "TODO" items in past lists turned out to already be shipped — re-grep + check `git log --all` before assuming.
- This file supersedes `tmp-todo-notes.txt` (now removed; all items folded into the tiers above).
- Source docs: `docs/path_forward.md`, `docs/tech_debt_log.md`, `docs/character_ai_refactor.md`, `docs/crate_split_plan.md`, `docs/events_refactor_plan.md`, `docs/mechanics_checklist.md`, `docs/progression_systems_2026-05-05.md`, `docs/mob_lab.md`. When closing an item, update the source doc too if it tracks the same state.
- Trim "Recently completed" entries here once they have an entry in `FEATURES.md`.
