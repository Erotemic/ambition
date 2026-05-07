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
> Test count: 561 across `cargo test --workspace`.

## Status legend
- `[ ]` not started · `[~]` scaffolded but not feature-complete · `[x]` recently completed (kept here briefly so it doesn't get re-added)
- **`[V?/D?]`** value (1–5) / difficulty (1–5). V: 1=marginal, 5=critical. D: 1=≤30min, 2=1–3hr, 3=half day, 4=multi-day, 5=week+ or risky.
- NOTE: don't always trust difficulty ratings, don't be afraid to tackle something because it is difficult.

## Recently completed (will migrate to FEATURES.md as they age)
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

- [ ] **Wall-cling teleport on mob_lab lock wall** `[V5/D3]` — y=434 → y=-23 snap then ping-pong; same snap-direction class as the resolved wall-jump. Source: `docs/tech_debt_log.md` (HIGH).
  - Capture exact repro positions in a new test file
  - Likely subsumed by the parry contact-normal fix below
- [ ] **Parry contact-normal in `sweep_player_x` / `sweep_player_y` (path_forward step D1)** `[V5/D3]` — replace bespoke snap direction with parry's `ShapeCastHit::normal1`. Retires the entire snap-direction bug class.
  - Plumb `normal1` through hit struct
  - Replace snap branches in [movement.rs:1287](crates/ambition_engine/src/movement.rs#L1287)
  - Re-run `repro_walls.rs` + add fuzz (see B)
- [x] **Double-tap-up Interact trigger audit** — `register_up_tap` in [SandboxRuntime](crates/ambition_sandbox/src/lib.rs#L282) emits the `door_double_tap_up` gesture; [app.rs:1519](crates/ambition_sandbox/src/app.rs#L1519) ORs it into `controls.interact_pressed` before doors / NPCs / chests see it. Single-press Up never fires interact via any current site (chest_open / npc_talk / LoadingZone Door all read `interact_pressed` only). Save-point sites don't exist yet; when they land, route through `interact_pressed` to inherit the same gating.
- [ ] **Glitchy platform behavior (intermittent)** `[V4/D3]` — vague repro. Bug record/replay (see C) would catch this. Source: `tmp-todo-notes.txt`. _Diagnostic logging on riding-state transitions landed via `Player::was_riding_platform` + `tracing::debug!` (target=`ambition::platform`); enable with `RUST_LOG=ambition::platform=debug` to capture the contact transition trail._

## A — Sandbox expressiveness

### Mechanics & combat
- [ ] **Ladder + climbable-zone primitive** `[V4/D3]` — needed before ladder sprite wiring.
  - `ClimbableZone` component
  - Engine state: climb up/down, dismount
  - LDtk IntGrid value + entity option
- [ ] **Ledge grab + climb-up promotion to engine** `[V4/D3]` — move `LedgeProbe`, `Ability::ledge_grab`, state branch into `ambition_engine::player_state` (currently sandbox-side). Add diagonal-corner probe test. Climb-up animation slot separate from grab.
- [x] **NPCs become enemies on hit (in certain circumstances)** — already wired. `NPC_HOSTILE_STRIKE_THRESHOLD = 3` flips NPCs hostile after three player strikes; `apply_save` then replaces the `NpcRuntime` with an `EnemyRuntime` carrying the same id. Hostility persists via `npc_<id>_hostile` save flag. Tested by `striking_npc_three_times_flips_them_hostile` + `apply_save_with_hostile_flag_replaces_npc_with_enemy`. [features.rs:2413](crates/ambition_sandbox/src/features.rs#L2413).
- [~] **More enemy varieties (S/M/L × aggression bands)** `[V4/D4]` — `EnemyArchetype` now has 7 combat archetypes (Combatant, SmallSkitter, SmallLurker, MediumStriker, LargeBrute, LargeColossus, AggressiveSeeker) plus 2 sandbag training variants. Cross-axis invariants tested (HP grows with size; aggro radius grows with aggression; damage scales). LDtk authors enable any of them with `EnemySpawn::brain` set to the brain id (e.g. `large_colossus`). Remaining for the full 9-cell matrix: dedicated low-aggression Medium and high-aggression Medium/Small variants. Authoring trivial — add to `EnemyArchetype::from_brain` and the per-archetype tuning matches.

### Test rooms (sandbox = component showcase)
- [ ] **Crawl/morph proof room** `[V3/D2]` — low-ceiling corridor demo (drivers exist; needs the room).
  - Spec ready at `tools/examples/ldtk_specs/crawl_lab.yaml` and validates clean via `python tools/author_ldtk_area.py … --dry-run`. To apply: add a `connect_to:` block reciprocating into `central_hub_main` at a free LoadingZone position (the spec currently sets `target_zone: east_exit` which already routes to scroll lab — pick a different anchor or extend central_hub_main first).
- [ ] **Save-point lab + persisted-switch test room** `[V4/D3]` — switch state survives reload; reset-switches sub-room; extensible test-state schema (boss defeated, mob room cleared). Source: `tmp-todo-notes.txt`.
- [ ] **Quest test room** `[V3/D3]` — small fetch/talk quest end-to-end ([quest.rs](crates/ambition_sandbox/src/quest.rs) is scaffolded).
- [ ] **Cutscene test room** `[V3/D2]` — entry trigger fires "you're finally awake"; demonstrates cutscene + skip flow.
  - Cutscene infrastructure works today via `central_hub_main` ↔ `test_intro` binding ([cutscene.rs:123](crates/ambition_sandbox/src/cutscene.rs#L123)). What's missing: a dedicated test room (separate from the spawn hub) demonstrating the skip flow on a non-default cutscene. Authoring via `tools/author_ldtk_area.py` once a target connection point is decided.

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

## C — Engine cleanups (compounding)

- [x] **ADR 0012 events refactor — main work** — Slices 1-5 (sfx / vfx / debris messages, setup split, headless `sandbox_update`) all landed. `app.rs` has zero direct `play_sound` / `spawn_burst` / `spawn_dust` / `spawn_impact` calls; `fx.rs` only calls them as the consumer of `VfxMessage`. Headless ticks `sandbox_update` cleanly via `run_headless`. Remaining cleanup ([ ] hardening only):
  - [ ] Tighten `SandboxRuntime` field visibility from `pub` to `pub(crate)` (deferred — risks breaking `bevy-inspector-egui` field reflection; revisit when the inspector wiring uses Reflect-only access patterns).
  - [x] Add `tests/scripted_gameplay.rs` integration test (3 scenarios: 30 idle frames, Reset press emits Reset message, heterogeneous Reset/Jump/move sequence runs to completion).
- [ ] **CharacterAi authoritative migration** `[V3/D4]` — convert one enemy archetype's movement to read evaluator output (currently observed-only); then one boss pattern; parity test. Source: `character_ai_refactor.md`.
- [ ] **Bug record/replay ring buffer** `[V4/D4]` — last 600 frames of `ControlFrame + SimMessages + player snapshot`, F12 / auto-OOB dump, replay binary. Would have caught the glitchy-platform bug. Source: `path_forward.md` step F.
- [ ] **`bevy_rl` integration for AI playtesting** `[V4/D4]` — RL agents that exercise the sandbox to surface bugs (and eventually for proper RL training).
  - Define observation space (player state, nearby blocks/enemies, room id)
  - Define action space (mirror `SandboxAction`)
  - Reward shaping for "explore + don't die"
  - Wire to `headless` binary as a separate run mode
  - Doubles as a fuzz harness for movement / collision

## D — Compile-time investments

- [ ] Split `features.rs` (2819 lines) into `features/{hazards,enemies,bosses,breakables,pickups,npcs}.rs` `[V4/D4]`
- [ ] Split `ldtk_world.rs` (2567 lines) into 7 modules per `path_forward.md` step C — only `bevy_runtime.rs` extracted (1 of 7) `[V3/D4]`
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
- [ ] Improved boss movement patterns — traversal choreography (boss dash, arena reposition) `[V4/D3]`. Source: `tmp-todo-notes.txt`.
- [ ] Intro cutscene polish — "Hey you, you're finally awake" beat `[V2/D3]`

---

## Proposed (agent drop-zone — Jon triages into Accepted / Rejected)

> Agents may append new TODO directions here freely. Do not insert into the
> tier sections above without explicit acceptance. Format: one bullet, V/D
> guess, source/context, ~2 lines max.

*(empty — populate as ideas arise)*

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
