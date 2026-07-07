# Movement refactor benchmark candidates — 2026-05-11

These candidates were distilled from the movement split overlay that compiled
only after several follow-up import fixes, then failed the full-world wall-cling
repro and headless random-walker fuzz. Keep this file additive so it can be
reviewed independently from the shared `rust-questions.md` corpus.

## Q1: Split a Rust movement module without losing import closure

### Level A prompt

You are splitting a large Rust file:

```text
crates/ambition_engine/src/movement.rs
```

into:

```text
crates/ambition_engine/src/movement.rs
crates/ambition_engine/src/movement/blink.rs
crates/ambition_engine/src/movement/collision.rs
crates/ambition_engine/src/movement/control.rs
crates/ambition_engine/src/movement/integration.rs
crates/ambition_engine/src/movement/simulation.rs
crates/ambition_engine/src/movement/tests.rs
```

The original file had top-level imports for `Vec2`, `Aabb`, extension-trait
methods such as `AabbExt::bottom()` / `AabbExt::center()` /
`AabbExt::half_size()`, and test helpers. After extraction, some code that used
to be lexically below those imports now lives in child modules.

What should the refactor plan explicitly verify before handoff?

### Expected answer

Each child module needs an explicit import closure. Parent-module `use` items do
not flow into private children, and extension-trait methods are only available
when the trait is in scope in the module using the method. The plan should check
all extracted modules, including `#[cfg(test)]` modules, for:

- concrete types moved behind module boundaries (`Vec2`, `Aabb`, `AbilitySet`),
- extension traits needed for method syntax (`AabbExt`),
- facade re-exports needed by crate-root `pub use movement::{...}`,
- commands that compile both the library and extracted tests.

Validation command:

```bash
cargo fmt --all
cargo test -p ambition_engine --lib
```

This catches missing imports in `movement/tests.rs` and child modules rather
than only checking the production facade.

### Why this was easy to miss

The pre-split file made imports look global because every item lived in one
lexical module. The extracted files still had the same Rust text, but not the
same module scope.

Tags: `rust-module-refactor`, `rust-import-closure`, `extension-trait`.

## Q2: Do not replace guarded collision semantics with raw shape-cast normals

### Level A prompt

A game engine has custom platformer movement using Parry shape casts. Existing
`movement::sweep_player_x` and `movement::sweep_player_y` contain bespoke guards
for Ambition semantics:

- immediate-contact hits from Parry can represent resting/edge-touching rather
  than meaningful penetration,
- side-wall contacts during a vertical sweep must not be snapped to the wall's
  top or bottom edge,
- wide floor/ceiling contacts during a horizontal sweep must not push the player
  to the far horizontal face,
- one-way platforms are solid only for valid landing-from-above cases.

You are asked to plumb `ShapeCastHit::normal1` through `geometry` and `world`
and use contact normals to simplify snap direction. What must the refactor do
before replacing the existing snap heuristics?

### Expected answer

Plumbing normals is not the same as proving they encode the engine's movement
semantics. The refactor should first preserve the existing guarded logic, add a
small adapter or helper that classifies Parry hits into engine-level contact
classes, and validate that helper against full-world repros before changing snap
behavior.

A safe staged answer is:

1. Add `normal1` to `geometry::SweepHit` and `world::BodySweepHit`.
2. Leave `sweep_player_x` / `sweep_player_y` behavior unchanged.
3. Add targeted tests for edge-touching, pre-existing side-wall overlap,
   landing-from-above, ceiling hit, and one-way platform pass-through.
4. Only then replace a single branch with the normal-derived helper if tests
   show the helper makes the same classification.

Validation command:

```bash
cargo test -p ambition_engine --lib
cargo test -p ambition_actors --test repro_walls
```

Expected invariant:

```text
raw Parry contact normal != Ambition movement correction direction
```

until it has passed project-specific collision classification tests.

### Why this was easy to miss

The API name `normal1` sounds like it should remove hand-coded snap direction.
But the old code was not just choosing a direction; it was filtering out contacts
that should be owned by the perpendicular axis or ignored as pre-existing side
contacts. Replacing those guards with a normal check erased project-specific
semantics and reintroduced the `y=-23` wall-cling teleport.

Tags: `game-physics`, `shape-cast`, `platformer-collision`,
`semantic-adapter`.

## Q3: Simulation-side systems must not depend on presentation-only resources

### Level B prompt

A headless `SandboxSim` test builds an app with simulation plugins but not
presentation plugins. A quest reward system runs during `Update` and has this
parameter:

```rust
mut inventory: ResMut<PlayerInventory>,
```

The visible binary inserts `PlayerInventory::starter()` in
`add_presentation_plugins`, near pause-menu and inventory UI setup. Headless
fuzz fails with:

```text
Parameter `ResMut<PlayerInventory>` failed validation: Resource does not exist
```

Where should the resource be initialized, and why?

### Expected answer

`PlayerInventory` is gameplay state because quest reward systems mutate it in
the simulation schedule. It should be inserted in the simulation resource setup
path (`init_sandbox_resources` or another simulation-only initialization seam),
not only alongside presentation UI state. UI resources such as
`InventoryUiState` may remain presentation-only, but any resource required by a
simulation system must exist in `SandboxSim`.

Validation command:

```bash
cargo test -p ambition_actors --test fuzz_random_walker
```

Tags: `bevy-resource`, `headless-simulation`, `presentation-sim-seam`.
