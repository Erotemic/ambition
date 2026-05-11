# 2026-05-11 movement refactor lessons

This additive journal captures the failed movement split / normal-plumbing
overlay without editing the shared `lessons_learned.md` file. It should be
merged or summarized later if the team wants a single chronological journal.

## Symptom 1: compile failures after extracting child modules

The movement split initially failed with missing imports after code was moved
from `movement.rs` into child modules:

- `AabbExt::bottom()` was unavailable in `movement/integration.rs` because the
  extension trait was not imported in that module.
- `Vec2`, `Aabb`, and `AabbExt` were unavailable in `movement/tests.rs` because
  test code lost the original parent module's import scope.
- `AabbExt::sweep_time_of_impact(self, ..., rhs: Self)` needed `where Self:
  Sized` on the default trait method.

### Root cause

The refactor preserved text but not lexical scope. Rust child modules do not
inherit parent `use` imports, and method syntax for extension traits requires
that the trait be in scope at the call site.

### Fix

Add explicit imports to each extracted module and put the `Self: Sized` bound on
the default method.

### Takeaway

For Rust module splits, verify each extracted file's import closure separately,
including tests. Do not trust the original file's top-level imports as evidence
that the child module will compile.

## Symptom 2: full-world wall-cling repro teleported to `y=-23`

After replacing bespoke sweep snapping with a normal-derived correction,
`cargo test -p ambition_sandbox --test repro_walls` failed:

```text
FULL WORLD step: pos=(62, -23) vel=(0, 0) on_ground=true on_wall=true cling=false
teleported out of world: pos=Vec2(62.0, -23.0)
```

The simplified normal path classified a vertical sweep hit as a floor/landing
and snapped the body to a tall wall's top edge. That recreated the historical
wall-cling teleport that the existing overlap/approach guards were designed to
avoid.

### Root cause

The old code was not just choosing a snap direction. It encoded multiple
engine-level collision semantics on top of Parry's raw shape-cast result:

- ignore pre-existing side-wall contacts during y-sweeps,
- reject floor/ceiling contacts leaking into x-sweeps,
- only land on one-way platforms and normal floors when approaching from above,
- avoid using immediate-contact hits as proof of a valid landing.

`normal1` plumbing is useful, but raw contact normals do not by themselves
represent these semantics.

### Fix

Keep `ShapeCastHit::normal1` plumbed through `geometry` and `world`, but revert
`movement/collision.rs` to the guarded position/overlap/approach logic until a
targeted contact-classification adapter and tests exist.

### Takeaway

When a physics refactor replaces a bespoke heuristic, first identify whether the
heuristic is actually an adapter from library semantics into game semantics. If
it is, preserve behavior and add characterization tests before consuming the new
API.

## Symptom 3: headless random-walker fuzz panicked on missing inventory

`cargo test -p ambition_sandbox --test fuzz_random_walker` failed in Bevy's
system parameter validation:

```text
grant_quest_completion_rewards: Parameter `ResMut<PlayerInventory>` failed validation: Resource does not exist
```

### Root cause

`PlayerInventory` was inserted by `add_presentation_plugins` near inventory UI
resources, but `grant_quest_completion_rewards` is a simulation-side system.
`SandboxSim` loads simulation plugins without presentation plugins, so the
resource was absent in headless tests.

### Fix

Initialize `PlayerInventory::starter()` in the simulation resource setup path
(`init_sandbox_resources`) before the first `Update` tick. Keep UI-only
resources in presentation setup.

### Takeaway

Bevy resources required by simulation systems belong in simulation setup, even
when the visible UI is the most obvious consumer. Headless tests are the gate
for presentation/simulation seam mistakes.
