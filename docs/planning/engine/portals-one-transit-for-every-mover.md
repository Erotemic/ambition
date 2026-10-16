# One portal transit for every mover, measured by a seam lab - an engine plan

**Status:** S1–S3 BUILT 2026-10-16; S4 in progress (see "S4" and "Open"). It follows
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

## S4 (2026-10-16)

- **A turned item no longer snaps upright.** A pair that turns what it carries (a plank, a
  floor to a wall) drew the far-side piece turned, then put the item back upright on the
  transfer: a 45° snap on G. Ground items now carry `ActorRoll`. A crossing adds the pair's
  turn, as it does for a body, and the item rights itself at the actor rate
  (`right_ground_items`, `ease_roll`). Upright is the screen's, as items are drawn.
- **A shot no longer vanishes after it crosses.** The visibility resolver released its hide
  without restoring `Visibility`, on the rule that every body has a per-frame visibility
  owner. A shot's visual has none, so it stayed hidden for the rest of its flight, on every
  pair. The resolver now restores a hide that nothing wrote over.
- **The lab:**
  - It films at `low`. At `high`, bloom spreads a lamp's or a portal line's light over the
    gauge's colours, and that measured as a dip: F and G read 0.26–0.83 at `high` and
    0.97–0.98 at `low`.
  - It counts a blend of the gauge's two colours, so a whole gauge holds within 3% from
    frame to frame (8% before).
  - It sends the gauge as a shot (`--movers shot`, the projectile look `seam_gauge`) and
    launches the robot (`--movers robot`, `capture_scene --player-at X,Y@T:VX,VY`). The
    robot is measured against a clean plate.
  - Kernel bodies log `ambition_portal::transit_trace` as free bodies do.

Gauge thrown, `low`, the share of it drawn while it crosses (least / most):

| | slow | medium | fast |
|---|---|---|---|
| A floor↔floor | 0.97 / 1.05 | 0.98 / 1.02 | 0.98 / 1.03 |
| B wall↔wall | 0.90 / 1.06 | 0.94 / 1.05 | 0.88 / 0.96 |
| C floor↔ceiling | 0.96 / 1.03 | 0.92 / 1.02 | 0.94 / 1.01 |
| D floor↔wall | 0.99 / 1.01 | 0.97 / 1.02 | 0.99 / 1.02 |
| door | 0.91 / 1.06 | 0.94 / 1.07 | 0.89 / 1.05 |
| E slope↔wall | 0.93 / 1.03 | 0.93 / 1.01 | 0.98 / 1.03 |
| F moving plank | 0.98 / 1.03 | 0.98 / 1.02 | 0.98 / 1.04 |
| G spinning plank | 0.97 / 1.03 | 0.97 / 1.04 | 0.91 / 0.98 |

Shot (the gauge's look, 300 px/s) and robot, `low` (least / most):

| | shot | robot slow | robot medium | robot fast |
|---|---|---|---|---|
| A floor↔floor | 0.95 / 1.03 | 0.96 / 1.00 | 0.93 / 0.98 | 0.97 / 1.00 |
| B wall↔wall | 0.92 / 1.00 | missed | missed | 0.93 / 1.19 |
| C floor↔ceiling | 0.98 / 1.07 | 0.98 / 1.05 | 0.97 / 1.04 | 0.98 / 1.01 |
| D floor↔wall | 0.96 / 1.06 | 0.99 / 1.01 | 1.00 / 1.03 | 0.99 / 1.01 |
| door | 0.98 / 1.04 | missed | missed | 0.93 / 1.13 |
| E slope↔wall | 0.99 / 1.04 | missed | missed | missed |
| F moving plank | 0.96 / 1.01 | (plate) | (plate) | (plate) |
| G spinning plank | 0.98 / 1.02 | (plate) | (plate) | (plate) |

"missed": the launch did not carry the robot through the opening (on E it stood in the
plane). "(plate)": the plank moves, so the clean plate does not hold and the numbers are
not of the robot.

## Open

- **The robot's launch on the wall and slope pairs.** The aim assumes plain ballistic flight
  at the kernel's gravity, and the robot's air movement is not that. The lab should aim
  from a measured flight, or carry the robot by its own movement.
- **A measure of the robot on a moving pair.** It needs a plate that moves with the plank,
  or a robot drawn in colours of its own.
- **The wall pairs (B, the door) lose up to 12% of the gauge.** It is less than before, and
  not yet traced.
- **Velocities beyond 900 px/s.**
- **Closed:**
  - Cut pieces sampling the texture differently: at 3× on film the grid runs across the seam
    unbroken (A). The lost lines were the old colour test, which counted only pure cells.
  - A plank drawn over what comes out of it: a plain plank draws at `WORLD_Z_BLOCK + 4`,
    behind every body. The F and G dips were bloom.
