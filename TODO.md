# Ambition TODO

Centralized work queue for multi-hour autonomous agent sessions.

This file is intentionally **not** a changelog. It is the place to keep unfinished work that is useful enough for an agent to pick up without re-discovering it from git history, scratch notes, or old overlay readmes.

## Operating rules

- **Sandbox-first.** The sandbox is the vertical slice: gameplay components should be assembled in test-arranged rooms before they are story-arranged into the final hub / first zone / boss sequence.
- **Outstanding work lives here.** `FEATURES.md` is the compact capability inventory for landed behavior; this file is the centralized queue for unfinished behavior, docs, and validation work.
- **Verify before claiming done.** Re-grep code and docs before closing an item. Many old TODO entries turned out to be shipped or superseded.
- **Move, do not duplicate forever.** When a task lands, either remove it from this file or move the durable lesson to `docs/history/progression-log.md`, `dev/journals/`, or `docs/archive/`.
- **Prefer agent-sized tasks.** Each accepted item should be concrete enough for a 1-4 hour autonomous session with clear files, tests, or validation commands.

Useful companion docs:

- Current state: `docs/current/state.md`
- Current next moves: `docs/current/next.md`
- Planning sequence: `docs/planning/path-forward.md`
- Tech debt: `docs/planning/tech-debt-log.md`
- Capability inventory: `FEATURES.md`
- Mechanics status: `docs/mechanics/expressibility-checklist.md`

# LOW HANGING FRUIT MAYBE

- [ ] Increase the resolution of gnuton so the sprite is not drawn pixelated. 

- [x] **NO BAKED DROP SHADOWS (2026-05-22)** — Project rule in agent memory ([[feedback-no-drop-shadows-on-sprites]]). Active generators (`intro_cart`, `creator_lab_props`) stripped in 3a2f5e3. Future generators must omit baked shadows by default; cast shadows belong in the ECS visual layer. **Follow-up:** add a renderer-time assertion that fails if there are opaque pixels below the foot-anchor row on a generated sprite, so a baked shadow trips CI instead of silently shipping.

- [x] **LDtk authoring lint trio (2026-05-22, 3a2f5e3)** — `validate` now warns on (a) DebugLabel overlaps, (b) mid-air `Door` LoadingZones, (c) level boundaries that have no Solid AND no EdgeExit. Surfaces 32 warnings on the live intro slice — the actual content cleanup is the next item.

- [ ] **Fix the 32 intro warnings surfaced by the new lint** `[V4/D3]` — run `python -m ambition_ldtk_tools validate crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk` and resolve each warning (missing walls → add Solids covering boundaries; mid-air doors → either add support under them or convert to EdgeExits; DebugLabel overlaps → space them out). Touches `intro_*_area.yaml` specs and the live `intro.ldtk`. Pair this with the gridvania conversion below.

- [x] **News Board sprite (2026-05-22, 16641b6+)** — procedural `news_board` target ships a wall-mounted bulletin board with Disruptor Industries header + blinking LED, wired through `NEWS_BOARD_SHEET` so the bulletin no longer renders as a person.

- [x] **Gate-stack reveal cutscene removed (2026-05-22)** — auto-fire cutscene that interrupted every entry to `gate_stack_lower` is gone. Future polish can re-add a quieter banner-only beat.

- [ ] **Avoid doors + teleports in the vertical slice; build a gridvania world.** `[V5/D5]` — Current intro chains 10+ rooms via Door LoadingZones, which makes the slice feel like a hub with a million side-rooms. Convert to EdgeExits (stitched corridors) and connected geometry so the player traverses continuously. The new mid-air-doors lint already flags the doors that need to go.

- [ ] **Better door sprites + variable door sizes** `[V3/D3]` — current door visuals look the same regardless of width/height. Procedurally generate frame variants (small / standard / tall / wide / double) keyed off `LoadingZone.size`, with the door art aligned to the size. Could probably reuse the lasersword DESIGN/OUTPUT split pattern.

- [~] **Smash-Bros style ledge feel** `[V4/D4]` — first chunk shipped 2026-05-22 (ffbaaf1): `LedgeGetupKind::{Climb, Roll}`, shield-held triggers a forward roll with full invulnerability via `dodge_roll_timer`, brief intangibility window on grab, tightened `LEDGE_MIN_CLIMB_DELAY` for snap feel. Follow-ups: ledge getup-attack (attack from hang), Smash-style fall-through-then-up-B reclaim, document the ledge contract as an ADR so the port-to-Smash goal stays explicit, optional second-roll cooldown so spammers can't permastall.

- [ ] **Wall-clipping bugs in the intro** `[V4/D4]` — Jon's noted ongoing bad-clipping-through-walls errors during intro playthroughs. Probably a mix of (a) sub-pixel collision drift at high speeds, (b) thin-wall (16-px) corner cases, (c) the trace-replay-driven lock-wall debt already in the active blockers list. Cross-link with the existing wall-cling teleport entry in `docs/planning/tech-debt-log.md`.

