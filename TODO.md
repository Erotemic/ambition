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

## Persistent autonomous-loop instruction

When you wake up here, pick the next task from this list and work on it without asking permission. Honor the long-running discipline (`[[never-stop-during-long-run]]` in memory): never stop until the time limit, even if the headline task feels finished — there's always more on this file. When you do close a task, leave this instruction in place so the next agent finds it.

- [~] Gnutons hitbox around his head does not follow the animation, so either the sprite generator is not emitting the ron data properly, or we don't have it hooked up right. **Diagnosed 2026-06-02 (autonomous):** the generator IS emitting correct per-frame hurtbox data — `gnu_ton_boss/gnu_ton_boss_spritesheet.ron` `animations."rest".hurtbox.frames[*]` carries the head bob (y 144–148) and `gnu_head_descent`/`head_down` carry the 106px descent. So it's wiring, two mechanisms:
  1. **Idle frames don't sample the live animator frame.** `ecs_boss_animation_frame_sample` ([anim_helpers.rs](crates/ambition_sandbox/src/content/features/ecs/anim_helpers.rs#L270)) only returns a `BossAnimationFrameSample` when an active/telegraph **attack profile** matches; for idle/rest it returns `None`, so [actors.rs](crates/ambition_sandbox/src/presentation/rendering/actors.rs#L824) removes the component. Then `damageable_volumes` ([boss_attack_geometry.rs](crates/ambition_sandbox/src/content/features/boss_attack_geometry.rs#L486-L491)) takes the `None`-profile branch → `animation_frame_index(entry, 0.0)` → **frame 0 forever** during idle. Fix needs `BossAnimationFrameSample.profile` to become `Option`, the sampler to emit an idle sample (profile `None`, `animation_key:"rest"`, live `frame_index`), and the `None` branch to consult `ctx.animation_frame`. Small visible effect for GNU-ton (±~9px world) but the principle is wrong and affects the `hit`/recoil rows too.
  2. **(Likely the headline "hit before the visible box") hurtbox vertical alignment vs sprite anchor.** `world_aabb_from_pixel_rect` ([boss_attack_geometry.rs](crates/ambition_sandbox/src/content/features/boss_attack_geometry.rs)) centers the hurtbox so the frame-center maps to `world_center = boss.pos`. But the boss sprite is drawn with `BossSheetSpec::collision_anchor` ([sprites.rs](crates/ambition_sandbox/src/boss_encounter/sprites.rs#L375)) — a non-center, collision-scaled feet/body anchor — so the **rendered frame-center is offset from `boss.pos`** by the anchor amount. The head hurtbox therefore sits vertically off the drawn head. `boss.aabb()` (orange) already adds `combat_offset` (≈(0,-41) for GNU-ton) but `damageable_volumes` does not. Fix: pass `world_center = boss.pos + (the same anchor/​combat offset the sprite uses)`, or fold the anchor delta into the metrics derivation. **Blocked on visual verification** — this is a pixel-alignment fix and there is no headless screenshot path yet (see TODO "Headless screenshot / visual verification path"); a wrong sign here makes it worse, so do not land blind. Recommend building the screenshot harness first, or add a unit test that pins the expected world-center given (render_size, anchor, frame dims).

- [x] The dialog boxes when talking to NPCs have the same mouse hover bug as the menu did. Need to fix that. 

- [x] The player running into an enemy hurts the enemy. (probably due to the unification). The fix should be that a character should have a flag that enables their body hitbox. Generally when a human is controlling a character this should be turned off - maybe in some cases it is turned on if gameplay does make your body hazardous to your foes. But otherwise we just want the classic behavior where running into an enemy hurts you (or at least the option of it, with that as the default). 

- [x] Improve shark without a rider attack patterns so it feels maybe more wild than the pirate who was controlling it. Maybe it charges and if it hits a wall because it charges too far it explodes. 

- [ ] Develop a Boss to flex / force the player / boss unification code (do a new boss with a more humanoid or maybe bipedal character like the trex).

- [ ] Develop an NPC to flex / force the player / boss unification code (do this with Alice).

- [ ] Goblins need to have an action set as rich as the player (flexing or giving a use case to force the player / enemy unification). _Scoped 2026-06-02 (autonomous):_ goblins are the `MediumStriker` archetype (`enemy_archetypes.ron`), which today carries only `melee: Swipe` + `move_style: Walk` and runs the `Smash` brain. The `ActionSet` container can already hold `ranged` + `special`, so the real gap is the **brain's action-selection vocabulary**: `Smash` only ever commits melee, so adding ranged/dash/special to the goblin's set wouldn't be used until the brain learns to choose among verbs by range/situation the way the player can. Concrete next step: extend `Smash` (or add a richer melee+ranged brain template) that fires `ranged` at mid-range and closes for `melee`, then unit-test the range-based choice — that's the unification flex. Pairs with "Goblins need more sprites" (animations to read the new verbs) and is a balance/feel change best validated in-game, so it wants an interactive session rather than a blind autonomous one.

- [ ] Goblins need more sprites for more animations to go with the different actions.

- [ ] Grinning Colossus-like boss (Smirking Behemoth) where you have to "cut the rope". Big square, shoots circle-bubble eye beams. Exercise the pickup items systems.

- [ ] The player need more sprites for more animations to go with the different actions.

- [ ] Portal Gun - classic blue and orange. Carries momentum.

- [ ] Boss phase transitions "screams" / "animations"

- [ ] Implement the TODOS for the kernel NPC dialog tree

- [ ] Silksong levels of input buffering.

- [ ] Ledge grabbing on a moving platform leaves you stationary in air, but it should have you move with the platform or be knocked off if it would push you through a wall through something with heavy collision. 

- [ ] Increase the resolution of gnuton so the sprite is not drawn pixelated. 

- [ ] **Drop-shadow CI assertion follow-up** — generator-side rule [[feedback-no-drop-shadows-on-sprites]] is in memory, but it would help to add a renderer-time assertion that fails if there are opaque pixels below the foot-anchor row on a generated sprite (so a baked shadow trips CI instead of silently shipping).

- [x] **Fix the 32 intro warnings surfaced by the new lint** `[V4/D3]` — _Resolved 2026-06-02 (autonomous)._ The "32" was long stale: validating `intro.ldtk` now surfaces **0 warnings**. Two real warnings were fixed via `ambition_ldtk_tools intgrid paint` (no hand-editing): `pirate_sky_arena` got a full-width Solid ceiling cap on its open top edge (was: walk off the world); `under_town_pipes` got a 48px OneWayUp ledge directly under the mid-air `Door` LoadingZone (target `drain_alley`) so it has a walkable surface. The two remaining `error:` lines about `central_hub_complex` are **false positives from isolated validation** — that room lives in `sandbox.ldtk`, so the canonical command is `validate intro.ldtk --secondary-world sandbox.ldtk` (resolves cross-world LoadingZone targets, exits clean). The paint pass re-serialized `intro.ldtk` to the tool's canonical JSON formatting (one-time whitespace normalization; 17 Rust loader tests still pass).

- [ ] **Avoid doors + teleports in the vertical slice; build a gridvania world.** `[V5/D5]` — Current intro chains 10+ rooms via Door LoadingZones, which makes the slice feel like a hub with a million side-rooms. Convert to EdgeExits (stitched corridors) and connected geometry so the player traverses continuously. The new mid-air-doors lint already flags the doors that need to go.

- [ ] **Better door sprites + variable door sizes** `[V3/D3]` — current door visuals look the same regardless of width/height. Procedurally generate frame variants (small / standard / tall / wide / double) keyed off `LoadingZone.size`, with the door art aligned to the size. Could probably reuse the lasersword DESIGN/OUTPUT split pattern.

- [~] **Smash-Bros style ledge feel** `[V4/D4]` — first chunk shipped 2026-05-22 (ffbaaf1): `LedgeGetupKind::{Climb, Roll}`, shield-held triggers a forward roll with full invulnerability via `dodge_roll_timer`, brief intangibility window on grab, tightened `LEDGE_MIN_CLIMB_DELAY` for snap feel. Follow-ups: ledge getup-attack (attack from hang), Smash-style fall-through-then-up-B reclaim, document the ledge contract as an ADR so the port-to-Smash goal stays explicit, optional second-roll cooldown so spammers can't permastall.

- [ ] **Wall-clipping bugs in the intro** `[V4/D4]` — Jon's noted ongoing bad-clipping-through-walls errors during intro playthroughs. Probably a mix of (a) sub-pixel collision drift at high speeds, (b) thin-wall (16-px) corner cases, (c) the trace-replay-driven lock-wall debt already in the active blockers list. Cross-link with the existing wall-cling teleport entry in `docs/planning/tech-debt-log.md`.

- [ ] **Contextual button label follow-ups** — `player::affordances` module + touch HUD overlay shipped 2026-05-23. Outstanding: (a) thread the player's selected `KeyboardPreset` through the glyph adapter (today reads the default Arrows+ZXC); (b) wire gamepad-kind detection once Bevy 0.18's gamepad API is verified; (c) add a regression test once gameplay subsystems start consuming the same resolvers so HUD/sim can't drift. See `docs/systems/control-affordances.md`.


- [ ] **Renderer-time "no shadow below foot" assertion** `[V2/D1]` — pair with the no-drop-shadow rule. After auto-cropping a generated sprite, assert that the bottom row of the alpha bbox has at least some opaque pixels (true foot present), AND that there are no opaque pixels below the body in any frame (no soft baked shadow). Fails the renderer at CI time before any artist re-introduces a shadow.


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

- [~] Enemy hurt boxes still don't visualize correctly. I can throw a fireball at gnuton and he will get hit before the fire ball collides with the visible box, so something is up with collision or the box isn't drawing.

- [~] **Wall-cling / lock-wall / collision-correction debt** `[V5/D3]` - Keep the trace-backed lock-wall problem central until the root collision-correction behavior is fixed. The current authoritative write-up is `docs/planning/tech-debt-log.md`; use `docs/systems/gameplay-trace-recorder.md` for dump/replay workflow. Do not hide this by widening OOB margins.
  - Good session shape: build or improve a reproduction around the goblin-encounter lock wall / ceiling geometry, then assert that position correction cannot exceed the frame's velocity budget unless Reset or RoomTransition fired.
  - Validation anchors: `cargo test -p ambition_sandbox --test repro_walls`, `cargo test -p ambition_sandbox trace`, focused movement/collision tests.
  - _Progress 2026-06-02 (autonomous):_ the velocity-budget assertion is now landed in `tests/repro_walls.rs` (`assert_within_displacement_budget` = `vel*dt + 16px depenetration margin`), replacing the four per-test `dy < 50.0` magic thresholds and adding a 2400-pose sweep (`wall_cling_displacement_budget_holds_across_pose_sweep`). The regression net is much tighter; the **root collision-correction behaviour is still unfixed** — keep this open.

- [ ] **Goblin encounter music transition still sounds like a section swap** `[V4/D3]` - The player hears the intro fade out / wave1 enter instead of one continuous musical idea. Existing scratch context was promoted into `docs/recipes/generated-music-workflow.md`; use `debug-notes-music.md` only as historical detail if still present.
  - Diagnose first: compare generated and installed OGGs, run `audit_cue_balance.py`, inspect logs for `gain_start=target`, then decide whether the fix is score arrangement, mastering, stem balance, or runtime fade policy.
  - Related follow-ups live under the audio section below: level report, live gain HUD, equal-power crossfade, and per-stem mastering.

- [x] **Cutscene/dialogue input and presentation polish** `[V4/D3]` - `issues.md` reports cutscenes that only show text in debug view and say the wrong continue prompt. Route acknowledgement through the same canonical input seam used by keyboard/gamepad/touch, and make the displayed prompt match the active control method.
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

- [x] **Sprite frame dimensions read from YAML at runtime, not hardcoded** `[V3/D3]` — _Verified done 2026-06-02 (autonomous):_ `sheets.rs` no longer hardcodes frame geometry. All 44 sheet statics are `LazyLock::new(|| load_spec(target, &TUNING))`, and `spec_from_record` pulls `frame_width` / `frame_height` / `label_width` / `rows` / `feet_anchor_y` from the on-disk `*_spritesheet.ron` manifest `record`. The only remaining per-sheet consts are `SheetTuning` (`collision_scale`, `frame_sample_inset`) + `y_offset` — deliberately in code because they're gameplay-*usage* decisions, not facts about how the sprite was drawn (see the `CharacterSheetSpec` doc comment). The old drift-guard test was retired when the consts were dropped. Original stale note follows for history: ~25 `CharacterSheetSpec` consts with `frame_width` / `frame_height` / `label_width` hardcoded. Every regen of a character generator can change those numbers (the `pirates/common::build_sheet` auto_crop fits each frame to the union alpha bbox + 2px margin). On 2026-05-22 the pirates and shark drifted hard — game extracted 128×128 windows while the actual frames were 103×114 / 162×76, so the URect picked up the wrong region of the PNG (visible as misaligned bounding boxes around characters). Quick fix landed (53b4edf or follow-up commit) that resynced 17 sheets to current YAMLs and added a `sheet_consts_match_their_yaml_manifests` test so future drift fails the build. Long-term: drop the consts, load the manifest at startup (or build.rs codegen from the YAML), so the source of truth is the renderer output.

- [~] **Simple falling-sand sim room** `[V2/D4]` - Prototype landed in `crates/ambition_sandbox/src/falling_sand.rs` (feature `falling_sand`, on by default for `desktop_dev`). Spouts emit sand / water / oil / mixed, LDtk room walls are mirrored as `bevy_falling_sand` wall particles, dense tiles are projected as one-way `ae::Block`s so the player collides with piles, and a 1Hz `fs-diag` log streams counts + Y range + per-component coverage. Root cause of "particles never moved" was a missing `ChunkLoader` entity — v0.7.0's `update_chunk_loading` returns early when none exists, so no chunk entities ever get spawned and the movement-by-chunks query returns nothing (fixed bc1d5d5). Pile-up still reads "a bit weird" — leaving in-progress for polish. **Follow-ups (in rough priority):** (a) the `fs-diag` `band_grid_y` calculation picks the topmost LDtk block via `min(block.min.y)` and labels it the floor — should pick the largest `block.min.y` (visible floor) instead; (b) some material seems to pile near the room ceiling — investigate whether the spout/ceiling wall geometry leaves stranded pockets, possibly skip seeding walls on overhead blocks; (c) the projected one-way `Block` flicker remains (sort-by-count fix helped but particles flowing through dense tiles still oscillate the cap admission); (d) consider switching emit to `try_multiple` over a tall column so spouts choke gracefully on backed-up streams; (e) eventually port to v0.8 once it stabilizes (`Particle` becomes a marker, API simpler). Also: fire-from-fireballs and traceable-stream interactions aren't wired yet — that was the original scope.

### Combat, abilities, and interactions

- [ ] **Bubble shield dodge / roll extension** `[V3/D3]` - Bubble + Down should dodge; Bubble + direction should roll. Define resource cost, invulnerability frames, collision safety, and whether roll is a locomotion state or gameplay effect.


- [~] **More enemy varieties across size and aggression bands** `[V4/D4]` - `EnemyArchetype` already covers several combat shapes. Finish the missing low/medium/high combinations only if they create distinct encounters, not just more enum values. Validate HP, aggro radius, damage, and LDtk brain IDs.

- [ ] **Actor aggression / NPC-enemy distinction polish** `[V4/D3]` - `issues.md` states the architecture should distinguish enemy vs NPC mostly by aggressiveness, not by separate conceptual species. Check current `Actor`, faction, `NpcRuntime`, hostility conversion, and `EnemyRuntime` paths; reduce special cases where possible.

### Test rooms and progression laboratories

- [~] **Save-point lab + persisted-switch test room** `[V4/D3]` - `switch_lab` demonstrates switch persistence, but a dedicated save-point entity and save/lab flow are still useful. Add a save-point interactable distinct from regular switches, a reset-switches sub-room, and a broader test-state schema for boss defeated / mob room cleared / save restoration.

- [ ] **Additional body-mode traversal rooms** `[V3/D3]` - Current body-mode vocabulary is available, but more authored rooms should prove crouch, crawl, slide, morph, stand-up rejection, and compact traversal under real LDtk collision.

- [ ] **Tutorial refresher / quest reminder rule** `[V3/D3]` - Design rule: the game should never permanently strand a player without a way to review tutorial controls, current quests, or what they were doing. Add a durable UI/UX plan and a small sandbox proof.

- [x] **Alice/Bob/Eve/Mallory/etc. NPC cast** `[V2/D3]` - Potential cryptography-themed NPC set: Bob the architect, Alice the cryptographer, Eve listener, Mallory malicious attacker, Trudy intruder, Craig cracker, Sybil identity attacker, Trent arbitrator, Victor/Peggy verifier/prover, Walter warden, Olivia oracle, Judy judge. Add these NPCs to various rooms in the sandbox.

## B - Audio, generated assets, and authoring tools

- [ ] **Boss music binding extension** `[V4/D3]` - Extend adaptive music cue bindings beyond mob encounters so boss phases such as Intro / Phase1 / Transition / Phase2 / Stagger / Enrage can resolve to cue states. Mechanism exists in pieces; authored audio identity is the gating cost.

- [ ] **Music level / mixing debug tools** `[V3/D3]` - Add a diff-friendly level report for `assets/audio/music/generated/<cue>/`: integrated LUFS if available, true peak, RMS, duration, and target-loudness delta. Add an optional in-engine dev HUD showing live per-bank / per-slot gains and post-master dB so transition dips are visible while playing.

- [ ] **Equal-power crossfade in runtime gain smoothing** `[V2/D2]` - The current per-slot smoothing can create a midpoint power dip. Investigate a phase-driven sin/cos equal-power transition when two banks crossfade. Validate by ear and with level-report output.

- [ ] **Renderer: master per-stem outputs** `[V3/D3]` - The full mix may receive postprocess treatment that individual stems do not. If stem-driven runtime playback returns, make sure per-stem outputs are mastered/audited enough that state-level stem-gain changes are audible and balanced.

- [ ] **Generalize encounter music registration** `[V4/D3]` - `crates/ambition_sandbox/src/music/first_goblin.rs` is intentionally specific today, but future encounters need a reusable structure. Avoid hard-coding every encounter cue by hand.

- [ ] **Clarify audio asset staging vs production** `[V3/D2]` - Make generated-audio scripts and docs explicit about where staging lives, when files become runtime assets, and how to edit/publish a cue without accidentally committing generated scratch output.

- [ ] **Clarify image/sprite generation workflow** `[V3/D3]` - Consolidate generated visual asset scripts and docs so agents know where source prompts/specs live, how to review generated art, how to publish runtime sprites/tiles, and how to avoid ad-hoc one-off script forks.

- [~] **Sprite / tile wiring batch** `[V3/D3]` - Keep a visible list of assets that exist or are planned but are not wired into runtime/LDtk visuals yet. Examples from old TODOs and current issue notes: switch armed/disabled sprites, lock-wall tile, water-surface tile, ladder/vine/climbable tile, acid/lava tile, circuit/background tiles, creator/wagon/lab props, and replacing placeholder raid-enforcer/general sprites where the wrong asset is used.

- [ ] **Generated tile sprites for IntGrid layers** `[V3/D3]` - Replace colored placeholder rectangles for climbable/water/hazard/solid layers with real tileset textures while preserving canonical LDtk/bevy_ldtk usage.

## C - Bosses, encounters, and story-arranged slice

- [ ] **Sandbox-side boss controller hook for `BossMovementKind`** `[V4/D3]` - Engine pattern data can describe traversal beats, but the sandbox boss runtime needs to interpret `step.movement` into actual world transforms so bosses dash, orbit, reposition, or retreat instead of only firing attack verbs.

- [ ] **Per-boss pattern schedules in data** `[V3/D3]` - Replace hard-coded `(spec.id, phase) -> schedule` matches with an authored `BossEncounterSpec.schedules` map or equivalent data-driven structure so future bosses do not require code changes for every phase schedule.

- [ ] **GNU-ton boss apple-drop attack** `[V3/D3]` - Add an attack where apples drop from the ceiling and damage the player. Keep it as a reusable boss hazard pattern if possible.

- [ ] **Boss intro sequence audit** `[V3/D2]` - `TODO-drafts.md` flags the boss music / intro sequence as possibly bugged. Reproduce first; do not assume the diagnosis. Check cutscene gating, encounter start, music request, and boss phase state.

- [ ] **Real central hub authoring** `[V3/D4]` - Resume story-arranged slice content once the sandbox bar is met.

- [ ] **Basement / first-zone Gridvania authoring** `[V3/D4]` - Prefer a connected layout with side-scrolling exits and branching routes over door-heavy lab chunks.

- [ ] **Vertical intro lab maze pass** `[V4/D5]` - Explore Jon's current story shape: the intro climbs vertically through a lab maze with side branches, factions fighting each other, the creator trying to escape with the player, and faction/boss pressure at the top. Keep this as design work until mechanics and rooms support it.

<!-- Gradient Sentinel item closed 2026-05-25 — see FEATURES.md "Gradient Sentinel boss fight". -->



- [ ] **Intro cutscene polish** `[V2/D3]` - Old note: include a "Hey you, you're finally awake" beat or equivalent intro hook, but only if it fits the current tone.

## D - Engine, validation, and architecture

- [~] **CharacterAI authoritative migration** `[V3/D3]` - Movement half done as of 2026-05-21: every enemy archetype + every boss runs through the `ActorControlFrame` brain→sim seam and a uniform `step_kinematic` (commits `155171c`, `66c8b0b`). What remains: push per-brain knobs (`chase_speed`, `attack_radius`, `telegraph_seconds`, …) out of the `EnemyArchetype` / `BossBehaviorProfile` match arms into a small data table so adding an enemy stops needing a code change, and decide whether the boss `Cycle` / `Scripted` attack-pattern timer state machine should also migrate into the evaluator output or stay as a layered driver. See `docs/systems/character-ai-refactor.md` for the two-step plan.

- [~] **Player + multi-player unification onto `ActorControlFrame`** `[V4/D4]` - Brain → ActorControl seam landed for the player (2026-05-24): every player entity carries `Brain::Player(slot)` + `ActorControl` + `ActionSet`. **2026-05-28 push (phase 3d.1–3d.4):** every writable `&mut ae::Player` path in sandbox-side runtime has been refactored to `&mut PlayerClustersMut<'_>`. Touch sites converted: `player_control_phase`, `player_simulation_phase` (both now call `update_player_{control,simulation}_with_clusters` directly), `reset_sandbox`, `handle_player_events`, `is_riding`, `death_respawn_player`, `safe_respawn_player`, `apply_player_knockback`, `handle_player_damage_events`, `load_room`, `reload_ldtk_world_from_disk`, `start_attack`, `advance_attack`, `apply_player_body_profile`, `apply_movement_profile`, settings menu plumbing, `transition_for_player`, `remember_safe_player_position` (with new `classify_safety_from_kinematics` helper). `to_player()` / `write_from_player()` no longer called from sandbox-side runtime — only inside `engine_core/movement.rs` for its internal scratchpads. **What remains:** (a) three internal scratchpads in `update_player_{control,simulation}_with_clusters` (tick_active_ledge_grab, integrate_velocity, try_start_ledge_grab still take `&mut Player`); (b) read-only `&ae::Player` snapshot consumers in `dev/trace/*`, `dev/debug_overlay.rs`, conversion-tests, and runtime/setup constructors; (c) the actual `ae::Player` struct + legacy `update_player_*_with_tuning` entry points (deletable once a/b are done).

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
- **LDtk spatial-question tooling for LLM authoring** `[V3/D3]` - See `docs/concepts/llm-spatial-authoring-discipline.md`. Needed subcommands: `paths describe --level X` (list reachable exits from spawn + traversal kind), `intgrid query --px X,Y --size W,H` (read-only mirror of `intgrid erase` for "what's here"), `room measure --level X --entity foo` (entity width/height/center + nearest solids), `gates audit --level X` (list every gateable solid/blink/breakable and which encounter/boss/switch gates it). Each one shaves a "where exactly?" round trip during boss/encounter authoring.

- **Promote batch-2 crypto sketches to bespoke templates as needed** `[V2/D3]` - Batch 1 (Bob, Alice, Eve, Mallory, Trent, Judy) + batch 2 (Trudy, Craig, Sybil, Victor, Peggy, Walter, Olivia) all landed as toon-target sketches or bespoke templates. See `docs/concepts/cryptography-crew.md` → "Promotion criteria" for when to upgrade a sketch to a bespoke target. No specific promotion queued; do as story rooms demand.

- [~] **Universal "brain" interface — every controllable entity shares one input shape** `[V4/D5]` (2026-05-24: Chunks 1–4f shipped + stability polish + consistency fixes; EFFECTS-flip remains daytime work) - Design doc: [`docs/planning/universal-brain-interface.md`](docs/planning/universal-brain-interface.md), overview: [`docs/systems/brain-driver.md`](docs/systems/brain-driver.md), recipe: [`docs/recipes/extending-brains-and-action-sets.md`](docs/recipes/extending-brains-and-action-sets.md). Overnight session landed: extended ActorControlFrame for player verbs (Chunk 1); scaffolded `crates/ambition_sandbox/src/brain/` with Brain enum + 7 reusable templates (StandStill/Patrol/Wanderer/MeleeBrute/Skirmisher/Sniper/BossPattern) + ActionSet per-entity capability + player brain (Chunk 2); migrated NpcRuntime onto Brain::StateMachine — peaceful NPCs tick through the brain seam (Chunk 3); attached Brain + ActionSet + ActorControl components to player + every enemy + every boss; per-archetype ActionSet defaults (Swipe / Lunge / Slam / Bite / PunchWeak / Bolt / Rock / Pistol / Arrow + Walk / WalkHeavy / Hop / Strafe / Slither / Float); `tick_player_brains` fills the player frame; `emit_brain_action_messages` resolver writes `ActorActionMessage`s for each concrete `ActionRequest`. Hostile NPC flip swaps brain + ActionSet together. Boss spawn defaults to hostile-capable Bolt + BossSpotlight. Late-session stability polish: ~12 pin tests (dead-actor pre-poison, dead-target paths, hostile-patrol pins, melee-brute timer table, ranged-damage accessor, state.mode tracking, SignumOr fallback). 1141 workspace tests green. **Daytime work that remains:** (a) EFFECTS-stage consumer flip — read `ActorActionMessage` instead of `EnemyRuntime`/`BossRuntime`/`update_player` for combat spawns (overlap-then-delete per the stale-component benchmark, recipe doc has concrete procedure). (b) `ae::Player` decomposition — _largely done 2026-05-28_: every writable `&mut Player` path in sandbox is cluster-native; remaining `&ae::Player` reads are confined to debug/trace inspection and a few read-only fixtures. (c) `update_player` consume `ActorControl` instead of `PlayerInputFrame`. (d) Narrow `ActorControlFrame::fire` to `Option<Vec2>` once `RangedActionSpec` owns speed. Track in [`TODO-controllable-entity.md`](TODO-controllable-entity.md). Below is the original design context for posterity:
  - **One conceptual surface**: every entity in the game is controlled by a *brain* that, per tick, fills an `ActorControlFrame` (or successor). The brain is the only thing that changes — the sim integration is identical across all entity kinds.
  - **Brain backends are interchangeable**: state-machine (current NPC patrols, current enemy AI/choreography), scripted (cutscene puppet), RL-agent (policy network), human player (controller / keyboard → control frame), remote player (network-streamed inputs for co-op). The player brain is literally "copy Player 1's controller inputs into the frame." Multiplayer becomes "spawn another `Actor` whose brain reads remote inputs"; RL becomes "spawn an `Actor` whose brain queries an ML policy."
  - **Aggressiveness is the sole gate** for whether an actor initiates attacks. Today `NpcRuntime` vs `EnemyRuntime` is a hardcoded class boundary; post-refactor it's just `aggressiveness > 0` on a uniform `Actor`. The kernel guide's existing patrol path lives in the same data structure as a goblin's. Removes the current stop-gap `EnemyArchetype::attacks_player()` boolean (introduced 2026-05-23 for PirateHeavy + PuppySlug) — both would just declare `aggressiveness = 0`.
  - **Performance caveat**: dynamic-dispatch through `dyn Brain` is the obvious "elegant" path but trait-object virtual calls per actor per tick can add up. Likely mitigations to evaluate: (1) batch brains by backend so the hot loop is monomorphic (`for each enum variant: tick all actors of that variant`); (2) keep the brain backend as an enum rather than a trait object so the dispatch is a switch, not a vtable lookup; (3) for RL agents specifically, run policy inference in parallel across all RL-controlled actors and write all frames at once. Don't pay the trait-object cost just for elegance — the conceptual surface holds even with an enum dispatch.
  - **Touch sites**: `content/features/npcs.rs`, `content/features/enemies.rs`, `content/features/ecs/actors.rs::feature_view`, the entire AI / choreography stack in `ambition_engine`, the LDtk authoring layer (today `NpcSpawn` vs `EnemySpawn` are distinct entity defs; post-refactor they collapse into one `ActorSpawn` with a brain field), the player input pipeline (`PlayerMovementAuthority` becomes "player brain writes `ActorControlFrame`"), and the multiplayer / co-op design notes in memory.

## Closed-work policy

Do not add completed work here just to preserve history. Put durable history in `FEATURES.md`, `docs/history/progression-log.md`, `dev/journals/`, or `docs/archive/`. When closing an item, update the source doc too if it tracks the same state.
