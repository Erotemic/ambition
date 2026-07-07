# HANDOFF — autonomous execution run (2026-07-04)

**You are an executing agent. Finish EVERY item in "THE WORK" below, in green committed
slices, without stopping to ask.** These are all 100%-decided, no-fork, autonomous. When
ALL of them are done, STOP and report — Jon re-consults fable only after the whole list is
finished. Do NOT touch the two fable-reserved items in "OUT OF SCOPE."

The canonical state doc is `docs/reviews/fable-review-2026-07-02.md` (the "fable review").
Read its `## JON'S DESIGN DIRECTION (2026-07-04)` section (JD1–JD4), the E-log tail
(E56–E58), and the `## Synthesis` + `## A/B/C/D` audit sections. **The doc lags the code —
verify every claim against a fresh code read before implementing.**

---

## WORKING STYLE (non-negotiable — Jon's standing directives)

- **Land the architecture headless. Never pause to checkpoint, ask permission, or defer a
  slice because it is "feel-sensitive / large / entangled."** Feel-sensitive, player-touching,
  tactical-AI, or presentation-unverifiable changes SHIP — mark them `blind` / `BLIND` in the
  commit subject; Jon feel-checks after. Finishing an item is NOT a stopping point: the line
  after "committed" is you STARTING the next item. Only a GENUINE unworkaroundable blocker, or
  discovering a real design FORK not listed here, justifies pausing — and even then: note it in
  the doc for fable and KEEP GOING on everything else. Never ask "should I continue?".
- **Green committed slices.** Each slice compiles + its tests pass, then commit (a checkpoint)
  and move on. Prefer many small commits over one giant one.
