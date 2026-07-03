# Lessons learned

This journal records unexpected errors encountered while iterating on the Ambition sandbox, especially places where an overlay or generated build script looked reasonable but failed in a real local/device test. The goal is to make future LLM-generated patches less likely to repeat the same mistakes.

## 2026-06-21: Mockingbird boss "flew out of bounds and hovered above the arena" — an unbounded collision pushout-teleport

Symptom: intermittently (no player interaction needed) the flying mockingbird boss vanished; zooming the camera out revealed it hovering above the arena. "Never happened before, easy-ish to reproduce."

How it was caught (fast, by purpose-built tooling): the existing trace recorder was player-centric, so a non-player-centric **actor OOB flight recorder** was added first (`crates/ambition_gameplay_trace/src/actor_trace.rs` + `dev/trace/actor_oob.rs` — one `Query<&BodyKinematics>` over EVERY body, dumps the offender's trajectory on OOB). It nailed the symptom in one dump: in a single tick the boss jumped `pos (266, 265.3) → (266, -92.5)`, x unchanged. `y = -92.5 = -half_height` ⇒ the boss's bottom edge snapped to a block face at the world top (y=0), flinging the whole body above the room. A single-frame, x-preserving jump to ±half is a **pushout/teleport signature, not movement** (the boss's brain `velocity_target` is capped at its move speed and can't produce a 357px step).

Root cause: `step_kinematic`'s resting/penetration resolves snap feet-to-surface, and most were **unbounded**. A gravity-free, oversized boss (combat 500px wide in a 960px arena) sits pinned at the soft-clamp's left edge overlapping a tall full-height wall; the resolve snapped its feet to that block's FAR face (the top) instead of exiting via the near (side) face, teleporting it clear out of the world. Only `resolve_penetration` had a `snap.length() <= half_extent` guard; the support-stabilization snap and `resolve_axis`'s one-way + full-solid pushes did not.

Fix: factor `is_contact_range_snap` (a legitimate resting/contact resolve moves the body at most its own half-extent) and apply it to EVERY feet-to-surface / penetration resolve. A rejected snap leaves the body overlapping; its own velocity carries it out at the NEAR face next frame (transit emerges at the face — never artificially pushout, per the engine invariant).

Takeaways:
- **A single-frame, axis-pure jump to ±half-size is a pushout/teleport, not physics.** When a body "teleports out of the world," grep `step_kinematic` for every `body.pos += snap/push` and check each is bounded; the offender is usually an *un*guarded resolve sitting next to a guarded sibling.
- **Oversized + gravity-free bodies are pushout magnets.** A body wider/taller than its play space is always overlapping geometry, so any unbounded depenetration aims it at a far face. Bound the resolve; separately, right-size the body to the arena.
- **When the existing debugger is player-centric, generalize it before chasing the bug.** The shared `BodyKinematics` component made a relativity-respecting recorder a one-query system, and it paid off on the first capture. (See `reference_oob_trace_tooling`.)

## 2026-06-04: A second camera (3D cube pause menu) rendered black because the game's `With<Camera>` follow dragged it off-target

Symptom (a full session of blind iteration with the user, ~6 rebuild/test rounds): the new #31 OoT-style 3D inventory cube — a `Camera3d` overlaid on the 2D game — rendered as black, then purple, then gizmo-lines-only, with run-to-run variance, never showing the faces, even though the bevy_lunex meshes were demonstrably built.

Root cause: the game's camera systems — the follow in [`presentation/rendering/camera.rs`](../../crates/ambition_render/src/rendering/camera.rs), plus `parallax.rs` and `foreground.rs` — all query `With<Camera>`. Adding ANY second camera makes those queries match it too. The camera-follow obediently dragged the cube `Camera3d` up to the player's world position (`y≈120.5`), aiming it at empty space, while the 85 cube meshes sat correctly at the origin. The cube camera was rendering fine — at nothing.

Wrong turns (each a real but *different* issue, none of them the reported black):
- Pause-gating the cube camera (it was clearing black every frame over the game). Real fix, wrong layer.
- `IsDefaultUiCamera` on the main `Camera2d` — the second camera had also broken bevy_ui's implicit default-UI-camera pick, so the HUD + pause overlay silently rendered to the wrong camera and vanished (present + clickable, just unseen). Real, separate fix worth keeping.
- bevy_lunex faces missing `UiLayoutRoot::new_3d()` + `Dimension` → child `UiLayout::window()` planes resolve `Rl`/`Rh` against nothing → zero-size. Real, separate fix worth keeping.
- MSAA mismatch (game `Camera2d` defaults to `Sample4`, the cube forced `Msaa::Off`) sharing one window — a genuine fragility of layering 3D-over-2D. Addressed by making the cube the SOLE active camera while shown (disable the 2D camera), which also matches the mock demo's single-camera setup. Got a full-screen purple clear but STILL no geometry — because the camera was still aimed at empty space.

What finally cracked it: a probe system logging the camera's `GlobalTransform`, a marker mesh's `ViewVisibility`, and the total `Mesh3d` count. One line settled it: `camera at Vec3(0.0, 120.5, -2.28) … box at Vec3(0,0,0) ViewVisibility=false; total Mesh3d entities=85`. That reads as "85 meshes exist, the box is at the origin, the camera is 120 units away pointed elsewhere" — i.e. not a missing-geometry / pipeline / material / MSAA problem at all, but a camera-aim problem.

Fix: target the actual game camera — `With<Camera2d>` instead of `With<Camera>` — in the three camera systems. The cube `Camera3d` is then invisible to follow/parallax/foreground, stays pinned, and renders the cube. (Then: strip the debug scaffolding, set a dark backdrop, add snap-to-active ring rotation + Left/Right page nav.)

Lessons:
- **Adding a second camera to a Bevy game silently breaks every `With<Camera>` query in the codebase** — follow systems mutate the wrong camera, `.single()` queries start erroring, parallax reads the wrong transform. The moment a second camera exists, audit `With<Camera>` and narrow each to a specific marker (`With<Camera2d>`, a `MainCamera` tag).
- **3D-over-2D on one window is fragile** (MSAA / depth / clear / compositing, with run-to-run variance). Making the overlay the *sole* active camera while it's up sidesteps the whole class — and matches how standalone 3D demos are built.
- **When "nothing renders," probe `ViewVisibility` + `GlobalTransform` + entity count BEFORE touching materials / MSAA / pipeline.** It collapses a dozen hypotheses into the single bit "in view, or not."
- bevy_lunex 3D faces each need `UiLayoutRoot::new_3d()` + a `Dimension`, or their `UiLayout::window()` children collapse to zero size and the camera clears to a blank screen.

## 2026-06-04: Swept-collision parallel graze teleports the body out the wide ceiling's far X edge (the X analog of the May y-sweep edge-touch bug)

Symptom (reported across three sessions, ~a dozen traces): flying up to the hub ceiling and "moving around a bit" popped the player out of bounds — a single-frame teleport like `x1000 → x1919` (past the world's right wall at 1904) while the player was moving **LEFT**.

Root cause: axis-separated AABB collision. The player box is **30×48**, so resting against the wide thin ceiling (`0..1904 × 0..32`) its top sits exactly on the ceiling's bottom edge (y32). Flying LEFT at speed, the body slides *parallel*, just grazing the ceiling. Parry's swept `cast_shapes` returns a **non-immediate** contact with the ceiling (`time_of_impact` a hair above 0, even though the body is ~0.01px below it and never moves toward it). The swept de-pen only deferred floor/ceiling contacts to the Y pass when `immediate_contact` (toi ≈ 0), so the grazing hit fell through to the push branch, which shoved the body out the ceiling block's **far X edge**: `block.right(1904) − body.left ≈ 918px`.

This is the X-axis analog of the **2026-05-11 y-sweep edge-touch bug** (a body exact-edge-touching a tall wall, y-range nested, Parry `TOI=0`, snapped to the wall's far Y edge ~215px). That one was fixed by rejecting side contacts (`body_is_side_contact`) in the y-sweep predicate. The x-side never got the symmetric guard, so it sat latent for ~4 weeks until a player flew along the ceiling. **Lesson: when you guard one axis of an axis-separated collision against spurious shape-cast hits, mirror it on the other axis.**

Wrong turns (each fixed a real but *different* shape of the same teleport, none the reported one):
- Replaced an overlap-DEPTH heuristic with an exit-distance defer for *deep* penetration — gated on `immediate_contact`.
- Added a world-bounds clamp as a backstop. Actively harmful: the border walls sit AT the world edge, so clamping to `world.size` pinned the body INSIDE the right wall (turned "outside world" into "inside wall, stuck oscillating"). Reverted.
- Added an eject-guard for the *corner* case (near a ceiling edge the near X exit is the world boundary) — still gated on `immediate_contact`.
- I twice concluded "your build is stale," because the math said the committed defer should fire. It did fire — for the *immediate* case. The grazing case is non-immediate, so the immediate-only guard never ran. **Don't dismiss a reproducible report as environmental; reproduce it.**

What finally cracked it: the trace's per-frame dump already carried the player **size** (30×48, not the assumed 24×40) and velocity (moving LEFT but flung RIGHT — a "delta opposes velocity" tell). A unit test calling `sweep_player_x_clusters` **directly** with the exact captured `(pos, vel, size, dt)` reproduced the 918px teleport deterministically; the higher-level "fly around" repro did NOT — the exact sub-pixel graze is what triggers Parry's spurious non-immediate hit.

Fix: one `resolve_x_penetration(body, block, world_w)` helper backing both X de-pen paths (the swept de-pen and the positional `resolve_axis_clusters`) — defers to the Y pass whenever the vertical exit is shorter **regardless of `immediate_contact`**, otherwise pushes the nearer X face but never out of the world. The Y axis is already protected by `body_is_side_contact`, so the class is now closed on both axes.

Benchmark candidate: `dev/benchmark-candidates/swept-parallel-graze-far-edge-depenetration-2026-06-04.md` (the X analog of `movement-edge-touch-y-sweep-question-2026-05-11.md`).

## 2026-05-21: Bevy file watcher EMFILE is `inotify_instances`, not `max_user_watches`

Reported during the intro-v1 polish session: `cargo run -p ambition_gameplay_core --bin ambition_game_bin` on the user's host failed with `Failed to create file watcher from path "…/crates/ambition_gameplay_core/assets", Error { kind: Io(Os { code: 24, kind: Uncategorized, message: "Too many open files" }), paths: [] }`. First guess (in the original developer-tools doc patch) was that `max_user_watches` was exhausted by the ~320 files in `crates/ambition_gameplay_core/assets/`. The user immediately falsified that — `cat /proc/sys/fs/inotify/max_user_watches → 65536` — and confirmed that removing `bevy/file_watcher` from the default `dev_tools` feature resolved the error.

The actual scarce resource on Linux for `inotify_init()` is `max_user_instances` — the count of inotify INSTANCES a user may hold open simultaneously across every program they're running. The default is **128** on most distros (Ubuntu, Fedora, Arch at time of writing). VSCode language servers, file managers, sync clients (Dropbox, Syncthing), browser dev tools, watch-mode test runners, and so on each take one or more inotify instances. By the time the user hits `cargo run` on a Bevy app, the cap is already close to exhausted; Bevy's watcher tries `inotify_init()`, the syscall returns errno 24 (EMFILE), and `notify` surfaces it as `Too many open files`.

Things that won't help:
- Raising `max_user_watches`. That's the per-instance watch-count quota, not the per-user instance count. The original error never hit that limit.
- `ulimit -n`. Per-process fd limit, mostly orthogonal. `inotify_init()`'s EMFILE does NOT come from the process fd table in this failure mode (though a separately-low ulimit can trip the same errno later in unrelated paths).

Things that DO help:
- `echo 1024 | sudo tee /proc/sys/fs/inotify/max_user_instances` (and persist via `sysctl.d`). Bumps the per-user instance cap; cost is a small slab of kernel memory.
- Closing other inotify-heavy programs (VSCode language servers etc.) before `cargo run`. Quick verification step.
- Not enabling `bevy/file_watcher` unless the workflow actually needs live asset hot reload. Polish AS pulled it out of the default `dev_tools` feature and moved it to `dev_hot_reload` (already documented for LDtk hot reload).

Rule for the next agent looking at this error message: `code: 24` from `notify` + "file watcher" + a Linux host → check `max_user_instances`, not `max_user_watches`. If the file watcher feature isn't load-bearing for the current task, drop it; iterating with the watcher is a separate workflow.

## 2026-05-21: Area-spec `world_x` can drift from the live LDtk; treat the live file as truth

While building out the intro-v1 vertical slice (Task 02 reshape of `intro_escape_shaft`) the area spec `tools/ambition_ldtk_tools/specs/intro_escape_shaft_area.yaml` carried `world_x: 104000`, but the live `crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk` had `worldX: 2624`. Inspecting the other intro specs found the same drift across all five of them (`100000 / 102000 / 104000 / 106000 / 108000`), so a previous repo refactor had moved the intro levels in the live LDtk without re-applying the specs.

If you re-apply such a drifted spec with `area create --replace-existing --ldtk intro.ldtk`, the tool obediently authors the level at the spec's stale coordinates and leaves a duplicate-or-misplaced level far away from the active intro strip. The runtime keeps using the original level by `level_id`, so the failure mode is silent: tests still pass, validation still passes, but the LDtk editor view looks weird and the next area-create on a different room produces overlap errors against the misplaced ghost.

Rule: before `--replace-existing` on any historical spec, diff the spec's `world_x`/`world_y` against the live level (`python3 -c "import json,sys; d=json.load(open('...'))['levels']; print({l['identifier']:(l['worldX'],l['worldY']) for l in d})"`) and update the spec to match the live position first. The spec is the rebuildable source; the live LDtk is canon for layout state.

Benchmark candidate: `dev/benchmark-candidates/ldtk-questions.md` is the place to file this — a distilled question along the lines of "before `area create --replace-existing` on a historical spec, what should you reconcile first?"

Adjacent gotcha discovered in the same session: `doctor` did not forward `--secondary-world`, so it false-positives on cross-world LoadingZones (e.g. intro.ldtk's two zones into `central_hub_complex`). Fixed in the same commit run. The general rule is: `doctor` delegates raw `rest` args to both `roundtrip` and `validate`; any `validate`-only flag has to be teachable to `roundtrip` as a pass-through to keep `doctor` viable.

Adjacent tooling gap discovered: `tileset add-layer` errored out instead of being idempotent when the layer def already existed. That blocked recovery from `area create --replace-existing` for levels with a Tiles layer (the replacement dropped the per-level instance). Fixed by making the def-exists path call the existing `add_empty_layer_instance_to_levels` backfill. Rule: tooling that emits per-level instances should always be idempotent so `--replace-existing` is recoverable without manual JSON edits.

## 2026-05-11: Movement split broke extension-trait scope and `Self: Sized` assumptions

A movement refactor split [`crates/ambition_engine/src/movement.rs`](../../crates/ambition_engine_core/src/movement/mod.rs) into child modules and changed AABB sweeps from returning only `time_of_impact` to returning an `AabbSweepHit` with Parry's contact normal. The patch looked mechanically reasonable but failed immediately when the user ran the suggested checks.

The first handoff mistake was the command itself:

```text
cargo test -p ambition_engine movement geometry world
error: unexpected argument 'geometry' found
```

`cargo test` accepts one optional test-name filter, not a list of module names. For a refactor that moves module boundaries, recommend either the whole package (`cargo test -p ambition_engine`) or separate filtered commands. Do not invent a multi-filter syntax in handoff notes.

The real compile errors were two Rust boundary mistakes:

```text
error[E0277]: the size for values of type `Self` cannot be known at compilation time
  --> crates/ambition_engine/src/geometry.rs:57:29
   |
57 |     fn sweep_time_of_impact(self, delta: Vec2, rhs: Self) -> Option<f32> {
```

The new default method was added to an extension trait and accepted `Self` by value. Declarations had compiled before, but adding a default body forced the trait to type-check the method for potentially unsized implementors. In this codebase the trait is for concrete AABB values, so the compatibility method should keep the old by-value call shape and add `where Self: Sized`.

```text
error[E0599]: no method named `bottom` found for struct `Aabb2d` in the current scope
   --> crates/ambition_engine/src/movement/integration.rs:118:37
    |
118 |     let prev_bottom = player.aabb().bottom();
```

The original monolithic file imported `AabbExt` near the top. After extraction, `movement/integration.rs` became its own module, and extension-trait lookup does not inherit facade imports. Any child module that calls methods such as `.bottom()`, `.strict_intersects()`, or `.translated()` needs its own `use crate::geometry::AabbExt;` unless the call is rewritten to inherent fields/functions.

Takeaway: after a facade split, audit imports by *use site*, not by whether the facade still imports something. Extension traits, derive helper imports, and test-only imports are local to the destination module. Also treat handoff commands as part of the patch: if the environment cannot run them, at least check their syntax against Cargo's command model before recommending them.

Benchmark candidate: [`dev/benchmark-candidates/rust-questions.md`](../benchmark-candidates/rust-questions.md) now has a distilled question for this failure class under "Keep trait bounds and extension-trait imports during child-module splits."

## 2026-05-10: Movement-snap probes must validate world bounds, not just intra-block clearance

[`ambition_engine::probe_ledge_grab`](../../crates/ambition_engine_core/src/ledge_grab/mod.rs) checked that the platform on top of a candidate ledge was clear of *other* solid blocks (good), but did not check that the climbed-onto position lay inside the world rect. The mob_lab arena has a ceiling tile at y≈1; a wall-clinging player whose head touched that ceiling could pass `probe_ledge_grab`'s clearance test, get snapped to a `climb_target.y = -23`, and end up above the world.

The visible symptom was a teleport-loop trapping the player in the goblin encounter:

1. Wall-cling near the ceiling.
2. `update_ledge_grab` latches; Up + Jump snaps player to `climb_target` (OOB at y=-23).
3. Engine collision-correction or follow-up movement step bounces the player back to the wall (y≈423).
4. Wall-cling re-fires the probe → step 2.

F8 trace dumps showed two repeating `CollisionCorrection :: 446px (vel-budget 16px)` events alternating directions, with no input edges between them — the giveaway that physics, not input, was driving the loop.

Rule: any "snap-to-target" mechanic (ledge grab climb, blink, dash through, teleport, mantle) must reject targets that fall outside the world AABB before the snap commits, even if local clearance against blocks looks fine. World-bounds rejection belongs in the same probe that does block-clearance — it's the same family of "is this destination physically valid" check, and splitting them invites the snap to fire while the bounds check happens elsewhere.

In this codebase the World rect is `[Vec2::ZERO, world.size]` in top-left coordinates, so for a player body of half-extent `half`, the snap target's AABB must satisfy `target.y - half.y >= 0`, `target.x - half.x >= 0`, `target.x + half.x <= world.size.x`. (Bottom-edge checks aren't needed for upward snaps, but should be added for any future downward mechanics.)

When debugging similar teleport-loops: read the F8 trace's event list for paired `CollisionCorrection` events with equal-and-opposite deltas and no `InputEdge` between them; that's the fingerprint of a physics fight between a bad snap and a recovery system.

## 2026-05-10: Enemies and NPCs share the player's collision semantics via `KinematicBody`

The sandbox used to ship per-actor collision predicates `blocked` / `blocked_y` in `crates/ambition_gameplay_core/src/features/util.rs` that diverged from `ambition_engine::movement::sweep_player_y` in a load-bearing way: OneWay platforms always blocked vertical motion regardless of approach direction. Symptoms downstream:

- A hostile NPC (e.g. the kernel guide that turns into a goblin after three strikes) could not chase the player through a one-way platform. The goblin stood on top of the platform forever while the player fell through it.
- Free-falling enemies could become wedged on top of one-way platforms even when they should have walked off the edge, because the predicate didn't expose `prev_bottom` for the landing-from-above check.

Fix: `ambition_engine::kinematic::step_kinematic` is now the shared sweep used by both `EnemyRuntime::update` and `NpcRuntime::update`. It owns:

- Gravity application with `max_fall_speed` clamp.
- X sweep that matches the player: Solid + BlinkWall block, OneWay never blocks horizontally.
- Y sweep with the player's exact landing-from-above test (`falling && prev_bottom <= block.aabb.top() + 8.0`).
- A `drop_through` input that suppresses the OneWay block this tick — chasing enemies set this when the player is meaningfully below them and they're currently grounded, so the goblin can follow the player through one-way platforms.
- `on_ground` returned to the caller for chase / jump / animation logic.

When adding a new actor type that needs to walk in a level: do NOT roll a new `blocked_*` predicate. Construct an `ae::KinematicBody` from your runtime's `pos / vel / size / on_ground / facing`, call `ae::step_kinematic`, and write the result back. The engine's tests in `kinematic.rs::tests` cover the load-bearing invariants (lands on Solid, lands on OneWay from above, drop-through passes OneWay but still respects Solid, walks off ledge falls, rising body does not get stuck on OneWay) so failures regress as test failures rather than gameplay bugs.

When/if the player migrates to this primitive, the player's tuning grows the abilities-shaped fields and the duplicate sweep helpers in `movement` come out. For now both code paths agree on semantics; the player path keeps its extra affordances (jump buffer, blink, dash, climb, ledge grab).

## 2026-05-10: LDtk `LoadingZone.target_room` is the activeArea id, not the level id

The Ambition LDtk validator (`ambition_ldtk_tools doctor`) keys on a level's `activeArea` field, not the LDtk level identifier. `central_hub_main` and `central_hub_basement` both share the `central_hub_complex` activeArea — multiple LDtk levels can stitch into a single runtime room.

Authoring a `LoadingZone` with `target_room: central_hub_main` (the level id) parses fine and writes a structurally valid project, but `doctor` reports:

```text
error: LoadingZone 'foo_entry' in 'foo' targets unknown room/activeArea 'central_hub_main'
```

The runtime stitches by activeArea, so this is not a cosmetic warning — the warp will not resolve. The `examples/music_biome_lab.yaml` spec in `tools/ambition_ldtk_tools/specs/examples/` still uses the level-id form (likely never applied or pre-validator); do not copy that pattern. New specs should use the activeArea identifier.

Quick check for a destination's activeArea:

```bash
grep -B1 -A1 '"identifier": "central_hub_main"' crates/ambition_gameplay_core/assets/ambition/worlds/sandbox.ldtk \
  | head -2
# Then look for the level's "activeArea" field instance below.
```

## 2026-05-10: Sandbox.ldtk mutations go through `ambition_ldtk_tools`, never sed/Edit

The repo contract is "agents should not hand-edit the LDtk JSON" ([tools/ambition_ldtk_tools/README.md](../../tools/ambition_ldtk_tools/README.md)). The tool's repair + validate pipeline catches editor-roundtrip drift, normalises `realEditorValues` records, and ensures the file stays editor-safe.

When `ambition_ldtk_tools area create` writes the level but `doctor` reports an issue afterward, the right fix is one of:

1. `git checkout` the LDtk file, edit the spec, re-run `area create`. This was the right move for the `target_room` mismatch above.
2. Implement the missing functionality in the tool (some `entity {set-field, move, delete}` subcommands are still placeholders per the README).

Reaching for `sed` or the `Edit` tool is the wrong muscle: the file's `realEditorValues` mirror data has to round-trip with LDtk 1.5 cleanly, and direct edits skip that normalization.

## 2026-05-10: Calibrate `CharacterSheetSpec.collision_scale` against the generator's `body_pixel_bbox` fraction

`build_character_sprite` in `crates/ambition_gameplay_core/src/character_sprites/sheets.rs` (historical path) sizes the rendered quad as `collision.max() * collision_scale`. The visible body inside that quad is determined by the generator's frame layout: how much of `frame_height × frame_width` is opaque body pixels.

Robot/Goblin sheets use `collision_scale: 2.1` because their generator leaves big transparent margins (the silhouette occupies maybe 60% of the frame). Copying `2.1` for a generator like `absurd_general` whose `body_pixel_bbox` covers ~95% of the frame produces a sprite ~2× too tall — the General towers above the player.

Rule of thumb when adding a new sheet:

```
collision_scale ≈ 1 / (body_pixel_bbox_h / frame_h)
```

So a sheet whose body fills the frame ends up near `1.0`, while one with lots of margin ends up near `2.0`. Read `body_metrics.body_pixel_bbox` and `body_metrics.frame_height` from the generator's `<target>_spritesheet.yaml` instead of guessing.

`feet_anchor_y` should match the generator's `body_metrics.feet_anchor_norm.y` directly — that field already encodes the offset from frame center to feet in the same convention Bevy's `Anchor` uses (negative = below center).

## 2026-05-10: Bevy `add_systems` tuple chains cap at 20 systems

Adding `upgrade_npc_sprites` to the big presentation tuple in [crates/ambition_app/src/app/plugins.rs](../../crates/ambition_app/src/app/plugins.rs) (the chain that runs after `sandbox_update`) pushed it from 20 to 21 systems and produced this error:

```text
error[E0599]: the method `chain` exists for tuple `(..., ..., ..., …)`, but its trait bounds were not satisfied
   --> crates/ambition_app/src/app/plugins.rs:355:18
355 |                 .chain()
    |                 ^^^^^ method cannot be called due to unsatisfied trait bounds
```

`IntoSystemConfigs` (and the `chain()` extension) is implemented for tuples up to **20** elements in Bevy 0.18. There is no compile-time message about the cap; you only see the trait-bound failure on `.chain()`. Several earlier comments in this file said "16-system tuple budget" — that was right for older Bevy versions and is now stale.

The established pattern in [plugins.rs](../../crates/ambition_app/src/app/plugins.rs) is **not** to subdivide the chain (which would silently change ordering) but to pull the new system out into its own `add_systems(Update, sys.after(prev))` call. `sync_health_overlays`, `map_menu_pointer_dismiss`, and `update_quest_panel` are already wired this way; `upgrade_npc_sprites` joins them.

Why this beats splitting into two chained tuples:

- Two adjacent chained tuples are not the same as one chain — they don't enforce ordering between each other unless explicitly linked, which is easy to forget.
- A separate `add_systems` with explicit `.after(...)` declares the actual dependency (here: must run after `sync_visuals` populated `FeatureVisual`s) and lets Bevy parallelise the rest.
- Reviewers can see exactly what the system depends on, instead of having to reason about position in a 20-line tuple.

When extending the presentation tuple in the future: keep it at ≤ 20, and prefer adding new systems as standalone `add_systems` calls with `.after(...)` ordering against whichever existing system actually dictates the dependency.

## 2026-05-08: Android APK bring-up

### Prefer generated Android projects, but keep the generated Java/Gradle side explicit

The first Android APK path successfully built the Rust shared library with `cargo-ndk`, but Gradle/device launch uncovered several Java-side assumptions. Each failure happened before Bevy gameplay started, so the fix belonged in the generated Android shell rather than in gameplay code.

Observed fixes:

- `android.useAndroidX=true` is required because GameActivity is distributed through AndroidX artifacts.
- The manifest should launch an app-local `.MainActivity`, not a Maven-coordinate-looking class such as `androidx.games.activity.GameActivity`.
- `MainActivity` should extend `com.google.androidgamesdk.GameActivity`.
- GameActivity extends `AppCompatActivity`, so the app needs both `androidx.appcompat:appcompat` and an AppCompat-derived theme.
- Transitive Kotlin dependencies may mix old `kotlin-stdlib-jdk7/jdk8` artifacts with newer `kotlin-stdlib`; the generated Gradle project now aligns Kotlin artifacts and excludes obsolete compatibility jars.
- A repo-local Gradle user home under `target/android/gradle-user-home` avoids unrelated host `~/.gradle` cache permissions breaking this project.

### Do not assume adb install flags are portable

I suggested `adb install --no-stream`, but the target device rejected it as an unknown package-manager option. The build script should prefer conservative install flags (`-r -d --install-location 0`) and provide a `--fresh-install` mode that force-stops/uninstalls first.

### Overlay patches must not clobber platform entrypoints

A later Android usability overlay replaced `crates/ambition_gameplay_core/src/lib.rs` from a source snapshot that did not contain the Android shared-library entry point. The APK still built and installed, but launch failed with:

```text
UnsatisfiedLinkError: dlopen failed: cannot locate symbol "android_main" referenced by libambition_gameplay_core.so
```

The lesson is that files touched by multiple overlay series need special care. Before overwriting `lib.rs`, `Cargo.toml`, or generated build scripts, preserve platform-critical entrypoints and feature definitions added by earlier overlays.

For Bevy Android GameActivity builds, the Rust library must export `android_main`. In this project the intended pattern is:

```rust
#[cfg(target_os = "android")]
#[bevy::prelude::bevy_main]
fn main() {
    app::run_visible();
}
```

Desktop still enters through `src/main.rs`; Android packages the library as `libambition_gameplay_core.so` and needs Bevy's `#[bevy_main]` macro to generate the Android boilerplate.

### Keep asset behavior platform-aware

Android packages runtime assets into the APK. Host-side `CARGO_MANIFEST_DIR/assets/...` existence checks are not valid on-device. On Android, let Bevy's APK asset reader attempt the load; on desktop, host-side existence checks are still useful for clearer diagnostics.

### Treat device logs as the source of truth

The Android sequence progressed through distinct phases:

1. APK installed but manifest activity class was missing.
2. Java activity compiled but AppCompat dependency/theme was missing.
3. Native library loaded but `android_main` was missing.

Each phase required a different layer of the stack to be fixed. Avoid guessing from the symptom alone; use `adb logcat` and identify whether the failure is Gradle, install/package-manager, Java activity startup, native library loading, or Rust/Bevy runtime.

## 2026-05-08: Keep Android HUD defaults and menu toggles separate

The Android build can boot with the same desktop sandbox systems, but phone usability needs
coarse user-facing switches for large overlays. Do not only change `DeveloperTools::default`
when a HUD is too large: add an explicit persisted setting and make the render system clear
its text when the setting is off. Quest/objective UI and debug HUD text should be controlled
separately because the quest panel is useful during play while the debug dump can consume most
of a phone screen.


## 2026-05-08: Android size is a separate profile and platform-composition problem

A large Android APK/native library should not immediately trigger semantic
feature-gate churn. First separate the size mechanics from the gameplay feature
set:

- build Android with `--no-default-features --features android` so desktop-only
  inspector/file-watcher tooling does not enter the phone artifact by default;
- keep the playable sandbox, touch controls, audio, LDtk runtime, UI, and RL/test
  seams in the Android composite feature;
- add a dedicated `android-size` Cargo profile before removing gameplay systems;
- strip the final `.so` explicitly with the NDK `llvm-strip` as a backstop;
- print before/after sizes so future patches compare measurements instead of
  guessing from APK size alone.

The principle is platform composition, not release minimalism: Android can remain
a dev/test build while excluding desktop inspector/editor conveniences that are
not useful on a phone screen.


## 2026-05-08: Android APK assets are not regular files

The Android build copied `assets/audio/sfx.bank` into the APK, but the game still
fell back to generated/fundsp SFX. The reason was that the SFX bank loader used
`std::fs` and normal paths such as `/assets/audio/sfx.bank`; packaged APK assets
are not visible at those paths on-device. Bevy's `AssetServer` can load many
runtime assets from the APK, but this specific SFX-bank path is a synchronous
custom loader built around `BankProvider::from_path` / `from_bytes`.

Temporary fix: let `build_for_android.sh` statically embed the SFX bank with a
separate `static_sfx_bank` feature when the bank exists locally. Long-term fix:
teach the SFX bank loader to read bytes from Android APK assets or route it
through Bevy asset loading, then remove the static embedding workaround.

The lesson is to distinguish "copied into APK assets" from "readable via
`std::fs`". Any custom synchronous loader needs an explicit Android asset path,
static fallback, or Bevy asset pipeline bridge.

## 2026-05-08: Size diagnostics should be automatic for phone builds

A 200 MiB native library became a much more reasonable ~49 MiB `.so` after using
a size-oriented Cargo profile, disabling desktop-only default features for
Android, and stripping with the NDK toolchain. Future Android patches should keep
printing `.so`, APK, and asset-tree sizes so we notice regressions immediately.

## 2026-05-08: menu controls need their own semantic frame

Touch menu polish should not be implemented by making every menu read raw
`Touches`, raw `ButtonInput<KeyCode>`, or Leafwing `ActionState` directly.
That repeats the same problem we solved for gameplay with `ControlFrame` and
makes Android ergonomics fight keyboard/gamepad/RL semantics.

The better pattern is a parallel `MenuControlFrame` resource:

- gameplay systems consume `ControlFrame` only;
- menus/dialogue/cutscenes consume `MenuControlFrame` only;
- keyboard/gamepad, mouse wheel, touch buttons, and touch drag gestures fold
  into the menu frame before menu systems run;
- mobile touch can add scroll/back/confirm semantics without adding
  Android-specific branches to every menu.

This came up when the Android pause/settings menu was hard to use: there was no
standard touch scroll/back seam, and some systems still consumed raw keyboard
or Leafwing actions. The fix is an intent layer, not a collection of menu-local
touch hacks.

## 2026-05-08: Menu controls should be semantic, tabbed, and touch-visible

The Android phone test showed that mapping touch controls directly onto a few
keyboard/gameplay actions is not enough for menus. A menu needs its own semantic
input layer (`MenuControlFrame`) and visible touch affordances. In particular:

- Back/cancel must be visible in player-facing overlays, not only implied by a
  keyboard Escape key or gamepad button.
- Left/right should be reserved for changing high-level menu pages where a
  Zelda-style tab model is desired.
- Scroll/drag should manipulate text-heavy menu content without making gameplay
  `ControlFrame` or RL action shapes more complex.
- Phone polish and battery life are related: avoid adding per-frame heavy menu
  work just to support touch; prefer small semantic resources and simple UI
  state transitions.

The inventory panel now acts as a small adventure menu with Items / Map / Quests
tabs. That keeps the phone UI understandable while preserving the existing
keyboard/gamepad menu contract.

## 2026-05-08: Touch controls need controller-shaped affordances, not keyboard labels

The Android touch overlay originally exposed the keyboard interaction action as
an `E` button and arranged six actions as a dense 3x2 grid. That worked as a
mechanical input bridge, but it was not a good phone interface: `E` is a
keyboard mnemonic, not a player-facing action, and small grid buttons are hard to
hit while the left thumb is also holding the movement stick.

Prefer touch-native labels and controller-like spatial grouping:

- use `Use` / `Talk` / `Open` style labels instead of keyboard letters;
- put primary actions in a right-thumb diamond, with secondary actions nearby;
- make buttons large enough for thumbs and keep raw-touch hit testing aligned
  with the visible layout constants;
- keep a visible Back/Menu affordance for escape/cancel rather than assuming a
  keyboard Escape key.

This keeps the semantic input seam intact (`MenuControlFrame` / `ControlFrame`),
but makes the phone UI legible and ergonomic.

## 2026-05-08: Use `ParamSet` for tabbed UI systems that mutate several `Text` queries

The tabbed adventure-menu overlay compiled but the desktop game panicked during Bevy
system initialization with error `B0001`. The system had several separate mutable
queries over `Text`: title text, tab labels, item rows, tab-content text,
description text, and status text. Even though the entities are intended to be
different, Bevy cannot prove that arbitrary mutable queries over the same component
are disjoint unless the filters make that explicit or the queries are wrapped in a
`ParamSet`.

For UI sync systems that update several text-bearing widgets in one pass, prefer one
of these patterns:

- use `ParamSet` and touch each query sequentially;
- make query filters explicitly disjoint with marker components plus `Without<T>`;
- split the system into smaller systems if the updates do not need shared local state.

Do not assume a successful `cargo check` catches every Bevy ECS query conflict. Some
access conflicts are validated when schedules initialize at runtime, so `./run_game.sh`
and Android launch smoke tests remain important after UI refactors.

## 2026-05-08: ParamSet does not replace explicit UI disjointness

A follow-up inventory patch tried to fix Bevy `B0001` by wrapping several
text-mutating inventory queries in a `ParamSet`, but the desktop game still
panicked during schedule initialization. The safer pattern for Bevy UI sync
systems is to make the entity families explicit in the query filters as well:
`With<InventoryTitleText>` should also carry `Without<InventoryTabButton>`,
`Without<InventoryItemRow>`, and the other mutually exclusive marker components.

When a system updates several widgets that all carry `Text`, use both tools:

- group conflicting queries in a `ParamSet` when they need to share local state;
- add marker-component `Without<T>` filters so Bevy can prove each widget family
  is disjoint;
- always run a real Bevy startup smoke test (`./run_game.sh`), because query
  access conflicts can pass compile and fail only when the schedule initializes.

## 2026-05-08: For Bevy UI text sync, one role-tagged query is safer than many mutable `Text` queries

The adventure-menu panel repeatedly hit Bevy `B0001` during desktop startup.
Several fixes tried to convince Bevy that independent UI widget families were
disjoint by using `ParamSet` and marker `Without<T>` filters. That can work, but
it is fragile for UI panels where many different widgets all share `Text`,
`TextColor`, `BackgroundColor`, `Node`, and `Visibility`.

The safer pattern for one-panel sync systems is to use one role-tagged query:
query every relevant widget once with `Option<&RoleMarker>` components and
`Option<&mut ...>` presentation components, then branch by marker in code. This
creates exactly one mutable access path to `Text`, so Bevy's schedule validator
has no aliasing ambiguity.

Also watch overlay archive mtimes. If an overlay zip normalizes entries to times
older than the existing `target/release` output, Cargo may run the old binary and
appear to ignore a source fix. When a source-only overlay is meant to fix a
runtime panic, either preserve a current timestamp in the zip or explicitly run
`touch` on the changed file before the smoke test.


## 2026-05-08 - Bevy UI visibility is also a mutable component access

When fixing query aliasing in Bevy UI systems, remember that `Visibility`,
`Node`, `Text`, `TextColor`, and `BackgroundColor` are all independent ECS
components with their own access rules. Moving text widgets into one mutable
query fixes `Text` conflicts, but a separate root query mutating `Visibility`
still conflicts with any child-widget query that also asks for `&mut
Visibility`. Prefer a single visibility owner for panel roots and use
`Node.display` for child-level show/hide inside the widget query.

## 2026-05-08: Touch buttons should name actions, not keyboard keys

The mobile HUD is used on Android and as a desktop mouse-test overlay, so labels like `E` are misleading even when the default keyboard binding still uses E. Touch buttons should use semantic action labels such as `Interact`, `Jump`, `Dash`, and `Fly`. When adding a new touch button, update all three seams together: visible UI layout, raw multitouch/mouse hit testing, and `TouchButtonEdges` folding into `ControlFrame`. Missing any one of those can make the button appear on screen but not reach gameplay.


## 2026-05-08: Bevy 0.18 moved BorderRadius into Node

The touch-controller overlay tried to make circular mobile buttons by adding
`BorderRadius::all(...)` as a standalone UI component. That worked in older
Bevy examples and still looks plausible, but Bevy 0.18 moved border radius into
`Node::border_radius`; `BorderRadius` is no longer a component. The symptom is a
confusing `is not a Bundle` error for the whole spawn tuple, because one element
of the tuple is not a component bundle item.

For Bevy 0.18 UI, put radius styling inside the `Node` literal:

```rust
Node {
    width: Val::Px(size),
    height: Val::Px(size),
    border_radius: BorderRadius::all(Val::Px(size * 0.5)),
    ..default()
}
```

When updating UI style code, check the current Bevy migration notes or local
examples before assuming a visual style type is still a component.

## 2026-05-07: `ControlFrame` edge fields cannot be derived from a held axis

This bug class shipped *three times* in three different writers before the
held-input regression-test pattern became standard:

- `ebe3686` — `AgentAction` → `ControlFrame` converter set
  `down_pressed = move_y > 0.5` every frame.
- `42f3545` — the touch-input `fold_to_control_frame` did the same shape on
  the touch axis.
- `a63c258` — even with no touch input, the touch fold ran every frame and
  unconditionally overwrote `ControlFrame`, zeroing out keyboard-derived
  `down_pressed` between frames Leafwing had set it true.

### Symptom

Holding Down (keyboard or touch) caused the player sprite and camera to
"shake" or "blink" at ~30 Hz, oscillating between Standing and Crouching.
Two consecutive held-down frames also incorrectly fired MorphBall via
`SandboxRuntime::register_down_tap`'s double-tap-down detector. The bug
was invisible in single-frame unit tests; only multi-frame held inputs
reproduced it.

### Root cause

`ControlFrame.down_pressed` is documented as edge-triggered (true only on
the frame the input was just pressed). Leafwing populates it correctly
via `actions.just_pressed(MoveDown)`. But three other `ControlFrame`
writers each independently re-derived `down_pressed = move_y > 0.5`
from the held axis, producing `true` on every frame the user held Down.
`register_down_tap` counted each frame as a fresh tap, the double-tap
window fired on frame 2, MorphBall transitioned, and the next frame's
body-mode driver flipped back. Per-frame flip = ~30 Hz oscillation.

The third occurrence (touch fold stomping keyboard) was a related shape:
a stateless writer that runs every frame and unconditionally writes a
shared resource will overwrite state another writer just computed,
even when its own input is empty.

### Fix

Three coordinated invariants:

1. **Don't auto-derive edge fields from a held axis.** Source structs
   (`AgentAction`, `TouchInputState`) gain explicit `up_pressed` /
   `down_pressed` edge fields with `#[derive(Default)]` to `false`. The
   source must opt in by setting the field once on the desired edge
   frame.
2. **Compute touch edges from a one-frame history.**
   `read_joystick_messages` keeps a `Local<f32>` of the previous
   frame's `move_y` and emits explicit `move_y_just_crossed_up` /
   `move_y_just_crossed_down` flags only on threshold crossings.
3. **Gate the writer on its own activity.** `fold_to_control_frame`
   checks `touch_state_is_active(...)` before writing; with no
   deflection / no held button / no edge flag, the existing
   `ControlFrame` is left intact.

Regression tests live in
[`crates/ambition_gameplay_core/tests/crouch_stability.rs`](../../crates/ambition_app/tests/crouch_stability.rs)
(held Down for 30 frames must stay Crouching with per-frame `pos.y`
delta < 5 px) and
`fold_held_down_without_edge_flag_does_not_fire_down_pressed` (historical path: `crates/ambition_gameplay_core/src/mobile_input.rs`)
(pins the touch path).

### Takeaway

Edge fields are a contract, not a derivation. Any
`ControlFrame`-shaped resource with both axis fields and edge fields
needs an unambiguous answer to "who computes the edge, and from
what?" — and the answer can never be "from the held axis, in this
writer." When more than one source writes the same frame-rebuilt
resource, every writer additionally must gate on its own activity. A
"held axis for 30 frames" test on every new input source catches both
failure modes.

This is also a good signal that **lessons must propagate.** The same
class shipped three times because the lesson lived only in the
fix-commit's message; it wasn't in any project-level discipline doc
until the third occurrence. When a class of bug recurs, the lesson
should be promoted from commit-message to journal entry to benchmark
question (in this repo, see the corresponding entry in
`dev/benchmark-candidates/rust-questions.md`).

## 2026-05: Local-copy `ControlFrame` doesn't propagate to other Bevy systems

### Symptom

A double-tap-down gesture correctly entered fast-fall (visible in the
in-frame physics path inside `sandbox_update`), but the `body_mode`
driver — a separate Bevy system scheduled later — never saw the
`fast_fall_pressed = true` write that `input_timer_phase` performed.
Crouch worked because it reads the held `controls.axis_y` populated
upstream. MorphBall didn't fire.

### Root cause

`sandbox_update` mutates `ControlFrame` via `ResMut`. But
`populate_control_frame_from_actions` runs `.before(sandbox_update)`
each frame and rebuilds `ControlFrame` from the input pipeline; the
*previous* frame's mutation is gone. More importantly, `ControlFrame`
is the input boundary — overlaying a derived gameplay signal
("double-tap detected, please trigger MorphBall") onto an input field
conflates two layers and is exactly the kind of seam violation that
breaks silently in refactors.

### Fix

Add a separate "pending edge" field on the long-lived `SandboxRuntime`
resource:

```rust
pub struct SandboxRuntime {
    pub double_tap_down_pending: bool,
    // ...
}
```

`input_timer_phase` sets it whenever `register_down_tap` returns
`true`. The body-mode driver consumes via `mem::take` so a stale
signal can't latch across frames. `SandboxRuntime::reset` clears it
defensively.

Regression test
[`morph_ball_does_not_fire_from_control_frame_alone`](../../crates/ambition_gameplay_core/src/body_mode/mod.rs)
sets `controls.fast_fall_pressed = true` directly on the resource and
asserts the driver does **not** enter MorphBall. The negative
assertion pins the seam.

### Takeaway

Don't overlay derived gameplay signals onto an input-boundary
resource. `ControlFrame` is what the input pipeline says happened
this frame; its values are rebuilt every frame. Anything *derived*
from input — gesture detections, multi-frame edges, double-tap
timers — belongs on a separate state resource that lives across
frames. The discipline pays off twice: it's the right architectural
seam (driver reads what driver needs), and it makes the routing
testable independently of the input pipeline.

## 2026-05: Trace dump records state AFTER each step — replay must align accordingly

### Symptom

A determinism guard binary (`trace_replay`) re-ran a deterministic
sim against a `--dump-trace`-recorded fixture and reported sub-pixel
`dx` / `dy` divergence by frame 1, accumulating across the trace. The
sim was deterministic and the dump was stable; the divergence was in
the replay loop's frame alignment.

### Root cause

The dump convention is "record state AFTER each step." So
`frames[i]` holds `(controls applied during step i+1, player_pos
after step i+1)`. The replay loop did `skip(1)` and applied
`frames[i].controls` on step `i` — pairing the controls of frame `i`
with the post-state of frame `i-1` in the comparison. Any non-zero
velocity introduces a one-frame offset that drifts as the integrator
runs.

### Fix

```rust
for i in 0..frames.len() {
    sim.set_controls(frames[i].controls);
    sim.step();
    assert_eq!(sim.player_pos(), frames[i].player_pos);
}
```

After the alignment fix, a 30-tick round trip reports
`max_dx == 0.0`, `max_dy == 0.0`. That makes `trace_replay` a real
determinism guard usable as a CI fixture.

### Takeaway

Sub-pixel is not "close enough" on a deterministic sim — bit-exact
equality is the only acceptance criterion. When you see drift on a
re-run that's supposed to be deterministic, the bug is almost always
in the *alignment* of (controls, state) across the record/replay
seam, not in float precision. Write the alignment convention down in
plain words next to the format definition; the replay loop's first
line should read like prose: "frames[i].controls drives step i; the
resulting position must equal frames[i].player_pos."

## 2026-05-09: Music director plays lofi + adaptive at the same time after outro-restart race

### Symptom

Jon's report: "started the goblin encounter, beat it, but also died
at the same time, which reset me back to the start. I reset and
restarted the goblin encounter, so maybe the timed trigger to
restart the lofi music happened and then the trigger to start the
goblin music happened (because i reset the encounter), so they both
played at the same time."

The room's lofi base track and the encounter's adaptive layers were
audible at the same time — exactly what the director was supposed
to make impossible.

### Root cause

Two-part state machine break:

- `resume_simple_music(...)` was called near the end of the
  AdaptiveOutro tail to overlap the room's lofi return on
  `MusicChannel`. It flipped `director.mode = SimpleTrack`
  *immediately*, while `director.active_cue_id` was still
  `Some(goblin)` and the adaptive layer channels were still
  playing the outro section.
- `drive_adaptive_cue_state(...)` had a single same-cue fast
  path: when the new directive's cue id matched
  `active_cue_id`, it skipped the
  `base_music_channel.stop()` + `start_adaptive_state(...)` path
  and fell through to pending-state / crossfade bookkeeping.

When the encounter restarted during the overlap window, the cue id
still matched, so the fast path fired. The base channel kept
playing lofi while the adaptive layers ramped back up from
`start_adaptive_state` was never called.

The invariant the director was *supposed* to preserve, but didn't
state explicitly:

> Simple base track audible ⇔ no adaptive cue identity, no
> adaptive layers audible.

### Fix

Two coordinated changes in `crates/ambition_gameplay_core/src/music/director.rs`:

1. `resume_simple_music` takes `set_mode_to_simple_track: bool`.
   `drive_outro_tail` passes `false` (still in `AdaptiveOutro`
   until full duration completion). `shutdown_adaptive_cue` passes
   `true` (adaptive layers stopped, `active_cue_id` cleared, mode
   transition is safe).
2. The restart predicate inside `drive_adaptive_cue_state` extracts
   into a free `should_restart_adaptive(...)` and gains two more
   trigger conditions:
   - mode says a simple base track is currently audible
     (`SimpleTrack` / `Idle` / `AdaptiveFinished`) — defensive
     against any other code path leaving the director in an
     invariant-breaking state;
   - the cue is in `AdaptiveOutro` but the new directive points
     to a non-outro state — the encounter-restart-during-outro
     case Jon reported.

The function-extraction step matters because the predicate has six
scenarios to test (cue change, no prior cue, same-cue
steady-state, mode-says-simple, outro-to-active-restart,
outro-continues-to-outro) and each takes 5+ lines of director-state
setup. As a free function it's table-testable in milliseconds; as
a closure inside `drive_adaptive_cue_state` it would need the
whole Bevy `App` + audio channels + asset server to drive.

Regression tests live in
[`crates/ambition_gameplay_core/src/music/tests.rs`](../../crates/ambition_gameplay_core/src/music/tests.rs)
under `should_restart_adaptive_*`.

### Takeaway

State-machine bugs in audio mixers / music directors are
fingerprint-recognizable: "two sources playing at the same time
when the design says only one can." Write the invariant down at
the top of the module — even just a docstring on the mode enum —
so a future change that flips `mode = X` while `cue_id =
Some(Y)` lights up as suspect.

When a state-machine guard is "if (single condition) → take
restart path; else fall through", suspect that the fall-through
path is missing protection against rare-but-real states. Capture
the decision as a free function so each scenario takes one
unit test, not an `App` setup. The smallest reproduction is a
table test against the predicate; the in-game test is the user's
report. Both have a place but the predicate test is the one the
CI runs every commit.

---

## Migrated historical entries from `dev/journals/lessons_learned.md`

These older entries were originally maintained in `dev/journals/lessons_learned.md`. `dev/journals/lessons_learned.md` is now the canonical aggregate journal so future agents have one place to search lessons learned.

Debugging journals for surprises that took serious time to track down.
Ordered newest-first. Each lesson should make the next time you hit the
same class of bug 10× faster — symptom recognition, where to look, and
what the fix looks like in this codebase specifically.

## Wall-cling y-sweep teleports player to wall's far edge

**Date:** 2026-05-04. **Fixed in:** the next commit.

### Symptom

Player wall-clinging on a tall side-wall (e.g. `square_arena`'s left
wall, top at world `y=0`, bottom at the room floor) suddenly teleports
to `(prev_x, -23.0)` — exactly `0 − half_height` — and is reported as
`Grounded`. Subsequent leftward input then walks them off this invisible
ledge above the world, and the OOB detector fires a few frames later.

Two consecutive trace dumps captured the pattern. The post-fix-recorder
trace shows the smoking-gun event:

```
t= 1962  CollisionCorrection :: (62.0, 1678.7) → (62.0, -23.0)
                                [unexplained delta 1701.7px (vel-budget 17.2px)]
t= 1962  PlayerModeChanged    :: WallCling → Grounded
```

### Root cause

`movement::sweep_player_y` was returning a `time_of_impact = 0` swept
hit on the wall block the body was edge-touching / fractionally
penetrating on the X axis. The snap branch then unconditionally pushed
the body's bottom to the wall's TOP edge:

```rust
if delta.y > 0.0 || body.center().y < hit.block.aabb.center().y {
    player.pos.y += hit.block.aabb.top() - body.bottom();
    player.on_ground = true;
}
```

For a wall whose top is at world `y=0` and a body at `y≈1700`, this
push is `0 - 1700 = -1700` — a 1700-px upward teleport.

The symmetric guard already existed in `resolve_axis(Axis::X)` (with a
clear comment), but `sweep_player_y` and `resolve_vertical` were
missing it.

### Fix

Two-part:

1. New helper `dominantly_horizontal_overlap(body, block)` — true when
   the body's existing overlap with `block` is wider on the y axis than
   the x axis. Side-wall contacts have large y-overlap; floor/ceiling
   contacts have large x-overlap.
2. Both `sweep_player_y`'s `first_body_sweep` predicate and
   `resolve_vertical` skip blocks where this returns true. The X-axis
   sweep / resolve owns those.

Plus a regression test (`wall_cling_does_not_teleport_to_wall_top_on_y_sweep`)
that reproduces the exact pose: wall-cling on a tall left wall (top at
y=0) with `wall_slide_speed` downward, sub-pixel penetration into the
wall on x. Pre-fix: player teleports to y≈-23. Post-fix: |dy| < 50 px,
player stays in the world.

### Trace coverage that made the fix takeable

The ad-hoc trace recorder added shortly before this bug (see
`docs/systems/gameplay-trace-recorder.md`) made the diagnosis 10× faster. Two
recorder upgrades from this fix's patch are worth keeping in mind:

- **`nearby_collision` now uses the feature-augmented collision world.**
  The wall the player was clinging to wasn't in `GameWorld.0.blocks`
  (it came from `runtime.features` via `world_with_sandbox_solids`), so
  the trace's nearby-collision view was empty and the wall was
  invisible. The recorder now calls `features::world_with_sandbox_solids`
  the same way `sandbox_update` does.
- **`last_safe_player_pos` is gated by `classify_player_safety`.** The
  pre-fix trace recorded `last_safe_player_pos = (62, -23)` because
  the player was technically `on_ground` after the teleport. The new
  gate refuses to remember any position that the OOB detector would
  reject, and also refuses while the player is taking damage / in
  hitstun / in blink-grace / mid-room-transition.

The shared classifier (`ambition_engine::classify_player_safety`) is
the single source of truth so the trace's OOB detector and the
sandbox's safe-pos gate cannot drift again.

### Takeaway

**A swept hit with `time_of_impact = 0` on an already-overlapping
block is not a landing — it's an existing contact, and the snap
direction has to come from the *shape* of the overlap, not the
direction of `delta`.** When you see an unconditional `pos += block_top
- body.bottom` in collision code, ask: what if the block's top is
hundreds of pixels away from the body? Add the symmetric overlap-shape
guard `resolve_axis(Axis::X)` already had.

### Followup: predicate evolution

The first revision of the side-contact filter required `overlap_x > 0`
(actual penetration). It missed the *exact-edge-touching* case
(`body.left == wall.right` to within float precision), where Parry's
`cast_shapes(stop_at_penetration=true)` still returns `time_of_impact
= 0` and the snap teleports the player to a different position
(`pos.y = wall.top - half_height = 32 - 23 = 9` for the second
reproduction). The next dump (`debug_traces/ambition_gameplay_trace_1777905256-*`)
showed the exact-edge case as `inside solid (ldtk solid)` at `(62, 9)`.

The current predicate (`body_is_side_contact`) keys on the body's
y-range being nested inside the block's y-range — independent of
x-overlap — and catches edge-touching and penetrating side contacts
uniformly. The integration test
`square_arena_wall_cling_full_world_does_not_teleport` in
`crates/ambition_gameplay_core/tests/repro_walls.rs` replays the live
trace pose against the actual square_arena world; the unit test
`body_is_side_contact_classifies_walls_vs_floors` pins the
predicate against floor-landing, top-corner-landing, and ceiling-
hit cases so the side-contact filter cannot silently grow into
legitimate vertical-landing geometry.

When this trace recorder is used to diagnose a future collision-
escape bug, the **CollisionCorrection event** with the offending
`unexplained delta Npx (vel-budget Mpx)` line is the smoking-gun
event to look for. Compare the `before` and `after` x to see which
axis owns the bug, and look for the snap target by computing
`block_top = after.y + half_height` (or `block_bottom = after.y -
half_height` for upward sweeps).

---



## bevy_ecs_ldtk renders IntGrid cells by default, even with no tileset

**Date:** 2026-05-04. **Fixed in:** `ded1dc2`.

### Symptom

Geometry painted in LDtk's IntGrid layer (the `Collision` layer for
`central_hub_main`) appeared **duplicated** in the running game: the
"real" merged block at the correct position, plus what looked like
copies of the leftmost cell pattern repeating horizontally across the
level. The duplicates rendered in the same colors as the IntGrid value
defs (gray for Solid, light blue for OneWay, light purple for
BlinkSoft). Entities (NPCs, doors, loading zones) were *not* duplicated.

### Root cause

`bevy_ecs_ldtk-0.14.0/src/level.rs:557-595`. When an IntGrid layer has
**no tileset configured** (which ours doesn't), the plugin's default
`IntGridRendering::Colorful` mode spawns **a colored tile sprite per
non-zero cell**, using the color from `intGridValues[i].color`. For
`central_hub_main` that was 762 + 152 + 90 = 1004 plugin-owned sprites.

`LdtkWorldBundle` is spawned at the default transform (origin (0,0)),
so the plugin's tilemap renders in **raw LDtk world-pixel space**
(top-left origin, +y down). Our `compose_runtime_area` →
`int_grid_value_to_block` → `spawn_block` path renders in Ambition's
**centered Bevy frame** (`world_to_bevy`: `x = p.x - world.size.x*0.5`).
The two frames disagree by ~half-room-width on x — exactly the visible
horizontal offset.

Entities were unaffected because our `AmbitionLdtkMarkerBundle`
intentionally doesn't include a `Sprite` component, so the
plugin-spawned entity instances are invisible markers only.

### Fix

```rust
// crates/ambition_gameplay_core/src/app.rs
.insert_resource(LdtkSettings {
    level_background: LevelBackground::Nonexistent,
    int_grid_rendering: IntGridRendering::Invisible,  // <-- this
    ..default()
})
```

`Invisible` mode still spawns the `IntGridCell` components the runtime-
spine indexer uses (so `LdtkSolid` still works); it just suppresses the
plugin's per-cell sprite. Our `spawn_block` is now the only thing
rendering IntGrid visuals.

### Takeaway

**Audit every Bevy plugin's defaults before assuming our compose path is
the only render path.** The plugin had a sensible default for *its*
intended use case (paint-and-go IntGrid editor preview), which silently
double-rendered when we used IntGrid as a data layer with our own
visualization. The diagnostic that finally cracked it was reading the
plugin's source directly — neither cargo logs nor the data dump showed
extra sprites in *our* world.blocks, because they weren't ours. Any
future "data is correct, but something is rendering it wrong" symptom
should immediately suspect a plugin's default render path.

GPT-review's coordinate-translation hypothesis (in `docs/gpt-review.md`)
was on the right scent — same coordinate-frame mismatch — but proposed
translating the plugin's root to overlap our render. That would have
worked visually but kept ~1000 redundant sprites under our blocks.
`Invisible` kills the redundant render entirely.

---

## LDtk computes cWid as `ceil(pxWid / gridSize)`, not floor

**Date:** 2026-05-04. **Fixed in:** `56acf3b`.

### Symptom

Migration script (`tools/ldtk_intgrid_migration.py`) painted clean
rectangular IntGrid cells (verified by Python dump). After the user
opened the file in LDtk, every column of cells was **smeared into a
1-cell-per-row staircase** going left-down. LDtk re-saved the smeared
state on Ctrl+S, locking it in.

### Root cause

The migration set `__cWid = pxWid // GRID` (floor division). LDtk
expects `cWid = ceil(pxWid / gridSize)`. For `central_hub_main`
(1900×1024, GRID 16): floor → 118, LDtk → 119. The migration wrote a
7552-element `intGridCsv`. LDtk loaded it and read with **stride 119**
(its expected cWid) instead of 118 (the script's), so column N at row
M moved by `M / 118 * (119 - 118) = M` cells per row — pure stride
slip, exactly diagonal.

### Fix

```python
def cells_for_size(px: int) -> int:
    return (px + GRID - 1) // GRID   # ceil
```

Plus rerun migration from the pre-IntGrid baseline (the smeared file
was already canonicalised by LDtk; you can't fix it by changing the
reader, you have to repaint).

### Takeaway

**When interoperating with an editor that owns the canonical file
format, cross-check at minimum *one* derived field (here:
`__cWid * __cHei == len(intGridCsv)`) against what the editor
actually emits**, not just what the JSON schema says is allowed. The
schema would have accepted either floor or ceil; the editor's behavior
distinguished them.

---

## Greedy row-major rect-merge produces vertical bars on diagonals

**Date:** 2026-05-04. **Fixed in:** `8332349` (replaced earlier
`1739312`).

### Symptom

After landing the IntGrid migration, painted staircase / diagonal cell
patterns in the editor rendered in-game as **stacks of tall thin
vertical bars** instead of stair-stepped tiles.

### Root cause

The first-pass `emit_collision_blocks_from_intgrid` was greedy
row-major with vertical extension: for each unconsumed non-zero cell,
extend right, then extend the resulting rectangle down as long as
every column matched. On a staircase pattern:

```
......#   row 0  (start: width-1 run at col 6)
.....##   row 1  (col 6 still matches → extend down)
....###
...####
..#####
```

The first iteration finds a 1-wide run on row 0 at the rightmost
column, then walks down — every row has that column filled, so the
merge produces a 1×N vertical bar. Each subsequent diagonal step
becomes another 1×(N-k) bar. The staircase visually inverts into a
column of vertical strips.

### Fix

Two-pass merge:
1. **Per-row horizontal coalesce** — collapse adjacent same-value cells
   in each row into runs.
2. **Per-column vertical span-stack** — adjacent rows that produced
   the *same* `[cx, x_end)` span and value get stacked.

Vertical walls of N-wide cells stack into one N×H block. Horizontal
floors are one row from pass 1. Staircases produce per-row runs that
*can't* stack (varying widths), so they stay as the cell mosaic the
editor shows.

### Takeaway

**Greedy rectangle merging biases toward whatever direction it extends
first.** The fix isn't a smarter greedy choice — it's two passes with
strict matching on the second one. Worst-case is still per-cell on
truly irregular shapes; that's the right outcome (faithful to author
intent).

---

## How to add to this file

When you fix a bug that took effort to diagnose, ask yourself: **could
the lesson save the next person time, or is the fix obvious from the
diff?** If the *diagnosis* was the hard part, write it up here.

Template:

```
## One-line title of the lesson

**Date:** YYYY-MM-DD. **Fixed in:** `<commit hash>`.

### Symptom
What the bug looked like from outside.

### Root cause
What was actually wrong, including any non-obvious upstream code or
plugin-default that contributed.

### Fix
The minimum change. Code snippets if short.

### Takeaway
The general rule the next person should pattern-match against.
```

Skip the pretty narrative. The point is grep-ability — somebody
hunting for "duplicate sprite" or "staircase smear" should land on
the right entry.

---

## LDtk LoadingZone target_room must use activeArea id, not level identifier

**Date:** 2026-05-18. **Fixed in:** `d874187`.

### Symptom
Newly authored `hall_of_bosses` level was unreachable after `area create` even though the connecting door existed in `central_hub_main`. Walking into the door did nothing.

### Root cause
`LoadingZone.target_room` must contain the **activeArea** value — a separate level field — not the LDtk level `identifier`. `central_hub_main` and `central_hub_basement` both share `activeArea = "central_hub_complex"`. The `area create` spec used `target_room: central_hub_main` (level identifier) instead of `target_room: central_hub_complex` (activeArea). The Bevy LDtk runtime resolves warp targets by activeArea, so a level-identifier string produced a silent miss and the door was never activated.

### Fix
Use `entity set-field` to patch the generated zone:
```yaml
level_id: hall_of_bosses
edits:
  - target: {iid: "LoadingZone-4366"}
    fields:
      target_room: central_hub_complex
```
In future specs, check `entity query --level <name>` output for the `activeArea` field value — it may differ from the identifier when multiple LDtk levels share a logical room.

### Takeaway
**`target_room` = activeArea, not level identifier.** Whenever a spec writes `target_room:`, verify the destination level's `activeArea` field with `entity query` or by reading the `fieldInstances` in the LDtk JSON. The validator catches this as an "unknown room" error if the activeArea doesn't exist, but the inverse (a valid activeArea for the wrong level) is silent.

---

## ENOSPC during long autonomous runs is usually background cargo output, not a full build disk

**Date:** 2026-06-02. **Context:** mid-run during an 8-hour autonomous TODO sweep.

### Symptom
The Bash tool started returning "the temp filesystem at /tmp/claude-*/.../tasks is full (0MB free)" and dropped command output. `df -h /` reported the virtiofs root `/dev/vda1` at 100% (96G/96G, tens of KB free). It looked like the build disk was exhausted and code work was blocked.

### Root cause
Two separable things were conflated:
1. The **claude task-output files** (`/tmp/claude-*/<session>/tasks/*.output`) accumulate the *full* stdout/stderr of every command — and a backgrounded `cargo test -p ambition_gameplay_core --all-targets` writes a huge log. A few of those fill the small partition backing `/tmp`, after which *any* command's output capture fails with ENOSPC even though the command itself ran.
2. `df` reporting the virtiofs root as 100% is partly host-side (guest `rm` of a 1.8G dir did not move the `df` number), so chasing guest-side frees is a dead end.

Crucially, **cargo incremental builds kept working the whole time** — a `cargo build -p ambition_gameplay_core --lib` finished in ~5s and `cargo test --lib` ran fine. The build dir (`~/ambition-target`, 19G) writes deltas without issue.

### Fix / workaround
- Do **not** `run_in_background` heavy cargo commands; their output logs are what fill `/tmp`.
- Keep captured output tiny: pipe through `| tail -3` / `| grep -E 'test result|^error'`.
- Reclaim space if already wedged: `rm -f /tmp/claude-*/*/tasks/*.output` (these are your own logs).
- Confirm cargo still works with a quick foreground `cargo build --lib` before assuming the disk blocks code work.
- Do **not** `cargo clean` the warm build dir to "free space" — incremental builds keep working and a clean forces a ~10-min full rebuild that just refills the dir.

### Takeaway
**In a long run, ENOSPC is almost always task-output-log accumulation, not the build disk.** Keep command output small, avoid backgrounding heavy builds, and verify cargo still runs before pivoting away from code work.

### Follow-up: when *linking* a binary genuinely fails on a full disk
Later in the same run `df` really was at 100% (~68K free) and `mold` failed: `failed to write to an output file. Disk full?` — incremental `cargo build` of the lib still worked, but **linking a fresh example/test binary** (hundreds of MB) had no room. The fix that worked (and that `df` *did* reflect, ~+7G): remove the rebuildable parts of the target dir, **not** a full `cargo clean`:
```bash
rm -rf /home/joncrall/ambition-target/debug/{examples,incremental}
```
`examples/` holds the large per-example binaries; `incremental/` is a pure cache. Both rebuild on demand and together freed ~7G (target 19G → 12G), enough to link again. Re-run as it refills. (The "don't free space, builds keep working" note above holds for *lib incremental* builds but **not** for linking a fresh binary — that's the one case where clearing the cache is the right move.)

## 2026-06-03: A check that mirrors the thing it verifies can't catch a divergence (two silent-no-op bugs)

Two bugs shipped during the combat-expansion run that local checks *passed* — both because the verification re-stated the same wrong assumption as the code it was meant to guard.

**1. A boss reward keyed on the wrong id; the unit test keyed on the same wrong id.** `boss_signature_gauntlet` (the "defeat a boss, wield its attack" drop) matched `"smirking_behemoth"`, but the runtime calls it with `boss.config.behavior.id`, which is `"smirking_behemoth_boss"` (a `from_data("smirking_behemoth_boss")` profile). So the beam gauntlet *never dropped in-game* — yet `boss_signature_gauntlets_map_to_real_wielded_held_items` asserted `boss_signature_gauntlet("smirking_behemoth") == Some(beam)` and passed, because the test typed the **same wrong literal** the function matched. A literal-vs-literal test is a tautology: it confirms the arm exists, not that any real caller reaches it.
   - Fix: drive the lookups off each boss's REAL `behavior.id`, read from its constructor — `boss_signature_gauntlet(&BossBehaviorProfile::smirking_behemoth_boss().id)` — so a key typo makes the lookup return `None` and the test fails. The guard also counts the mapped drops, so an unmapped new boss is noticed.
   - Rule: when a function is keyed by a string the runtime derives elsewhere (an id, a dialog key, an archetype name), the test must derive the key the **same way the runtime does** (construct the entity, read its id), never hand-type the literal. Otherwise the test and the bug share the typo.

**2. `connect_to` placed reciprocal doors at the source's y, not the destination floor.** Authoring rooms with `area create … connect_to` (no `snap_to_surface`) inserted the hub-side entrance `LoadingZone` at the raw spec `px:[x, 600]` — the *source* room's door height — so three Door zones hung ~280px in mid-air over the hub with no floor to stand on. The rooms were effectively unreachable from the hub; the level loaded fine (headless room count went up), the targets resolved (validate's reachability check passed), and only the heuristic "Door with no walkable surface within 16px below" *warning* flagged it.
   - Fix: `entity move` the doors onto a hub Solid (found via `entity measure` / `door free-spots`), and for new rooms set `snap_to_surface: true` on the `connect_to` entry so the door snaps to the destination floor at authoring time.
   - Rule: a Door-activation `LoadingZone` needs a floor under it; "the target resolves" ≠ "the player can reach it." Treat the mid-air-Door warning as an error, and prefer `snap_to_surface` (or a `door free-spots` slot) over a hand-picked y. Tool follow-up (default-snap for Door activation) is tracked in TODO.md's drop-zone.

Common thread: both passed because the check re-encoded the code's own (wrong) assumption — the test used the same literal; the connect_to placement used the spec's own y. When verifying string-keyed dispatch or spatial placement, derive the expected value from an **independent** source (the runtime's real id; the actual collision grid), not from the same input the code already trusted.

## 2026-06-07: The 3D "kaleidoscope" cube-menu rabbit hole — four GUI-only bugs, fixed only once I diffed the working demo and wrote headless pinning tests

**Date:** 2026-06-07. **Context:** a single session that ballooned far past estimate. The lunex_kaleidoscope (OoT-style 3D cube) pause/inventory menu had a cascade of regressions after an alpha-fade feature + a "click fix" + a de-vendor. I burned hours **speculating and shipping plausible-but-wrong fixes I couldn't see the result of** (no GUI/DISPLAY in my env — the user runtime-tests). The user had to redirect me to the proven reference *twice* before I used it. Every bug only became tractable once I (a) diffed against the known-good version and (b) wrote a HEADLESS test that reproduced the exact break. Four distinct root causes:

**1. `AlphaMode::Blend` disables depth-write → coplanar UI layers z-fight (flicker as the cube rotates).** A "fade the menu in/out" feature set *every* cube material to `AlphaMode::Blend`. In Bevy's `StandardMaterial`, Blend renders in the transparent phase with **depth-write off**, so the per-face depth bands stop being resolved by the GPU depth test and instead flip via an unstable back-to-front sort — lines/icons/scrollbar flicker. My first "fix" was blanket `Opaque` when open → that turned **text glyphs and item icons into solid squares**, because their textures are mostly transparent and Opaque draws the transparent texels as the base-colour box.
   - Right fix (the proven scheme): **per-element** — solid planes (panels/lines/corners/scrollbar) → `Opaque` (write depth, populate the buffer); textured planes (`base_color_texture` = glyph atlas / icons) → `Blend` (they depth-*test* against the opaque background even though they don't write). Use Blend for everything only *during* the fade transition. Discriminate with `mat.base_color_texture.is_some()`.
   - Rule: **never blanket an alpha mode across a layered UI.** Opaque is for solid fills; transparent textures must stay Blend/Mask or they render as boxes. Coplanar same-depth surfaces z-fight regardless — give distinct things distinct depth bands (the scrollbar needed its own band; it was coplanar with the panel it overlays).

**2. The cursor highlight was a system-ORDERING race, not a logic bug.** Highlight (recolor + selection corners) is driven by the host writing `MenuVisualState` from its focus cursor, then two `Changed<MenuVisualState>` readers in the lib. These readers and `rebuild_cube_faces` were registered as **unordered** `Update` systems with no edge to the writer. So a republish-triggered rebuild could respawn controls with `focused:false` *after* the writer flipped the flag, and the `Changed` readers could run *before* the writer → no highlight, ever, for both keyboard and mouse. The isolated in-order logic test PASSED (logic was fine); only a test that reproduced the *unordered* wiring + a mid-frame rebuild failed.
   - Right fix: an explicit `KaleidoscopeFocusVisuals` system set, `.after(rebuild_cube_faces)`, with the host writer `.before` it. Pin it with a test that drives a republish on the focus frame and asserts the highlight survives (and fails on the un-ordered wiring).
   - Rule: when "the logic is obviously correct but nothing shows," suspect **scheduling/ordering and change-detection**, not the logic. A `Changed<T>` reader that can run before its writer, or a rebuild that resets state after a write, is invisible to a single-threaded in-order unit test — write the test against the *real registration order* (or a deliberately adverse one).

**3. Mouse click never worked: Bevy's compound `Pointer<Click>` needs press+release on the SAME entity.** The cube despawns+respawns controls on hover-driven republishes, so the press entity was gone by release and `Pointer<Click>` silently never fired (hover/`Pointer<Move>` worked, which masked that picking itself was fine). 
   - Right fix (matching the demo's deterministic approach): **entity-independent dispatch** — capture the control's action at `Pointer<Press>`, dispatch it on release from the *stored action*, not the release entity. Survives any rebuild between press and release.
   - Rule: don't rely on `Pointer<Click>` for controls that can be rebuilt mid-interaction; arm on press, dispatch on release from stored data. Pin with a test that despawns+respawns the controls between press and release and asserts the action still fires.

**4. The de-vendor was the right call but reframed the safety net.** The menu lib lived in a vendored git submodule (its own cargo workspace, only to satisfy `bevy_lunex`'s `.workspace=true` inheritance). Every host-state feature had to thread a "neutral channel" across the submodule edge + do a gitlink dance. Moving it to an in-repo crate (`crates/ambition_inventory_ui`, `bevy_lunex` from crates.io — it resolved clean against bevy 0.18) removed that tax. But it also **deleted the submodule's git history from the repo**, so the only proven reference left was a stale worktree checkout (`.worktrees/…/submodules/ambition_inventory_ui`, commit `cbc4ae8`) and a separate full demo clone (`/home/joncrall/code/ambition_inventory_ui`). Keep those reference checkouts findable.

### Takeaway (the meta-lesson that cost the most time)
**When you can't see the result (headless env, user runtime-tests), do NOT iterate by speculation.** Two moves would have saved hours, every time:
1. **Diff against the known-good version first.** The proven demo (`/home/joncrall/code/ambition_inventory_ui`) and the pre-regression worktree (`cbc4ae8`) had the working render scheme, the working highlight semantics, and the working *manual* click path. Each fix became obvious the moment I diffed instead of guessed. The user explicitly said "check the submodule" twice before I did.
2. **Reproduce the bug in a headless test before fixing.** All four bugs were headless-testable (material alpha mode by element; focus-flag survival under rebuild ordering; click dispatch surviving a press→rebuild→release). A failing test that mirrors the *real* wiring is the only way to know a GUI fix landed without the GUI. "Compiles + existing tests pass" proved nothing here — the existing click tests fired a synthetic `Pointer<Click>` that doesn't fire in the real app, so they were green while the feature was broken (see the 2026-06-03 "a check that mirrors the thing it verifies" entry — same trap).

## 2026-06-07: "Renders for one frame, then vanishes" — a second bevy_ui backend reused the cube's `AmbitionMenuPage` marker, and the cube's rebuild despawned its body

**Date:** 2026-06-07. **Context:** building a flat bevy_ui "Grid" menu as a SECOND presentation of the same `MenuPageModel` the 3D "kaleidoscope" cube renders (one content model, two interchangeable backends). The grid's panel + tab bar showed, but the BODY content (item cells / System rows) was invisible — and only *flashed* in for a frame during navigation. Hours.

### Symptom
The centered panel + tab bar rendered fine and persisted; the body was empty. Headless logs PROVED the renderer was correct: `[grid-render] rendered=System nodes=15 find_ok=true`, and nav/click/dispatch all worked. At idle the republish ran exactly ONCE (3 prints over several seconds) — so NOT an every-frame rebuild. Yet the body was empty, and tabbing made the correct content flash for ~1 frame.

### What it was NOT (the empirical bisection that finally worked)
Many rounds went into the wrong layer; the decisive moves were cheap, blunt tests:
- **Picking** (couldn't click anything): the build lacked the `bevy/ui_picking` feature — bevy_ui nodes generate NO pointer hits without it (the cube has its own custom 3D backend, so it worked regardless). Add the feature. (Separately: a click emits a `Pointer<Press>` for EVERY entity under the cursor — the tab PLUS its scrim/window ancestors — and the handler reset the capture to `None` on each, so a later ancestor press wiped the tab before release. Only SET the capture on an interactive hit; never clobber it on a non-interactive one.)
- **Color**: the model's `page.background` is (near-)transparent because the cube's 3D face is itself opaque; the flat renderer needs its OWN solid panel. Forced opaque → still empty → ruled color out.
- **Position**: dropped a bright opaque debug box into the body + bright debug colors on scrim/panel. Magenta scrim + cyan panel + tabs all rendered, body EMPTY, and the debug box ALSO only flashed → not a containing-block/position bug.
- **Lifecycle gap**: the republish despawned the old root immediately but spawned the new tree via a *deferred* `commands.queue(world.commands())` closure (a later flush) — a real 1-frame gap, fixed by spawning on the same command buffer. But content STILL vanished, and idle = one republish → not an every-frame rebuild.

### Root cause (the user's "feels like an ordering issue" was right)
The engine renderer tagged the flat menu's body with `AmbitionMenuPage` — the SAME ECS marker the cube's faces carry. The cube's `rebuild_cube_faces` does `for e in faces: Query<Entity, With<AmbitionMenuPage>> { commands.entity(e).despawn() }` whenever the shared `ActiveMenuPages` changes. The grid had UN-GATED `republish_kaleidoscope_pages` to run for BOTH backends (so both render the same model), so the cube's rebuild fired and **despawned the grid's `AmbitionMenuPage`-tagged body and all its content children** — while the panel + tab bar (no `AmbitionMenuPage`) survived. Exactly the symptom.

### Fix
The flat body uses only its OWN marker (`BevyUiMenuBody`), NOT `AmbitionMenuPage`, so the cube's despawn-by-marker query can't reap it. (Belt-and-suspenders not taken: gate the cube's republish/rebuild off in Grid mode.)

### Takeaway
- **Two presentations of one content model must NOT share the ECS *marker* components that either backend's despawn/rebuild systems query.** A `for e in Query<With<SharedMarker>> { despawn }` in backend A silently reaps backend B's entities that reuse `SharedMarker`. Give each presentation distinct markers; reserve shared components for genuinely shared interactive *data*, and audit every `despawn`/`remove_*` keyed on them.
- **"Renders correctly for one frame, then vanishes" is a DESPAWN smell, not a render bug.** When content is provably built (logs/tests) and isn't rebuilt every frame (idle is quiet), stop staring at the renderer — grep every `despawn`/`remove_*`/`Query<…, With<X>>` for the components your entity carries and find who reaps it.
- **Blunt empirical bisection beats code-reading when you can't see the result.** Bright opaque debug colors per layer + a single debug box + an idle-log frequency check isolated render-vs-color-vs-position-vs-lifecycle-vs-ownership in ~3 rounds, after many rounds of reasoning. Reach for the crayon earlier. (Related: the de-vendor/headless saga two entries up — same "diff/repro-empirically, don't speculate" lesson.)

## 2026-06-22: Re-keying a map (Stage 1a) silently broke boss music — a missed `.get()` call site no test covered

**Date:** 2026-06-22. **Context:** boss entity-local refactor (`docs/planning/boss-entity-local-refactor.md`). Stage 1a re-keyed `BossEncounterRegistry.encounters` from the shared archetype id to the per-entity runtime id (so a gauntlet's two same-archetype bosses get independent state). Found instantly while writing the R3 "test-first" safety net.

### What it was
`update_boss_encounters`'s music-LIFETIME pass (the "is any boss in an active phase? if not, clear the boss-music request" guard) still looked the encounter up by the **archetype** `encounter_id` (3rd tuple field), but the map is now keyed by the **runtime** id (1st field). For any LDtk/spawned boss whose runtime id ≠ archetype id (i.e. all of them), `encounters.get(archetype_id)` returned `None` → the guard took its "no active boss" branch → it **cleared `desired_track` the same frame the wake had set it**. Boss music never played. The damage/phase `.get()` sites were updated in Stage 1a; this read-only music site was missed.

### Why no test caught it
There was no headless test asserting boss music plays during a fight — the exact coverage gap. The new `boss_lifecycle::boss_music_plays_during_the_fight` pin failed on the first run and pointed straight at it. Fix: look up by the runtime id (1st tuple field).

### Takeaway
- **When you re-key a collection, grep EVERY `.get`/`.get_mut`/`.entry`/`.contains_key` on it — not just the mutators you came to change.** A read-only lookup with the old key fails silently (returns `None`/default), so it won't panic or fail to compile; it just quietly does the wrong thing.
- **Test-first before a big-bang earns its keep immediately.** The "write the safety net for the behaviors you can't see" step (here: boss music, save-cleared, reward drop) surfaced a pre-existing regression the moment it ran — before any of the risky R3 work began.

## 2026-06-23: "Elegant vs cheap" refactor + tests keeping dead code alive

**Date:** 2026-06-23. **Context:** boss entity-local refactor R5 (express the cut-rope fight via generic encounter pieces) + Jon's review ("are we slimming as we unify? was avoiding what you did in R5 elegant or cheap?").

### Two transferable lessons
- **A "script on top of the bespoke system" is cheap, not elegant.** The first R5 cut built a generic `EncounterScript` engine but bolted it onto the existing `arena.rs` — a gate that fired `ForceKill` while the anvil physics + steering stayed bespoke. That NET-ADDS (engine) without removing what it was meant to replace. The elegant version makes the bespoke logic GENERIC (`CommandMoveTo`→`CommandedMove`, `DropHazard`→`FallingHazard`) and DELETES the bespoke physics, so the cut-rope becomes data + a future puzzle reuses the mechanic. Test for elegance: *did the bespoke code get deleted, and is the new code reusable by a second caller?* If you only added, you cheaped it.
- **When a refactor obsoletes a system, delete it AND its tests in the SAME stage.** R3 made the registry-owned `BossEncounterState` live-state machine obsolete (the entity became the source of truth) but left it because `roster.rs` tests still exercised it — ~440 lines used ONLY by their own tests. Tests of dead code masquerade as coverage and silently keep the corpse compiling. After replacing a system, grep its methods for NON-TEST callers; if there are none, the system + its tests go together. (The replacement's behavior was already covered by the new mechanism's own tests.)

### Takeaway
Slimming is part of "done", not a follow-up: a unification that only adds is half-finished. Each refactor stage should leave the net production surface flat-or-smaller and the new code reused by ≥1 real caller, with the obsoleted code + its tests deleted in the same commit.

---

**Date:** 2026-07-03. **Context:** post-architecture-arc test/CI hardening — a warning-cleanup commit (`889c859d`) removed `mut` from `let snapshot = resolve_follow_camera_snapshot(...)` in `render/camera.rs` on a "does not need to be mutable" lint. The full-workspace verification (`cargo check --workspace`) then failed to compile render with E0594 (`cannot assign to snapshot.center_world`).

### Transferable lesson — a compiler warning can be FEATURE-CONFIG-SPECIFIC
- The "unused mut" was real ONLY without the `portal_render` feature: under it, a `#[cfg(feature = "portal_render")]` block reassigns `snapshot.center_world`/`.rotation_radians`, so `mut` is REQUIRED. `cargo check -p ambition_render` (default) and `-p ambition_content --all-features` (which compiles `portal_render` OUT) both showed the warning; neither exercised the config that NEEDS the `mut`. Blind-removing it broke the default render + workspace build.
- **Rule:** before "fixing" an `unused_mut` / `dead_code` / `unused_import` lint near a `#[cfg(feature = ...)]` block, check whether a gated path uses it under another config. Prefer `#[cfg_attr(not(feature = "X"), allow(unused_mut))]` (keep the `mut`, silence the lint only where genuinely unused) over deleting. Verify EVERY warning fix with `cargo check --workspace --all-targets`, not a single-crate/default-feature check.
- **Meta-lesson (validated the E39 recommendation):** the per-crate CI flow (`-p ambition_app`, `-p X --lib`) and default-feature checks are a feature-config BLIND SPOT — they missed both this regression AND the rotted leaf-crate tests (vfx `frame_down`, architecture_boundaries). A `cargo test --workspace` gate catches all three classes.

### Takeaway
Warnings are config-relative, not absolute. A "clean" fix under one feature set can be a compile break under another; only `--workspace --all-targets` sees them all. When in doubt, silence a config-local lint with a scoped `cfg_attr(allow)` rather than changing code.
