# Plugin Refactor Action Plan

This document turns the plugin refactor roadmap into an execution plan. It is intentionally more prescriptive than the surrounding design docs. Use it when creating branches, assigning agent tasks, or deciding whether a refactor stage is complete enough to continue.

The plan assumes a staged shotgun refactor: break the system along seams that should become permanent, avoid long-lived compatibility layers, and fix compile/test failures forward into the target architecture. Each stage should leave behind code, tests, docs, or module boundaries that remain useful in the final architecture.

## Execution rules

1. Prefer architecture-revealing breakage over bridge layers.
2. Do not do broad replacement of large hotspot files such as `portal.rs`, `app/plugins.rs`, `world_flow.rs`, or large content runtime files.
3. Make each branch move one architectural seam at a time.
4. Keep compatibility re-exports only when they are intended to be the final public API.
5. Add guardrail tests before or during the stage that needs them, not after the refactor is already large.
6. Every stage must end with concrete validation commands and a short note about what became easier to express.
7. When a stage reveals unexpected coupling, document it in this folder before papering it over.

## Branch strategy

Use one documentation branch and a sequence of implementation branches.

```bash
git switch -c docs/pluginized-platformer-runtime-roadmap
```

Use this branch to check in the planning folder, generated ECS inventory snapshots, stale-doc notes, and ADR updates.

Implementation branches should be short-lived and mergeable independently:

```bash
git switch -c refactor/runtime-lifecycle-boundary
git switch -c refactor/plugin-registration-ownership
git switch -c refactor/portal-plugin-shell
git switch -c refactor/split-portal-module
git switch -c refactor/portal-generic-runtime-extraction
git switch -c refactor/optional-portal-plugin
git switch -c refactor/ambition-content-boundary
git switch -c refactor/platformer-runtime-crate-extraction
```

The branch names are suggestions. The important rule is that each branch should have one primary architectural objective.

## Stage 0: Baseline and documentation alignment

Goal: make the intended architecture explicit before changing code.

### Steps

1. Apply this planning docs overlay.
2. Generate and check in baseline inventory snapshots.
3. Add a short index of stale or conflicting docs.
4. Decide which build personas are officially supported during the refactor.
5. Create an ADR that accepts the pluginized platformer-runtime direction.

### Commands

```bash
cd ~/code/ambition
python tools/ecs_inventory.py
mkdir -p docs/generated
cp target/ambition_ecs_inventory.json docs/generated/ambition_ecs_inventory.baseline.json
cp target/ambition_ecs_inventory.md docs/generated/ambition_ecs_inventory.baseline.md
```

If `ecs_inventory.py` supports test inclusion in the local checkout, also run the with-tests variant and check it in:

```bash
python tools/ecs_inventory.py --include-tests \
  --json target/ambition_ecs_inventory.with_tests.json \
  --markdown target/ambition_ecs_inventory.with_tests.md
cp target/ambition_ecs_inventory.with_tests.json docs/generated/ambition_ecs_inventory.with_tests.baseline.json
cp target/ambition_ecs_inventory.with_tests.md docs/generated/ambition_ecs_inventory.with_tests.baseline.md
```

### Deliverables

- `docs/planning/plugin_refactor/` checked in.
- `docs/generated/ambition_ecs_inventory.baseline.{json,md}` checked in.
- ADR for pluginized runtime direction.
- Stale-doc review notes.

### Validation

```bash
find docs/planning/plugin_refactor -maxdepth 1 -type f -print | sort
python - <<'PY'
from pathlib import Path
root = Path('docs/planning/plugin_refactor')
missing = []
for path in root.glob('*.md'):
    text = path.read_text()
    import re
    for match in re.finditer(r'\[[^\]]+\]\(([^)]+)\)', text):
        link = match.group(1)
        if link.startswith(('http:', 'https:', '#')):
            continue
        target = (path.parent / link.split('#', 1)[0]).resolve()
        if link and not target.exists():
            missing.append((path, link))
if missing:
    raise SystemExit('\n'.join(f'{p}: {l}' for p, l in missing))
print('plugin_refactor markdown links ok')
PY
```

### Stop condition

