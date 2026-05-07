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
> Last full re-audit: 2026-05-07 (against `git log --all`)

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
- [ ] **Double-tap-up Interact trigger** `[V4/D2]` — `Interact` action exists; the double-tap-up *gesture* binding may not. Audit door / NPC / save-point sites and ensure single-press Up never fires interact. Source: `tmp-todo-notes.txt`, feedback memory.
- [ ] **Glitchy platform behavior (intermittent)** `[V4/D3]` — vague repro. Add diagnostic logging on platform contact transitions. Bug record/replay (see C) would catch this. Source: `tmp-todo-notes.txt`.

## A — Sandbox expressiveness

### Mechanics & combat
- [ ] **Glide / slow-fall** `[V3/D2]` — held ability; reduced fall speed + air control. Source: `mechanics_checklist.md`.
- [ ] **Ladder + climbable-zone primitive** `[V4/D3]` — needed before ladder sprite wiring.
  - `ClimbableZone` component
  - Engine state: climb up/down, dismount
  - LDtk IntGrid value + entity option
- [ ] **Ledge grab + climb-up promotion to engine** `[V4/D3]` — move `LedgeProbe`, `Ability::ledge_grab`, state branch into `ambition_engine::player_state` (currently sandbox-side). Add diagonal-corner probe test. Climb-up animation slot separate from grab.
- [ ] **NPCs become enemies on hit (in certain circumstances)** `[V3/D3]` — define faction/threshold rules; test NPC archetype that flips after N hits. Source: `tmp-todo-notes.txt`.
- [ ] **More enemy varieties (S/M/L × aggression bands)** `[V4/D4]` — 9 archetype rows via existing `EnemyArchetype` data. Source: `tmp-todo-notes.txt`.

### Test rooms (sandbox = component showcase)
- [ ] **Crawl/morph proof room** `[V3/D2]` — low-ceiling corridor demo (drivers exist; needs the room).
- [ ] **Save-point lab + persisted-switch test room** `[V4/D3]` — switch state survives reload; reset-switches sub-room; extensible test-state schema (boss defeated, mob room cleared). Source: `tmp-todo-notes.txt`.
- [ ] **Quest test room** `[V3/D3]` — small fetch/talk quest end-to-end ([quest.rs](crates/ambition_sandbox/src/quest.rs) is scaffolded).
- [ ] **Cutscene test room** `[V3/D2]` — entry trigger fires "you're finally awake"; demonstrates cutscene + skip flow.
- [ ] **Time-decay breakable platforms** `[V2/D2]` — keep attack-break variant; add stand-too-long variant. Source: `tmp-todo-notes.txt`.

### Sprite wiring batch
- [~] `morph_ball.png` → `MorphBallSprite::handle` `[V2/D1]` — sprite type wired; verify asset path
- [ ] `switch_armed.png` / `switch_disabled.png` → Switch rendering `[V2/D1]`
- [ ] `lock_wall_tile.png` → runtime-inserted lock walls `[V2/D1]`
- [ ] `water_surface_tile.png` → overlay layer above water `[V2/D2]`
- [ ] `ladder_tile.png` → climbable zones `[V2/D1]` (gated on Ladder primitive)
- [ ] `acid_tile.png` / `lava_tile.png` → IntGrid value mappings `[V2/D1]`
- [ ] `bg_circuit_tile.png` → parallax in `central_hub_complex` `[V2/D2]`

### Architecture
- [ ] **Stitched (loading-zone-free) room transitions** `[V4/D4]` — user wanted basement reachable by "drop down" from hub, not via load. Source: `tmp-todo-notes.txt`.
  - Adjacency model in `RoomSet`
  - Camera + collision spans both rooms during traversal
  - Debug overlay zoom-out to view stitched layout
  - Decision: stitching vs single big room — prototype both

## B — Authoring ergonomics, validators, audio polish

- [ ] **Adaptive music phase transitions in boss room** `[V4/D3]` — stem-blend/layered tracks via existing [music.rs](crates/ambition_sandbox/src/music.rs) hooked to `BossPhase`. Cues already authored. Source: `tmp-todo-notes.txt`.
- [ ] `BodyShape::fits_at` property test (proptest) `[V3/D2]`
- [ ] Wall-jump start-position fuzz in `square_arena` `[V3/D2]`
- [ ] `cargo test` smoke for `headless` binary in CI `[V4/D2]`
- [ ] Reduce text in debug HUD now that bevy-inspector is integrated `[V2/D1]`. Source: `tmp-todo-notes.txt`.
- [ ] Per-cutscene "always skip if seen" flag `[V2/D2]`

## C — Engine cleanups (compounding)

- [ ] Promote sandbox-side `mana_current` / `mana_max` to engine `ResourceMeter` (engine has the type; sandbox still has separate fields) `[V2/D2]`
- [ ] `slash_damage` / `invincible` → per-player engine state `[V2/D2]`
- [ ] **Finish ADR 0012 events refactor** `[V3/D3]` — remaining call sites; confirm headless ticks `sandbox_update` cleanly. Source: `events_refactor_plan.md`.
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
- [ ] Quest panel: separate quest lines from debug HUD `[V2/D2]`
- [ ] Map menu: room-name labels + zoom controls (extends current minimap) `[V3/D2]`
- [ ] Camera ease parameterization (close LOW tech-debt) `[V2/D2]`

## F — Documentation / hygiene

- [ ] Docstrings on `ProgressionResources` and `SandboxQueues` `[V1/D1]`
- [ ] Sync `mechanics_checklist.md` against landed BodyMode + Hadouken work `[V1/D1]`
- [ ] Archive applied music-renderer overlay docs in `docs/` once their patches land `[V1/D1]`
- [ ] Wire FEATURES.md update into a checklist when closing TODO items `[V2/D1]`

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
