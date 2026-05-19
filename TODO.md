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

## Status legend

- `[ ]` not started
- `[~]` scaffolded / partially shipped but not feature-complete
- `[x]` done recently; keep only briefly when it prevents immediate re-adds
- **`[V?/D?]`** value / difficulty, both 1-5. V: 1=marginal, 5=critical. D: 1=<=30min, 2=1-3hr, 3=half day, 4=multi-day, 5=week+ or risky.

## S - Active blockers and high-signal defects

- [~] **Wall-cling / lock-wall / collision-correction debt** `[V5/D3]` - Keep the trace-backed lock-wall problem central until the root collision-correction behavior is fixed. The current authoritative write-up is `docs/planning/tech-debt-log.md`; use `docs/systems/gameplay-trace-recorder.md` for dump/replay workflow. Do not hide this by widening OOB margins.
  - Good session shape: build or improve a reproduction around the mob-lab lock wall / ceiling geometry, then assert that position correction cannot exceed the frame's velocity budget unless Reset or RoomTransition fired.
  - Validation anchors: `cargo test -p ambition_sandbox --test repro_walls`, `cargo test -p ambition_sandbox trace`, focused movement/collision tests.

- [ ] **Goblin encounter music transition still sounds like a section swap** `[V4/D3]` - The player hears the intro fade out / wave1 enter instead of one continuous musical idea. Existing scratch context was promoted into `docs/recipes/generated-music-workflow.md`; use `debug-notes-music.md` only as historical detail if still present.
  - Diagnose first: compare generated and installed OGGs, run `audit_cue_balance.py`, inspect logs for `gain_start=target`, then decide whether the fix is score arrangement, mastering, stem balance, or runtime fade policy.
  - Related follow-ups live under the audio section below: level report, live gain HUD, equal-power crossfade, and per-stem mastering.

- [ ] **Cutscene/dialogue input and presentation polish** `[V4/D3]` - `issues.md` reports cutscenes that only show text in debug view and say the wrong continue prompt. Route acknowledgement through the same canonical input seam used by keyboard/gamepad/touch, and make the displayed prompt match the active control method.
  - Good session shape: find the current cutscene UI path, make one visible prompt accurate, then add or update a scripted-gameplay test for dialogue acknowledgement.

- [ ] **Menu mouse-hover vs keyboard navigation conflict** `[V3/D2]` - If the mouse is hovering an option while the user navigates by arrow keys / controller, the hover state can fight selection movement. Make menu focus policy explicit: pointer motion may set focus, but stale pointer position should not override a newer keyboard/controller navigation edge.

- [ ] **Touch controls can affect the player during cutscenes/dialogue** `[V4/D2]` - `TODO-drafts.md` flags this as evidence that the control abstraction contract is still leaky. Consumers should see a canonical control frame that has already been mode-gated, not raw touch state that bypasses dialogue/cutscene restrictions.

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

- [ ] **Simple falling-sand sim room** `[V2/D4]` - Prototype a small in-game cellular sim: sand, oil, water, fire from fireballs, switch-controlled spouts, and traceable streams. Keep it isolated until performance and determinism are understood.

### Combat, abilities, and interactions

- [ ] **Bubble shield dodge / roll extension** `[V3/D3]` - Bubble + Down should dodge; Bubble + direction should roll. Define resource cost, invulnerability frames, collision safety, and whether roll is a locomotion state or gameplay effect.

- [ ] **Context-sensitive action buttons / control HUD** `[V3/D3]` - On-screen buttons should communicate current affordances: Interact should name the interaction when available, Projectile may become a general Special button, and desktop mode could show available controls in an OOT-like HUD so players understand their options.

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

- [ ] **CharacterAI authoritative migration** `[V3/D4]` - Convert one enemy archetype's movement to read evaluator output, then one boss pattern, then add parity tests. Current tech debt says `EnemyRuntime` / `BossRuntime` still carry ad-hoc state machines.

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

- **PyO3 binding for `SandboxSim`** `[V3/D3]` - Wrap `SandboxSim::{new, step, observation, reset_episode}` plus `AgentAction` / `AgentObservation` as a Python module.
- **N64/OOT-style spinning-cube inventory** `[V2/D5]` - Deferred on purpose. Keep as a design idea, not active implementation, until inventory/menu identity becomes a priority.

## Closed-work policy

Do not add completed work here just to preserve history. Put durable history in `FEATURES.md`, `docs/history/progression-log.md`, `dev/journals/`, or `docs/archive/`. When closing an item, update the source doc too if it tracks the same state.