- [x] **Wire contextual button labels into the UI (2026-05-23)** — Replaced by a richer `player::affordances` module: typed per-verb `*Variant` enums + pure resolvers + a `PlayerAffordances` resource updated each frame. Touch HUD reads the resource through `ButtonVerb` + `ButtonGlyph` + `ButtonPressed` components (per-device key/button glyphs for keyboard, gamepad-stub, touch; pressed-state highlight doubles the overlay as a streamer-style input display). The L-stick has no separate label overlay — per Jon, the knob's drag position is the direction indicator on its own. `contextual_actions.rs` deleted. See `docs/systems/control-affordances.md`. Followups: thread the player's selected `KeyboardPreset` through to the glyph adapter (today reads the default Arrows+ZXC preset); wire gamepad-kind detection once Bevy 0.18's gamepad API is verified; add a regression test once gameplay subsystems start consuming the same resolvers so HUD/sim can't drift.

- [x] **Auto-spacer for overlapping DebugLabels** `[V2/D2]` — Done 2026-05-24. `space_debug_labels` subcommand under `ambition_ldtk_tools edit` shoves overlapping labels into a vertical stack with sensible padding, in place. Used to clear the last 2 overlap warnings from intro.ldtk. _Bonus deferred:_ widen the entity `width` field to fit the actual text when text is longer than authored width — no concrete need yet.

- [ ] **Renderer-time "no shadow below foot" assertion** `[V2/D1]` — pair with the no-drop-shadow rule. After auto-cropping a generated sprite, assert that the bottom row of the alpha bbox has at least some opaque pixels (true foot present), AND that there are no opaque pixels below the body in any frame (no soft baked shadow). Fails the renderer at CI time before any artist re-introduces a shadow.

- [x] **Character catalog refactor + Hall of Characters (2026-05-24)** — Full plan executed in 7 phases (`TODO-character-catalog-and-hall.md`). Lands `assets/data/character_catalog.ron` as the single source of truth for spawnable characters (99 entries), `CharacterCatalogPlugin` with Startup validator, LDtk `NpcSpawn.character_id` schema, the auto-generated Hall of Characters room (89 main + 10 basement pedestals), YAML→RON area-spec migration, `NPC_SPRITE_REGISTRY` + `npc_sprite_label` deletion, and the renderer `review_npcs`→`characters` category merge. Architectural posture codified in ADR 0017. See `docs/systems/character-catalog.md` for the live system doc.

  Follow-ups deferred from this refactor:
  - **`boss_encounters/<id>.ron` per-boss phase schedules** `[V3/D3]` — `BossPattern` brain preset references encounter ids but the schedules are still hardcoded in Rust. Move to RON files under `crates/ambition_sandbox/assets/data/boss_encounters/`. _Status 2026-05-24:_ numeric-fields half landed for all 3 authored bosses (`gnu_ton`, `mockingbird`, `clockwork_warden`); each ships an `<id>.ron` that overrides the hardcoded `BossEncounterSpec` constructor via `default_boss_profiles`. Remaining: move per-phase brain schedules once `BossPattern` has hooks for them.
  - **`SheetRegistry`-driven sprite specs** `[V3/D4]` — currently the per-character `CharacterSheetSpec` consts come from hardcoded Rust statics in `presentation/character_sprites/sheets.rs` paired with a `sheet_for_character_id` match. Drive the spec from the on-disk manifest at startup so adding a new character is a single catalog-row + manifest pair, no Rust patch.
  - **Per-instance brain override semantics** `[V2/D2]` — `NpcSpawn.brain_override` / `brain_overrides` fields. Planned in the original RFC but no concrete use case has landed yet; defer until a story room demands it.

## Status legend

- `[ ]` not started
- `[~]` scaffolded / partially shipped but not feature-complete
- `[x]` done recently; keep only briefly when it prevents immediate re-adds
- **`[V?/D?]`** value / difficulty, both 1-5. V: 1=marginal, 5=critical. D: 1=<=30min, 2=1-3hr, 3=half day, 4=multi-day, 5=week+ or risky.

## S - Active blockers and high-signal defects

- [x] Morph ball is broken. Sprite appears spawns in the room and moves when you enter morph ball and then stays when you leave. The robot sprite is also drawn when in morph ball mode, squished on top of the morph ball.

- [x] Pickups are broken. They don't disappear when collected.

- [x] The guy on gnuton's shoulders looks too much like Harry potter. Get rid of the glasses. He should look like Isaac Newton with a powdered wig.

- [x] A character should generally not bark a dialog response line if they are already in the middle of another one. 

