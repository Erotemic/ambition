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

## An aperture is on its wall

The aperture of a portal is a plane, and the plane is the face of a wall: a
body goes into the face of one wall and comes out of the face of the other.
It is on the wall, not in it and not in front of it.

- A portal the gun places is on the face its shot hit (2 px out, which the
  host adapter keeps as the lift).
- A portal of a level is an LDtk `Portal` box drawn on a wall. The box says
  which face, and the face says where: the conversion moves the box along its
  normal until its center is on the nearest face of that normal that goes
  through the box (`settle_portals_on_faces` in
  `ambition_platformer2d_ldtk`). Before this, the center of the box was the
  plane, and the boxes of `portal_lab` were drawn from 10 px in the wall to
  8 px in front of it: the line of a portal was drawn off its wall, and a
  body crossed off the wall.
- A box with no face of its normal through it keeps its center: a portal in
  open air.

## Presentation

A portal is drawn as a thin line of light along its opening
(`crates/ambition_portal2d_presentation/src/glow.rs`, shader
`portal_glow.wgsl`): a bright core, a glow that is wider on the room side, a
node at each end of the opening, and faint streaks drawn into it from the room
side. The line has one colour, the portal's own. The colour of its partner is
seen through the portal, when its window opens and shows the far side with
the partner's line in it. `PortalGlowStyle { partner_colour: true }` draws the
far side of each line in its partner's colour, for a game with no windows or
one that wants each line to say where it leads. The colour-name label stays.

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
  capture photographs it. It has one depth (`PORTAL_FRAME_Z`): over the world,
  and under the far slice of a body and under each view window. A body that
  crosses is thus drawn whole over the line, and the line does not cut it.
- A line belongs to the session that drew it and to its room
  (`spawn_session_scoped` with the room). A new session keeps no line and no
  room age of the old one: the next session's rooms can have the same
  ordinals, and a line or an age kept by ordinal would be given to a
  different room.

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

- The far slice of a body is put where the map sends the point its sprite is
  drawn at, and not where it sends the body's centre. A sprite can be drawn
  off its body (a foot anchor, a pose), and the two slices then did not meet
  by that offset.
- A window opens only for a viewer whose centre is in front of its face. The
  window of the other end is cut at once: it would draw its image of the
  viewer's own side over that side.
- A window opens with an ease and closes with one, each time
  (`eased_blend`): a snap reads as a fault in the world. A window whose view
  is lost eases shut on the cones it had. A rig that is built again goes on
  from the blend it had.
- At a crossing the window of the far end goes on from the blend of the near
  one (`came_through`): the two are one window seen from its two sides, and
  the picture does not change. The measure is the crossing, and not a whole
  line of sight: at the door of a thin wall two corners of the body are past
  the plane when the centre crosses, and a rule that asked for the whole view
  opened the far window from nothing, so each crossing showed the bare world
  for some frames.
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

A second case: a body that crosses the END of the opening of a door in a wall
that stands free. The part of it inside the opening is through the door and
the part of it past the end is above the wall, and the two are the wall's
thickness apart. `straddles` admits each body that overlaps the opening, so
this state can occur, and no picture joins it. The repair is a simulation
rule (a body goes through only where its box is inside the opening), not a
presentation one.

To see it: `capture_scene portal_lab X,Y out.png` and
`--portal-shot TICK:X,Y:DX,DY[:b]`, which fires a shot of a pair that no gun
owns, and `--player-at X,Y[@TICK]`, which puts the player at a point (repeat
it to step the player through a portal). A gun's portals go when no gun is in the room, so a gun shot in a
capture leaves nothing to photograph.

### Two portals that face each other

The window of a portal shows the room in front of its partner. When the two
ends of a pair look at each other across open space, that room has the first
end in it, with its window: a body between them is seen again and again, in a
row. So the capture of an end draws that end's own window
(`pair_looks_at_itself` in `view_cones/geometry.rs`). The picture in the
picture is one frame late, and each level is darker by the tint.

A capture of an end whose pair does not look at itself never draws its own
window. On a thin wall the window of an end is in the same place as the room
in front of its partner, and a capture that drew it would film its own
picture in place of the room.

Seen in a capture (2026-10-10), with a pair shot onto two faces 144 units
apart in `portal_lab` and the player between them: the row of images goes on,
with five images of the player in the view.

```bash
capture_scene portal_lab player OUT.png 1280x720 --warmup 240 \
    --player-at 2136,864@60 \
    --portal-shot 80:2136,850:-1,0 --portal-shot 90:2136,850:1,0:b
```

The same capture with `--camera-zoom combat` and `--camera-zoom arena` shows
the row too (seven images at `arena`). An earlier note here said that a wider
zoom shows no portal window: that was the pair of that capture, whose windows
are shut when the eye is far from the two ends, and not the zoom.

The gizmos of the developer overlays are drawn in a window too: the same
capture with `--combat-overlay` shows the boxes, the arrows and the bars of
each image of the player. A capture camera is on the world layer, and the
gizmos are on that layer.

Not seen: a window on a GPU. The rule is pinned by
`a_capture_draws_its_own_window_only_when_its_pair_looks_at_itself`.

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
