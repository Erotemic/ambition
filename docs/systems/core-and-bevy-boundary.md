# Engine and Bevy boundary

`ambition_engine` is no longer trying to be a renderer-neutral core. It is the
reusable Bevy-native mechanics crate for Ambition. Sandbox and future story
crates should stay thin: they choose content, wire presentation, and provide
room/story data without reimplementing movement or collision semantics.

## Boundary rule

```text
ambition_engine
  movement, collision semantics, abilities, combat, enemies, reusable mechanics,
  generated-audio specs, and testable data structures

ambition_sandbox / future story crates
  room/story data, Bevy app setup, rendering, input bindings, debug UI,
  presentation, and platform-specific glue
```

The important separation is **mechanics vs presentation**, not "no Bevy in the
engine".

## Math and geometry policy

The engine should use battle-tested Bevy-adjacent primitives instead of
maintaining bespoke math infrastructure:

- `ambition_engine::Vec2` re-exports `bevy_math::Vec2`.
- `ambition_engine::Aabb` re-exports Bevy's `Aabb2d` representation.
- `ambition_engine::AabbExt` is reserved for Ambition-specific semantics such as
  strict platformer overlap, contact interpretation, and swept collision helpers.
- `parry2d` handles lower-level geometry queries such as swept boxes and future
  raycasts/spawn validation.

Do not add a new project-local vector or AABB type unless an ADR explains why
Bevy/Parry primitives are not enough.

## Archived background

The original Bevy-math and `glam` migration notes are archived because the
migration has landed:

- [`../archive/historical-roadmaps/glam-migration.md`](../archive/historical-roadmaps/glam-migration.md)
- [`../archive/historical-roadmaps/bevy-math-engine-refactor.md`](../archive/historical-roadmaps/bevy-math-engine-refactor.md)
