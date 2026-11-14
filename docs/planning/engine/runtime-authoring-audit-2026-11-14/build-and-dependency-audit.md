# Static build and dependency audit

Snapshot: `34f03976b639`, supplied archive dated 2026-11-14.

## Scope and method

This pass inspected all root workspace manifests and selected source paths. It used `scripts/agent_query.py` and the generated crate/ECS packets to find owners. It then checked implementation source. It did not run Cargo, compile code, execute tests, change source, or measure elapsed build time.

The root manifest lists **112 packages**: 92 under `crates`, 18 under `game`, one workspace-policy package under `tests`, and one extension fixture under `fixtures`. The generated declared-dependency index lists 111 because it omits `ambition_extension_fixture_modules`. The generated resolved graph includes all 112. This is a denominator difference to retain in the audit; it does not invalidate the inventory as a navigation aid.

The bounded census is in `build_census.json`. It counts `.rs` files under each root member's `src` directory: **2,179 files, 964,837 raw lines, 906,805 nonblank lines**. These figures include comments and inline test modules. Separate member `tests` directories contain **475 Rust files and 182,231 raw lines**. These are source-size figures, not production executable size or compile-cost measurements. The count excludes independent fixture workspaces, authoring submodules, and nonmember tools.

The dependency census has two static views:

1. Declared normal internal edges, including optional and target-specific declarations. Reverse reach through this graph is a potential rebuild reach, not the work a particular Cargo invocation does.
2. Nonoptional, target-independent normal internal edges only. This deliberately ignores optional/default-feature expansion. Its closure gives a lower bound that proves some dependencies cannot be removed through feature choice alone. It is not a feature-resolved consumer measurement.

The snapshot's source uses **Bevy 0.19.1**. Older 0.18.1 notes do not describe its current dependency selection. The generated resolved graph reports 916 packages in the workspace resolution. That number includes the workspace's unified dependency graph and must not be presented as a minimal game's compile closure.

## Main assessment

The strongest route to faster game authoring is already present: build an engine host once, then load authored data and executable behavior through its prepared-content and extension interfaces. The remaining work is to make this the normal product path and to extend its expressive surface. Further crate extraction can improve engine development, but it cannot by itself give game authors a compiler-free workflow.

There are also concrete sources of avoidable rebuilding. The application build stamp watches source throughout the workspace and emits a changing timestamp into the application. The usual launcher runs Cargo, by default in the release profile. The portable module builder has a watch list tied to one game despite accepting another module package. These can obstruct the intended iteration path even before any deep refactor.

## Existing improvements to preserve

### The content compiler and authoring SDK are distinct from the engine host

`ambition_extension_sdk` has no dependencies. `ambition_content_pack` has no Bevy dependency and keeps schema ownership outside its generic compiler. The independent `fixtures/content_builder` workspace depends only on the value vocabulary and pack compiler. These are real boundaries, not aliases to the full game facade.

Evidence:

- `crates/ambition_extension_sdk/Cargo.toml:1-12`.
- `crates/ambition_content_pack/Cargo.toml:1-19`.
- `fixtures/content_builder/Cargo.toml:16-31`.

The module crates use the SDK and selected pure domain ports. For example, the target-independent mandatory internal closure of `ambition_content_modules` contains only six packages, including itself. The closure includes no ECS or host package. The module also directly uses `bevy_math`, so describe this as ECS-free rather than claiming that every Bevy-named package is absent.

Evidence: `game/ambition_content_modules/Cargo.toml:1-20` and the census's selected closure.

### Disk content does not normally become a Rust input

`embed_pack` emits only a `build.rs` watch and returns when `static_content` is absent. It embeds declared sources only for static builds. This directly supports RON and manifest edits without rebuilding the provider. Do not undo this while changing content packaging.

Evidence: `crates/ambition_content_pack/src/embed.rs:26-65`; `game/ambition_content/build.rs:7-10`; `game/ambition_content/Cargo.toml:25-31`.

### Native module code can already load without relinking the running engine

The extension builder compiles the selected module as a WASM `cdylib` into the module target directory. The runtime has a `wasm_modules` feature, and the app selects it. Boss specials are declared through extension modules rather than production native technique systems. Their older native systems are test-only reference code.

Evidence:

