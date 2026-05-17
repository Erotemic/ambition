Here’s a candidate design memo you can hand to another agent. It is intentionally framed as a proposal to evaluate, not a decision.

---

# Candidate Design Plan: Ambition Crate Restructure

## Status

**Draft / review candidate only.**

This document proposes a possible long-term crate/module restructuring direction for `git@github.com:Erotemic/ambition.git`. It should not be treated as accepted architecture. The next agent should review the repository, compile locally, measure build times, and decide whether this direction is useful, excessive, premature, or missing something.

The current workspace has two crates, `crates/ambition_engine` and `crates/ambition_sandbox`, using Cargo resolver 2.  The sandbox crate currently depends on Bevy 0.18.1 plus a broad set of runtime, authoring, audio, physics, inspector, UI, LDtk, input, RON/Serde, and graph dependencies.  The uploaded `cargo tree -e features` also shows the resulting dependency graph is large, including Bevy proc-macro machinery and broad Bevy/render/devtool feature activation. 

## Review Goal

Evaluate whether crate restructuring can improve:

```text
1. Compile times and incremental build behavior.
2. Headless / CI / replay / possible RL simulation paths.
3. Clear ownership of simulation vs presentation.
4. LDtk authoring/runtime maintainability.
5. Long-term ability to add story/content crates without copying sandbox glue.
```

This proposal assumes the project should remain Bevy-native where that is useful. It does **not** propose returning to a pure engine abstraction. Existing project notes already say the engine may depend on Bevy, and reusable mechanics should live in the engine while sandbox/story crates supply content, data, presentation, and composition. 

## Current Architectural Pressure

The likely issue is not simply “too few crates.” The issue is that `ambition_sandbox` appears to mix several axes:

```text
visible app assembly
headless app entry
gameplay simulation orchestration
LDtk parse/validate/compile/runtime-spine bridge
audio generation/playback
VFX/presentation
debug UI / inspector tooling
physics debris / Avian integration
sandbox-specific feature labs
content/world/story experiments
```

ADR 0012 already identifies a related problem: simulation systems currently call audio, particle, and physics-debris APIs directly, preventing clean headless execution and blocking CI/RL style simulation.  It proposes typed events as the boundary: simulation emits events, presentation consumes them, and headless builds omit presentation subscribers. 

The current `ldtk_world.rs` also appears to combine multiple responsibilities: LDtk JSON structures, validation, runtime-room composition, `bevy_ecs_ldtk` registration, hot reload, runtime-spine indexing, and collision migration scaffolding.  It already contains comments about moving collision authority from a JSON-derived path toward plugin-spawned typed ECS components once parity is verified. 

## Candidate North Star

Do **not** immediately split into many crates. First make the boundaries boring inside the existing crate. Then extract crates only where the dependency boundary is proven useful.

A possible long-run shape:

```text
crates/
  ambition_engine/
    reusable Bevy-native mechanics:
    movement, collision vocabulary, abilities, combat, actors,
    hazards, interactables, pickups, debug semantics, state-machine primitives

  ambition_sim/
    headless/gameplay runtime:
    GameWorld, RoomSet, runtime state, ControlFrame stepping,
    typed gameplay events, transition graph, replay/RL hooks

  ambition_ldtk/
    mostly pure authoring/data pipeline:
    LDtk JSON structs, validation, conversion to engine/sim data,
    snapshot tests, no Bevy rendering/window/audio dependencies if feasible

  ambition_bevy_ldtk/
    Bevy + bevy_ecs_ldtk adapter:
    entity registration, runtime-spine indexing, hot reload,
    plugin-spawned entity promotion, ECS-side collision authority migration

  ambition_bevy_presentation/
    visible app presentation:
    rendering, sprites, camera, HUD, VFX, audio playback, inspector/devtools,
    physics debris visualization

  ambition_sandbox/
    thin playable shell:
    sandbox content, app composition, feature labs, current vertical-slice experiments

  future ambition_story_* crates/
    campaign/story/world-specific content and progression
```

This should be reviewed critically. It may be too many crates for the current project. The practical near-term direction may be only:

```text
ambition_engine
ambition_sandbox
```

with better modules and features, followed later by:

```text
ambition_sim
ambition_ldtk
```

only when the seams are clear.

## Candidate Phase Plan

### Phase 0: Measure Before Moving Code

Before restructuring, run:

```bash
cargo build --timings -p ambition_sandbox
cargo check -p ambition_sandbox
cargo check -p ambition_sandbox --no-default-features
cargo tree -e features -p ambition_sandbox
cargo tree -d
```

Review questions:

```text
Which crates dominate first clean build?
Which crates dominate incremental rebuild?
Which dependencies are pulled only for devtools/audio/UI/LDtk/physics?
Can headless compile without visible/presentation dependencies?
Are duplicate crate versions meaningful?
```