- [x] I would like to have a better ledge get-up effect that isn't just a diagonal motion. I would also like a rolling getup option in addition to the normal getup option. In both cases we should make sure the path doesn't go through a wall or we should not allow the getup / roll (really we shouldn't even allow a ledge grab unless the ledge itself has an open path to it)

- [~] Enemy hurt boxes still don't visualize correctly. I can throw a fireball at gnuton and he will get hit before the fire ball collides with the visible box, so something is up with collision or the box isn't drawing.

- [~] **Wall-cling / lock-wall / collision-correction debt** `[V5/D3]` - Keep the trace-backed lock-wall problem central until the root collision-correction behavior is fixed. The current authoritative write-up is `docs/planning/tech-debt-log.md`; use `docs/systems/gameplay-trace-recorder.md` for dump/replay workflow. Do not hide this by widening OOB margins.
  - Good session shape: build or improve a reproduction around the goblin-encounter lock wall / ceiling geometry, then assert that position correction cannot exceed the frame's velocity budget unless Reset or RoomTransition fired.
  - Validation anchors: `cargo test -p ambition_sandbox --test repro_walls`, `cargo test -p ambition_sandbox trace`, focused movement/collision tests.

- [ ] **Goblin encounter music transition still sounds like a section swap** `[V4/D3]` - The player hears the intro fade out / wave1 enter instead of one continuous musical idea. Existing scratch context was promoted into `docs/recipes/generated-music-workflow.md`; use `debug-notes-music.md` only as historical detail if still present.
  - Diagnose first: compare generated and installed OGGs, run `audit_cue_balance.py`, inspect logs for `gain_start=target`, then decide whether the fix is score arrangement, mastering, stem balance, or runtime fade policy.
  - Related follow-ups live under the audio section below: level report, live gain HUD, equal-power crossfade, and per-stem mastering.

- [ ] **Cutscene/dialogue input and presentation polish** `[V4/D3]` - `issues.md` reports cutscenes that only show text in debug view and say the wrong continue prompt. Route acknowledgement through the same canonical input seam used by keyboard/gamepad/touch, and make the displayed prompt match the active control method.
  - Good session shape: find the current cutscene UI path, make one visible prompt accurate, then add or update a scripted-gameplay test for dialogue acknowledgement.

- [ ] **Menu mouse-hover vs keyboard navigation conflict** `[V3/D2]` - If the mouse is hovering an option while the user navigates by arrow keys / controller, the hover state can fight selection movement. Make menu focus policy explicit: pointer motion may set focus, but stale pointer position should not override a newer keyboard/controller navigation edge.

## A - Sandbox expressiveness and mechanics

### Movement, traversal, and collision

- [~] **Ledge grab + climb-up polish / engine contract** `[V4/D3]` - `ambition_engine::ledge_grab` exists, but the mechanic still needs polish, animation coverage, and corner/one-way validation. Add diagonal-corner probe tests and confirm the sandbox driver uses the engine primitive rather than duplicating probe logic. Instead of 

- [ ] **Ladders pass through solid blocks option** `[V3/D3]` - Avoid requiring authors to carve a gap in the platform whenever a ladder reaches a floor. Consider an engine-side rule: while `BodyMode::Climbing`, ignore `BlockKind::Solid` overlaps that coincide with the active climbable contact region, or expose an authored ladder-top passthrough flag.
  - Validate against `ladder_lab` / climbable zones and one-way platforms.

- [ ] **Ladder movement polish** `[V3/D2]` - Moving on a ladder should not feel sluggish, and the player should be able to jump or dash off it cleanly. Check `BodyMode::Climbing`, `ControlFrame`, gravity suspension, and transition rules.

- [ ] **Stitched / loading-zone-free room transitions** `[V4/D4]` - Prefer Gridvania-style side-scrolling exits and large connected spaces over door-heavy room hops where practical. Current LDtk concepts mention stitched boundaries; make one robust prototype where camera, collision, and transition safety work across adjacent rooms without a door load.
  - Decide per case whether the better authoring answer is stitched rooms or one large LDtk level.

- [ ] **Projectile collision with one-way and non-solid surfaces** `[V3/D2]` - `TODO-drafts.md` says the fireball should respect and bounce correctly on all relevant surfaces, not only solids. Current sandbox code has one-way handling, but keep this open until the behavior is visibly tested in a room and traces show the expected bounce/pass-through cases.

- [ ] **Gravity room / gravity columns** `[V3/D4]` - Prototype columns that change gravity direction, with switches that toggle them. Some columns may ride moving platforms. This should exercise coordinate remapping, player movement policy, and room authoring rather than becoming one hard-coded trick.

- [ ] **Sprite frame dimensions read from YAML at runtime, not hardcoded** `[V3/D3]` — `crates/ambition_sandbox/src/presentation/character_sprites/sheets.rs` declares ~25 `CharacterSheetSpec` consts with `frame_width` / `frame_height` / `label_width` hardcoded. Every regen of a character generator can change those numbers (the `pirates/common::build_sheet` auto_crop fits each frame to the union alpha bbox + 2px margin). On 2026-05-22 the pirates and shark drifted hard — game extracted 128×128 windows while the actual frames were 103×114 / 162×76, so the URect picked up the wrong region of the PNG (visible as misaligned bounding boxes around characters). Quick fix landed (53b4edf or follow-up commit) that resynced 17 sheets to current YAMLs and added a `sheet_consts_match_their_yaml_manifests` test so future drift fails the build. Long-term: drop the consts, load the manifest at startup (or build.rs codegen from the YAML), so the source of truth is the renderer output.

- [~] **Simple falling-sand sim room** `[V2/D4]` - Prototype landed in `crates/ambition_sandbox/src/falling_sand.rs` (feature `falling_sand`, on by default for `desktop_dev`). Spouts emit sand / water / oil / mixed, LDtk room walls are mirrored as `bevy_falling_sand` wall particles, dense tiles are projected as one-way `ae::Block`s so the player collides with piles, and a 1Hz `fs-diag` log streams counts + Y range + per-component coverage. Root cause of "particles never moved" was a missing `ChunkLoader` entity — v0.7.0's `update_chunk_loading` returns early when none exists, so no chunk entities ever get spawned and the movement-by-chunks query returns nothing (fixed bc1d5d5). Pile-up still reads "a bit weird" — leaving in-progress for polish. **Follow-ups (in rough priority):** (a) the `fs-diag` `band_grid_y` calculation picks the topmost LDtk block via `min(block.min.y)` and labels it the floor — should pick the largest `block.min.y` (visible floor) instead; (b) some material seems to pile near the room ceiling — investigate whether the spout/ceiling wall geometry leaves stranded pockets, possibly skip seeding walls on overhead blocks; (c) the projected one-way `Block` flicker remains (sort-by-count fix helped but particles flowing through dense tiles still oscillate the cap admission); (d) consider switching emit to `try_multiple` over a tall column so spouts choke gracefully on backed-up streams; (e) eventually port to v0.8 once it stabilizes (`Particle` becomes a marker, API simpler). Also: fire-from-fireballs and traceable-stream interactions aren't wired yet — that was the original scope.

### Combat, abilities, and interactions

- [ ] **Bubble shield dodge / roll extension** `[V3/D3]` - Bubble + Down should dodge; Bubble + direction should roll. Define resource cost, invulnerability frames, collision safety, and whether roll is a locomotion state or gameplay effect.

- [x] **Context-sensitive action buttons / control HUD (2026-05-23)** - The on-screen touch HUD now reads as a context-sensitive control overlay even on desktop builds. Each button shows the verb it would invoke right now (Attack → `Jab` / `D-Tilt` / `N-Air` / `D-Air` / `B-Air` / `Pogo` based on stick + aerial + facing + pogo-target proximity; Shield → `Roll` on a ledge; Jump → `Climb` / `Unmorph` / `Stroke`; Interact → `Talk` / `Open` / `Activate` / authored prompt; Special → `N-Special` / `S-Special` / `U-Special` / `D-Special` / `Hadouken` seam) plus a per-device glyph subtitle ("Z" on keyboard, "A" / "X" / "RB" on Xbox-like pads, shape names on PlayStation; touch buttons render as themselves). Held buttons brighten as a streamer-style input display; the L-stick stays unlabelled so the knob's drag position is the only direction indicator (per Jon's "the control stick itself would be the thing that moves"). See `docs/systems/control-affordances.md` for the architecture.