Stop Stage 0 when the team can point to one folder that explains the target plugin topology, crate topology, portal design, build personas, risks, and action plan.

## Stage 1: Architecture guardrails before movement

Goal: add tests that make the desired dependency direction and lifecycle policy visible.

### Steps

1. Add `tests/architecture_boundaries.rs` or an equivalent crate-local test module.
2. Add a forbidden-import check for the future proto-runtime module path, even before the module exists.
3. Add a raw-spawn policy check for room-feature spawn modules.
4. Add an allowlist file if necessary so the test remains readable.
5. Add documentation for how to update the architecture tests when a boundary intentionally changes.

### Initial guardrail checks

The first guardrails should check for these policies:

```text
platformer_runtime/ must not import:
  crate::content
  crate::intro
  crate::boss_encounter
  crate::assets::sandbox_assets
  crate::music
  crate::quest
  crate::portal
  crate::app
  crate::presentation
  crate::dev

content/features/ecs/spawn*.rs should avoid raw commands.spawn for room-authored entities.
room-authored entities should use spawn lifecycle helpers once those helpers exist.
app/plugins.rs should shrink over time rather than accumulate new subsystem registration.
```

The first version can be intentionally simple: scan source files for forbidden strings. A crude test is acceptable because it gives agents fast feedback and makes dependency direction explicit.

### Validation

```bash
cargo test -p ambition_sandbox architecture_boundaries
```

### Stop condition

Stop when architecture tests exist and fail with actionable messages if a future patch imports game content from the proto-runtime or adds obvious new lifecycle-policy violations.

## Stage 2: Create the proto-runtime lifecycle boundary

Goal: introduce a same-crate proto-runtime module that behaves like a future crate, starting with lifecycle because it already fixed real bugs.

### Steps

1. Create `crates/ambition_sandbox/src/platformer_runtime/`.
2. Add `platformer_runtime/mod.rs` and `platformer_runtime/prelude.rs`.
3. Add `platformer_runtime/lifecycle/`.
4. Move or re-export lifecycle markers into the new module.
5. Add spawn extension helpers.
6. Update room-local spawn sites to use the helpers.
7. Update architecture tests to enforce the new path.

### Target files

```text
crates/ambition_sandbox/src/platformer_runtime/
  mod.rs
  prelude.rs
  lifecycle/
    mod.rs
    markers.rs
    spawn_ext.rs
    cleanup.rs
```

### Target API

```rust
pub trait SpawnScopedExt {
    fn spawn_room_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;
    fn spawn_run_scoped<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;
    fn spawn_persistent<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;
}
```

The names can change, but the goal should not: spawn sites should declare lifecycle policy instead of remembering marker components manually.

### Migration targets

Start with room-authored and room-local dynamic entities:

```text
content/features/ecs/spawn_static.rs
item_pickup.rs
portal.rs or mechanics/portal/* once split
app/feedback.rs if projectile-like room-local entities are spawned there
```

### Expected breakage

- Imports of `RoomScopedEntity`.
- Tests that refer to the old marker path.
- Spawn helpers that currently return `EntityCommands` in a shape incompatible with extension methods.

Fix forward. Do not leave two competing lifecycle APIs.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox architecture_boundaries
cargo test -p ambition_sandbox room_scoped
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox gravity_room_reachability
```

### Stop condition

Stop when new room-local spawn sites have an obvious lifecycle helper to use, and the previous item/gravity/portal-gun leak class is guarded by tests.

## Stage 3: Introduce schedule vocabulary for plugin boundaries

Goal: define stable sets that plugin code can register into without depending on concrete function names in other modules.

### Steps

1. Add or consolidate platformer runtime schedule sets.
2. Add subsystem-specific sets only when the subsystem owns those sets.
3. Replace cross-subsystem `.before(function)` / `.after(function)` edges with set-level ordering when possible.
4. Document the scheduling rule: cross-plugin ordering uses sets/messages; intra-plugin ordering can use concrete system chains.

### Target API

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlatformerSet {
    Input,
    Intent,
    Simulation,
    Physics,
    Interaction,
    RoomTransition,
    LifecycleCleanup,
    PresentationSync,
}
```

