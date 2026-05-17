# Parry2D geometry layer

Ambition still owns its movement feel. The player controller remains a handcrafted kinematic platformer controller, not a rigid-body physics object. `parry2d` is used as the shared geometry/query library underneath Ambition-specific collision semantics.

The public AABB type is now `ambition_engine::Aabb`, a re-export of Bevy's `bevy_math::bounding::Aabb2d`. `geometry.rs` adds only the extra semantics Ambition needs:

- `aabb_from_min_size` for room data authored as min+size rectangles.
- `AabbExt::strict_intersects` for platformer overlap where touching edges do not count as overlap.
- `AabbExt::sweep_time_of_impact` for Parry-backed swept AABB queries.
- `World::first_body_sweep` for movement systems that need the first accepted block hit along a delta.

This is not a full physics migration. Rapier or Avian can still be considered later for crates, debris, sensors, and other non-player physics objects, but the core movement verbs stay deterministic and readable in `ambition_engine`.

## Edge-touching contacts

Parry reports some starts that are merely touching as immediate contacts. That is useful for many physics workflows, but Ambition treats resting contact as non-overlap so a player standing on a floor can still move horizontally and a body sliding along a wall can still move vertically. `AabbExt::sweep_time_of_impact` keeps Parry as the geometry backend but filters zero-time contacts unless the requested delta is actually moving into the touching face.

## Next steps

Good follow-up refactors:

1. Replace the remaining ad-hoc rebound positional snap with a shared collision response helper.
2. Add `approx` and `rstest` cases around swept player movement, blink blockers, one-way platforms, and dummy knockback.
3. Extend the Parry wrapper to raycasts/shape casts for grapple, line of sight, and blink target preview.
