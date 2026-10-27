# ADR 0038: Persistent physical objects do not require a full physics backend

## Status

**Proposed architecture (2026-10-26).** Object interaction and persistence are desired engine
capabilities; the specific physics implementation and gameplay use of ragdolls remain game
choices.

## Context

Carrying and placing a persistent object is a different responsibility from solving its motion.
`ambition_held_items` already models some item custody; `ambition_platformer2d_physics_debris`
already integrates optional Avian2D secondary simulation. Neither establishes a general
persistent-prop authority. Ragdoll presentation can also be purely cosmetic, while a corpse that
blocks or presses objects becomes gameplay-authoritative.

## Proposed decision

- Give persistent, placeable world objects stable identity, room/world location, ownership or
  custody, and an explicit save/disposition horizon independent of the chosen physics solver.
- Provide capability tiers: static placeable object; simple deterministic
  falling/throwing/collision; optional Avian2D rigid-body props; optional articulated ragdoll
  simulation.
- Keep the existing custom kinematic platformer controller as a valid independent choice;
  neither props nor ragdolls mandate Avian for player movement.
- Distinguish **authoritative mechanics** (collisions, switches, carrying, loot, interactable
  corpses) from **presentation-only motion** (cosmetic limbs, particles, debris). Local quality
  settings may disable or simplify only non-authoritative physics.
- A network participant must not independently disable authoritative rigid-body rules while
  remaining in the same game simulation. A designated simulation authority may choose the
  physics policy for its world or region; deterministic compatibility is verified rather than
  assumed.
- Saving object identity and settled disposition is separate from saving every solver variable.
  The latter is required only when active motion must persist exactly through pause, handoff or
  replay.
- Authoring a physical object describes intent and mechanical consequences, not a mandatory
  backend type.

## Alternatives and risks

Extending weapon-specific `GroundItem` into every furniture or prop type creates semantic
coupling. Storing all Avian solver state in a global rollback population makes even cosmetic
motion expensive. Per-client cosmetic ragdolls may differ visually and cannot be used for
authoritative collision. A scene with building-machine puzzles has stronger persistence and
replication requirements than a scene with movable decorative crates.

## Validation before a package split

- Carry a general prop through a room transition, place it, unload/reload, and find exactly one
  object in the same canonical place.
- Drive the same object intent with simple collision and optional Avian simulation and compare
  correctness, save requirements and compute cost.
- A confirmed death spawns a cosmetic ragdoll on one client while another uses an authored death
  pose, with identical gameplay state; an interactive corpse uses authoritative collision and is
  tested separately.

See [ADR 0007](0007-avian2d-secondary-physics.md) and [object/physics planning](../planning/engine/architecture-reassessment-and-composable-multiplayer.md).
