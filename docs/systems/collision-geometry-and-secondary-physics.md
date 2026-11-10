---
status: current
related_docs:
  - docs/concepts/movement-collision.md
  - docs/mechanics/body-modes.md
  - docs/systems/portals.md
---

# Collision geometry and secondary physics

Authoritative geometry belongs to platformer simulation. Render bounds and
visual-only physics do not decide body movement, strikes, hazards, or
progression.

## Geometry ownership

`ambition_platformer2d_world` lowers provider world data into typed collision
surfaces. The core movement and contact code owns body sweeps, collision
repair, and responses. `ambition_platformer2d_core::frame::material_cast`
provides material-space casting for bodies whose geometry occupies multiple
spatial cells, including portal-carried partitions. The optional portal
capability supplies its passage geometry.

These representations must agree on the physical outcome. An ordinary
unpartitioned body may use a simpler AABB path, but a root AABB must not be
treated as the complete occupied space of a body spanning a portal. Any
fast path needs an explicit eligibility condition and consistent contact
semantics, including when the body has little or no movement.

The same authoritative collision vocabulary should serve movement, blink,
projectiles, strikes, hazards, and safe spawning. New mechanics should not
invent collision based on sprite bounds or maintain a second copy of room
solids.

## Secondary physics

A separate backend may animate debris, decorations, or toy-world objects
whose output has no gameplay consequences. Once another system reads its
positions or contacts to decide health, movement, collection, or quest
outcomes, those facts must be admitted into authoritative simulation and
snapshot/restore behavior.

## When changing collision

Inspect both the common-body path and any active portal/material path.
Check the actual affected gameplay, especially stationary overlap, moving
platforms, rotated gravity, and partial portal occupancy when relevant.
Measure the portal-free path before adding expensive general work to every
body tick. Use targeted regression coverage for a demonstrated failure,
not a separate battery of tests for each internal helper.
