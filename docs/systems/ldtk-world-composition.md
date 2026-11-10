---
status: current
related_docs:
  - docs/concepts/ldtk-world-composition.md
  - docs/recipes/ldtk-authoring.md
---

# LDtk world composition and room transitions

The authored LDtk world is provider data. Loading and changing rooms use
preflight and canonical construction rather than spawning a partial target
world and repairing it afterward.

## Flow and owners

1. The provider supplies `.ldtk` worlds and a manifest.
2. `ambition_platformer2d_ldtk` imports layers, entities, and fields into
   typed world records.
3. `ambition_platformer2d_world` owns room, loading-zone, and collision
   vocabulary plus construction validation.
4. Provider registrations lower specific authored objects into a
   `RoomConstructionPlan` without mutating the live room.
5. `ambition_load` tracks outstanding required/degradable work. The runtime
   coordinator commits the target when its prerequisites are satisfied.
6. Canonical construction retires the source room and installs the target;
   `ambition_load_presentation` can display loading state but cannot grant
   permission to commit.

Some construction integration still resides in
`ambition_platformer2d_actor_monolith` and app adapters. Follow the
existing canonical path for a new authored object.

## Loading-zone direction

`RoomSet::way_of` derives whether a zone connects in both directions,
outward only, or inward only from the authored room links. This is not a
separate flag that authors must duplicate on the visual. Presentation makes
one-way and arrival-only directions conspicuous, including door/nameplate
labels; the source of truth remains the link graph.

## Transition behavior

Repeated overlap with one zone must not launch duplicate transitions, and
failure must leave the current playable room intact. Preflight must not leak
target entities; arriving bodies must be placed safely using the destination
geometry. Cover, loading UI, and grace/ready-hold effects are presentation
policy rather than substitutes for the commit barrier.

For authoring, use the LDtk editor and `ambition_ldtk_tools` rather than
editing generated LDtk JSON by hand. See
[LDtk authoring](../recipes/ldtk-authoring.md) and
[transition spawn validation](transition-spawn-validation.md).
