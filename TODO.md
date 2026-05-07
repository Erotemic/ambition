# Ambition TODO

> **Sandbox-first.** The sandbox *is* the vertical slice — every gameplay component
> assembled in a test-arranged form rather than a story-arranged one. Story-arranged
> slice content (hub → first zone → Gradient Sentinel) resumes once the sandbox
> bar is met.
>
> Last updated: 2026-05-07

## Status legend
- `[ ]` not started
- `[~]` partial / scaffolded but not feature-complete
- `[x]` recently completed (kept here briefly so it doesn't get re-added)

## Recently completed (do not re-add)
- [x] Wall-jump OOB fix: `cast_shapes(toi=0)` rejection in `sweep_player_x/_y` — [movement.rs:1321](crates/ambition_engine/src/movement.rs#L1321)
- [x] Wall-jump repro test — [tests/repro_walls.rs](crates/ambition_sandbox/tests/repro_walls.rs)
- [x] `BodyMode::Crawling` and `BodyMode::Sliding` drivers — [player_state.rs](crates/ambition_engine/src/player_state.rs)
- [x] `evaluate_character_ai` engine fn + per-brain knobs (chase_speed, aggro_radius) on `EnemyArchetype` — [character_ai.rs](crates/ambition_engine/src/character_ai.rs)
- [x] `RoomSet::layout_warnings` branch tests — [rooms.rs:351](crates/ambition_sandbox/src/rooms.rs#L351)
- [x] LDtk validator: blank `activeArea` rejection — [ldtk_world.rs](crates/ambition_sandbox/src/ldtk_world.rs)
- [x] Room data: `ambient_profile` and `visual_theme` fields exist (consumers still pending — see B)

---

## S — Active sandbox blockers (do first)

- [ ] **Wall-cling teleport on mob_lab lock wall** — player y=434 → y=-23 snap then ping-pong; same snap-direction class as the resolved wall-jump bug. Source: `docs/tech_debt_log.md` (HIGH).
- [ ] **Parry contact-normal in `sweep_player_x` / `sweep_player_y` (path_forward step D1)** — replace bespoke snap direction with parry's `ShapeCastHit::normal1`. Retires the entire snap-direction bug class (wall-jump + mob_lab teleport are instances).
- [ ] **Double-tap-up Interact binding** — single-press Up still triggers doors/NPCs in places. Add dedicated Interact action (E / F / RB / double-tap-Up). Sources: `todo.txt`, feedback memory.

## A — Sandbox expressiveness

### Mechanics (new gameplay verbs)
- [ ] **Swim** + `water_lab` room — add `BlockKind::Water`, integrate through ~10 hazard match sites, author LDtk room. Source: `progression_systems_2026-05-05.md`.
- [ ] **Glide / slow-fall** — reduced fall speed + air control while held. Source: `mechanics_checklist.md`.
- [ ] **Ladder + climbable-zone primitive** — needed before ladder sprite wiring lands.
- [~] **Morph Ball** — engine drivers exist; needs collision-safe morph-tunnel tests + visible scale per room. Source: `mechanics_checklist.md`.
- [ ] **Ledge grab promotion to engine** — move `LedgeProbe`, `Ability::ledge_grab`, movement-state branch into `ambition_engine::player_state`. Currently sandbox-only.
- [ ] **Swim post-update mutator → engine state machine** — fold sandbox mutator into engine `Player`.

### Sprite wiring batch
- [~] `morph_ball.png` → `MorphBallSprite::handle` (sprite type wired; asset path unverified)
- [ ] `switch_armed.png` / `switch_disabled.png` → Switch rendering
- [ ] `lock_wall_tile.png` → runtime-inserted lock walls
- [ ] `water_surface_tile.png` → overlay layer above water bodies
- [ ] `ladder_tile.png` (paired with climbable-zone primitive above)
- [ ] `acid_tile.png` / `lava_tile.png` → IntGrid value mappings
- [ ] `bg_circuit_tile.png` → parallax layer in `central_hub_complex`

### Test rooms (sandbox = component showcase)
- [ ] Crawl/morph proof room — low-ceiling corridor demo
- [ ] Save-point lab — exercises `save_point` sprite + save system end-to-end
- [ ] Cutscene test room — triggers "you're finally awake" intro on entry; demonstrates cutscene system + skip flow
- [ ] Water lab room — pairs with Swim above

## B — Authoring ergonomics & validators

- [ ] **EncounterReward** field on `EncounterSpec` — replace hardcoded `Health{amount:2}`
- [ ] **encounter_id** LDtk field on `BossSpawn` — replace name-derived id with explicit field
- [ ] LDtk validator: warn on unknown `music_track` ids
- [ ] `ambient_profile` consumer — drives ambient SFX layer from Room data
- [ ] `visual_theme` consumer — drives renderer palette swap from Room data
- [ ] `BodyShape::fits_at` property test (proptest over random rect placements)
- [ ] Wall-jump start-position fuzz test in `square_arena`
- [ ] Diagonal-corner ledge-grab test (fills the gap in existing ledge_grab tests)
- [ ] `cargo test` smoke for `headless` binary in CI

## C — Engine cleanups (compounding, low urgency individually)

- [ ] `runtime.player_died_pending` boolean → `PlayerDiedMessage` (Bevy 0.18 Message API)
- [ ] `mana_current` / `mana_max` on `SandboxRuntime` → `ResourceMeter` on Player
- [ ] `slash_damage` / `invincible` → per-player engine state
- [ ] **Finish ADR 0012 events refactor** — remaining call sites; confirm headless ticks `sandbox_update` cleanly. Source: `events_refactor_plan.md`.
- [ ] **CharacterAi authoritative migration** — convert one enemy archetype's movement to read `evaluate_character_ai` output (currently observed-only); then one boss pattern; add parity test against timer-driven baseline. Source: `character_ai_refactor.md`.
- [ ] **Bug record/replay ring buffer** — last 600 frames of `ControlFrame + SimMessages + player snapshot`, dump to JSON on F12 / auto-OOB; replay binary for deterministic repro. Source: `path_forward.md` step F.

## D — Compile-time investments (heavy commits, real wins)

- [ ] Split `features.rs` (single 104KB file) into `features/{hazards,enemies,bosses,breakables,pickups,npcs}.rs`
- [ ] Split `ldtk_world.rs` into 7 modules per `path_forward.md` step C (only `bevy_runtime.rs` extracted so far — 1 of 7)
- [ ] Promote `KinematicPath` to typed components + index (matches LDtk migration pattern)
- [ ] **Extract `ambition_game` crate** — engine / game / sandbox 3-crate layout. Holds encounter, boss_encounter, quest, cutscene, save, ledge_grab, swim, map_menu, NPC AI, audio, rendering primitives. Source: `crate_split_plan.md`.

## E — UI / inventory / polish (non-blocking)

- [ ] **N64 OOT/MM-style spinning-cube inventory** — 4 faces (map / loadout / quests / system options); modernize contents but keep nostalgic cube spin + menu-change SFX
- [ ] Quest panel: separate quest lines from debug HUD into its own UI node
- [ ] Settings page reset-to-defaults flow
- [ ] Per-cutscene "always skip if seen" flag
- [ ] Map menu: room-name labels + zoom level controls
- [ ] Camera ease parameterization

## F — Documentation / hygiene

- [ ] Docstrings on `ProgressionResources` and `SandboxQueues`
- [ ] Sync `mechanics_checklist.md` — Morph Ball marked `[ ]` despite engine drivers landing
- [ ] Archive pre-Bevy-0.18 port notes (`docs/bevy_port.md`, `docs/leafwing_input_manager_port.md`)
- [ ] Archive root-level audio overlay variants (`README_audio_*.txt`, `README_music_renderer_*.txt`) — historical iteration snapshots, not actionable
- [ ] Archive applied music-renderer overlay docs in `docs/` once their patches land

## G — Story-arranged slice (resume after sandbox bar is met)

- [ ] Real central hub authoring
- [ ] Basement / first zone authoring
- [ ] Gradient Sentinel boss implementation
- [ ] Improved boss movement patterns — traversal choreography (boss dash, arena reposition), not just attack telegraphs
- [ ] Boss-phase music tracks in `sandbox.ron` (3-4 entries)
- [ ] Intro cutscene polish — "Hey you, you're finally awake" beat; visor-blip / sprite-emerge; skip-or-tutorial fork

---

## Notes
- This file supersedes `tmp-todo.txt` at the repo root — fold any new captures here.
- Items reference docs that exist at `docs/path_forward.md`, `docs/tech_debt_log.md`, `docs/character_ai_refactor.md`, `docs/crate_split_plan.md`, `docs/events_refactor_plan.md`, `docs/mechanics_checklist.md`, `docs/progression_systems_2026-05-05.md`. When closing an item, update the source doc too if it tracks the same state.
- The "Recently completed" block exists so audits don't re-add finished work. Trim entries older than ~2 weeks.
- Be on the lookout for things that claim they were done, but were not actually done. 