Do not assume crate splitting helps unless measurements show a bottleneck that crate or feature boundaries can isolate.

### Phase 1: Add Feature Boundaries Before Crate Boundaries

Make heavy sandbox dependencies optional where practical. Current sandbox dependencies include `avian2d`, `bevy_yarnspinner`, `bevy_material_ui`, `bevy_asset_loader`, `bevy_ecs_ldtk`, `bevy-inspector-egui`, `bevy_kira_audio`, `fundsp`, and others.  A first candidate feature layout:

```toml
[features]
default = ["visible"]

visible = [
    "ldtk_runtime",
    "input",
    "audio",
    "ui",
    "physics_debris",
]

headless = []

dev_tools = [
    "visible",
    "dep:bevy-inspector-egui",
    "bevy/file_watcher",
    # maybe "bevy/dynamic_linking" locally only, if desired
]

ldtk_runtime = [
    "dep:bevy_ecs_ldtk",
    "dep:bevy_asset_loader",
]

audio = [
    "dep:bevy_kira_audio",
    "dep:fundsp",
]

ui = [
    "dep:bevy_material_ui",
    "dep:bevy_yarnspinner",
]

physics_debris = [
    "dep:avian2d",
]

input = [
    "dep:leafwing-input-manager",
]
```

Candidate rule: **do not make `headless` depend on presentation/audio/window/devtools unless absolutely required.**

This phase should preserve existing behavior under default features. The first milestone is simply that feature groups exist and heavy dependencies are no longer always mandatory.

### Phase 2: Finish the Sim/Presentation Event Boundary

Implement or continue ADR 0012 before extracting `ambition_sim`. ADR 0012 already proposes the slices: audio events, VFX events, physics-debris events, setup split, and app-builder split. 

Candidate event vocabulary:

```rust
pub enum SfxEvent {
    Jump { pos: Vec2 },
    Dash { pos: Vec2 },
    Hit { pos: Vec2 },
    Pickup { pos: Vec2 },
}

pub enum VfxEvent {
    Burst { pos: Vec2, kind: BurstKind },
    Dust { pos: Vec2 },
    Blink { from: Vec2, to: Vec2 },
    Impact { pos: Vec2, normal: Vec2 },
}

pub enum DebrisBurstEvent {
    BreakableDestroyed { pos: Vec2, impulse: Vec2 },
    EnemyDefeated { pos: Vec2 },
}
```

Review points:

```text
Are event payload coordinates room-local, world-local, or active-area-local?
Are all spatial payloads clearly documented?
Should AMBITION_REVIEW(spatial) mark any conversion-sensitive event?
Can tests tick the sim without AudioPlugin/Sprite/Window/Inspector?
```

This phase is likely higher priority than new crates. It creates the seam that later crate extraction can follow.

### Phase 3: Split Large Sandbox Modules Internally

Before creating new crates, split big modules into smaller internal modules.

Candidate LDtk module split:

```text
src/ldtk/
  mod.rs
  json.rs              # LDtkProject, LdtkLevel, raw JSON structs
  validate.rs          # validation report and authoring checks
  compile.rs           # LDtkSurfaceSpec -> ae::Block / ae::RoomObject / RoomSpec
  runtime_index.rs     # LdtkRuntimeIndex, area bounds, LevelSet sync
  runtime_spine.rs     # plugin-spawned entity projection/indexing
  hot_reload.rs        # file polling/apply/reject state
  bevy_plugin.rs       # bevy_ecs_ldtk registration and plugin setup
```

This is motivated by `ldtk_world.rs` currently owning LDtk parsing, typed surface specs, runtime index/spine, hot reload, entity registration, and Bevy integration together.  

Candidate sandbox gameplay module split:

```text
src/features/
  mod.rs
  hazards.rs
  enemies.rs
  bosses.rs
  breakables.rs
  pickups.rs
  npc.rs
  events.rs
```

Review point: only split where the file is already too broad or where it separates dependencies. Avoid moving code just to create a pretty tree.

### Phase 4: Consider Extracting `ambition_sim`

Extract only after the sim/presentation boundary is real and tests can run headless.

Candidate ownership:

```text
ambition_sim owns:
  GameWorld or successor
  RoomSet / RoomSpec / RoomLink / LoadingZone
  SandboxRuntime successor, or smaller runtime resources/components
  ControlFrame stepping boundary
  typed simulation events
  gameplay schedule / systems
  headless app builder
  observation structs for replay/RL/CI
```

Candidate dependencies:

```toml
[dependencies]
ambition_engine = { path = "../ambition_engine" }
bevy = { version = "0.18.1", default-features = false, features = [
    "default_app",
    "bevy_state",
    "bevy_time",
    # only what the sim genuinely needs
] }
serde = { version = "1", features = ["derive"], optional = true }
petgraph = "0.8"
```