Portal should later define its own sets:

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortalSet {
    InputAdapter,
    Gun,
    Shot,
    Placement,
    Transit,
    GateProgression,
    Cleanup,
    Presentation,
}
```

### Expected breakage

- Systems that were ordered by direct function references.
- Plugins that assumed order from registration position in `app/plugins.rs`.

### Validation

```bash
cargo test -p ambition_sandbox plugin_minimal_app
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when subsystem plugins can register into stable sets without importing unrelated subsystem internals just to order themselves.

## Stage 4: Move plugin registration ownership out of app/plugins.rs

Goal: make subsystems own their own Bevy plugin registration.

### Steps

1. Identify direct registrations in `app/plugins.rs` by subsystem.
2. Create module-local plugin structs for the first few candidates.
3. Move system/resource/message registration into those plugin impls.
4. Leave `app/plugins.rs` as a composer that installs plugins.
5. Add an architecture test that warns when new subsystem internals are registered directly from `app/plugins.rs`.

### First candidates

```text
Portal plugin shell
Held item plugin
Gravity plugin
Projectile plugin or ability plugin group
Room lifecycle plugin
```

### Target shape

```rust
app.add_plugins((
    PlatformerLifecyclePlugin,
    HeldItemPlugin,
    GravityPlugin,
    PortalPlugin,
));
```

Instead of:

```rust
app.add_systems(Update, crate::portal::some_internal_system);
app.add_systems(Update, crate::item_pickup::some_internal_system);
```

### Expected breakage

- Missing resource initialization that used to happen in `app/plugins.rs`.
- Schedule ordering assumptions.
- Test setup code that manually registers old systems.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox plugin_minimal_app
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when at least portal, held items, and gravity can be installed as plugins, even if their internals are still in old files.

## Stage 5: Portal plugin shell

Goal: make portal registration plugin-owned without splitting or renaming the entire portal module yet.

### Steps

1. Convert `src/portal.rs` to `src/portal/mod.rs` with `git mv`.
2. Add `src/portal/plugin.rs`.
3. Add `src/portal/schedule.rs` or `sets.rs`.
4. Define `PortalPlugin`, plus smaller subplugins if practical.
5. Move portal system/resource/message registration out of `app/plugins.rs` and presentation registration out of presentation hubs where appropriate.
6. Keep existing public portal paths working through the facade.

### Commands

```bash
git mv crates/ambition_sandbox/src/portal.rs crates/ambition_sandbox/src/portal/mod.rs
mkdir -p crates/ambition_sandbox/src/portal
```

If the `mkdir` fails because the directory exists after `git mv`, that is fine.

### Target plugin split

```rust
pub struct PortalPlugin;
pub struct PortalCorePlugin;
pub struct PortalGunPlugin;
pub struct PortalTransitPlugin;
pub struct PortalGatePlugin;
pub struct PortalPresentationPlugin;
pub struct PortalDebugPlugin;
```

The first patch can install all subplugins from `PortalPlugin`, but the names should describe final ownership.

### Expected breakage

