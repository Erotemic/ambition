# Journal index

Use this index when debugging a symptom. Search `dev/journals/` when in doubt; this file is a routing aid, not a complete replacement for grep.

## Movement / collision

| Symptom | Read |
|---|---|
| Wall-cling or y-sweep teleports the player to a wall top/far edge | [`movement-edge-touch-y-sweep-lessons-2026-05-11.md`](movement-edge-touch-y-sweep-lessons-2026-05-11.md), [`lessons_learned.md`](lessons_learned.md) |
| Movement module split causes compile failures, lost extension-trait scope, or `Self: Sized` surprises | [`movement-refactor-lessons-2026-05-11.md`](movement-refactor-lessons-2026-05-11.md), [`rust-module-split-import-visibility-lessons-2026-05-11.md`](rust-module-split-import-visibility-lessons-2026-05-11.md) |
| Movement-snap probes pass local clearance but escape world bounds | [`lessons_learned.md`](lessons_learned.md) |
| Enemies/NPCs need player-like collision semantics | [`lessons_learned.md`](lessons_learned.md) |

## LDtk / editor interop

| Symptom | Read |
|---|---|
| `LoadingZone.target_room` is valid JSON but fails doctor/runtime lookup | [`lessons_learned.md`](lessons_learned.md) |
| `sandbox.ldtk` needs mutation or editor roundtrip repair | [`lessons_learned.md`](lessons_learned.md) |
| IntGrid cells render unexpectedly, cWid/cHei are wrong, or rect merge output is surprising | [`lessons_learned.md`](lessons_learned.md) |

## Bevy schedule ordering

| Symptom | Read |
|---|---|
| Migrating a global resource read to a per-entity-component read silently delivers stale data because the mirror system runs after the migrated reader | [`per-player-input-mid-chain-vs-late-chain-2026-05-20.md`](per-player-input-mid-chain-vs-late-chain-2026-05-20.md) |

## Rust refactors / module boundaries

| Symptom | Read |
|---|---|
| Child module extraction loses local imports, helper visibility, attributes, or tests | [`rust-module-split-import-visibility-lessons-2026-05-11.md`](rust-module-split-import-visibility-lessons-2026-05-11.md), [`rl-sim-refactor-lessons-2026-05-11.md`](rl-sim-refactor-lessons-2026-05-11.md), [`music-director-refactor-lessons-2026-05-11.md`](music-director-refactor-lessons-2026-05-11.md) |
| Content validation refactor changes error-mode counts unexpectedly | [`content-validation-refactor-lessons-2026-05-11.md`](content-validation-refactor-lessons-2026-05-11.md) |
| Boss profile validation misses event-bus import closure | [`boss-profile-event-bus-import-closure-2026-05-11.md`](boss-profile-event-bus-import-closure-2026-05-11.md) |
| Collapsing a workspace crate into another — sed patterns, bin-vs-lib `crate::` resolution, dep migration | [`engine-crate-collapse-2026-05-28.md`](engine-crate-collapse-2026-05-28.md) |
| Refactoring every `&mut ae::Player` sandbox path to `&mut PlayerClustersMut`; reborrow / mut-aliasing gotchas; the final `ae::Player` struct deletion | [`player-cluster-native-push-2026-05-28.md`](player-cluster-native-push-2026-05-28.md) |
| Porting deterministic sim/integration code to a new data shape (struct→ECS components) safely+fast; lockstep parity test as the verification harness; "build the safety net first, then port boldly" | [`enemyruntime-ecs-dissolution-2026-06-02.md`](enemyruntime-ecs-dissolution-2026-06-02.md) |

## Build / commands / platform

| Symptom | Read |
|---|---|
| `cargo test` command grammar rejects multiple filters or unexpected args | [`cargo-test-command-lessons-2026-05-11.md`](cargo-test-command-lessons-2026-05-11.md) |
| Android install/launch/build failure moves between Gradle, manifest, native library, assets, or logcat layers | [`lessons_learned.md`](lessons_learned.md) |
| Overlay patch clobbers platform entrypoints or stale feature/event APIs | [`lessons_learned.md`](lessons_learned.md), [`parallax-overlay-module-graph-clobber-2026-05-11.md`](parallax-overlay-module-graph-clobber-2026-05-11.md) |
| Headless sandbox build still pulls in winit even after moving render features off the base bevy dep (`ui_api` → `default_app` → `bevy_window` → `bevy_winit` transitive closure) | [`bevy-headless-feature-graph-2026-05-20.md`](bevy-headless-feature-graph-2026-05-20.md) |

## UI / input / Bevy

| Symptom | Read |
|---|---|
| Bevy UI mutable query conflict, `ParamSet` confusion, visibility mutation, or text sync collisions | [`lessons_learned.md`](lessons_learned.md) |
| Menu controls confuse held axes, edge presses, or semantic frames | [`lessons_learned.md`](lessons_learned.md) |
| Bevy `add_systems` tuple chain unexpectedly fails on `.chain()` | [`lessons_learned.md`](lessons_learned.md) |

## Audio / rendering labs

| Symptom | Read |
|---|---|
| Adaptive music/director plays two sources at once or module split breaks helper visibility | [`music-director-refactor-lessons-2026-05-11.md`](music-director-refactor-lessons-2026-05-11.md), [`lessons_learned.md`](lessons_learned.md) |
| Parallax minimal app or visibility scaffold fails due to asset server/module graph/run command assumptions | [`parallax-minimal-app-asset-server-2026-05-12.md`](parallax-minimal-app-asset-server-2026-05-12.md), [`parallax-visibility-and-run-command-2026-05-11.md`](parallax-visibility-and-run-command-2026-05-11.md) |

## Adding a journal entry

Prefer a short standalone file when the lesson came from a focused incident. Add a row here using the symptom words a future debugger would search for.
