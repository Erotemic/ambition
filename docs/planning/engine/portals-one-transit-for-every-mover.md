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

## Review fixes (2026-10-16)

A source review of S1-S4 found these. Each has a test in `ambition_portal2d`
(`body_contract_tests.rs`), the projectile tests or the content portal tests.

- **A shot's leg ends at a portal.** The stepper resolved walls and bodies along the whole
  leg in the room the shot was leaving, and the transit took it a tick later. Contacts now
  stop where its centre enters a paired opening (`first_portal_crossing`).
- **A held item is not a free body.** `FreePortalBody::Presence` says whether the body is in
  the world. A held item opens, crosses and straddles nothing, and its straddle ends.
- **A crossing that skipped `Begin` straddles the exit**, so it is drawn cut as every
  crossing is.
- **A straddle ends when the last pair closes.** The transit returned early with no portal
  placed, so `PortalTransit` stayed and kept withholding wall verbs.
- **A free body has a sweep** (`FreePortalBody::last_step`; a ground item records the step
  it moved), **and a closing plane pushes it clear** (`evict_free_straddlers_on_portal_change`).
- **Closing planes are taken in a fixed order** (room, then channel), not hash-map order.
- **Host depth is the host's.** Measured through moving blocks where they are; a portal on a
  bare line (a plank) has depth 0, so the rescue does not take a body under the plank.
- **One host pose a tick** (`PortalSet::Pose`). The platforms advance
  (`MovingPlatformsAdvanced`), the hosted apertures take their pose, the links resolve, the
  carves are cut, and then the bodies integrate. Before, the carve was cut from the last
  tick's pose: on F it lagged the plank by 0.54 px and 0.4° (`a_moving_portal_is_carved_where_it_is`).
  The lab after it (`low`): every pair crossed, gauge dip 0.88-0.995, shot 0.92-0.985.

## The snag, and a cone that eases out behind you (2026-10-16)

Jon: a body completes a crossing only if all of it passes through the opening; where part
would go through and part around, it snags on the ends, like a rope.

- **The rule** (`snag.rs`): a body that straddles a portal cuts the portal's plane along one
  segment, and that segment must be in the opening. Past an end the plane is the host, from
  either side, before the body's centre crosses and after (the exit's plane is the same seam).
  A body that came at the plane from one side goes back to it along the normal; one that slid
  along the plane goes back to the end. It is a no-worse rule: a box that already cut the plane
  past an end (its feet standing into a slope beside the opening) is held where it was, not
  pushed, so the kernel's feet rule on slopes does not jitter.
- **Where:** `PortalSet::Frame`, after eviction and before transit, for kernel bodies
  (`snag_straddlers`) and free bodies (`snag_free_straddlers`). A body is read whether or not it
  straddles yet: one step can take it across the plane.
- **Measured on E:** the robot sunk in the slope's opening and walking right is held at the end
  (x stays at 2974.9 while it holds right), and goes through only when it fits. Walking up the
  slope past the end is unchanged. The lab is unchanged on every pair.
- **Later:** the rule is for a box. A physics body answers the same question with its own
  contact against the plane beyond the ends.
- **Cones:** a viewer turned away from a portal keeps its window within `turned_away_reach`
  (40 px, body edge to the opening), fading with the distance, so where a body came out eases
  out as it walks away.

## Crossings begin from the face (2026-10-16)

Jon's frames (`screenshots/frame-step`, ticks 4786-4824): the robot came up through E's slope
from behind, stood with its feet behind the plane past the opening's lower end, and was taken
at tick 4824, its centre 0.1 px past the plane. Three rules now:

- **The back of a pair** (`PortalBackside`, rollback-registered, schema v353). A body not
  crossing that cuts a paired opening's plane with its centre behind it is on the back of
  that pair, as a body coming up through a one-way platform is. While it is, the pair does
  not begin its crossing, open its hole for it, snag it, take it, or show it a window. It
  is cleared once the body is wholly past the plane. So a floor portal on a one-way platform
  can be jumped up through, and then fallen through once cleared (measured on E: rising through
  stays `Idle`; rising past it, it begins at tick 42 and lands through at 60).
- **Whole, or not at all** (`fits_through`): every transfer (centre crossing, rescue, sweep)
  asks that where the body cuts the plane is in the opening.
- **The snag's no-worse rule** now holds the part past an end on its side: it may go no deeper
  into the side it is not on (it could sink before, which is how tick 4824 happened).

Frame 4810 drew only the robot's feet: the crossing it began from behind cut the sprite at the
seam. A body on the back has no crossing and is drawn whole (measured in the capture).

## A crossing hands the window over whole (2026-10-16)

Jon's frame-step capture (ticks 10555-10585, the looping corridor c146/c147: two ends that
face each other across 256 px of floor). On the crossing frame the exit window eased in from
nothing, so the real scene past the exit showed for frames before the picture of where the
body had been came back. Two causes:

- The takeover asked the eye whether it came through (`came_through`: behind the partner's
  plane). Ends that face each other leave the eye in front of both after a crossing, so it
  said no. The window of the end the body crossed into (`PortalViewer::crossed_into`, from
  the sim's `PortalTransit`) takes over now.
- A window that takes over eased in from its partner's blend. It is the picture of where the
  viewer just was, which was drawn whole, so it takes over whole (`eased_blend`).

Measured (`capture_scene portal_lab player`, `high`): the exit window covers the corridor on
the crossing frame.

## Open

- **The foreground parallax layer** (z 45, in front of the play) was copied into every
  capture, so a window drew it twice: the far side's copy inside, the near side's over it.
  Captures draw the backdrop only now (`seen_through_a_portal`). The foreground's place is
  the camera's place in the room, so a 256 px camera jump through a pair moves it ~49 px:
  still a pop at a crossing, and not yet decided (ease the parallax camera across a portal
  jump, or leave it).
- **`capture_scene` cannot measure a crossing's background.** It pins its camera to a
  player snapshot after `camera_follow` and without the portal camera continuity, so at the
  c146/c147 crossing its camera moved 226.4 px, not 256: every backdrop patch is 66 screen
  px off, at every depth. The host view now samples the pinned camera (the window anchors
  agree with the frame), but the continuity is still the game's alone. Jon's frames, which
  have it, show the left of the screen unchanged across the crossing.
- **A strip at the screen edge** on the crossing frame: the exit window's capture does not
  reach the new edge of the screen for one frame (black at `high` in the capture).
- **Two ends that face each other** show a picture in a picture; one level of recursion leaves
  a strip at the far edge that the next frame shows as a window.

- **The robot's launch on the wall and slope pairs.** Hands-off, the kernel's air stop
  assist (3750 px/s²) stops a 150 px/s launch in about 3 px. The lab should hold the move
  toward the portal, or aim from a measured flight.
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
