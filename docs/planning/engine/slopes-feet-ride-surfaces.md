# Slopes: a body keeps its support (proposed 2026-10-11, Jon asked for it)

IN PROGRESS (2026-10-15). It was deferred on 2026-10-11 until after the
GNU-ton work; Jon lifted that on 2026-10-15, when the Math town's stair was
built of 64 invisible treads: "we now have several instances of ramps,
gnutons back, the cart ramp, and now the stairs in math town. we should make
sure that our physics implementation of them is not hacky and elegantly mix
game play on ramps efficient and smooth. or more generally angled platforms
and walls. this also means we also need to make portals work on angled
surfaces." The first proposal was for a slope mechanic in the movement
kernel; Jon on 2026-10-11: "It needs to be a smooth ride when we have
walkable slopes. We need to find the elegant solution."

## The defaults this work takes (2026-10-15; feel is Jon's, each is a knob)

- The foot span is the width of the box (the proposal's default, below).
  The picture is put down onto the surface under its middle.
- A walker's speed on a slope is its speed ACROSS the floor: up or down a
  ramp it covers the same ground per second as on the flat.
- A segment steeper than a walker's limit (default 50 degrees) is not ground
  for it. Below the limit it does not slide.
- At a crest a walking body stays on the ground (it is held down onto the
  surface on the far side within its limit). A jump leaves it. The momentum
  rider keeps its own launch rule.
- A stair is a slope: the art draws the steps, the body rides one line.

## The sites that move to it

| Site | Today | After |
|---|---|---|
| The Math town's switchback stair | 64 still one-way platforms | two one-way slope lines and a landing |
| The gate cradle's stairs | 24 still one-way platforms | a one-way slope line at each end |
| The cart of the wake room | 14 still one-way platforms | one line (phase C: it moves) |
| The giant gnu's back | 10 typed ledges a tick | one line from the art (phase C) |

## Portals on angled surfaces (the second track)

Read from the code, 2026-10-15 (an audit; not run). The frame math is
general already (`frame.rs`: points and velocities map through any normal,
and Alice's up-B tilts a portal 32 degrees in free air). What is cardinal:

- A portal's host: the gun's ray (`gun_projectile.rs` -> `raycast_solids`)
  hits only boxes and gives a face normal; it can not hit a chain. The LDtk
  `Portal` takes only up/down/left/right.
- `portal_fits` rounds the normal to an axis.
- The piece layer (`ambition_portal2d/src/pieces.rs`): `map_aabb`,
  `clip_halfspace`, `clip_to_aperture`, `straddles` and the carve all pick
  one axis.
- The extents and the capture box are boxes round a tilted face
  (`types.rs`); the view cone's source and the compositor are exact only
  for cardinal portals.

The order: (P1) the ray hits chains and gives their normal, the LDtk portal
takes an angle, `portal_fits` measures along the tangent; (P2) the pieces
are clipped by half planes of any normal and a carve on a chain cuts an
interval of it; (P3) the capture and the view are exact for a tilted face.

## What is true today (read from the code, 2026-10-11; not run)

- An ordinary body (the axis-swept model: the player, each NPC, each mount)
  has NO slopes. Its ground is the top face of a box, and nothing else.
- "On the ground" is found again each tick. Nothing remembers what the body
  stands on, and nothing holds it to the ground. A body that leaves a step
  is in the air until gravity brings it down.
- So a "ramp" is a stair of one-way boxes: the cart of the wake room (14
  steps of 4), the back of the giant gnu (10 steps of 12 to 27). The jumps
  come from that:
  - Up: the feet are put on the next step in one tick (a pop of 4 to 8).
  - Down: the body falls from each step and lands again. Each landing is a
    `Landed` edge: a sound, dust, and a tick of "in the air".
  - The body stands at the height of the step under its uphill EDGE, not
    under its middle, and which step wins depends on the order of the boxes.
  - Faster than about 14 per tick, the next step is too high over the feet:
    the body runs under the ramp.
- A second mover has real slopes: surface momentum (`SurfaceChain`, the
  Sanic demo). An ordinary body can not use it. It has its own jumps: at
  each vertex of a chain the middle of the body moves by `2 r sin(θ/2)` in
  one tick (about 5 at 11 degrees, 18 at 45, for a radius of 24), and its
  direction and its roll turn by θ in that tick. A launch at a crest is
  decided by the angle of ONE vertex, so the same hill launches or holds by
  how many points it was drawn with.
- Only boxes can be ground that moves. A room's chains are fixed. A mount
  can give boxes each tick; it can not give a slope.

## The idea (revised 2026-10-11 after a review by GPT, which Jon sent)

**A body on the ground remembers its support. The support is a surface with
an identity that lasts. The shape of the body says how it sits on the
surface. The policy of its mover says how it travels and when it lets go.**

The first draft said more than that: that the middle of the feet is ON the
line, and that each mover moves one number `s` along it. The review showed
two faults in that, and both are right:

- A body has a width. A box 32 wide with the middle of its feet on a slope
  of 30 degrees has its uphill corner 9.2 under the slope
  (`w / 2 * tan(θ)`). The draft did not say if that is allowed.
- The kernel has three movers (`AxisSwept`, `SurfaceMomentum`,
  `AdhesiveCrawler`), not two, and the third has its own chain attachment
  (`CrawlAttachment::Chain`). What they can share is the surface: its
  identity, its frame, its motion, the loss of it. Not the law of motion.

### What is shared: the support

- `SupportAttachment { surface_id, location }` is rollback state. The id
  lasts: not an index into a list that is made again each tick (today's
  `SurfaceRef::Chain(usize)` / `Block(usize)` and the crawler's chain
  index). Blocks have `GeoId`; chains need the same, in the scope of their
  live room. A block that a portal carves into parts must resolve to the
  part that is still under the body, or to no support.
- A surface is geometry in its own coordinates, and a pose. A surface that
  moves changes its pose; the kernel reads the displacement the surface
  DID make in the tick (not the velocity it asked for: a mount can be
  stopped). Rigid motion first. A surface that changes shape needs a
  `location` that names a place of the material, not an arc length; that
  waits for a mechanic that needs it.
- From the attachment and the body's shape the kernel derives a contact:
  point, tangent, normal, the surface's velocity there, and if the body may
  stand there at all.
- Where a surface ends and exactly ONE other surface that the body may
  stand on starts at that place, the support goes to it. Where there is a
  choice, an authored junction decides (`SurfaceJunction`): two lines that
  cross are not joined by being at one place.
- A supported body is swept along the path it does travel, cut at the
  joints of the surface, with the sweep and contact rules of today. Not a
  sweep to the side and then a sweep down: on a slope that is not the path,
  and a low ceiling or a wall on the way would be missed or struck falsely.

### What each mover owns: how it sits, how it travels, when it lets go

- **A walker (the ordinary body).** OPEN, AND THE FIRST THING TO DECIDE:
  how its box sits on a slope. The proposal is a FOOT SPAN: a length along
  the bottom of its box, and the body rests on the highest point of the
  surface under that span.
  - With the span as wide as the box, no part of the box is under the
    slope, and a flat box top is the same rule (a body with any part over
    it stands on it, as today). But the middle of the body is over the
    slope by up to `w / 2 * tan(θ)`: 10 for the player (28 wide) on the
    giant's back (36 degrees). The picture can be put down onto the
    surface under its middle; the body can not.
  - With a narrow span the feet look planted, and the uphill corner of the
    box is under the line of the slope by the difference.
  - So the span is a property of a body, and its default is the width of
    the box.
  Its `location` is where the middle of its span is across the floor, not
  an arc length: a surface a walker can stand on has one height at each
  place. A segment steeper than its limit is not ground for it; if the
  surface is the edge of solid earth, that segment is a wall for its box.
- **A rider (Sanic).** A circle. Around a sharp convex corner its middle
  goes on an arc about the corner, one radius out: the offset path of a
  circle, not the line itself. Its launch at a crest is from the load on
  the surface, `g_press - v² κ`, with the curvature `κ` taken over a
  stated length in world units, so the count of points does not change it.
  This is its own improvement, not a condition for a walker on a ramp.
- **A crawler.** It holds to a surface on purpose, walls and ceilings
  also. It keeps its law; it takes the shared support.

### Three things that "smooth" means, and they are not one

| What | What gives it |
|---|---|
| No jump of place | The place is a continuous function of the support |
| No jump of the direction of travel | A tangent that is continuous, or a path made smooth |
| No jump of the picture's turn | A bounded rate of turn |

A walker needs the first, a grounded state that does not break, and a true
sweep. The turn of its picture is apart from that.

## The order of the work

| Phase | What it gives |
|---|---|
| A | The support that lasts: ids for blocks and chains in a composed world (carved blocks also), the contact derived from it, and the decision of the foot span. |
| B | A walker on fixed slopes, WHOLE: floor to ramp to floor with no tick in the air, the true swept path, a jump, a drop through, a hit that knocks it off, the events of the ground. Not a walk on a lone chain. |
| C | Slopes that move as a rigid thing: chains in the room's overlay with an id and a pose, the carry on both axes, the velocity at release. The cart's ramp and the giant's back become one line each. The fault of `mount_backs_are_ground` (below) is fixed here, with the displacement the mount did make. |
| D | The rider and the crawler on the shared support; the circle's path at a corner and the launch from curvature. |
| Later | Flat box to flat box by the same support; surfaces that change shape. Only with a use for them. |

"A room with no chain runs today's code byte for byte" is a check for the
rollout, not a law. The game is not released: a rule that is better and
keeps what the play needs may replace the rule of today.

## What must pass before the wake room or the giant use it

- Up and down a slope of some short segments: no jump of place, no `Landed`
  edge after the first.
- A wide box on a slope of 30 degrees: no part of it in the earth that it
  is not allowed to be in (by the foot span that was chosen).
- Flat box to ramp to flat box: supported all the way.
- A sprint over some segments in one tick: no tunnel, the support kept.
- A jump, a drop through, a knock-back from a slope: the right release,
  velocity, coyote time and jumps.
- Up a slope into a wall, and under a low ceiling: stopped where the swept
  box does touch.
- A ramp that rises and goes to the side: the body on it follows on both
  axes with no drift.
- A mount that stops, turns back, or is gone: carried right, or let go; no
  support that names a surface that is not there.
- The order of the surfaces in the composed world changes: the attachment
  resolves to the same surface.
- A rollback over the taking and the loss of a support: the same state and
  the same events.
- A block with a portal's aperture carved in it: the body is on the part
  that is left, and is not held over the part that is gone.
- The same Sanic hill drawn with different counts of points: the same
  path and the same launch, within a stated tolerance.
- A room with no chain: the cost of a tick is still acceptable.
  (`SurfaceChain::frame_at` scans from the start of the chain. For a few
  riders that is fine; for each body each tick it needs a measure, and
  may need a cursor or a table of lengths.)

## What it costs, and the risks

- A change in the movement kernel, in the part each body runs each tick.
- A rollback schema bump in phase A or B (the attachment), with its three
  baselines.
- Feel, which is Jon's: the foot span; how fast a body goes up and down a
  slope (its speed along the surface, or across the floor); if it slides
  on a steep one; if a run off a crest leaves the ground.

## Found on the way (not fixed)

- `mount_backs_are_ground` gives a block `velocity: mount.vel`. A body's
  `vel` is per second; a block's `velocity` is per frame and is added to a
  place. The giant does not move, so nothing shows. A mount that moves
  would carry what stands on it 60 times too far. (Read from the two
  definitions by two readers; not run. The fix belongs to phase C: the
  displacement the mount did make in the tick, not `vel * dt`.)
