---
status: current
last_verified: 2026-10-09
related_docs:
  - docs/concepts/movement-collision.md
  - docs/systems/transition-spawn-validation.md
---

# Portals

Portals are reusable spatial transforms over ordinary simulation entities. They
must preserve the one-body model: actors, projectiles, and other eligible bodies
transit through the same portal vocabulary rather than separate player and enemy
implementations.

## Contract

A portal pair/connection is provider-authored world data lowered to typed runtime
records. Transit resolves:

- source/destination identity;
- eligibility and re-entry/cooldown policy;
- position and orientation transform;
- velocity/gravity/frame transform;
- safe destination placement;
- lifecycle/trace/effect outcome.

The result commits atomically. Presentation consumes the transit fact for VFX,
SFX, camera treatment, and interpolation.

## Invariants

- Portal IDs are stable provider/world IDs; Bevy entities are runtime handles.
- Transit uses shared body/projectile/world geometry semantics.
- Transform composition is deterministic and gravity-aware.
- Destination placement cannot embed the body in blocking geometry.
- Re-entry prevention is scoped per transit/body, not global accidental state.
- Headless and visible compositions produce the same authoritative result.
- Reset/room replacement/snapshot restore cannot retain stale connections or
  cooldown markers.

## Presentation

A portal is drawn as a thin line of light along its opening
(`crates/ambition_portal2d_presentation/src/glow.rs`, shader
`portal_glow.wgsl`): a bright core, a glow that is wider on the room side, a
node at each end of the opening, and faint streaks drawn into it from the room
side. The room side has the portal's own colour and the other side has its
partner's. The colour-name label stays.

- A portal that is added to a room opens from its middle (0.42 s). A portal
  that is removed breaks up and closes (0.5 s) where it was.
- A portal that is in its room when the room comes (authored level content)
  does not open.
- Presentation decides "added" from what it sees; the simulation has no such
  fact. A portal first seen while its room is younger than 0.25 s was there
  with the room. An entity that goes and another that comes on the same
  channel at the same place in one frame are one aperture, because a rollback
  gives a restored portal a new entity.
- The line is on the world layer and in its room's render band, so a portal
  capture photographs it. Its z follows the pane-dominance rule: over the
  glass for the portal you are in front of, under it for the far one.

To see it: `capture_scene portal_lab X,Y out.png` and
`--portal-shot TICK:X,Y:DX,DY[:b]`, which fires a shot of a pair that no gun
owns. A gun's portals go when no gun is in the room, so a gun shot in a
capture leaves nothing to photograph.

## Validation

Prefer transformation properties:

- through-portal covariance;
- round-trip behavior where topology permits;
- velocity/orientation mapping;
- actor/projectile controller symmetry;
- rejection leaves state unchanged;
- provider validation catches missing/duplicate destinations.

```bash
python scripts/agent_query.py "portal transit transform"
python scripts/agent_query.py tests "portal projectile gravity"
./run_tests.sh -k portal
```