- **Convergence = LESS code, one path.** A byte-identical refactor gets a byte-identical test
  (the strongest canary — see E58's `strike_geometry_is_byte_identical_to_the_old_hardcoded_match`).
  A behavior-shifting AI/physics change gets a headless / symmetry-under-gravity test; when a
  cadence canary MOVES, loosen it to its SPIRIT (as E54/E56 did — e.g. "the actor integrated
  through the shared phase" instead of a specific direction), never delete the invariant.
- **Keep the fable-review doc live.** Add an E-log entry (E59+) per landed slice, and update
  that item's status bullet in the `## Next` / decisions area. This is what fable reads next.

### Repo mechanics
- `cargo` is at `~/.cargo/bin` (`export PATH="$HOME/.cargo/bin:$PATH"`). Full builds are ~10 min
  — batch checks; prefer `cargo check -p <crate> --all-targets` then targeted `cargo test -p`.
- Commit **directly to `main`** (solo dev, no feature branches). **NEVER `git add -A`** — stage
  explicit paths only (the working tree carries dev junk + untracked tool dirs).
- End EVERY commit message with:
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`
- **Known pre-existing RED:** `unified_melee::a_hostile_actor_enters_the_same_body_melee_lifecycle`
  (`--features rl_sim`) fails at HEAD — a moveset-fold cadence gap, NOT yours. Don't chase it.
  Some `ambition_app` integration tests need `--features rl_sim` (it's in the default `desktop_dev`
  feature set, so a plain `cargo test -p ambition_app` runs them).
- Start by confirming a clean working tree (`git status`); the last commits are docs (`aa649206`).

---

## THE WORK (all decided; do a sensible order, each in green slices)

Suggested order: C4 → B frame-bugs → A3/A4 damage → A1 boss fold → C1 items → C6 sheet-specs →
C7. But use judgment; none blocks another except where noted. If an item turns out to hide a
design fork, note it in the doc for fable and SKIP it — do not stop the run.

### 1. C4 — app-thinness: fold `sim_systems.rs` into owning library plugins
`crates/ambition_app/src/app/sim_systems.rs` (7 systems, ~579 LOC) holds real gameplay-sim logic
in the app binary. Move the LOGIC down to its owning `ambition_actors` module; the app's
schedule registration (in `app/plugins.rs` — `register_player_input_systems` ~L245-342 and the
`cleanup_timers_system` at ~L456) keeps owning the ordering but references the moved `pub fn`.
- **Movable to `gameplay_core` (render-free, app-only-free):** `sync_live_player_dev_edits_system`
  → `gameplay_core::dev`; `apply_suspended_time_scale_system` → `gameplay_core::time::time_control`;
  `input_timer_system`, `interaction_input_system`, `cleanup_timers_system` → `gameplay_core::player`.
  In each moved fn, rewrite `ambition_actors::` paths to `crate::`; `ambition_input::` /
  `ambition_sfx::` stay (external to gameplay_core). Verify each still compiles — `gameplay_core`
  has NO `ambition_render` dep (checked), so a fn using `ambition_render::fx::VfxMessage` CANNOT move.
- **Blocked-until-`reset_sandbox`-moves:** `apply_player_reset_input_system` +
  `apply_cut_rope_room_replay_request_system` use `super::world_flow::reset_sandbox` (app-only) AND
  `VfxMessage` (render). `reset_sandbox` (`app/world_flow.rs`, ~76 LOC) would have to move down first,
  and it may pull app-only types + it uses render — likely it CAN'T cleanly move to gameplay_core.
  If so, LEAVE these two in the app (they are genuine host/reset concerns) and note why. Don't force it.
- The `apply_cut_rope_room_replay_request_system` is NAMED CONTENT (cut-rope boss) — it belongs in
  `ambition_content::bosses`, not the app. Moving it is a content-out-of-core win IF the schedule
  hook can be expressed content-side; if that needs the rooms world-hook seam (JD4, fable-reserved),
  leave it and note it. Don't build the rooms seam here.
- Extend `crates/ambition_app/tests/architecture_boundaries.rs` (mirror
  `architecture_boundaries_touch_input_crate_is_extracted`) to pin that the moved systems are no
  longer DEFINED in the app.
- The `PlatformerEnginePlugin` group (collect the ~30 engine plugins in `plugins.rs`) is a nice-to-have
  but sprawly — do it only if it stays mechanical; otherwise note it and move on.

### 2. B — physics/gravity frame-bug residue (ship BLIND; C4-symmetry testable)
Fix the reaction-seam frame bugs the audit found (fable review `## B`). Each is a verb correct in
its main path with a screen-frame epilogue. Build/extend a `update_body_with_tuning_clusters`-level
symmetry harness (like the `step_kinematic` rig) so a scenario that fails only on rotated gravity
trips it. All ship `blind` (Jon feel-checks). Targets (verify line numbers against code):
- **B1** — moveset hitboxes spawn in the SCREEN frame: `combat/moveset.rs:~138,143` build the volume
  offset unrotated. Rotate the authored offset through `AccelerationFrame::to_world` at spawn (the
  seam `spawn_melee_strike` already uses). This is BOTH the bug fix AND unification with the melee path.
- **B3** — post-blink velocity damp/clamp on world X/Y: `ambition_engine_core/src/movement/blink.rs`
  `complete_blink_clusters`. `to_local` via `AccelerationFrame::new(tuning.gravity_dir)`, damp `.x`,
  clamp `.y`, `to_world` back.
- **B4** — slash recoil along world X: `ambition_engine_core/src/movement/control.rs:~130`
  `vel.x -= facing * slash_recoil` → `vel -= frame.side * (facing * slash_recoil)`.
- **B5** — spurious-graze guards welded to world axes: `ambition_engine_core/src/movement/collision.rs`
  (`body_is_side_contact` etc.). Phrase both guards in axis-ROLE terms so they rotate with gravity.
- **B6** — wall-ability ordering differs between the two gravity-axis branches:
  `ambition_engine_core/src/movement/integration.rs:~176-217`. Make the ordering identical.
- **B2** — `ActorSurfaceState::surface_normal` is a stale frame source for non-surface-walkers
  (consumers should use `gravity.dir_at(kin.pos)` unless `surface_walker && on_ground`). This overlaps
  the A3/A4 damage work (the shield/knockback/muzzle consumers) — do it there.

### 3. A3 + A4 — victim-side damage unification (relational, gravity-correct)
Route the remaining forked damage paths through ONE relational victim resolver (fable review §A3/§A4;
§A2 + A5/A6 already landed — verify with the E-log). The three-loops-in-one-system in
`combat/hitbox/mod.rs` (~L57-337) and the player-SCOPED world emitters (`combat/hazards.rs` is
`With<PlayerEntity>` only; `apply_actor_contact_damage`; the boss tick's damage) should iterate "every
vulnerable body whose faction the source can damage," using the shared `body_vulnerable()` + one
gravity-oriented hurtbox accessor (fixes B2/B5's player-vs-actor hurtbox divergence at the same time).
This unlocks emergent play (an NPC in lava, a boss lured into a hazard). Behavior-shifting → headless
tests + ship the feel-sensitive parts BLIND.

### 4. A1 — boss driver fold (JD3: FINISH IT; shape settled Path B)
Dissolve the remaining boss island so a boss is just an actor archetype. Movement/damage/brain-tick/
specials/geometry/attack-state-projection already unified (E15/E47–E53). Remaining:
- Retire `BossStatus` / `BossAttackState` as AUTHORITIES; fold `update_ecs_bosses` + `tick_boss_brains`
  into the actor systems (`tick_actor_brains` / `integrate_sim_bodies`), boss as a capability-mask +
  `BossPattern` brain (via the `Brain::StateMachine` seam) + phase-state component.
- **The nuance (E53):** retiring the `BossAttackState` brain-WRITE is NOT a dead-write removal — the
  move trigger READS it as its intent signal, so it needs an intent-component split (a small intent
  component the pattern writes and the trigger reads), THEN the projection can be the sole state
  authority. Do that split first.
- Boss possession's bespoke input→special mapping dies with the fold (it becomes the shared path).
- `BossAnim` → `CharacterAnim` render rows is the last slice and is BLIND (presentation-unverifiable) —
  it also retires the `animate_bosses` render→sim write-back (move the animator sim-side). Ship BLIND.
- Keep the boss suites green each slice: `boss_lifecycle`, `boss_contact_iframes`, `boss_motion_parity`,
  `boss_possession_specials` (all in `ambition_app/tests/`).

### 5. C1 — item catalog (JD2: BUILD IT as prep)
Convert the 24-item `Item` enum (with baked flavor text in machinery) → an installable `ItemCatalog`,
following the PROVEN roster-install pattern (enemies/bosses/characters/specials — e.g.
`install_boss_profiles` / `BossProfileRegistry`, and the `ambition_content` `AmbitionContentPlugin`).
The `Item` enum's flavor text → content data ("content out of core"). Incremental is fine — C1 (the
Item catalog) first; the held-item registry (C2 — `HELD_ITEMS` static in a foundation crate) and the
projectile-spec chain (C5 — retire `ProjectileKind`) are the natural follow-ons if time allows. This is
prep for a future second game; there IS no second-game consumer yet and that is fine (Jon's explicit call).

### 6. C6 — boss sheet-specs → RON (content out of core)
`crates/ambition_actors/src/boss_encounter/sprites/mod.rs` holds hardcoded `pub const`
`BossSheetSpec`s (`MOCKINGBIRD_SHEET`, `GNU_TON_SHEET`, …) with `rows: &'static [(BossAnim, AnimRow)]`.
Make them RON-authorable so a content boss authors its sheet layout as data (same "out of core" as the
E58 `StrikeRect` authored-override I just landed — use that as the pattern). This needs `&'static` →
owned (`Vec<...>`) type surgery + `serde` on `BossAnim`/`AnimRow`/`BossSheetSpec` + a loaded registry
(mirror `BossProfileRegistry` / `boss_profiles.ron`) + updating the ~7 consumers + `dedicated_boss_sheets()`.
Keep the built-in bosses byte-identical (a fixture-vs-const test). **The 11-variant `BossAttackProfile`
enum collapse is a SEPARATE, larger item** — the variants also key anim rows / overlays / behavior (72
refs across 8 files), so collapsing the enum needs those keys to become authored too. Attempt it ONLY
if you can keep the named variants as thin constructors and preserve the anim/pose keying cleanly; if it
needs a design call on how content authors anim keys, note it for fable and skip.

### 7. C7 — rider-name half (build the missing LDtk-tools subcommand)
Mount composition currently parses `" on Shark"` out of the spawn NAME string. Build a `mount:` spawn
field in `ambition_ldtk_tools` (a new subcommand / field per the "always use ambition_ldtk_tools, never
hand-edit .ldtk" rule), then re-author the mounted spawns to use it and drop the name-parsing. Roundtrip-
check the .ldtk after.

---

## OUT OF SCOPE — wait for the fable round, do NOT touch

- **JD1 — abilities / params spec (the player-melee fold).** The DIRECTION is decided (parameterized-
  prefab effects as DATA + arbitrary content code via `Effect{key}` Techniques, params from the published
  character data, input→move mappings in that data). But fable is specing the params VALUE TYPE (opaque
  serde value vs Bevy `Reflect` vs HashMap), the dispatch shape (message vs component/observer), the
  item↔params interaction, and the published-character-data schema. **The player stays on the flat melee
  path until fable specs it. Do not build the ability-params system or fold player melee.**
- **JD4 — rooms world-registration SEAM.** Jon adjudicated the direction (LDtk permanent; content owns
  the `.ldtk` worlds via a registration seam; per-room mechanics split by kind) but fable SIZES the seam
  shape first. **Do not start moving the `.ldtk` files, the world list, or the room-id mechanic branches,
  and do not build the world-registration / room-hook seam.** (This is why item #1's cut-rope-system move
  is conditional.)

Also NOT in scope: the DEFERRED-TUNING sweep (feel/value tuning — Jon's own work) and feel-checking the
BLIND commits (Jon's).

---

## Definition of done
Every item 1–7 landed in green committed slices (or explicitly noted-and-skipped with a reason, if it
revealed a hidden fork), the fable-review doc's E-log + status bullets updated to match, the workspace
compiling and all suites green except the known pre-existing `unified_melee` red. THEN stop and report a
summary (what landed, what was skipped-for-fable, the BLIND commits Jon must feel-check).