- [~] **More enemy varieties across size and aggression bands** `[V4/D4]` - `EnemyArchetype` already covers several combat shapes. Finish the missing low/medium/high combinations only if they create distinct encounters, not just more enum values. Validate HP, aggro radius, damage, and LDtk brain IDs.

- [ ] **Actor aggression / NPC-enemy distinction polish** `[V4/D3]` - `issues.md` states the architecture should distinguish enemy vs NPC mostly by aggressiveness, not by separate conceptual species. Check current `Actor`, faction, `NpcRuntime`, hostility conversion, and `EnemyRuntime` paths; reduce special cases where possible.

### Test rooms and progression laboratories

- [~] **Save-point lab + persisted-switch test room** `[V4/D3]` - `switch_lab` demonstrates switch persistence, but a dedicated save-point entity and save/lab flow are still useful. Add a save-point interactable distinct from regular switches, a reset-switches sub-room, and a broader test-state schema for boss defeated / mob room cleared / save restoration.

- [ ] **Additional body-mode traversal rooms** `[V3/D3]` - Current body-mode vocabulary is available, but more authored rooms should prove crouch, crawl, slide, morph, stand-up rejection, and compact traversal under real LDtk collision.

- [ ] **Tutorial refresher / quest reminder rule** `[V3/D3]` - Design rule: the game should never permanently strand a player without a way to review tutorial controls, current quests, or what they were doing. Add a durable UI/UX plan and a small sandbox proof.

- [ ] **Alice/Bob/Eve/Mallory/etc. NPC cast** `[V2/D3]` - Potential cryptography-themed NPC set: Bob the architect, Alice the cryptographer, Eve listener, Mallory malicious attacker, Trudy intruder, Craig cracker, Sybil identity attacker, Trent arbitrator, Victor/Peggy verifier/prover, Walter warden, Olivia oracle, Judy judge. Add these NPCs to various rooms in the sandbox.

## B - Audio, generated assets, and authoring tools

- [ ] **Boss music binding extension** `[V4/D3]` - Extend adaptive music cue bindings beyond mob encounters so boss phases such as Intro / Phase1 / Transition / Phase2 / Stagger / Enrage can resolve to cue states. Mechanism exists in pieces; authored audio identity is the gating cost.

- [ ] **Music level / mixing debug tools** `[V3/D3]` - Add a diff-friendly level report for `assets/audio/music/generated/<cue>/`: integrated LUFS if available, true peak, RMS, duration, and target-loudness delta. Add an optional in-engine dev HUD showing live per-bank / per-slot gains and post-master dB so transition dips are visible while playing.

