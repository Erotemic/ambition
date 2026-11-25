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

**Current continuation (2026-11-22):** SUP1 below is planned from static
source review at `00bf92c22142`, after planning commit `e389726ea716`.
GNU's drawn support already changes length. The rigid-only motion record
cannot describe both its previous and current shape. The earlier condition
that shape change waits for a customer is met. Preserve the completed
walker, mount-displacement and tilted-ledge work below. No new engine run,
test, render or performance result is claimed.

**Curves and grounded appearance (tentative, 2026-11-22):**
[Curved surfaces and local gravity](curved-surfaces-and-local-gravity.md)
separates the existing hill-contact goal from optional curve authoring,
continuous local-frame walking and a static planetary proof. The source
already has curve-to-chain preparation. It does not yet establish ordinary
walking under continuously turning gravity. None of CV1–CV5 is selected
in the queue by that investigation; SUP1 retains its current selection.

## The defaults this work takes (2026-10-15; feel is Jon's, each is a knob)

- The foot span is the width of the box (the proposal's default, below).
  The intended picture placement is onto the surface under its middle.
  Static review at `00bf92c22142` finds no selected-support input in the
  current body pose/quad placement road. [CV1](curved-surfaces-and-local-gravity.md#cv1--ground-the-picture-from-accepted-support)
  states the missing handoff, its bounded visual offset, and the separate
  choice of root tilt or terrain feet. This sentence is a goal, not a
  completed visual acceptance receipt.
- A walker's speed on a slope is its speed ACROSS the floor: up or down a
  ramp it covers the same ground per second as on the flat.
- A segment steeper than a walker's limit (default 50 degrees) is not ground
  for it. Below the limit it does not slide.
- At a crest a walking body stays on the ground (it is held down onto the
  surface on the far side within its limit). A jump leaves it. The momentum
  rider keeps its own launch rule.
- A stair is a slope: the art draws the steps, the body rides one line.

## Built (2026-10-15): a walker on fixed slopes

`crates/ambition_platformer2d_core/src/walkable_lines.rs`, called from the
walker's step after its box sweeps (`movement/integration.rs`). It is
stateless, not the lasting attachment proposed below: what a body stands on
is found again each tick, and the line it stood on at the start of the step
is what it keeps to. The lasting attachment comes with lines that move
(phase C).

- A chain segment within 50 degrees of up is a one-way floor for a walker;
  it stands on the highest point under its box; it is held on the line down
  a ramp, over a crest and off the line's end onto a floor below, with no
  tick in the air.
- Forks: a body keeps to the line it stands on while that line goes on; it
  steps up only onto a line ahead of it; at a fork (its ground goes on) it
  takes a line whose `boarding` is `up` only while holding up. A stair
  beside a street is `up`, so the street is walked past its foot. Walking
  off the end of a terrace onto a stair down takes the stair.
- A chain with `visual: "none"` is not drawn (the room's art draws it).
- Tests: `movement/tests/slopes.rs` (ramp up and down with no landing, a
  fall onto a ramp, a jump up through one, a drop through, a stair passed
  and boarded, a switchback both ways, a terrace edge to a stair down).
- Moved to it: the gate cradle (one line) and the Math town's switchback
  (two lines and a parapet). The wake room's cart stands still, so its ramp
  is one line too (its first step stays, as the holder of the ramp's
  picture).

## Built (2026-10-15): lines that move (phase C)

- The room's collision overlay takes lines (`FeatureEcsWorldOverlay::chains`),
  composed after the room's own by name.
- A mount states a `slope` (points about its centre, mirrored with its
  facing); while it is ridden it contributes it as a hidden line. The giant
  gnu's back is one line from the rump to the shoulders in place of ten
  steps a body hopped (`gnu_ton_fight::a_body_walks_up_the_giants_back_to_his_shoulders`).
- A line or block of a mount moves by the displacement the mount made in its
  last step (`SweepSample`), not by its `vel` (per second: it carried a
  rider 60 times too far).
- A body that stood on a moving line is carried by that line's displacement
  before its own motion; which line it stood on is measured with each line
  moved back by its displacement (`walkable_lines::stood_on_line`). Kernel
  test: a body stands still on a ramp that moves both ways and ends where the
  ramp went, and walks up it, with no tick in the air.

## Built (2026-10-26): ledges that tilt, and lines that lift and turn

Jon: "we could let a slightly tilted platform have a ledge grab, no? or have
a tilt knock a ledge grabber off?"

- A LEDGE is a lip, the way its standing surface goes in from it, and the
  side face (`ledge_grab::Ledge`). A block's top corner is the level case;
  the free end of an open one-way line (`ledge_grab::chain_ledges`) is a
  ledge that tilts as the line does. One probe for both
  (`probe_ledge_grab_in_frame`): the anchor is at the lip, the climb target
  stands on the tilted surface. A surface tilted more than
  `LEDGE_TILT_LIMIT_DEG` (20) is no ledge, and a moving line that tilts past
  it under a hanging body knocks it off (`LedgeCarry::KnockOff`); under it,
  the line carries the hanger with its lip (its move and its turn).
- A line's motion is a move and a turn about a pivot (`SurfaceChain::turn`,
  `pivot`): the giant's drawn shoulders lift and tilt as the art does.
  Kernel test: `a_body_rides_a_plank_that_lifts_and_tilts`.
- A frozen body (hitlag, a tick with no time) is still carried by the line
  or block it stands on (`movement::carry_frozen_rider`): a rider was left
  under a lifting shoulder for the length of its own blow's hitlag and fell
  through it.
- A one-way BLOCK that rose toward a body this tick is crossed from where it
  was (`one_way_landing_from_previous_feet` takes the block's velocity).

## SUP1 — Keep a changing support's material motion

**Invariant:** the surface at the start of an interval equals the surface
published at that start. Its geometry at contact time, material-point
carry, point velocity and swept bounds describe the same accepted motion.
The shape at the end does not determine the shape or travel before it.

### Source evidence and the missing fact

The current customer is `npc_giant_gnu` and its whole-giant forms in the
[character catalog](../../../game/ambition_content/assets/data/character_catalog.ron):
drawn shoulders plus an eleven-point slope with `slope_follows: "hump"`.
The [sprite author](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/gnu_ton_boss/giant_art.py)
publishes named endpoint anchors. These are simulation metadata, lowered by
[`surfaces_of`](../../../crates/ambition_sprite_sheet/src/surfaces.rs), not
coordinates read from a rendered picture.

| Boundary | Current fact | Fact lost or assumed |
| --- | --- | --- |
| [`SurfaceTable::segment`](../../../crates/ambition_characters/src/actor/surfaces.rs) | Interpolates both endpoints between frame centers; loops or holds the last frame | A sample alone does not retain the intervening spans or selected source phase |
| [`BackLedge::slope_line`](../../../crates/ambition_mount/src/lib.rs) | Maps the typed polyline to the current endpoints by a similarity, including scale | The current shape can change length |
| `DrawnLedge::motion` and `mount_backs_are_ground`, in the same file | Keep midpoint displacement, angle change and pivot | Previous length and local material-point travel are discarded |
| [`SurfaceChain` and `MaterialCollider`](../../../crates/ambition_platformer2d_core/src/world.rs), [`material_lines`](../../../crates/ambition_platformer2d_core/src/walkable_lines.rs), [`line_ledge_carry`](../../../crates/ambition_platformer2d_core/src/ledge_grab/runtime.rs) | Reconstruct rigid previous geometry, carry, bounds and velocity | They cannot recover the discarded change; a length-only change even reports `moves() == false` |

The existing mount fixture
`a_slope_that_follows_the_drawing_is_where_its_ends_are_drawn` changes its
endpoint vector from `(200,-60)` to `(200,-80)`. The lengths are
`sqrt(43600)` and `sqrt(46400)`. No rigid map can match both. The fixture
checks current placement, midpoint displacement and turn, so it does not
establish previous geometry or material-point carry.

A smaller exact witness is contraction from `[-20,20]` to `[-10,10]` on a
horizontal line at a fixed center. Published displacement and turn are
zero. The time-zero query therefore sees the short current line, and the
swept bounds omit the old outer parts. A point three quarters of the way
along the source material moves from x=10 to x=5; rigid carry keeps x=10.
This is a source counterexample, not a reproduced fall or measured GNU drift.

### Retain a source place and a motion interval

Keep the current endpoint-similarity gameplay shape in the first slice.
Use immutable local support geometry and one prepared interval that states
its motion. A place on that material is a source segment plus a parameter
`u`, with `0 <= u <= 1`, in the stable surface occurrence. World arc length
is a measurement of the current shape; it is not a stable material address
when that shape stretches. A walker still uses its foot-span rule to find
contact and change its place. This does not pin a walker to the floor.

For one moving segment, with `t` measured in the named shared comparison
clock and endpoint rates expressed in that clock:

```text
point(t,u) = (1-u) * a(t) + u * b(t)
velocity(t,u) = (1-u) * a_rate(t) + u * b_rate(t)
```

The velocity is at fixed material `u`. Compose it with the host's accepted
point motion once. A walker's own change of `u`, a crawler's adhesion and a
ledge release are separate policies. Reuse the point-derivative rule in
[body anchors A3](body-anchor-bindings.md); a point helper alone does not
define a swept edge or its bounds.

The native support sample must supply all of these from one law:

- geometry, tangent and outward normal at a declared time;
- a source location for a selected contact and its material-point motion;
- carry between two times and velocity at the actual contact time;
- conservative bounds for the whole admitted interval;
- an explicit cut when continuous correspondence ends.

Keep logical source endpoints and segment names when reflection or winding
changes their query order. A left-to-right sort is not material identity.
For a carved segment, retain the source parameter range. A transient
coordinate inside one operation need not wait for all persistent support
attachments. A coordinate retained across publications must use the lasting
surface/occurrence rules already planned here, never a rebuilt list index.

### The source phase is not the world clock

[`follow_drawn_ledges`](../../../crates/ambition_platformer2d_runtime/src/drawn_ledges.rs)
samples `PinnedRow.elapsed + 1/60` in Progression for the next overlay.
[`PinnedRow`](../../../crates/ambition_sprite_sheet/src/character/anim.rs)
states that `elapsed` is the content owner's clock. The current
[GNU conductor](../../../game/ambition_content_modules/src/gnu_ton/conductor.rs)
advances walking `stride` by `abs(giant_vel.x) * dt / stride::SPEED` and pins
that value. Other rows hold, restart or run at another rate. Replacing the
constant with world `dt` would still predict a source phase the owner did
not select.

The producer must give the resolved row/surface occurrence, its source
coordinate over a named simulation interval, and the host's accepted
motion for that same interval. A held source phase has zero local shape
motion even while the host moves. Do not reconstruct the conductor's
phase law in the support reader or add a free-running support clock.

Make the rate conversion explicit. For fixed material `u`, a source shape
`q` at phase `phi(T)` composed with accepted host map `H(T)` gives:

```text
P(T,u) = H(T)(q(phi(T),u))
dP/dT = host_point_velocity + H.linear * (dq/dphi * dphi/dT)
```

Here `host_point_velocity` is the derivative of the host map at that
material sample, including admitted turn or scale. The phase producer
supplies `phi(T)`; an endpoint rate already resolved in `T` gets no second
pace factor. A source phase held still contributes zero local deformation,
while host carry can remain nonzero. Contact and portal readers use the
[same comparison-clock contract](portals-are-moving-frames.md#one-chronology-does-not-give-rates-the-same-unit).
Native action clocks and frozen-body policy stay with their existing owners.

For SUP1, preserve the table's stated interpolation. Split at its known
frame-center boundaries, loop spans and terminal hold, and at boundaries
of the source phase law. Retain unwrapped source travel: equal phases after
a full loop do not prove no motion. Within a table span, endpoint positions
and the fitted similarity coefficients are affine in source phase. Vertex
paths are affine in simulation time only when the source-phase mapping
and host translation are affine over that same interval. Otherwise retain
the declared curve and its conservative bound. Rigid host and portal motion
can also add curvature; use their accepted legs rather than treating the
composed path as a chord. The first source case can use piecewise-affine
phase spans; an unsupported law must not silently select that fast case.

A row change or occurrence restart, disappearing surface, facing/parity
change or root relocation is a cut unless its owner supplies a continuous
transition.
Two valid endpoint samples do not prove the intervening similarity is
invertible. Check for a collapsed segment or zero scale within each span;
report a source error during preparation where it is known. At a live cut,
use native support-loss/admission policy. Do not turn a reset or missing
sample into a large finite velocity, hold an absent floor, or silently use
rigid history as a fallback.

The current schedule publishes mount geometry in WorldPrep from the last
`SweepSample`, then samples drawn surfaces in Progression. Name the cut and
make the host, source phase and geometry agree there before claiming
moving-GNU completion. A stationary-host proof can land first. A prior
host step plus a forecast source phase is not proof of one accepted tick.
[M1/M2](portals-are-moving-frames.md#bounded-delivery-packets) owns accepted
host history and chart changes; the native content/action owner still owns
source phase. SUP1 does not require a new global schedule or dependency graph.

### Keep the efficient cases

Static, translating and rigid producers retain direct native variants.
Select a variant from its complete admitted law, not equal endpoint poses
or a tolerance that rounds a real scale change to zero. Only a producer
with changing support geometry needs the new moving-edge path. Fixed
topology and piecewise-linear source endpoints are enough for this customer;
there is no general soft-body or nonuniform body-scale requirement.

Prepare source geometry and the interval once per changed support and
share them among contact, carry and ledge readers. Retain repeated source
travel compactly and visit its spans on demand under the native contact
work/refusal rules; do not allocate every crossed loop. Broad-phase bounds
must also have finite work: a prepared full-loop source envelope can bound
repeated cycles once, composed conservatively with host motion. Do not
enumerate every lap before the contact work limit takes effect. Keep
ordinary static and rigid samples fixed-size; add no per-rider point-vector
copy or global per-body solve. For a linear vertex span, endpoint bounds
contain the whole span; union the actual spans, including a shrink's old extent.
Curved composed motion must use a valid curved bound. Keep moving edges
inside the native material query and one-way rules, not a second solver
whose result a later rigid query can contradict.

Do not require a benchmark to establish the algebraic defect. Measure
preparation, candidate/contact work and allocations before a speed claim
or further cache. The existing translation contact implementation itself
allocates a region vector; this plan does not claim it is allocation-free.
Preserve the cost of unrelated static and rigid traffic.

### Delivery, deletion and acceptance

1. Add the bounded native support interval and source-coordinate queries.
   Extend the existing mount fixture to verify its true start and end
   geometry and the carry of a fixed source endpoint. Keep the producer's
   previous sample until all readers of its accepted interval finish.
2. Move the producer and its native readers together: prior support,
   walking/frozen carry, ledge endpoint carry, material contact, point
   velocity and swept bounds. Delete the changing-shape-to-rigid reduction
   and private inverse-rigid reconstruction for this case. Keep proven
   rigid producers on their direct path.
3. Through the native mount overlay, check one expanding/shrinking support
   with a contact and a ledge or frozen rider. Bounds must admit the old
   extent; contact time, motion and velocity must agree. Then use one real
   GNU source transition, including its selected source phase and host cut.
   This is the production seam the algebra-only fixture cannot see.

These are future acceptance cases. A local no-portal slice can land before
the whole persistent-attachment or portal program. P3/M3 consumes the new
kinematics for relative response; it cannot infer discarded source motion.
A portal-hosted or crossing support must also use P2's accepted interval
and admission. For a changing support admitted as a portal host, migrate
host attribution and refresh to the same source place and point trajectory;
the current world arc-distance attachment slides over stretched material.
The [host contract](portals-are-moving-frames.md#driven-aperture-motion-is-admitted-before-the-host-commits)
keeps aperture size/lift policy separate and requires refusal when that
capability or fit is unavailable. Include one stretching-line attachment
and a rigid control before claiming this conditional delivery complete.
Do not mark those cases complete from the local witness.
Preserve the existing frozen-body clock policy and GNU visual receipts.

### Landed (2026-11-25): the shape motion and its time-sliced readers

Delivery step 1 and the time-sliced part of step 2 are done.

**The law.** `SurfaceChain::shape_before` holds the line's points as the
tick began when its shape changed. Each point goes straight to its place in
`points`, which is the table's own interpolation within a span: a drawn
ledge's ends are interpolated linearly, and the slope's similarity
coefficients are affine in them. A place on the line is a segment and a
fraction `u`. The chain gains these queries:

- `place_before`, `travel_at` (the per-tick travel at fixed `u`) and
  `segment_at_time`;
- `segment_before`, which a rigid line answers by its inverse motion.

**Readers moved to the law:**

- carry (`carried`) and `moves`;
- the prior-support query at the start of the tick (`material_lines` at
  `time`, which sees the segment where it stood then);
- `lines_under`'s carried probe and its turn slack, which becomes the
  difference of the ends' travel;
- the line velocity and carry a support reports;
- `MaterialCollider::velocity_at` and `normal_at_time`;
- the ledge latch (the lip where that end began the tick) and the ledge
  carry;
- swept bounds, which add the start shape, so a shrink keeps its old
  extent;
- the navigation filter, which now uses `moves()`.

`clears_material_contact` keeps the continuous result for such a line.

**The producer.** `mount_backs_are_ground` publishes the start shape only
when a drawing changed (`DrawnLedge::changed`). A held frame and a
translating mount keep the rigid fast path, and nothing is copied per
rider. Both shapes come from one end order: `DrawnLedge::ends` chooses the
order once, left to right now, so an end keeps its identity when the
drawing turns it past upright.

**Witnesses:**

- `a_contracting_line_carries_its_material_and_keeps_its_old_extent` (core)
  covers the `[-20,20]` to `[-10,10]` case above. It checks carry 10 to 5,
  the bounds, support at the start but not now, and a rigid control.
- `a_slope_that_follows_the_drawing_is_where_its_ends_are_drawn` (mount)
  now checks the true start geometry, both lengths (`sqrt(43600)` and
  `sqrt(46400)`), and that every point is carried to its own place where
  the rigid fit falls short.

**Continuous sweeps (landed the same day).** `first_material_contact` (the
kernel, projectile and righting sweeps) casts a changing segment by its own
motion. `MaterialCastCells::against_deforming_where` puts each end in the
cell's chart with its own relative motion. `deforming_segment_cast` then
finds the first contact from two events:

- an end reaching the body, which is the existing point cast with that
  end's motion;
- a body corner reaching the segment between its ends, which is a quadratic
  in time. A circle body uses conservative advancement, whose step is the
  clearance over the faster end's speed, so it cannot pass a contact.

`MaterialCollider::displacement_at` gives the travel of the material under
a contact. The kernel's approach test, its carry and its contact's surface
velocity read it. The rigid fit is still published only for a cell cut by a
portal or carried on a curve, which this cast does not admit.

Witnesses (core):

- `a_deforming_segment_cast_with_equal_end_motion_is_the_translation_cast`
  compares 400 random configurations against the translation cast;
- `a_stretching_segment_meets_the_body_when_its_end_arrives` (contact at
  `t = 0.5`, where the rigid fit reaches `t = 1`);
- `a_tilting_segment_meets_a_body_corner_between_its_ends` (`t = 7/15`, and
  a circle);
- `the_sweep_meets_a_stretching_line_when_its_end_arrives`, through
  `World::first_material_contact`.

**Not yet moved** (MOVING-SUPPORT stays open on these):

- **Portal cells.** A cell cut by a portal or carried on a curve still
  reads the rigid fit (P2's accepted interval owns that case).
- **Source phase.** The source-phase / host cut (`follow_drawn_ledges`
  `elapsed + 1/60`) is unchanged.
- **The real GNU transition.** It has no witness in the running game yet.
- **Portals.** Portal hosting on a stretching line still slides by arc
  distance.

### Separate authoring decision: the whole curved hump

GNU's `_barrel_placed` stretches along the barrel's axis. The native hump
uses a uniform endpoint-fitted similarity. For a curved source line, these
can differ between the endpoints. Fixing temporal consistency does not
prove exact correspondence to the art, and that difference alone does not
prove the current gameplay shape is wrong.

If the full hump must follow the axial stretch, publish its support map
or vertices from the same authoring recipe that draws it. Two endpoints
cannot determine a general affine map. Keep this as a separate product
choice; do not infer missing deformation, read renderer transforms into
simulation, or scale the giant's entire collision body to repair a surface.

## The sites that move to it

| Site | Today | After |
|---|---|---|
| The Math town's switchback stair | 64 still one-way platforms | two one-way slope lines and a landing |
| The gate cradle's stairs | 24 still one-way platforms | a one-way slope line at each end |
| The cart of the wake room | 14 still one-way platforms | one line (done; the cart stands still) |
| The giant gnu's back | 10 typed ledges a tick | one moving line (done); its shoulders are drawn lines that lift and tilt (done) |

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

Built 2026-10-15: P1 and P2, and the capture of P3. `FaceRect` (core
`frame`) is the face-turned rectangle every capture, approach, carve and
free-body test uses (it is the old box for a cardinal portal, so those are
unchanged); the pieces are the boxes that bound the turned pieces. A portal
rides a line (`PortalHost::Line`): a slope, or a plank platform
(`MovingPlatform` with `line: true`, which may `spin` or `swing` and
collides as a turning line). The portal lab's right bay has three stations
for it: E (slope to wall), F (a sweeping, swinging plank to the floor), G
(a spinning plank to the shelf); a body goes through each (filmed). Left:
the see-through window's source for a turned face is still the box that
bounds it (`view.rs`).

## Original problem (2026-10-11; superseded by the built slices above)

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
  stopped). Rigid motion keeps its fast path. GNU's changing support is the
  customer for [SUP1](#sup1--keep-a-changing-supports-material-motion): its
  `location` names source material, and its motion retains the full
  supported shape change. A changing world arc length is not that address.
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

- **A walker (the ordinary body).** The current default above is a full
  FOOT SPAN along the bottom of its box. The body rests on the highest point of the
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
  the surface, `g_press - v² κ`, with speed and outward-crest curvature
  measured on the same **center path**. State the world-length window used
  to estimate curvature so the count of points does not decide release.
  [CV2's finite-body distinction](curved-surfaces-and-local-gravity.md#a-support-curve-is-not-the-path-of-a-finite-body)
  explains why source arc distance and center distance differ, even for a
  smooth curve. A new smooth `frame_at` alone does not complete phase D.
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
| C | Preserve the built rigid/displacement slices and complete lasting support and release velocity. The cart ramp and giant back are already lines. SUP1 adds the current GNU customer's shape motion; it does not reopen the fixed per-second/per-step displacement fault. |
| SUP1 | Current GNU support retains source place, source phase and one accepted shape trajectory through bounds, contact and carry. It can start before the complete persistent-attachment migration. |
| D | The rider and the crawler on the shared support; the circle's path at a corner and the launch from curvature. |
| Later | Flat box to flat box by the same support; richer shape laws beyond SUP1 when a concrete mechanic needs them. |

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
