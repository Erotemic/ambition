# Parry2D geometry layer

Ambition still owns its movement feel. The player controller remains a handcrafted kinematic platformer controller, not a rigid-body physics object. This patch adds `parry2d` as the shared geometry/query library underneath Ambition's small `Aabb` type so the engine can stop growing one-off collision math.

The current integration is intentionally narrow:

- `Aabb::intersects` now delegates the narrow-phase check to Parry while preserving Ambition's old strict rule that merely touching edges do not count as overlap.
- `Aabb::sweep_time_of_impact` wraps Parry shape casting for a moving AABB against a static AABB.
- `World::first_body_sweep` gives movement systems one reusable way to ask "what is the first accepted block hit by this body moving along this delta?"
- Player X/Y integration uses swept casts before the old positional repair, reducing the chance that high-speed dash or future impulse movement skips through thin walls.
- Dummy/enemy knockback uses the same swept path, replacing the old hard-coded substep limit.
- Blink uses Parry to find hard blockers along the path, then keeps the existing pass-through-wall sampling policy so blink-through upgrades can cross soft/hard blink walls without allowing the final body to rest inside them.
- Transition spawn validation now uses the engine's shared overlap helper instead of duplicating block iteration in the Bevy sandbox.

This is not a full physics migration. Rapier or Avian can still be considered later for crates, debris, sensors, and other non-player physics objects, but the core movement verbs stay deterministic and readable in `ambition_engine`.

## Next steps

Good follow-up refactors:

1. Replace the remaining ad-hoc rebound positional snap with a shared collision response helper.
2. Add `approx` and `rstest` cases around swept player movement, blink blockers, one-way platforms, and dummy knockback.
3. Extend the Parry wrapper to raycasts/shape casts for grapple, line of sight, and blink target preview.
4. Consider switching `Vec2` to `glam` or adding explicit conversion helpers if Parry/Bevy interop starts to feel noisy.

## Edge-touching contacts

Parry reports some starts that are merely touching as immediate contacts. That is useful for many physics workflows, but Ambition treats resting contact as non-overlap so a player standing on a floor can still move horizontally and a body sliding along a wall can still move vertically. The `Aabb::sweep_time_of_impact` wrapper keeps Parry as the geometry backend but filters zero-time contacts unless the requested delta is actually moving into the touching face. This preserves the old platformer semantics while still letting wall pushes and falling landings report immediate hits when appropriate.