- `scripts/regen/build_extension_modules.sh:29-48`.
- `crates/ambition_platformer2d_runtime/Cargo.toml`, `wasm_modules` feature.
- `game/ambition_app/Cargo.toml`, facade dependency with `wasm_modules`.
- `game/ambition_content/src/bosses/specials/mod.rs:1-8,50-95`.

This meets a different contract from scripting with no Rust toolchain. A Rust-to-WASM edit still compiles the module. It does, however, establish the engine boundary on which a source-language backend can build.

### Engine build hygiene has already been addressed

The development profile uses unoptimized workspace code, no debug information, and incremental compilation. Third-party dependencies remain optimized. Build scripts and proc macros have their own low-optimization override. Linux uses Clang plus mold. App and actor libraries are `rlib` only, with target packaging requesting `cdylib` explicitly. The app's integration tests are aggregated to avoid repeated linking of one executable per test file.

Evidence: root `Cargo.toml:173-207`; `.cargo/config.toml:1-29`; `game/ambition_app/Cargo.toml:8-25`; `crates/ambition_platformer2d_actor_monolith/Cargo.toml:9-13`.

A recommendation to add incremental builds, a faster linker, basic crate splitting, separate content files, or a portable module SDK would repeat work that already exists.

### Some feature boundaries are real

The facade's host dependency disables host default features; renderer selection forwards rendering deliberately. The facade's rollback backend, LDtk adapter, portal presentation, and several other capabilities use optional dependencies. The runtime profile registry also supports omission of installed systems and resources. Keep these two kinds of optionality distinct.

Evidence: `crates/ambition_platformer2d/Cargo.toml:105-126,243-285`; `crates/ambition_platformer2d_runtime/src/profile.rs:16-25,49-91`.

## Findings

### B01 — The app build stamp creates a workspace-wide invalidation path

**Priority: high. Evidence: direct source. Timing impact: not measured here.**

`game/ambition_app/build.rs` iterates every package under `crates/` and `game/`. It emits `cargo:rerun-if-changed` for each package's `src`, `build.rs`, and `Cargo.toml`. It also watches Git `HEAD`, `index`, `packed-refs`, and the current branch ref. When it runs without an explicit build ID, it includes the current Unix time in `AMBITION_BUILD_LABEL`. The app consumes that environment value with `env!` in its menu.

Evidence:

- Workspace source watches: `game/ambition_app/build.rs:18-33`.
- Git watches: `game/ambition_app/build.rs:34-48`.
- Changing label: `game/ambition_app/build.rs:49-57`.
- Compiled consumer: `game/ambition_app/src/menu/build_label.rs:62`.

This adds an invalidation path outside the selected app dependency graph. An edit to a package that the app does not use can rerun the app's build script; the timestamp then changes a value compiled into the app. Updating the Git index can do the same. A concrete candidate is the optional relativity implementation: the default facade intentionally omits it, but the stamp still watches its source directory.

The build identity is useful. Its current implementation conflicts with the fast-iteration goal. Separate these identities:

- engine binary build identity, set when the engine is actually built;
- active content generation identity;
- active executable-module identity;
- working-tree state, if desired as a runtime developer display.

Keep release stamping explicit through `AMBITION_BUILD_ID` or release packaging. For local use, do not make a timestamp or unrelated workspace source a reason to recompile the app. A runtime sidecar can carry current workspace provenance without becoming a Rust input. A source commit and content revision also tell an author more than a changing clock stamp.

**Completion evidence:** warm the existing app build; edit a source file outside that app's resolved dependency graph; stage an unrelated text file; then inspect actual Cargo work and process invocation. Neither action should compile or link the app. This is a one-time acceptance experiment, not a new permanent source-text guard.

### B02 — The normal launch command is still a build command

**Priority: high for author workflow. Evidence: direct source.**

`run_game.sh` defaults to release and ends with `cargo run`. `--dev` chooses the cheaper engine-edit profile, but does not remove Cargo from the launch path. The script exposes a useful `--print-plan` surface with the selected binary path, runtime arguments, runtime environment, and build environment; a direct-executable path can reuse that work.

Evidence:

- Default profile: `run_game.sh:129`.
- User-facing choices: `run_game.sh:250-280`.
- Cargo selection: `run_game.sh:552-560`.
- Plan data: `run_game.sh:658-741`.
- Invocation: `run_game.sh:766-770`.

The desired content-author command should launch an already built engine and report the engine/profile version it found. It should not silently build because a native source tree is dirty. If the required host is absent, it should describe the missing installed runtime and give the explicit engine-install/build action. A developer can still choose the existing engine-build lane.