- [ ] **Equal-power crossfade in runtime gain smoothing** `[V2/D2]` - The current per-slot smoothing can create a midpoint power dip. Investigate a phase-driven sin/cos equal-power transition when two banks crossfade. Validate by ear and with level-report output.

- [ ] **Renderer: master per-stem outputs** `[V3/D3]` - The full mix may receive postprocess treatment that individual stems do not. If stem-driven runtime playback returns, make sure per-stem outputs are mastered/audited enough that state-level stem-gain changes are audible and balanced.

- [ ] **Generalize encounter music registration** `[V4/D3]` - `crates/ambition_sandbox/src/music/first_goblin.rs` is intentionally specific today, but future encounters need a reusable structure. Avoid hard-coding every encounter cue by hand.

- [ ] **Clarify audio asset staging vs production** `[V3/D2]` - Make generated-audio scripts and docs explicit about where staging lives, when files become runtime assets, and how to edit/publish a cue without accidentally committing generated scratch output.

- [ ] **Clarify image/sprite generation workflow** `[V3/D3]` - Consolidate generated visual asset scripts and docs so agents know where source prompts/specs live, how to review generated art, how to publish runtime sprites/tiles, and how to avoid ad-hoc one-off script forks.

- [~] **Sprite / tile wiring batch** `[V3/D3]` - Keep a visible list of assets that exist or are planned but are not wired into runtime/LDtk visuals yet. Examples from old TODOs and current issue notes: switch armed/disabled sprites, lock-wall tile, water-surface tile, ladder/vine/climbable tile, acid/lava tile, circuit/background tiles, creator/wagon/lab props, and replacing placeholder fascist/general sprites where the wrong asset is used.

- [ ] **Generated tile sprites for IntGrid layers** `[V3/D3]` - Replace colored placeholder rectangles for climbable/water/hazard/solid layers with real tileset textures while preserving canonical LDtk/bevy_ldtk usage.

## C - Bosses, encounters, and story-arranged slice

- [ ] **Sandbox-side boss controller hook for `BossMovementKind`** `[V4/D3]` - Engine pattern data can describe traversal beats, but the sandbox boss runtime needs to interpret `step.movement` into actual world transforms so bosses dash, orbit, reposition, or retreat instead of only firing attack verbs.

- [ ] **Per-boss pattern schedules in data** `[V3/D3]` - Replace hard-coded `(spec.id, phase) -> schedule` matches with an authored `BossEncounterSpec.schedules` map or equivalent data-driven structure so future bosses do not require code changes for every phase schedule.

- [ ] **GNU-ton boss apple-drop attack** `[V3/D3]` - Add an attack where apples drop from the ceiling and damage the player. Keep it as a reusable boss hazard pattern if possible.

- [ ] **Boss intro sequence audit** `[V3/D2]` - `TODO-drafts.md` flags the boss music / intro sequence as possibly bugged. Reproduce first; do not assume the diagnosis. Check cutscene gating, encounter start, music request, and boss phase state.

- [ ] **Real central hub authoring** `[V3/D4]` - Resume story-arranged slice content once the sandbox bar is met.

- [ ] **Basement / first-zone Gridvania authoring** `[V3/D4]` - Prefer a connected layout with side-scrolling exits and branching routes over door-heavy lab chunks.

- [ ] **Vertical intro lab maze pass** `[V4/D5]` - Explore Jon's current story shape: the intro climbs vertically through a lab maze with side branches, factions fighting each other, the creator trying to escape with the player, and faction/boss pressure at the top. Keep this as design work until mechanics and rooms support it.

- [ ] **Gradient Sentinel boss implementation / replacement decision** `[V3/D4]` - Old TODO tracked this as the first story boss. Re-evaluate against current GNU-ton / hall-of-bosses direction before investing.

- [ ] **Intro cutscene polish** `[V2/D3]` - Old note: include a "Hey you, you're finally awake" beat or equivalent intro hook, but only if it fits the current tone.

## D - Engine, validation, and architecture

- [~] **CharacterAI authoritative migration** `[V3/D3]` - Movement half done as of 2026-05-21: every enemy archetype + every boss runs through the `ActorControlFrame` brain→sim seam and a uniform `step_kinematic` (commits `155171c`, `66c8b0b`). What remains: push per-brain knobs (`chase_speed`, `attack_radius`, `telegraph_seconds`, …) out of the `EnemyArchetype` / `BossBehaviorProfile` match arms into a small data table so adding an enemy stops needing a code change, and decide whether the boss `Cycle` / `Scripted` attack-pattern timer state machine should also migrate into the evaluator output or stay as a layered driver. See `docs/systems/character-ai-refactor.md` for the two-step plan.