- Module path conflicts from converting `portal.rs` into a folder.
- Ordering relative to player movement, ground items, gravity, and presentation sync.
- Test setup that imported concrete portal systems directly.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox portal_bridge_reachability
cargo test -p ambition_sandbox plugin_minimal_app
```

### Stop condition

Stop when the app installs portal through plugin structs and no unrelated module has to register portal internals manually.

## Stage 6: Extract generic helpers out of portal

Goal: make non-portal systems stop importing `crate::portal` for generic platformer operations.

### Steps

1. Move solid-world raycast helpers out of portal.
2. Move generic body transit math out of portal if it is not portal-specific.
3. Move actor/body orientation helpers out of portal if they describe gravity/upright behavior rather than portal behavior.
4. Move gravity zone ownership out of portal.
5. Update all callers to import from the new generic module.

### Target moves

```text
portal::raycast_solids -> platformer_runtime::world_query::raycast_solids
portal::ray_aabb -> platformer_runtime::world_query::ray_aabb
portal_transform_velocity -> platformer_runtime::transit::rotate_velocity_between_normals
ActorRoll / update_actor_roll -> platformer_runtime::orientation or movement::orientation
GravityZone / GravityField / BaseGravity -> mechanics/gravity or platformer_runtime/gravity
```

Only move a helper if the name and API become generic. Portal-specific placement policy should stay portal-specific.

### Expected breakage

- `blink.rs`, `grapple.rs`, `dive.rs`, item projectiles, and portal placement imports.
- Tests that refer to raycast helpers through `crate::portal`.
- Debug/presentation code that used portal-owned actor orientation.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox blink
cargo test -p ambition_sandbox grapple
cargo test -p ambition_sandbox dive
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when disabling the portal module would not remove generic world-query, movement, orientation, or gravity primitives needed by other mechanics.

## Stage 7: Mechanically split portal internals

Goal: turn the giant portal module into navigable responsibility files without semantic redesign in the same patch.

### Steps

1. Create submodules under `src/portal/`.
2. Move code by responsibility.
3. Keep final-intended facade exports in `portal/mod.rs`.
4. Avoid compatibility aliases unless the alias is intended to be public long-term.
5. Keep behavior changes out of this stage.

### Target layout

```text
crates/ambition_sandbox/src/portal/
  mod.rs
  plugin.rs
  schedule.rs
  config.rs
  color.rs
  types.rs
  gun.rs
  pickup.rs
  shot.rs
  placement.rs
  transit.rs
  gate.rs
  lifecycle.rs
  presentation.rs
  debug.rs
  tests.rs
```

### Move map

```text
PortalGun, gun active/next-color state -> gun.rs
PortalGunPickup and pickup/drop systems -> pickup.rs or gun.rs
PortalProjectile / PortalShot logic -> shot.rs
surface hit and fit checks -> placement.rs
player/actor/item teleport -> transit.rs
authored portal registry/phases/loading-zone gating -> gate.rs
orphan cleanup and persistence policy -> lifecycle.rs
visual sync and indicators -> presentation.rs
debug overlay helpers/dev toggles -> debug.rs
PortalColor and channel parsing/display -> color.rs
shared structs/config -> types.rs/config.rs
```

### Expected breakage

- Imports throughout debug overlay, LDtk conversion, tests, and presentation.
- Private helper visibility errors.
- Cycles between portal submodules that reveal missing conceptual boundaries.

Fix forward by moving shared types down into `types.rs` or by narrowing function APIs.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox portal_bridge_reachability
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when each portal responsibility has a file home and agents no longer need to edit a multi-thousand-line portal file for routine changes.

## Stage 8: Semantic portal cleanup

Goal: clarify overloaded portal concepts after the file split makes the compile errors manageable.

### Steps

1. Split dynamic placed portals from authored gate portals.
2. Split gun-pair colors from authored channel colors.
3. Make transit cooldown body-generic.
4. Make player transit independent from holding the gun.
5. Replace one-frame teleport/debug flags with semantic transit messages.
6. Make portal persistence explicit.

### Target renames and replacements

```text
Portal -> PlacedPortal, when referring to player-placed portal bodies
PortalProjectile -> PortalShot
PortalCooldown -> PortalTransitCooldown
PortalRegistry -> GatePortalRegistry
PortalPhase -> GatePortalPhase
PortalConfig -> GatePortalConfig, if it refers to authored gates
PortalColor -> PortalGunColor + PortalChannelColor
IntentionalTeleport -> BodyTeleported / PortalTransit message
```

### Important design rules

- `PortalGun` creates and replaces placed portals.
- Holding a `PortalGun` is not required to transit existing placed portals.
- Transit cooldown belongs on the body, not on the gun.
- Authored gate portals and player-placed portals have different persistence rules.
- Portal debug/trace should consume messages, not own simulation state.

### Expected breakage

This stage should break many call sites. That is intended. Compile errors should force every caller to choose which portal concept it actually meant.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox portal_bridge_reachability
cargo test -p ambition_sandbox replay_fixture_regression
cargo test -p ambition_sandbox scripted_gameplay
```

### Stop condition

Stop when portal naming distinguishes gun portals, authored gates, colors/channels, shots, bodies, and transit messages without relying on comments.

## Stage 9: Portal adapters and Ambition-specific portal integration

Goal: make reusable portal mechanics stop depending on Ambition content concepts.

### Steps