Do not solve this simply by making every content author run unoptimized Rust. A fixed optimized host can provide fast gameplay and fast content iteration at the same time. The build profile is a property of the selected engine binary; it need not be paid again for a content edit.

**Completion evidence:** launch, edit content, reload, restart the scenario, and package a local content project with Cargo unavailable on `PATH`, using a prebuilt host and authoring tools. The behavior must come from the edited generation, not a stale embedded fallback.

### B03 — The module path still has two workflow traps

**Priority: medium; one concrete defect and one boundary limitation.**

First, `build_extension_modules.sh` accepts a package argument at line 29 but hardcodes its watch roots to `game/ambition_content_modules` plus selected ports at lines 51-54. A `--watch ambition_demo_mary_o_modules` or Smash module invocation does not watch the selected game's source directory. Adding more module customers without fixing this discovery will make the authoring workflow appear to ignore edits.

Second, game providers still compile native module crates as ordinary dependencies and register their static definitions. For example, `ambition_content` directly depends on `ambition_content_modules`; its boss plugin calls `ambition_content_modules::modules()`. The loaded WASM route can replace those linked modules in a running process, but running Cargo after editing the shared Rust module source still sees a real native dependency and can rebuild the provider/host. The new app build stamp adds another route.

Evidence:

- `scripts/regen/build_extension_modules.sh:29,38-39,51-64`.
- `game/ambition_content/Cargo.toml:49-62`.
- `game/ambition_content/src/bosses/specials/mod.rs:82-93`.
- `game/ambition_fighter_specials/Cargo.toml:11-16`.

Derive the watch inputs from the selected module package and its local source dependencies, or accept an explicit complete input manifest. Keep the already-built host running during Rust module iteration. For a binary-only authoring distribution, make module discovery and admission data-driven; compiled fallback algorithms should not be required to introduce a project module. Preserve the native implementations as a supported engine-development option where they serve a real customer.

**Completion evidence:** edit one Mary-O module and one Smash module under their selected watcher; each produces a new artifact and host acknowledgment without a host build. Restart through the reuse-only launcher and verify the same module identity is active.

### B04 — Runtime profiles do not remove most gameplay dependencies

**Priority: medium for native engine development and minimal distributions; lower than compiler-free content workflow.**

The profile registry explicitly promises absence of installed plugins/resources, not absence of compiled crates. Its `omit` implementation calls `PluginGroupBuilder::disable` on types that are already linked into the runtime. The facade, runtime, session, construction, and actor paths retain mandatory edges to inventory, bosses, dialogue, cutscenes, held items, and other domains.

Evidence:

- `crates/ambition_platformer2d_runtime/src/profile.rs:16-21,49-91`.
- `crates/ambition_platformer2d/Cargo.toml:183-240,251-295`.
- `crates/ambition_platformer2d_runtime/Cargo.toml:10-115`.
- `fixtures/headless_profile/Cargo.toml:29-32` selects no optional facade features.

The nonoptional target-independent manifest traversal reaches **70 internal packages including the facade**. This does not count third-party packages, optional defaults, build dependencies, or target-specific additions. It is a lower bound, not a newly measured featureless closure. The stored `minimal_game` baseline reports 73 packages on November 10, but that fixture explicitly requests the renderer and is a different subject.

Feature-gating one facade reexport cannot remove a package that its lower dependencies also require. Work from a complete capability cut: find the domain's authoritative state, construction/restore participation, schedule installation, observations, and dependency parents. Put the adapter that joins it to the actor at the composition layer. Only then make the capability optional through the whole selected profile.

The first success should be one useful external fixture: a body, world collision, input intent, and observable motion, without dialogue, boss, inventory, presentation, or rollback backends. Preserve the existing profile behavior tests; upgrade their compile-absence claim as each domain leaves. Do not start with a full feature Cartesian product.

### B05 — Volatile policy and implementation still live below many consumers

**Priority: medium. Evidence: source plus static graph.**

The current actor kernel is not the only broad compilation unit. The largest sampled source packages are:

| Package | Raw member `src` lines | Declared normal reverse reach |
| --- | ---: | ---: |
| `ambition_platformer2d_core` | 73,014 | 82 |
| `ambition_combat` | 69,769 | 49 |
| `ambition_render` | 56,072 | 17 |
| `ambition_platformer2d_actor_monolith` | 54,640 | 26 |
| `ambition_characters` | 51,021 | 58 |
| `ambition_platformer2d_shared_tangle` | 32,268 | 67 |

