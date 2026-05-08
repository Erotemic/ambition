# Path Forward

> Status: **draft strategy** synthesizing `crate-remap-idea.md` and
> `candidate-optimization-ideas.md` with the active threads as of this
> commit. Read with `docs/AGENT_HANDOFF.md`, `docs/CURRENT_STATE.md`,
> ADR 0012 (events refactor), and ADR 0013 (compile-time hygiene).

## What this document is

A sequencing proposal so the next session — agent or human — can pick up
cold and know which thread to work on, why it's first, and what the
"stop here and ask" gates are. It is **not** a contract; the
recommendation column is opinionated and call-out the alternatives.

## Where we are

- Two-crate workspace: `ambition_engine` + `ambition_sandbox`. Sandbox
  build is ~10 min clean, multi-minute incremental — the dominant cost
  per ADR 0013. The crate mixes simulation, presentation, audio, LDtk
  authoring, devtools, physics, content. ADR 0012 already identified the
  cross-cutting events problem; **all five of its slices have landed**
  (audio / VFX / debris / setup-split / app-builder-split — see ADR
  0012's "Implementation progress (as of 2026-05-07)" section). The
  follow-on threads (crate refactor, IntGrid LDtk migration, bug
  record/replay) build on the landed seam.
- **An audio agent is in flight in parallel** — owns `audio.rs`,
  `pause_menu.rs` music wiring, and related work. Anything new this
  session must avoid touching audio code paths.
- **Recent visible regressions still open:**
  - Wall-jump occasionally catapults the player out of bounds in the
    square arena. Two regression tests landed in `movement.rs` that
    cover the snap-direction tunneling failure mode, but the in-game
    repro persists from at least one *third* code path that the tests
    don't yet hit. Root cause is the bespoke snap-direction logic in
    `sweep_player_x` / `resolve_axis`; parry2d is doing only the raw
    math (`cast_shapes`, `intersection_test`) — everything around it is
    hand-coded.
  - Sprite art still doesn't fit the collision rect tightly. Per-sheet
    union-bbox crop is in (`sheet.py`), per-target anchor calibration is
    in, but the *runtime body-bbox sample-rect* approach (use
    `sprite.rect = body_pixel_bbox`, custom_size = collision-fitting)
    has not been implemented.
- **Architectural threads on the table:**
  1. Crate refactor — two candidate plans (`crate-remap-idea.md`,
     `candidate-optimization-ideas.md`).
  2. LDtk IntGrid + AutoLayer migration for tile-based level art (the
     Hollow-Knight-to-Celeste-spectrum question).
  3. Bug record/replay infrastructure (idea raised this session).
  4. Bespoke collision rewrite to use parry contact normals (or a swap
     to Avian's character controller).

## Reconciling the two crate proposals

Both proposals agree on the long-term shape (`engine` + `sim` + `ldtk` +
`bevy_*` + `sandbox` + future `story_*`). They disagree on **sequencing**:

| | `crate-remap-idea.md` (mine) | `candidate-optimization-ideas.md` (other agent) |
|---|---|---|
| Phase 1 | extract `ambition_audio` | **measure** clean+incremental builds |
| Phase 2 | extract `ambition_world` | **feature-gate** the sandbox crate without moving anything |
| Phase 3 | extract `ambition_ldtk` | finish ADR 0012 events |
| Phase 4 | extract `ambition_bevy` | split big modules internally |
| Phase 5+ | — | only **then** evaluate crate extraction |

The other agent's plan is more honest. My plan jumped to extraction
before measurement; the conservative plan refuses to assume the seams
are correct until the events refactor proves them. **Recommendation:
adopt the candidate plan as primary, with one carve-out** —
`ambition_audio` continues independently because the audio agent is
already doing it and music/SFX data have a clear, narrow boundary that
doesn't depend on the events seam.

## Recommended sequencing

Each step has a "why now" and a "stop gate" — a question that should be
answered before moving on.

### A. Measure & feature-gate (~1 day)

`candidate-optimization-ideas.md` Phase 0 + Phase 1 verbatim.

```bash
cargo build --timings -p ambition_sandbox
cargo tree -e features -p ambition_sandbox
cargo tree -d
```

Then add feature groups:

```toml
[features]
default = ["visible"]
visible = ["ldtk_runtime", "input", "audio", "ui", "physics_debris"]
headless = []
dev_tools = ["visible", "dep:bevy-inspector-egui", "bevy/file_watcher"]
ldtk_runtime = ["dep:bevy_ecs_ldtk", "dep:bevy_asset_loader"]
audio = ["dep:bevy_kira_audio", "dep:fundsp"]
ui = ["dep:bevy_material_ui", "dep:bevy_yarnspinner"]
physics_debris = ["dep:avian2d"]
input = ["dep:leafwing-input-manager"]
```

`#[cfg(feature = "...")]` gates around the corresponding module imports
and plugin installations.

**Stop gate:** does `cargo check --no-default-features --features
headless` succeed without dragging Kira / inspector-egui / Avian /
bevy_ecs_ldtk into the dependency graph? If yes, the gate works. If no,
the next sub-step is decoupling whatever's leaking; do not move to step
B without this answered.

**Coordination:** the audio agent's branch is the natural owner of the
`audio` feature gate.

#### A.1 + A.2 status (landed)

Scaffolding + five of six gates are in. All heavy deps are `optional =
true`; `default = ["visible", "dev_tools"]` preserves prior behavior.

| Feature | Status | Approach |
|---|---|---|
| `dev_tools` | **landed** | Inspector imports + EguiPlugin + 4× ResourceInspectorPlugin + WorldInspectorPlugin extracted into `add_dev_tools_plugins`. |
| `physics_debris` | **landed** | Sim-side types (PhysicsSandboxSettings, PhysicsDebrisCue, DebrisBurstMessage, PhysicsRoomEntity) stay always-available; avian impl + presentation systems cfg-gated; `retire_physics_entity` and `spawn_static_collider_for_block` get no-op stubs. |
| `ui` | **landed** | `bevy_yarnspinner` + `bevy_material_ui` plugin installs gated; the dialogue runtime / overlay (`DialogState`, `dialog_input`, `sync_dialog_ui`) draw with core Bevy UI and stay installed unconditionally. |
| `input` | **landed** | After step B's ControlFrame extraction the sim is leafwing-free; A.2 finishes the job by gating `input.rs`'s leafwing items, the `attach_player_input_components` startup, the `populate_control_frame_from_actions` bridge, the leafwing-driven preset cycler, and the `inventory_input` / `pause_menu_toggle` / `pause_menu_navigate` / `debug_overlay::draw_debug_overlay` systems. The visible-side install moves into `add_input_plugins`. |
| `audio` | **landed** | `SfxMessage` + `SoundCue` + `From<SoundCue> for SoundCueKey` stay always-available; everything kira/fundsp gets cfg-gated, including `MusicChannel`/`SfxChannel`/`AudioLibrary`/`MusicPlaybackState`/the rendering pipeline. `setup::presentation_world` and `setup_presentation_system` have audio-on/audio-off twins; `pause_menu` provides audio-on/off `label`/`sync_pause_menu` variants; `pause_menu_navigate` is `cfg(all(input, audio))` since music switching needs both. Visible-side install lives in `add_audio_plugins`. The `tune_preview` bin gets `required-features = ["audio"]`. |
| `ldtk_runtime` | **deferred → step C** | `ldtk_world.rs` (~2k lines) interleaves the bevy_ecs_ldtk plugin glue with the sandbox-side parser/validator headless uses. Step C explicitly splits this file; once split, the bevy half can be cleanly gated and the parser stays unconditional. |

Verified:

- `cargo check -p ambition_sandbox` (defaults) — passes.
- `cargo check -p ambition_sandbox --no-default-features --features headless,ldtk_runtime` — passes; **kira / fundsp / inspector / avian / yarnspinner / material_ui / leafwing-input-manager all drop from the dep tree** (verified via `cargo tree`).
- `cargo check -p ambition_sandbox --no-default-features --features ldtk_runtime` (no headless either) — passes.
- 31/31 sandbox lib tests pass.

The doc's literal stop gate is "succeed without dragging Kira / inspector-egui
/ Avian / bevy_ecs_ldtk." Three of four are stripped today; `bevy_ecs_ldtk`
remains and only comes out after step C. Headless without `ldtk_runtime`
hits 12 errors all routed through `LdtkPlugin` / `register_ldtk_entity` /
`init_collection<SandboxAssetCollection>` — exactly the call sites step C
needs to relocate.

### B. Finish ADR 0012 events refactor (~1-2 days)

ADR 0012 has identified the slices: SFX, VFX, debris, setup-split,
app-builder split. Some are in. Finish the rest. The audio agent's work
on music tracks is one of the slices already.

This is **the load-bearing seam for everything else.** Without it:

- Headless can't tick the real gameplay loop without pulling presentation.
- Sim can't be extracted into its own crate (the boundary doesn't exist).
- Bug record/replay (step F) has no event vocabulary to record.
- Tests can't drive sim without window/render plugins.

**Stop gate:** can a unit test tick `sandbox_update` end-to-end with
`MinimalPlugins` only — no AudioPlugin, no RenderPlugin, no
inspector — and observe SfxMessage/VfxMessage/DebrisBurstMessage flow?
If yes, the seam holds.

### C. Split big sandbox modules internally (~half day)

`ldtk_world.rs` is currently:

```
LDtk JSON structs + validation + runtime-room composition +
bevy_ecs_ldtk registration + hot reload + runtime-spine indexing +
collision migration scaffolding
```

Split into `src/ldtk/{json,validate,compile,runtime_index,runtime_spine,hot_reload,bevy_plugin}.rs`.

Same for `features.rs` (hazards/enemies/bosses/breakables/pickups/npc
each get a file) — although that one is less urgent since the file is
already coherent.

This is *cheap* and creates the natural homes for the IntGrid migration
(step E) and any future `ambition_ldtk` crate extraction.

**Stop gate:** does the build still pass with no behavior change? `git
log --stat` should show pure code movement (`-N M`).

#### C status (multi-module split in progress)

`features.rs` has been split into a small facade plus domain modules under
`src/features/` (`runtime`, `bus`, `events`, `hazards`, `enemies`, `bosses`,
`breakables`, `pickups`, `chests`, `npcs`, `path_motion`, `world_overlay`, and
focused tests). This is the preferred shape for future gameplay-system edits:
load the domain file instead of the whole historical feature runtime.

`ldtk_world.rs` is now also partly decomposed. The earlier
`ldtk_world/bevy_runtime.rs` extraction still owns the bevy_ecs_ldtk-facing
runtime-spine surface. Additional non-Bevy pieces now live in `hot_reload.rs`,
`intgrid.rs`, `surfaces.rs`, `fields.rs`, and `tests.rs`, with the top-level file
kept as the public facade plus the remaining schema/validation/room-composition
body. The remaining cleanup is to split that facade body into explicit schema,
validation, and room-compiler modules.

Default and headless-ish build checks should continue to pass after each slice;
there should be no behavior change from these moves.

### D. Stabilize collision (~1-2 days)

The bespoke snap logic in `movement.rs::sweep_player_x` /
`sweep_player_y` / `resolve_axis` picks push direction from `delta.x`
sign, not from contact geometry. Two regression tests in `movement.rs`
cover known failure modes; the user reports a third in-game repro that
the tests don't trigger. Two paths:

**D1: replace hand-coded snap direction with parry's contact normal.**
`cast_shapes` returns `ShapeCastHit` with `normal1: Vector2` (the
contact normal on the moving shape). `contact_manifold` returns a
proper minimum-translation-vector for overlap repair. Replacing the
"pick face from `body.center` vs `block.center`" heuristic with parry's
contact data closes the whole class of snap-direction tunneling bugs.
Estimate: 1 day with regression tests.

**D2: swap to Avian's character controller.** ADR 0007 currently has
Avian as secondary physics for debris only. Avian has a built-in
kinematic character controller that does what we're hand-coding.
Estimate: ~3 days; bigger risk but cleaner end state. Probably wait
until after the crate refactor settles.

**Recommendation: D1 now**, D2 deferred to a separate session.

**Stop gate:** the in-game wall-jump-OOB repro should be unreproducible
by hand AND a fuzz-style test that randomizes wall-jump start positions
within the square arena should not OOB.

### E. LDtk IntGrid + AutoLayer migration (~1-2 days)

After step C (LDtk modules split out), IntGrid lands cleanly in
`ldtk/compile.rs`. The Hollow-Knight ↔ Celeste spectrum (per the prior
exchange) is one engine path with two authoring extremes.

Concrete: add an IntGrid "Collision" layer to the LDtk file. The runtime
reads collision from cells. AutoLayer rules drive the visual tile layer.
Existing `Solid`/`OneWayPlatform`/`BlinkWall`/etc. entity rectangles get
ported via a one-time migration script (mechanical: cells = bounding
boxes / 16). Existing entity-driven things (chests, NPCs, breakables,
enemies, bosses, loading zones, kinematic paths) **stay as entities** —
they don't fit a grid.

**Stop gate:** can the `central_hub_complex` room render via IntGrid
collision + AutoLayer tiles AND ship the existing gameplay (no
regressions)? If yes, the migration template works for the rest.

#### E status (collision IntGrid landed across every gameplay level)

Every gameplay level now uses an IntGrid `Collision` layer for its static
collision. `tools/ldtk_intgrid_migration.py` adds the layer def + per-level
instances and lowers the existing rectangular Solid / OneWayPlatform /
BlinkWall entities into cells (107 entities → 14581 cells across the 12
levels in sandbox.ldtk). The Rust runtime reads `intGridCsv` and lowers
non-zero values into engine blocks — value 1 = Solid, 2 = OneWayUp,
3 = BlinkSoft, 4 = BlinkHard. `int_grid_value_to_block` is the
authoritative mapping; new tile types extend it.

What's next on this thread:
1. Visual layer — currently the cells render via `spawn_room_visuals`
   (one colored quad per cell). AutoLayer rules with a tileset PNG would
   replace those quads with real tile art; we don't ship a tileset yet
   so this stays as follow-up.
2. Optimization — adjacent same-value cells emit one block per cell
   today. The two-pass rectangle merge already collapses runs and
   stacks; profile under load to see if a more aggressive merge is
   warranted.

#### E known bug (resolved): cWid mismatch caused staircase smear

**Root cause** (diagnosed 2026-05-04): the migration script computed
`__cWid = pxWid // GRID` (floor division). LDtk uses
`ceil(pxWid / GRID)`. For `central_hub_main` (1900×1024, GRID 16),
floor gave 118, LDtk expected 119. When LDtk loaded the file it read
my 7552-element `intGridCsv` with stride 119 instead of 118, so every
column of cells drifted left by one cell per row — producing a
clean 1-cell-per-row staircase smear. LDtk then re-saved the
mangled-as-it-was-read array, locking the staircase into the file.

**Fix:** added `cells_for_size(px) = (px + GRID - 1) // GRID` to
`tools/ldtk_intgrid_migration.py` and routed every cWid/cHei call
site through it. Re-ran the migration from the pre-IntGrid baseline
(`8bd0641`) and the cells now render as the rectangles the migration
intended (verified by tools-side dump and `cargo test`).

Lesson: when interoperating with an editor that owns the canonical
file format, cross-check at minimum *one* derived field
(here `__cWid * __cHei == len(intGridCsv)`) against what the editor
emits, not just what the schema documents.

### F. Bug record/replay infrastructure (~1 day, depends on B)

User's idea, expanded:

```text
While the game runs in `dev_tools` builds:
  - Maintain a ring buffer of the last 600 frames (~10 s at 60 fps) of:
      * ControlFrame inputs
      * Per-system simulation events (SfxMessage / VfxMessage /
        DebrisBurstMessage / future SimMessage)
      * Player snapshot (pos, vel, on_ground, on_wall, hitstun, etc.)
      * Active room id
  - On `--record` flag OR `F12 Ctrl-B` keypress, dump the ring buffer
    to `target/bug_reports/<timestamp>.json` plus the LDtk world hash
    so it's reproducible.
  - A separate `cargo run --bin replay <path>` rebuilds a deterministic
    sim from the dump and fast-forwards through the events; agents and
    humans can step through frames.
```

This subsumes the user's two variants ("record on demand" and "rewind
on bug report key"). It's specifically what the events refactor enables;
without typed events it'd be a bespoke logging spaghetti.

For the wall-jump bug specifically: the dump captures the exact frame
sequence so replay reproduces it deterministically and a regression test
can be derived directly from the dump.

**Stop gate:** does a recorded session round-trip — replay produces the
same player.pos sequence as the original run? If yes, deterministic
record/replay works.

### G. Crate extraction — only if measurements support it

After A-F, re-run timings. Where do crate boundaries make sense?

- `ambition_audio` — already in flight via the audio agent.
- `ambition_sim` (gameplay loop, room graph, runtime state, headless app
  builder, replay observation structs) — extract if the sim/presentation
  events seam (B) is rock-solid AND test latency benefits clearly.
- `ambition_ldtk` (pure parse/validate/compile, no Bevy) +
  `ambition_bevy_ldtk` (bevy_ecs_ldtk plugin + runtime spine + hot
  reload) — extract once IntGrid (E) and the LDtk module split (C)
  prove the boundary.
- `ambition_bevy_presentation`, `ambition_devtools` — only when a real
  second binary (e.g. `ambition_game`) needs them.

The candidate doc's success criteria stand:

```bash
cargo check -p ambition_sandbox                              # visible default
cargo check -p ambition_sandbox --features dev_tools         # devtools on
cargo check -p ambition_sandbox --no-default-features --features headless
cargo build --timings -p ambition_sandbox                    # before/after compare
```

## Specific decisions to make before step A

| Decision | Default | Alternative | Why this matters |
|---|---|---|---|
| Order of D vs everything else | D after C, before E | D first if wall-jump OOB blocks playtesting | Collision robustness affects every iteration |
| Collision approach (D1 vs D2) | D1: parry normals | D2: Avian character controller | D2 is cleaner long-term but bigger refactor |
| IntGrid scope (E) | One room (`central_hub_complex`) first, then expand | Big-bang migrate all 11 levels | Per-room is safer for hot-reload semantics |
| Bug-replay storage | JSON dumps, target/ folder | bincode / msgpack / SQLite | JSON is human-readable for an agent reviewer; size is OK at ~600 frames |
| Crate refactor commit | Defer until after A-F land | Start now in parallel with audio agent | Refactor disrupts a parallel-running agent; conservatism wins |

## What NOT to do (from candidate doc, slightly adjusted)

1. Big-bang crate split before A-D land.
2. Rename crates for aesthetics.
3. Make `ambition_engine` Bevy-independent as a goal.
4. Replace the player controller with Avian *until* D1 is in and
   confirmed insufficient.
5. Adopt `bevy_rl` / `bevy_dev_tools` as an immediate dependency — the
   replay system in step F is a smaller, scoped subset.
6. Move the existing audio code while the audio agent is mid-flight.
7. Touch the LDtk file's structure during step E without an editor
   round-trip — the standing rule is anything we author must round-trip
   cleanly through the LDtk 1.5.3 GUI.

## Concrete first session a fresh agent could pick up

If the next session has a fixed scope, the most-useful single step is
**A** (measure + feature-gate). It's mechanical, low risk, doesn't
collide with the audio agent, and produces immediate compile-time wins
plus the data needed to argue for or against any later phase. ~1 day.

If they have appetite for two:

1. A (measure + feature-gate)
2. C (split `ldtk_world.rs` into modules)

These together unblock both step E (IntGrid) and step G (crate
extraction) without touching gameplay code.

If the user wants visible playtesting unblocked sooner, **D1** instead —
fix collision robustness first, eat the slight architecture-debt cost
for a few sessions. The wall-jump OOB and any related collision
surprises become unreproducible, after which architecture work resumes.

## References

- `docs/adr/0012-sim-presentation-split-and-events-refactor.md` — the
  events boundary; load-bearing for steps B, F, G.
- `docs/adr/0013-compile-time-hygiene.md` — why compile-time discipline
  exists; the refactor must not regress this.
- `docs/adr/0009-world-composition-and-ldtk-authoring.md` — the LDtk
  authoring contract that step E must preserve.
- `crate-remap-idea.md` — earlier crate-extraction proposal.
- `candidate-optimization-ideas.md` — the more conservative measurement-
  first plan; primary basis for steps A-G above.