1. Create `ambition_content/portal/` inside the current crate or future content crate.
2. Move Ambition input adaptation there.
3. Move Ambition inventory/item binding there.
4. Move Ambition LDtk entity naming/schema glue there, or prepare it for `portal_ldtk`.
5. Move debug overlay integration there if it depends on Ambition debug UI.
6. Keep reusable portal code message/component based.

### Target layout during proto-crate phase

```text
crates/ambition_content/src/portal/
  mod.rs
  plugin.rs
  input_adapter.rs
  inventory_adapter.rs
  ldtk_adapter.rs
  save_adapter.rs
  debug_adapter.rs
```

### Reusable API expectation

Portal core should expose messages/components such as:

```rust
FirePortalGun
TogglePortalGun
DropPortalGun
PortalPlaced
PortalPlacementFailed
PortalTransit
PortalGun
PlacedPortal
PortalShot
PortalBody
```

Ambition adapters should map:

```text
ControlFrame -> FirePortalGun / TogglePortalGun / DropPortalGun
Item::PortalGun -> PortalGun ability/equipment
OwnedItems / inventory UI -> equip/drop/pickup policy
LDtk PortalGunSpawn -> PortalGunPickup or spawn request
Debug overlay -> portal diagnostics
Trace -> PortalTransit expected movement
```

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox inventory
cargo test -p ambition_sandbox plugin_minimal_app
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when reusable portal modules no longer import Ambition item, inventory, input, debug, or LDtk schema types directly.

## Stage 10: Feature gate portal

Goal: make portal optional after generic helpers and Ambition adapters have been separated.

### Steps

1. Add a `portal` Cargo feature.
2. Gate portal plugin installation.
3. Gate portal tests.
4. Gate Ambition portal content binding.
5. Gate portal rendering and LDtk adapters separately if they are separable.
6. Make LDtk conversion fail loudly if portal-authored entities are loaded while portal is disabled.

### Suggested feature shape

```toml
[features]
portal = []
portal_render = ["portal", "visible"]
portal_ldtk = ["portal", "ldtk"]
```

In the future crate split these become dependency features:

```toml
portal = ["dep:ambition_mechanics_portal"]
portal_render = ["portal", "visible", "dep:ambition_portal_render"]
portal_ldtk = ["portal", "ldtk", "dep:ambition_portal_ldtk"]
```

### Expected breakage

- Inventory code assuming `PortalGun` always exists.
- LDtk conversion assuming `PortalGunSpawn` always exists.
- Debug overlay assuming portal diagnostics always exist.
- Tests assuming portal systems are always registered.

### Validation

```bash
cargo check -p ambition_sandbox --no-default-features --features headless
cargo check -p ambition_sandbox --features desktop_dev
cargo check -p ambition_sandbox --no-default-features --features "headless portal"
cargo test -p ambition_sandbox --features portal portal
```

Adjust exact feature names to match the repository.

### Stop condition

Stop when non-portal builds still have movement, world query, room transitions, gravity, and inventory infrastructure, but do not compile portal mechanics, portal rendering, or portal LDtk conversion.

## Stage 11: Ambition content boundary

Goal: group named Ambition game content behind an explicit content plugin.

### Steps

1. Create `src/ambition_content/` in the current crate.
2. Add `AmbitionContentPlugin`.
3. Move item roster registration behind it.
4. Move quest registry/default quest construction behind it.
5. Move named boss roster/profile construction behind it.
6. Move named world manifest and asset ID bindings behind it.
7. Move intro/cut-rope content hooks behind it, even if implementation remains in old modules temporarily.

### Target layout

```text
crates/ambition_content/src/
  mod.rs
  plugin.rs
  worlds/
  items/
  quests/
  bosses/
  enemies/
  music/
  dialogue/
  portal/
```

### Expected breakage

- App setup expecting to register named content directly.
- Tests importing content registries from old paths.
- Save/default inventory setup assuming Ambition item roster is globally available.

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox content
cargo test -p ambition_sandbox quest
cargo test -p ambition_sandbox boss
cargo test -p ambition_sandbox plugin_minimal_app
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when named Ambition nouns are discoverable under `ambition_content/` and the app installs them through `AmbitionContentPlugin`.