- [~] **Player + multi-player unification onto `ActorControlFrame`** `[V4/D4]` - Brain → ActorControl seam landed for the player (2026-05-24): every player entity carries `Brain::Player(slot)` + `ActorControl` + `ActionSet` (commits `c41997b`, `32c37e3`). `tick_player_brains` translates `PlayerInputFrame` into the frame each tick. `PlayerBody` expanded to cover the migration surface (water/climbable contact, wall state, dash timer, blink-aiming — `506b06c`). One reader cluster (audio/environment, ECS actor tick) already migrated off `PlayerMovementAuthority`. **What remains:** `update_player` still drives the body — the frame is built but the integration ignores it. Polarity flip + delete of `ae::Player` is the final step; the per-cluster reader migration map lives in [`dev/journals/ae-player-field-usage-2026-05-24.md`](dev/journals/ae-player-field-usage-2026-05-24.md). See [`TODO-controllable-entity.md`](TODO-controllable-entity.md) for the full multi-chunk plan.

- [~] **Headless simulation for AI playtesting** `[V4/D4]` - `SandboxSim` and trace replay exist. Remaining useful work: PyO3 binding for Python research code, reward shaping examples, and a decision on whether to adopt `bevy_rl` or keep the custom adapter.

- [ ] **Headless screenshot / visual verification path** `[V3/D4]` - Make the headless or test harness capable of rendering screenshots so agents can visually verify rooms, HUD, sprites, and regressions without manual desktop capture.

- [ ] **Unified `ControlFrame` consumer seam** `[V4/D4]` - Move menu navigation, inventory, cutscene dismissal, and other consumers away from raw `ActionState<SandboxAction>` where practical so keyboard, controller, and touch all pass through the same mode-gated abstraction.

- [ ] **Move dev hotkeys into settings/menu surfaces** `[V3/D3]` - Keep hotkeys for developer speed where useful, but every persistent toggle should have a canonical non-hotkey place in settings or dev UI. F-key overlap has caused confusion before.

- [ ] **Promote `KinematicPath` to typed components + index** `[V2/D3]` - Moving platforms have a working path contract, but future path-authored hazards/platforms may need a generic typed runtime index.

- [ ] **Continue Bevy ECS slice extraction** `[V4/D4]` - Shrink procedural orchestration and ad-hoc SystemParam bundles when tests make it safe. Do not do broad architecture churn without a focused validation plan.

- [ ] **Documentation health remains green** `[V3/D1]` - Run doc checks after doc moves, rewrites, or archive changes.

```bash
python scripts/check_doc_links.py
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```

## Proposed / agent drop-zone

Agents may append new ideas here freely. Jon promotes them into the accepted sections above or moves them to `TODO-drafts.md` / brainstorm docs.

- **Puppy slug per-sprite deep-dream Material2d shader doesn't render** `[V2/D3]` (BUG, 2026-05-23) - The `PuppySlugDeepDreamMaterial` overlay at [`crates/ambition_sandbox/src/presentation/rendering/deep_dream.rs`](crates/ambition_sandbox/src/presentation/rendering/deep_dream.rs) attaches correctly per slug (confirmed via `info!(target: "deep_dream", "attaching puppy-slug deep-dream overlay …")` logs firing for every spawn with correct uv_rect, render_size, seed) but never produces visible output on screen. A solid-magenta force-output probe at the top of the WGSL fragment shader **also** doesn't render — so the problem is the Material2d pipeline not committing draws, not the shader logic. The full-screen post-process variant of the same dream effect (`presentation/screen_effects.rs`, gated on `deep_dream_strength`) works fine, so the WGSL itself isn't the problem either. **Ruled out so far:**
  - Wrong sprite (HSV cycle from `sync_puppy_slug_deep_dream_overlays` IS visible — proves the slug renders, just not the Material2d sibling).
  - Shader compile error (no startup compile warnings in `RUST_LOG=info`).
  - Attach not firing (logs confirm 3-4 attach lines per room).
  - Bind-group geometry (uv_rect log shows correct frame UVs).
  - Explicit `InheritedVisibility::default()` / `ViewVisibility::default()` defaults being `Self(false)` and stalling extract (tried removing them — slug still no shader).
  - Z-bias / sort order (bumped to `+0.9` — no difference).
  - Sibling vs child architecture (GPT moved from child to sibling — no difference).

  **Strong leads to try next when debugging in-game:**
  1. The Mesh2d entity might need explicit `Aabb` because Bevy 0.18's `calculate_bounds` only runs for `Mesh3d`. Without an Aabb, `check_visibility` still sets visible by skipping frustum check, but `extract_mesh_materials_2d` extraction or the `queue_material2d_meshes` queue might silently skip the entity. Add `bevy::math::bounding::Aabb::from_min_max(vec3(-0.5, -0.5, -0.5), vec3(0.5, 0.5, 0.5))` (the unit Rectangle bounds) at spawn and re-test.
  2. The Bevy 0.18 examples spawn Material2d via `commands.spawn((Mesh2d(...), MeshMaterial2d(...)))` with **nothing else** and they render. Try stripping the spawn down to the absolute minimum (no `Transform::from_translation` explicit, no `Visibility::Visible` explicit, no `RoomVisual`, no `Name`) and see if it renders — then add components back one at a time. Some `RoomScopedEntity` cleanup hook may be despawning between attach and render.
  3. Check `RUST_LOG=wgpu=warn` or `bevy_render=trace` for silent pipeline-validation errors when Material2d specializes the pipeline.
  4. Confirm the overlay entity makes it into `RenderMaterial2dInstances<PuppySlugDeepDreamMaterial>` by adding a temporary render-app system that logs the resource's len. If the resource is empty, the bug is between main-world extract and material registration. If it's non-empty but no draw call happens, the bug is between queue and execute.

  Until then, the per-slug HSV cycle in `sync_puppy_slug_deep_dream_overlays` provides the user-visible "surreal" look. The full-screen variant remains available via `settings.video.shaders.deep_dream_strength`.
