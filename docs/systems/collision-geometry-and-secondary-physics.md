
# Collision, geometry, and secondary physics

This is the current entry point for collision and physics-adjacent systems. Older standalone notes about Avian2D, Parry2D, enemy collision, and moving platforms were consolidated here because agents need one trusted map instead of several patch-era documents.

## Decision

The primary player controller remains custom kinematic gameplay code. It owns platformer feel, coyote/buffered jump, dash, blink, wall behavior, pogo/rebound, body modes, and collision-safe resizing.

Avian2D is allowed as **secondary physics** for props, debris, ragdoll-like chunks, experiments, and future presentation-heavy interactions. It is not the default player controller.

Parry2D-style geometry is useful as an implementation aid for shape casts and geometry queries, but the durable concept is Ambition gameplay geometry, not a dependency-specific API.

## Coordinate spaces

Spatial code is review-sensitive because Ambition frequently bridges:

- LDtk/grid/world pixels,
- simulation coordinates,
- Bevy transforms,
- camera/active-area-local coordinates,
- optional Avian/physics coordinates.

Use `AMBITION_REVIEW(spatial): ...` near plausible but hard-to-prove seams.

## Moving platforms and enemy collision

Moving platforms and enemy collision are gameplay systems layered on top of the same kinematic/geometry vocabulary:

- moving platforms should preserve player carry semantics and edge-case tests;
- enemies and hazards should use actor/faction/damage vocabulary rather than one-off collision code;
- authored collision remains LDtk/IntGrid-driven where possible;
- presentation and debris may use secondary physics without changing player collision semantics.

## Edit protocol

When changing collision/geometry behavior:

1. Search `dev/` for prior movement/collision traps.
2. Identify whether the change affects primary kinematic movement, authored LDtk collision, actor/hazard collision, or secondary physics.
3. Add or update focused tests before broad refactors.
4. Keep platform presentation and physics debris separate from core player movement.
5. Update `docs/concepts/movement-collision.md` if a durable invariant changes.

Useful search:

```bash
rg -n "wall_cling|ledge|sweep|pogo|moving platform|collision|Avian|Parry" crates docs dev
```

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib engine_core::movement
cargo test -p ambition_sandbox --lib kinematic
cargo test -p ambition_sandbox --lib ldtk
cargo test -p ambition_sandbox --test wall_cling_fuzz
```

Use narrower filters if a concept page or benchmark candidate names the exact test.