The line figures include tests and comments. Reverse reach includes optional and target-specific declarations and gives no elapsed-time estimate. These figures identify sampling priorities only. Splitting a file because it is large does not establish a better boundary.

Two concrete examples show a useful boundary question.

**Developer keyboard policy.** `shared_tangle/developer_hotkeys.rs` contains the stable `DeveloperAction` message, physical key chords, a default binding table, matching logic, and the input-reader plugin. The manifest adds `bevy_input_focus` explicitly for this file. A physical developer shortcut change therefore edits a foundation with broad potential reach. Keep the action vocabulary low if its consumers need it. Move the physical binding data, matching, and plugin to the host/input owner. Read the bindings as developer settings. This preserves one input path without making a keybinding an engine rebuild.

Evidence: `crates/ambition_platformer2d_shared_tangle/src/developer_hotkeys.rs:12-38,40-176,210-257`; `crates/ambition_platformer2d_shared_tangle/Cargo.toml:19-29`; `crates/ambition_platformer2d_host/src/lib.rs:37-40`. A targeted scan found `KeyCode` and `ButtonInput` only in that file within `shared_tangle`.

**Camera implementation.** `shared_tangle/camera_ease.rs` carries per-view state, the easing function, defaults, and request types. Simulation consumers can legitimately need a feedback request contract, but that does not require every lower domain to compile the presentation algorithm. Separate the request/value contract from the camera implementation and its tuning. Do not move the simulation's settled movement facts into presentation.

Evidence: `crates/ambition_platformer2d_shared_tangle/src/camera_ease.rs:1-75,89-125`; `crates/ambition_platformer2d_runtime/src/sim_core_resources.rs:170-178,252-282`; `crates/ambition_platformer2d_actor_monolith/src/body/damage/mod.rs:109`.

The same question applies to `core`: the public contract, three motion models, collision execution, snapshots, and movement tuning share a package. Its `movement` subtree accounts for 36,708 raw lines and its `frame` subtree for 11,574. A value-contract/implementation split is justified only where actual consumers need the values but not the implementation. Preserve the one movement entry point and shared body authority. Do not create independent motion stacks merely to make the graph smaller.

Evidence: `crates/ambition_platformer2d_core/src/lib.rs:7-41,65-151`; `crates/ambition_platformer2d_core/src/movement/mod.rs:1-19,24-117`.

### B06 — The validator's first build still reaches runtime domains

**Priority: medium for SDK/bootstrap; not proof of a slow per-edit validator.**

The generic pack compiler is small. The composed engine schema registry depends on the runtime capability crates that own schemas. `engine_schemas_without` constructs all schema registrations and then filters omitted capabilities. The CLI depends on that registry, several domain crates, and the LDtk adapter. Its mandatory internal closure lower bound is 42 packages including itself; the registry's is 40.

Evidence: `crates/ambition_engine_schemas/Cargo.toml:20-42`; `crates/ambition_engine_schemas/src/lib.rs:20-71`; `crates/ambition_content_cli/Cargo.toml:12-29`; census.

This does not mean content validation recompiles these crates for every RON edit. Reusing a prebuilt validator already avoids that cost. The near-term product solution is to ship the validator with the selected runtime and give it the same installed-schema identity. The longer-term build solution is to separate a domain's value/schema preparation code from its Bevy systems where the cut is coherent. Keep schema ownership with the domain; do not introduce an unrelated central parser or duplicate validators just to remove dependencies.

The CLI and runtime should share validation semantics, but need not share every implementation package. Compare preparation results from the existing fixtures when moving that boundary.

### B07 — A featureless facade still requests UI API surface

**Priority: medium; source fact with unresolved target cost.**

Both the facade and the actor kernel request Bevy's `ui_api` without a presentation feature gate. The actor manifest explains that nonpresentation modules still mention UI types. This is a concrete architectural dependency even when no renderer plugin is installed.

Evidence: `crates/ambition_platformer2d/Cargo.toml:297-302`; `crates/ambition_platformer2d_actor_monolith/Cargo.toml:329-349`.