- **PyO3 binding for `SandboxSim`** `[V3/D3]` - Wrap `SandboxSim::{new, step, observation, reset_episode}` plus `AgentAction` / `AgentObservation` as a Python module.
- **N64/OOT-style spinning-cube inventory** `[V2/D5]` - Deferred on purpose. Keep as a design idea, not active implementation, until inventory/menu identity becomes a priority.
- **Hurtbox = debug-drawn invariant** `[V4/D3]` - Witnessed bug: GNU-ton's head hurtbox at `damageable_aabbs()` was never drawn in the debug overlay, so hits "registered in empty space" until 2026-05-20. Manually fixed in `debug_overlay.rs` by adding a `for hurtbox in boss.damageable_aabbs()` pass, but the underlying invariant ("every AABB consulted for damage MUST be drawable in debug") is unenforced. Options: (a) trait `DebugDrawable` that damage-check sites require, (b) a single source-of-truth function returning a Vec of (AABB, role) for each gameplay entity, used by both damage and debug overlay, (c) a snapshot test that ticks each boss/enemy/hazard through a synthetic strike and asserts every emitted damage AABB also appears in the debug overlay's gizmo buffer. (c) is the cheapest. Touch sites: `features/bosses.rs::damageable_aabbs`, `dev/debug_overlay.rs::draw_feature_debug`, mirror for actors/hazards.
- **LDtk spatial-question tooling for LLM authoring** `[V3/D3]` - See `docs/concepts/llm-spatial-authoring-discipline.md`. Needed subcommands: `paths describe --level X` (list reachable exits from spawn + traversal kind), `intgrid query --px X,Y --size W,H` (read-only mirror of `intgrid erase` for "what's here"), `room measure --level X --entity foo` (entity width/height/center + nearest solids), `gates audit --level X` (list every gateable solid/blink/breakable and which encounter/boss/switch gates it). Each one shaves a "where exactly?" round trip during boss/encounter authoring.

- **Wire ledge-momentum-carry boost params into the dev-tools panel** `[V2/D2]` (2026-05-23) — Jon's "I don't feel the boost — I want to tweak it live." Today `MovementTuning::ledge_momentum` (`window`, `x_gain`, `y_gain`, `x_cap`, `y_cap`) is plumbed end-to-end but the dev panel hands the engine `LedgeMomentumTuning::DEFAULT` unconditionally (see comment at `dev/dev_tools.rs::as_engine`). To make these editable: add the 5 fields to `EditableMovementTuning`, add matching slider rows in the dev panel UI (mirror the dodge-roll / dash-cooldown pattern), and swap the constant for `self.ledge_momentum` in `as_engine`. Range hints: window 0.0–0.6 s (0.0 disables), gains 0.0–1.0, caps 0–600 px/s. Touch sites: `crates/ambition_sandbox/src/dev/dev_tools.rs`, the dev-panel UI render code in the same crate.

- **Re-examine `ambition_engine` crate scope** `[V2/D4]` (NOTE, 2026-05-23) — Jon observed: "I do think there is a separable component here, but maybe its current state isn't it. … the engine might not be doing us any favors and maybe we can gradually reduce its scope as needed." Concrete frictions surfaced so far:
   - `ae::Player` is a 50+-field god-struct that forces `PlayerMovementAuthority(ae::Player)` wrapper + `PlayerBody` read model + `write_player_ecs_components` sync system + per-call `authority.player.foo` deref everywhere. A per-concern ECS-component decomposition (position+velocity, abilities, ledge state, body mode, water contact, …) on the player entity would erase that indirection.
   - AGENTS.md already declares "Ambition is Bevy-native; do not resurrect backend-neutral constraints" and the engine re-exports `Aabb = Aabb2d` from `bevy_math`, so the engine-agnostic justification for the boundary is gone.
   - Sandbox depends on the engine, so an engine rebuild forces a sandbox rebuild anyway — the boundary doesn't buy meaningful compile-time isolation in practice.
   - Counter-pressure: the boundary still buys (a) a clean sim/presentation seam, (b) deterministic-sim test cycles, (c) a parking spot for mechanics a future story crate / lab / sibling-game could reuse — there are no such consumers yet, so that promise is unclaimed.
   No action today; capturing the concern so we can let scope shrink opportunistically (e.g. when a feature is naturally cleaner as ECS components, lift it out of the engine instead of growing the engine struct). If the friction continues to compound, promote to an ADR with a concrete migration plan.

- **Promote batch-2 crypto sketches to bespoke templates as needed** `[V2/D3]` - Batch 1 (Bob, Alice, Eve, Mallory, Trent, Judy) + batch 2 (Trudy, Craig, Sybil, Victor, Peggy, Walter, Olivia) all landed as toon-target sketches or bespoke templates. See `docs/concepts/cryptography-crew.md` → "Promotion criteria" for when to upgrade a sketch to a bespoke target. No specific promotion queued; do as story rooms demand.

