---
id: movement-collision
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-13
implemented_by:
  - crates/ambition_platformer2d_core/src/movement
  - crates/ambition_geometry/src/geometry.rs
  - crates/ambition_platformer2d_world/src/collision.rs
  - crates/ambition_combat
  - crates/ambition_projectiles
related_docs:
  - docs/mechanics/expressibility-checklist.md
  - docs/mechanics/body-modes.md
  - docs/planning/engine/collision-and-ccd.md
  - docs/archive/planning-superseded/2026-08-13/engine/collision-and-ccd.md
---

# Movement and collision

Movement/collision is a deterministic body pipeline shared by every controller
kind. Presentation may visualize contacts and actions but does not define their
geometry or outcomes.

## Invariants

- One actor/body kernel serves player, AI, possessed, and harness-controlled
  bodies.
- Fast movers and triggers use swept/path-aware evaluation; discrete endpoint
  overlap is not enough.
- One-way behavior is directional and reference-frame aware.
- Contact at an edge must not become a far-block correction or teleport.
- Body-shape changes are transactional: reject a target shape that cannot fit.
- Hitboxes/hurtboxes/projectiles derive from explicit action/spec state, never
  from a renderer sprite as authority.
- Gravity/orientation assumptions are explicit. Tests should exercise rotated
  or transformed frames where the mechanic claims covariance.
- Axis-swept collision is independent of the selected control law. Responsive
  target-velocity movement and momentum/phased-gravity platforming are authored
  policy data that share the same sweep/contact authority.
- Gravity-responsive movement may scale only the frame's gravity contribution;
  external acceleration remains independently applied in world space.
- Moving geometry and attached bodies use explicit reference-frame semantics.
- Out-of-bounds behavior is a world/lifecycle contract, not an ad hoc player
  clamp.

## Current collision vocabulary

The settled implementation uses a small set of shared authorities rather than
per-feature collision derivations:

- `SweepSample` is the simulation phase's canonical travelled segment. Movement
  writes it; path-dependent readers consume it. Teleports and transfers outside
  that phase are not retroactively turned into travelled collision paths.
- `cast::aabb_path_contacts` is the shared body-vs-static-volume swept trigger
  primitive. Loading zones already use it.
- water and climb volumes are state regions rather than first-TOI triggers;
  `World::thin_region_warnings` rejects authoring thin enough to be tunnelled.
- `GeoId` / `GeoFaceRef` identify authored/composed geometry without relying on
  iteration position.
- `PortalFrame` / `PortalAperture` and explicit map conventions own portal frame
  geometry and velocity mapping.

The remaining source-confirmed CCD gap is tracked in
`docs/planning/engine/collision-and-ccd.md`: the common hazard gate still uses
endpoint overlap instead of the canonical sweep sample.

## Edit protocol

1. Query the exact mechanic and prior failure class.
2. Find the pure kernel/model and the ECS integration caller.
3. Add the smallest geometry/property test that would fail on the bug.
4. Verify body/controller parity and at least one headless assembled path.
5. Emit visual/audio feedback through messages/read models after simulation.

```bash
python scripts/agent_query.py "<mechanic> collision sweep"
python scripts/agent_query.py tests "<geometry case>"
./run_tests.sh -p ambition_platformer2d_core
./run_tests.sh -k <test-substring>
cargo run -p ambition_app_tools --bin headless -- 120
```
