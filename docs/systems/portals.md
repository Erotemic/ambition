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

### A body that crosses

Each body that straddles a portal is drawn as two clipped pieces, one at each
face (`sync_portal_body_pieces`): the player, an NPC, a dog. A body that is
not the player is drawn by another entity than its own, so the host publishes
the facts onto the visual that draws it while it is in transit
(`publish_transiting_feature_bodies`: the scene-body tag, the pose, and
`PortalTransitView`). Presentation reads that fact and not the simulation's
`PortalTransit`.

### The picture is the viewer's chart

Near a portal its view window takes over the half-plane behind its face: the
near side is drawn as it is, and the far side is drawn joined to it at the
seam. The far half of a body that crosses is seen through the window, joined
to the near half. This is so for each pair. A door through a thin wall (two
opposed faces a wall's thickness apart) is not a special case:

- Its map is a translation by the wall's thickness. A window held to the slab
  showed the far side moved by that thickness beside the far side itself, so
  a body that crossed was drawn two times. The door's window takes over as
  each other window does, and the wall's thickness is not drawn while you are
  at the door.
- The camera cuts at the crossing by the same translation, so the picture on
  screen is the same before and after it.
- A window that closes is still the near one or the far one
  (`pane_dominant`), so the far portal's line and label are under the glass
  after a crossing too.

- The window's capture is the mapped camera snapshot
  (`PortalCaptureCameraMode::MappedCameraSnapshot`, the default): the host
  view mapped through the pair, at the screen's density. Each texel of it is
  one pixel of the pane, so the far half of a body joins its near half with no
  step. The cone rect, the default before, is drawn into a texture of a fixed
  size at a scale that is not the screen's; the far half sat about a pixel
  off. The cost is a capture of the size of the view for each open window,
  under the same capture budget.

What cannot be removed: away from a door, its window is a wedge, and the far
side inside the wedge is moved by the wall's thickness against the far side
outside it. The screen has the wall's thickness more space than the two sides
have between them, so a picture that shows both has one seam where they do
not meet. At the door the seam is the door's own line.

To see it: `capture_scene portal_lab X,Y out.png` and
`--portal-shot TICK:X,Y:DX,DY[:b]`, which fires a shot of a pair that no gun
owns, and `--player-at X,Y[@TICK]`, which puts the player at a point (repeat
it to step the player through a portal). A gun's portals go when no gun is in the room, so a gun shot in a
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
