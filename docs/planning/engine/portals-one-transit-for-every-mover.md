# One portal transit for every mover, measured by a seam lab - an engine plan

**Status:** S1–S3 BUILT 2026-10-16; S4 open (see "Measured" and "Open"). It follows
[portals are moving frames](portals-are-moving-frames.md).

Jon, 2026-10-15:
- "Nothing should teleport instantly. Portals should work the same for every object that
  moves."
- "I would love to get it to the point where we actually can get rid of seams and to have
  arbitrary game characters, props, and projectiles work with portals perfectly."
- "It might be helpful to build a prop with a grid or other measurable patterns on it so we
  can use it to measure how seemless portals actually are. We should have steps to see how
  these objects pass through portals with different cases and velocities. Although maybe not
  all of these run in the default test suite."

## Where it stands (read 2026-10-15)

- **Kernel bodies** (the player, NPCs, bosses, mounts) use the one machine,
  `transit_step_with_tuning`. They straddle the portal, and the transfer happens when the
  centroid crosses. `sync_portal_body_pieces` draws them split across the pair.
- **Projectiles** carry `BodyKinematics`, so `portal_transit` sees them too. But
  `step_projectiles` also teleports them instantly (`try_projectile_portal_transit`), and
  whichever road sees the crossing first wins. They record no `SweepSample`, and their visuals
  are never drawn split.
- **Ground items** teleport the moment they touch the opening
  (`portal_teleport_free_bodies`). They open no carve, raise no `PortalBodyTransited`, have no
  cooldown, and are never drawn split.

## Steps

- **S1. The seam lab: measure first.**
  - **A gauge prop:** an authored PNG with a grid and corner marks in colours that appear
    nowhere else in the lab. It is a ground item that can be thrown.
  - **`capture_scene` flags:**
    - one that launches the player at a velocity;
    - one that spawns an item at a point with a velocity.
  - **A scenario matrix** (`scripts/portal_seam_lab.py`, not in the default suite):
    - movers: the robot, the gauge, a projectile;
    - pairs: floor↔floor, floor↔wall, wall↔wall, slope↔wall, a moving plank, a spinning plank;
    - speeds: slow, medium, fast.
  - **Metrics, per frame:**
    - **Conservation:** the gauge's visible pixel area, which must not dip or double at the
      crossing.
    - **Continuity:** the grid's lines on both sides of the seam.
    - **Window pops:** `portal_cone_trace.py`.
  - The baseline is taken before S2.
- **S2. One transit for every mover.**
  - A typed adapter per body representation runs the same machine, with the same
    `PortalTransit` and cooldown components, the same carves and the same
    `PortalBodyTransited` event.
  - Projectiles drop the inline teleport and record a `SweepSample`. Ground items straddle
    and cross like bodies.
- **S3. Drawn split.** Projectile and item visuals are cut at the seam, as the body pieces
  are.
- **S4. Re-measure, then fix what the numbers show.** Commit the lab's report.

## What landed (2026-10-16)

- **The seam lab.**
  - `scripts/portal_seam_lab.py` throws the gauge (renderer target `seam_gauge`, a
    magenta and blue grid) into each `portal_lab` pair at three speeds. The throw is aimed
    ballistically into the opening.
  - The films are made with `capture_scene --tick-per-frame`: one sim tick per shot, with the
    clock held while a shot is read back.
  - For the first crossing, the lab reports how much of the gauge is drawn against the whole
    gauge (`dip` / `double`). It aligns frames to the sim through
    `ambition_portal::transit_trace`, and reads what the cutter drew from
    `ambition_portal::piece_trace`.
  - It is not in the default suite.
- **One transit.**
  - Free bodies (`FreePortalBody`, today `GroundItem`) run `portal_transit_free_bodies`: the
    same machine as kernel bodies, with the same transit and cooldown components and the same
    event. They open carves by the same rule (`body_opens`, `publish_free_body_carves`).
  - Projectiles no longer teleport inline. They record a `SweepSample`, and `portal_transit`
    takes them.
  - A body that straddles a pair keeps both ends carved, so a shot taken this tick does not
    meet the exit's wall.
- **Drawn split.**
  - Item and shot visuals get the seam facts (`publish_transiting_drawn_bodies`).
  - `PortalBodyView::body` names the sim body, because an item's visual is made again each
    frame.
  - Ground items draw at `WORLD_Z_DUMMY - 0.2`, over the portal frame and window.
  - A sprite with a stated size is cut without its main-world image (`sprite_frame_basis`).
- **No blank frame.**
  - A piece is not drawn on the frame it is first made, so the first body cut in a session
    vanished for one frame. Characters were affected too.
  - Pieces are now drawn from the frame a body touches a portal, and its sprite is hidden only
    once its pieces were drawn the frame before.

## Measured (2026-10-16, `high`, windows off)

| | before (teleport) | after |
|---|---|---|
| A floor↔floor | jumps 225 px in a frame | 0.96 / 0.96 / 0.99 of the gauge drawn (slow / medium / fast) |
| D floor↔wall | jumps 230–350 px | 0.94 / 0.93 / 0.92 |
| E slope↔wall | jumps 400–480 px | 0.93 / 0.93 / 0.92 |
| B wall↔wall | jumps ~200 px | 0.76 / 0.86 / 0.67 |
| door (thin wall) | — | ~0.75 |
| F moving plank | jumps ~300 px | 0.47 / 0.71 / 0.76 |
| G spinning plank | jumps ~250 px | 0.53 / 0.30 / 0.39 |

No crossing vanishes, and no frame falls back to a whole copy.

## Open

- **Cut pieces sample the texture differently from the sprite.** A crossing gauge loses its
  dark grid lines for a few frames; the clip material's sampling and sizing need checking
  against the sprite renderer's.
- **A plank's art is drawn over what comes out of its portal.** Planks draw on the front layer
  (`WORLD_Z_PLAYER + 6`). It is seen over the gauge in the F and G films; how much of their
  dip it accounts for is not yet measured.
- **Labels and the wall pairs.** The portal labels and the wall pairs (B, the door) cover part
  of a crossing gauge; still to be traced.
- **Velocities beyond 900 px/s, and characters and shots in the matrix.** The lab throws only
  the gauge today.