- [~] **Universal "brain" interface — every controllable entity shares one input shape** `[V4/D5]` (2026-05-24: Chunks 1–4f shipped + stability polish + consistency fixes; EFFECTS-flip remains daytime work) - Design doc: [`docs/planning/universal-brain-interface.md`](docs/planning/universal-brain-interface.md), overview: [`docs/systems/brain-driver.md`](docs/systems/brain-driver.md), recipe: [`docs/recipes/extending-brains-and-action-sets.md`](docs/recipes/extending-brains-and-action-sets.md). Overnight session landed: extended ActorControlFrame for player verbs (Chunk 1); scaffolded `crates/ambition_sandbox/src/brain/` with Brain enum + 7 reusable templates (StandStill/Patrol/Wanderer/MeleeBrute/Skirmisher/Sniper/BossPattern) + ActionSet per-entity capability + player brain (Chunk 2); migrated NpcRuntime onto Brain::StateMachine — peaceful NPCs tick through the brain seam (Chunk 3); attached Brain + ActionSet + ActorControl components to player + every enemy + every boss; per-archetype ActionSet defaults (Swipe / Lunge / Slam / Bite / PunchWeak / Bolt / Rock / Pistol / Arrow + Walk / WalkHeavy / Hop / Strafe / Slither / Float); `tick_player_brains` fills the player frame; `emit_brain_action_messages` resolver writes `ActorActionMessage`s for each concrete `ActionRequest`. Hostile NPC flip swaps brain + ActionSet together. Boss spawn defaults to hostile-capable Bolt + BossSpotlight. Late-session stability polish: ~12 pin tests (dead-actor pre-poison, dead-target paths, hostile-patrol pins, melee-brute timer table, ranged-damage accessor, state.mode tracking, SignumOr fallback). 1141 workspace tests green. **Daytime work that remains:** (a) EFFECTS-stage consumer flip — read `ActorActionMessage` instead of `EnemyRuntime`/`BossRuntime`/`update_player` for combat spawns (overlap-then-delete per the stale-component benchmark, recipe doc has concrete procedure). (b) `ae::Player` decomposition — 38 `authority.player.*` reads still in the sandbox; walk reader clusters per audit. (c) `update_player` consume `ActorControl` instead of `PlayerInputFrame`. (d) Narrow `ActorControlFrame::fire` to `Option<Vec2>` once `RangedActionSpec` owns speed. Track in [`TODO-controllable-entity.md`](TODO-controllable-entity.md). Below is the original design context for posterity:
  - **One conceptual surface**: every entity in the game is controlled by a *brain* that, per tick, fills an `ActorControlFrame` (or successor). The brain is the only thing that changes — the sim integration is identical across all entity kinds.
  - **Brain backends are interchangeable**: state-machine (current NPC patrols, current enemy AI/choreography), scripted (cutscene puppet), RL-agent (policy network), human player (controller / keyboard → control frame), remote player (network-streamed inputs for co-op). The player brain is literally "copy Player 1's controller inputs into the frame." Multiplayer becomes "spawn another `Actor` whose brain reads remote inputs"; RL becomes "spawn an `Actor` whose brain queries an ML policy."
  - **Aggressiveness is the sole gate** for whether an actor initiates attacks. Today `NpcRuntime` vs `EnemyRuntime` is a hardcoded class boundary; post-refactor it's just `aggressiveness > 0` on a uniform `Actor`. The kernel guide's existing patrol path lives in the same data structure as a goblin's. Removes the current stop-gap `EnemyArchetype::attacks_player()` boolean (introduced 2026-05-23 for PirateHeavy + PuppySlug) — both would just declare `aggressiveness = 0`.
  - **Performance caveat**: dynamic-dispatch through `dyn Brain` is the obvious "elegant" path but trait-object virtual calls per actor per tick can add up. Likely mitigations to evaluate: (1) batch brains by backend so the hot loop is monomorphic (`for each enum variant: tick all actors of that variant`); (2) keep the brain backend as an enum rather than a trait object so the dispatch is a switch, not a vtable lookup; (3) for RL agents specifically, run policy inference in parallel across all RL-controlled actors and write all frames at once. Don't pay the trait-object cost just for elegance — the conceptual surface holds even with an enum dispatch.
  - **Touch sites**: `content/features/npcs.rs`, `content/features/enemies.rs`, `content/features/ecs/actors.rs::feature_view`, the entire AI / choreography stack in `ambition_engine`, the LDtk authoring layer (today `NpcSpawn` vs `EnemySpawn` are distinct entity defs; post-refactor they collapse into one `ActorSpawn` with a brain field), the player input pipeline (`PlayerMovementAuthority` becomes "player brain writes `ActorControlFrame`"), and the multiplayer / co-op design notes in memory.

## Closed-work policy

Do not add completed work here just to preserve history. Put durable history in `FEATURES.md`, `docs/history/progression-log.md`, `dev/journals/`, or `docs/archive/`. When closing an item, update the source doc too if it tracks the same state.