## Stage 12: Disk-layout mirror before crate extraction

Goal: make source layout match final crate topology while still inside one crate.

### Steps

1. Move proto-runtime modules under `src/platformer_runtime/`.
2. Move mechanics under `src/mechanics/`.
3. Move Ambition content under `src/ambition_content/`.
4. Move app-only composition under `src/app/`.
5. Update architecture tests to enforce import direction among these folders.

### Target same-crate layout

```text
crates/ambition_sandbox/src/
  platformer_runtime/
  mechanics/
    portal/
    gravity/
    held_items/
    combat/
    encounter/
  ambition_content/
  app/
  host/
  dev/
  presentation/
```

### Validation

```bash
cargo fmt --all
cargo test -p ambition_sandbox architecture_boundaries
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when the code already looks like a multi-crate architecture even though it is still one crate.

## Stage 13: First real crate extraction

Goal: move the smallest stable reusable boundary into a real crate.

### Recommended first extraction

Extract in this order:

```text
1. ambition_platformer_core or ambition_platformer_runtime
2. ambition_platformer_ecs, if split from core
3. ambition_mechanics_portal
4. ambition_content
```

Do not extract portal before generic helpers and Ambition adapters are separated.

### Steps

1. Create the new crate under `crates/`.
2. Move the proto-module contents with `git mv`.
3. Add dependencies only in the direction allowed by the architecture docs.
4. Update workspace `Cargo.toml`.
5. Update imports in the sandbox crate.
6. Add a `cargo check -p new_crate` validation command.

### Validation

```bash
cargo check -p ambition_platformer_runtime
cargo check -p ambition_sandbox --no-default-features --features headless
cargo test -p ambition_sandbox --lib
```

### Stop condition

Stop when the first extracted crate compiles independently and does not depend on the sandbox app or Ambition content.

## Stage 14: Adapter crate extraction

Goal: split heavy optional dependencies out of headless/core builds.

### Candidate adapter crates

```text
ambition_platformer_ldtk
ambition_platformer_render
ambition_portal_ldtk
ambition_portal_render
ambition_audio
ambition_dialogue
ambition_devtools
```

### Steps

1. Extract one adapter at a time.
2. Ensure the core mechanic/runtime crate does not depend on the adapter.
3. Gate the adapter behind a feature or build persona.
4. Add one compile check that excludes the adapter and one that includes it.

### Validation example

```bash
cargo check -p ambition_sandbox --no-default-features --features headless
cargo check -p ambition_sandbox --features desktop_dev
```

### Stop condition

Stop when headless/runtime builds avoid rendering, LDtk runtime, audio, UI, and devtools unless explicitly requested.

## Stage 15: Cleanup and consolidation

Goal: remove temporary seams and make the new architecture pleasant to work in.

### Steps

1. Remove temporary re-exports that were not intended as final public API.
2. Remove old module paths and TODO comments that pointed to completed migrations.
3. Rename docs from planning language to current-state language where appropriate.
4. Update onboarding docs to point to the new crate/module topology.
5. Re-run ECS inventory and compare to baseline.
6. Add final architecture diagrams or dependency summaries if useful.

### Inventory comparison

```bash
python tools/ecs_inventory.py
cp target/ambition_ecs_inventory.json docs/generated/ambition_ecs_inventory.after_plugin_refactor.json
cp target/ambition_ecs_inventory.md docs/generated/ambition_ecs_inventory.after_plugin_refactor.md
```

### Success metrics

The refactor should be considered successful when:

```text
app/plugins.rs primarily composes plugins.
Portal can be enabled/disabled as a mechanic.
Generic raycast/transit/lifecycle helpers are not under portal.
Ambition named content is grouped behind an Ambition content plugin.
Headless/persona builds avoid obvious optional dependencies.
Architecture tests catch dependency-direction and lifecycle regressions.
Agents can make portal changes without editing a multi-thousand-line file.
```

## Agent task queue

Use these as handoff-sized tasks.

### Task A: Documentation baseline

```text
Apply plugin_refactor docs, generate ECS inventory snapshots, add a stale-doc index, and add an ADR accepting the pluginized platformer runtime direction. Do not change source code.
```

### Task B: Architecture guardrails

```text
Add tests/architecture_boundaries.rs with forbidden-import checks for the planned platformer_runtime boundary and lifecycle spawn-policy checks. Make failures actionable. Do not move modules yet.
```

### Task C: Lifecycle proto-runtime

```text
Create src/platformer_runtime/lifecycle with lifecycle markers and spawn helpers. Update room-local spawn sites to use the helpers. Add or update room-scoped regression tests.
```

### Task D: Portal plugin shell

```text
Convert portal.rs to portal/mod.rs, add portal/plugin.rs and portal/schedule.rs, and move portal system registration into PortalPlugin and subplugins. Do not split portal internals yet.
```

### Task E: Generic helper extraction

```text
Move non-portal helpers such as raycast_solids, ray_aabb, generic transit math, and orientation helpers out of portal into platformer_runtime modules. Update blink/grapple/dive/item/portal callers.
```

### Task F: Mechanical portal split

```text
Split portal/mod.rs into gun, pickup, shot, placement, transit, gate, lifecycle, presentation, debug, color, config, and types modules. Preserve behavior and final-intended facade exports.
```

### Task G: Semantic portal cleanup

```text
Rename overloaded portal concepts: PlacedPortal, PortalShot, PortalGunColor, PortalChannelColor, GatePortalRegistry, GatePortalPhase, PortalTransitCooldown. Make transit cooldown body-generic and player transit independent from holding the gun.
```

### Task H: Ambition portal adapters

```text
Move ControlFrame, Item::PortalGun, OwnedItems, LDtk schema, save, and debug overlay integration into ambition_content/portal adapters. Reusable portal modules should consume messages/components only.
```

### Task I: Portal feature gate

```text
Add the portal feature and gate portal gameplay, render, LDtk, tests, and Ambition content binding. Non-portal builds must retain generic movement/world-query/room/gravity/inventory infrastructure.
```

### Task J: Content boundary

```text
Create ambition_content and move named item, quest, boss, enemy, world, intro, cut-rope, music, dialogue, and portal binding registrations behind AmbitionContentPlugin.
```

### Task K: First crate extraction

```text
Extract the proto-runtime into a real platformer runtime crate only after architecture tests prove it does not import game content or app modules. Then extract portal and Ambition content in later branches.
```

## Decision checkpoints

Pause and decide explicitly at these points:

1. After Stage 2: Are lifecycle helpers ergonomic enough to become final API?
2. After Stage 5: Did portal plugin ownership reveal schedule set gaps?
3. After Stage 6: Which helpers are truly generic versus portal-specific?
4. After Stage 8: Is the split between gun portals and gate portals correct?
5. After Stage 10: Which feature combinations are officially supported?
6. After Stage 11: Is Ambition content a module boundary or ready for a crate?
7. Before Stage 13: Is the proto-runtime import-clean enough for a real crate?

## Anti-goals during this refactor

Do not:

- Create a permanent compatibility layer that preserves every old path.
- Create a single mega-crate named `engine` that recreates `ambition_sandbox`.
- Data-drive complex behavior before the Rust plugin boundary is clean.
- Split every ability into its own crate before the core runtime boundary exists.
- Gate portal before generic helpers have been extracted out of portal.
- Make LDtk silently ignore portal-authored entities when portal is disabled.
- Let `app/plugins.rs` remain the owner of subsystem internals.

## Recommended commit sequence

A good high-level commit sequence would look like:

```text
1. Document plugin refactor roadmap and action plan
2. Add architecture boundary guardrail tests
3. Add platformer runtime lifecycle helpers
4. Use lifecycle helpers for room-local spawns
5. Add platformer schedule sets
6. Move held item and gravity registration into plugins
7. Move portal registration into PortalPlugin
8. Extract generic world-query helpers from portal
9. Split portal module by responsibility
10. Rename overloaded portal concepts
11. Move Ambition portal adapters into content boundary
12. Add portal feature gate
13. Add AmbitionContentPlugin and move named content registration
14. Mirror final disk layout inside the sandbox crate
15. Extract first runtime crate
```

This sequence is intentionally staged so each breakage creates work that remains useful in the final architecture.