Review questions:

```text
Can ambition_sim compile without bevy_render, bevy_window, audio, inspector, egui, Kira, or Avian?
Does ambition_sim expose enough for visible app composition without making fields public everywhere?
Does extracting this crate reduce rebuilds, or merely add friction?
Would ambition_sandbox still be able to prototype quickly?
```

### Phase 5: Consider Extracting LDtk Authoring

A useful split may be:

```text
ambition_ldtk:
  raw LDtk JSON parse
  validation
  compile to RoomSet / RoomSpec / ae::Block / ae::RoomObject
  snapshot tests

ambition_bevy_ldtk:
  bevy_ecs_ldtk plugin registration
  hot reload
  runtime-spine and solid-index projection
  Bevy ECS component promotion
```

This directly follows the existing migration direction in `ldtk_world.rs`, where LDtk-authored solids are still compiled into the runtime collision world through the JSON adapter, while plugin-spawned `Solid` entities also receive typed components so the ECS-side path can eventually become collision authority.  

Review questions:

```text
Can LDtk validation tests run without Bevy app boot?
Can the pure compiler be snapshot-tested with insta?
Can the Bevy adapter prove parity between JSON-derived collision and plugin-spawned collision?
Is a separate ambition_bevy_ldtk crate worth it, or should this remain modules for now?
```

### Phase 6: Presentation/Devtools Extraction Only If Needed

Do not rush this. `ambition_bevy_presentation`, `ambition_audio`, or `ambition_devtools` are useful only if multiple apps/story crates need them or compile-time measurements show clear benefits.

Candidate rule:

```text
Extract presentation/devtools only after:
  1. it has a stable API,
  2. it is reused by more than one binary/crate, or
  3. it demonstrably improves build isolation.
```

## Candidate Immediate PR Sequence

A conservative sequence another agent could evaluate:

```text
PR 1: Add feature groups and optional dependencies in ambition_sandbox.
      Preserve current default behavior.

PR 2: Gate devtools/audio/UI/physics modules behind features.
      Add check commands for visible, dev_tools, and headless.

PR 3: Implement the ADR 0012 audio-event slice.
      Simulation emits SfxEvent; visible plugin plays audio.

PR 4: Implement VFX and debris event slices.
      Simulation no longer directly calls presentation spawn helpers.

PR 5: Split setup/app-builder into honest simulation vs presentation helpers.
      Headless path ticks the real simulation loop.

PR 6: Split ldtk_world.rs into internal modules.
      No crate extraction yet.

PR 7: Add parity tests/snapshots for LDtk compile output and runtime-spine output.

PR 8: Re-evaluate whether ambition_sim extraction is now low-risk.
```

## Non-Goals

This plan should **not**:

```text
1. Force a big-bang crate split.
2. Rename crates for aesthetics before boundaries are proven.
3. Make ambition_engine Bevy-independent as a goal.
4. Replace the custom player controller with Avian.
5. Adopt bevy_rl immediately.
6. Adopt bevy_dev_tools immediately.
7. Move story/content systems before the vertical slice is clearer.
8. Optimize compile time by making the architecture harder to work in.
```

## Success Criteria

The restructuring is useful only if it produces measurable improvements:

```text
cargo check -p ambition_sandbox                 # still works for visible/default path
cargo check -p ambition_sandbox --features dev_tools
cargo check -p ambition_sandbox --no-default-features --features headless
cargo test -p ambition_engine
cargo test -p ambition_sandbox
cargo build --timings -p ambition_sandbox       # compare before/after
```

Functional success criteria:

```text
1. Visible game still runs.
2. Headless sim can tick without display/audio/window/devtool plugins.
3. LDtk authoring validation can run without full visible app setup.
4. Debug/dev tools remain available when explicitly enabled.
5. The sandbox remains pleasant for rapid prototyping.
```

## Key Questions for the Reviewing Agent

Please evaluate these before writing code:

```text
1. Is feature-gating enough, or is a new crate actually needed now?
2. Which dependency group is the biggest compile-time offender in practice?
3. Does the event boundary in ADR 0012 need to land before any crate split?
4. Should RoomSet/RoomSpec belong in engine, sim, sandbox, or LDtk?
5. Should LDtk compilation output engine-native data or sim-native data?
6. Is ambition_sim a stable enough concept yet, or should it remain sandbox modules?
7. Can devtools be isolated without hurting the normal dev loop?
8. Does dynamic linking belong in Cargo features, local .cargo config, or docs only?
9. What is the smallest PR that improves the architecture without destabilizing gameplay?
```

## Provisional Recommendation for Review

The most likely good path is:

```text
Feature-gate first.
Finish sim/presentation events second.
Split large modules third.
Only then extract ambition_sim / ambition_ldtk if the seams are boring.
```

The reviewing agent should treat this as a hypothesis, not as marching orders.