The actor manifest also claims specific transitive Bevy windowing implications. This pass did not resolve an independent consumer or inspect the external Bevy feature source, so it does not repeat that claim as a measured result. Start with the code that names UI types and move UI implementation to presentation. Then measure the actual selected Bevy packages and features in the independent headless fixture. Avoid a blanket replacement of `bevy` imports with subcrates: useful domain boundaries matter more than import spelling.

## What the existing measurements say, and do not say

`docs/planning/engine/extension-iteration-evidence.md` already records relevant results. Treat these as historical evidence, not timings from this audit:

- Rust WASM module edits: 1.36 seconds for the first edit after a cold module build and 0.34 seconds for a warm constant edit on an agent machine.
- Comparable native content constant: 7.05 seconds to relink the app, plus restart.
- Disk data edit: about 0.4–0.5 seconds; embedded form 5.8–7.2 seconds.
- An October 9 engine-edit/capture experiment: about 67 seconds total, with two builds comprising 91% of the time.
- The live rules reload recorder measured publication acknowledgment; its own table states that it did not observe the changed gameplay behavior.

The evidence page is careful about feature sets, no-op controls, cache state, selected machines, and what each experiment witnessed. Preserve those distinctions. An unchanged executable hash does not prove no linker invocation. A successful `cargo check` does not measure the compiled and linked game loop. A plugin-absence test does not prove compile absence. A source line count does not rank rustc code generation costs.

For the current snapshot, the following remain unknown: actual edit/build/link latency on the user's machine, per-unit rustc phase cost, procedural-call runtime overhead at representative populations, exact independently resolved featureless package closure, and current cross-target module execution support.

The incremental setting has a scoped override. `.cargo/config.toml` enables it globally. The release, ship, and profiling branch in `run_game.sh` sets `CARGO_INCREMENTAL=0` unless the caller supplied a value. Thus the planning statement that optimized incremental builds are disabled describes the supported launcher, not every bare Cargo invocation. This is no basis to recommend another toggle.

## Prescriptive work sequence

### Package B-A: make ordinary authoring use an existing host

1. Add the reuse-only launch path using the current launch-plan owner.
2. Report engine build/profile and active content/module generations.
3. Remove workspace-wide build-stamp triggers from the authoring path.
4. Fix selected-module watch discovery.
5. Publish prebuilt engine, validator, and inspection tools as one versioned tool bundle.

Acceptance is a complete data edit cycle with Cargo unavailable, plus admission of a new module artifact with host building disabled. The existing Rust module source still needs its own compiler; a script backend adds procedural source edits without that toolchain. Retain the explicit engine-build command. This package supplies immediate workflow value without waiting for every optional-capability refactor.

### Package B-B: remove rebuild triggers from high-frequency policy

Select concrete edit classes: developer binding, camera easing parameter, move data, and one special behavior. Put plain configuration in authored data. Put behavior in the admitted extension path where its required ports already exist. Move host/presentation implementation upward when lower consumers need only a value contract.

Measure a real before/after edit loop for each cut. Do not declare success because a new crate exists or because an allowlist shrank.

### Package B-C: make one minimal native profile compile only its needs

Use `fixtures/headless_profile` and its independent lockfile. Trace all parents that pull in the first selected domain, including construction and restore. Move the cross-domain integration above the domain owners, then gate installation and dependencies together. Prove a useful body/world run and domain absence. Proceed to the next domain only after the first cut works.

This work improves engine iteration, engine bootstrap, package footprint, and maintainability. It is a separate contract from the content author's zero-host-build loop.

### Package B-D: measure the engine lane after the product boundary is in place

Use existing `scripts/measure/compile_cost.py` and M0 conventions. Preserve exact toolchain, target, features, profile, linker, flags, filesystem, cache state, and machine load. Record both the no-op and changed build and the first observed behavior. Separate library checking, code generation, linker work, content preparation, module load, and scenario restoration.

Choose further compiler tactics from the dominant measured phase. Do not promise a percentage improvement from splitting `core`, replacing generics, changing linkers that are already optimized, or moving functions out of `lib.rs`. There is no evidence from this static pass that any one of those would dominate the current user loop.

## Recommended priority for the combined audit

Make the fixed engine executable and its authoring protocol the product. Use the existing pack compiler, prepared generations, extension SDK, domain ports, and module host as its foundation. Correct the concrete launch/build/watch defects. Add a script backend that obeys the same state and authority contract. Continue native crate and feature improvements as measured engine work; do not make completion of the native decomposition a prerequisite for game authors to stop rebuilding the host.
