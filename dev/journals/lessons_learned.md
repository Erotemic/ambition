# Lessons learned

This journal records unexpected errors encountered while iterating on the Ambition sandbox, especially places where an overlay or generated build script looked reasonable but failed in a real local/device test. The goal is to make future LLM-generated patches less likely to repeat the same mistakes.

## 2026-05-11: Movement split broke extension-trait scope and `Self: Sized` assumptions

A movement refactor split [`crates/ambition_engine/src/movement.rs`](../../crates/ambition_engine/src/movement.rs) into child modules and changed AABB sweeps from returning only `time_of_impact` to returning an `AabbSweepHit` with Parry's contact normal. The patch looked mechanically reasonable but failed immediately when the user ran the suggested checks.

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

[`ambition_engine::probe_ledge_grab`](../../crates/ambition_engine/src/ledge_grab.rs) checked that the platform on top of a candidate ledge was clear of *other* solid blocks (good), but did not check that the climbed-onto position lay inside the world rect. The mob_lab arena has a ceiling tile at y≈1; a wall-clinging player whose head touched that ceiling could pass `probe_ledge_grab`'s clearance test, get snapped to a `climb_target.y = -23`, and end up above the world.

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

The sandbox used to ship per-actor collision predicates `blocked` / `blocked_y` in `crates/ambition_sandbox/src/features/util.rs` that diverged from `ambition_engine::movement::sweep_player_y` in a load-bearing way: OneWay platforms always blocked vertical motion regardless of approach direction. Symptoms downstream:

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
grep -B1 -A1 '"identifier": "central_hub_main"' crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
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

`build_character_sprite` in `crates/ambition_sandbox/src/character_sprites/sheets.rs` (historical path) sizes the rendered quad as `collision.max() * collision_scale`. The visible body inside that quad is determined by the generator's frame layout: how much of `frame_height × frame_width` is opaque body pixels.

Robot/Goblin sheets use `collision_scale: 2.1` because their generator leaves big transparent margins (the silhouette occupies maybe 60% of the frame). Copying `2.1` for a generator like `absurd_general` whose `body_pixel_bbox` covers ~95% of the frame produces a sprite ~2× too tall — the General towers above the player.

Rule of thumb when adding a new sheet:

```
collision_scale ≈ 1 / (body_pixel_bbox_h / frame_h)
```

So a sheet whose body fills the frame ends up near `1.0`, while one with lots of margin ends up near `2.0`. Read `body_metrics.body_pixel_bbox` and `body_metrics.frame_height` from the generator's `<target>_spritesheet.yaml` instead of guessing.

`feet_anchor_y` should match the generator's `body_metrics.feet_anchor_norm.y` directly — that field already encodes the offset from frame center to feet in the same convention Bevy's `Anchor` uses (negative = below center).

## 2026-05-10: Bevy `add_systems` tuple chains cap at 20 systems

Adding `upgrade_npc_sprites` to the big presentation tuple in [crates/ambition_sandbox/src/app/plugins.rs](../../crates/ambition_sandbox/src/app/plugins.rs) (the chain that runs after `sandbox_update`) pushed it from 20 to 21 systems and produced this error:

```text
error[E0599]: the method `chain` exists for tuple `(..., ..., ..., …)`, but its trait bounds were not satisfied
   --> crates/ambition_sandbox/src/app/plugins.rs:355:18
355 |                 .chain()
    |                 ^^^^^ method cannot be called due to unsatisfied trait bounds
```

`IntoSystemConfigs` (and the `chain()` extension) is implemented for tuples up to **20** elements in Bevy 0.18. There is no compile-time message about the cap; you only see the trait-bound failure on `.chain()`. Several earlier comments in this file said "16-system tuple budget" — that was right for older Bevy versions and is now stale.

The established pattern in [plugins.rs](../../crates/ambition_sandbox/src/app/plugins.rs) is **not** to subdivide the chain (which would silently change ordering) but to pull the new system out into its own `add_systems(Update, sys.after(prev))` call. `sync_health_overlays`, `map_menu_pointer_dismiss`, and `update_quest_panel` are already wired this way; `upgrade_npc_sprites` joins them.

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

A later Android usability overlay replaced `crates/ambition_sandbox/src/lib.rs` from a source snapshot that did not contain the Android shared-library entry point. The APK still built and installed, but launch failed with:

```text
UnsatisfiedLinkError: dlopen failed: cannot locate symbol "android_main" referenced by libambition_sandbox.so
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

Desktop still enters through `src/main.rs`; Android packages the library as `libambition_sandbox.so` and needs Bevy's `#[bevy_main]` macro to generate the Android boilerplate.

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
[`crates/ambition_sandbox/tests/crouch_stability.rs`](../../crates/ambition_sandbox/tests/crouch_stability.rs)
(held Down for 30 frames must stay Crouching with per-frame `pos.y`
delta < 5 px) and
`fold_held_down_without_edge_flag_does_not_fire_down_pressed` (historical path: `crates/ambition_sandbox/src/mobile_input.rs`)
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
[`morph_ball_does_not_fire_from_control_frame_alone`](../../crates/ambition_sandbox/src/body_mode.rs)
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

Two coordinated changes in `crates/ambition_sandbox/src/music/director.rs`:

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
[`crates/ambition_sandbox/src/music/tests.rs`](../../crates/ambition_sandbox/src/music/tests.rs)
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
reproduction). The next dump (`debug_traces/ambition_trace_1777905256-*`)
showed the exact-edge case as `inside solid (ldtk solid)` at `(62, 9)`.

The current predicate (`body_is_side_contact`) keys on the body's
y-range being nested inside the block's y-range — independent of
x-overlap — and catches edge-touching and penetrating side contacts
uniformly. The integration test
`square_arena_wall_cling_full_world_does_not_teleport` in
`crates/ambition_sandbox/tests/repro_walls.rs` replays the live
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
// crates/ambition_sandbox/src/app.rs
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
