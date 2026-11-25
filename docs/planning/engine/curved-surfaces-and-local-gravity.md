# Curved surfaces, grounded poses and local gravity

**State:** tentative design and decision aid, 2026-11-22. No new
implementation card is selected by this document. The maintainer wants to
understand whether curved terrain and a possible 2D Galaxy mechanic are
worth their cost. Declining planetary movement must leave ordinary slopes,
GNU support and grounded character art useful and complete.

**Evidence:** static review of runtime source `00bf92c22142`, with planning
base `23bf0a94a41d`. No game, build, test, render or benchmark ran. The
reported floating robot is not reproduced. Source findings below explain
possible causes and capability limits; they do not measure that scene.

## Recommendation and decision boundary

Keep four investments separate:

1. **Ground the existing picture on its accepted support.** The current
   slope plan already wants this. It helps the flagship without changing
   gravity, the collision shape or the movement law.
2. **Author curves, prepare bounded native geometry.** The engine already
   samples curves into chains. Improve that boundary when an author needs
   it, preserving fast line/chain queries and stating the approximation.
3. **Prove continuous local-frame walking before promising a planet.**
   The frame types accept arbitrary directions. The ordinary walker still
   contains cardinal collision and support assumptions. This is a real
   movement change, not a new field formula alone.
4. **Choose one planetary feel.** Spatial attraction, gravity chosen from
   an attached surface, and deliberate adhesion have different jump and
   release behavior. A human-controlled static circular scene can compare
   them before curve fields, moving planets or general AI are selected.

The recommended first choice is the existing hill-contact goal. A planetary
proof remains optional. Exact analytic curve collision, a general field
graph, universal foot IK and a new whole-tick mover are not prerequisites.

### Decisions the maintainer would eventually make

| Choice | Conservative starting point | What the other choice costs or changes |
| --- | --- | --- |
| Ordinary hill stance | Keep the accepted physical box and add bounded visual grounding | Narrow feet or a rotated collider change edge support, clearance and combat geometry |
| Curve representation | Authored curves compiled to native chains at a fixed mechanical tolerance | Analytic collision must cover casts, overlap, corners, bounds and contact response, with explicit supported/unsupported portal-host cases |
| Planet walking | A selected local-frame walking policy with an explicit attachment rule | Sanic momentum remains viable, but has different speed, jump and release feel |
| Attraction | Constant-magnitude radial gravity in one region | Distance-dependent strength changes jump tuning; overlapping sources need a composition law |
| Staying on the ground | A declared walking constraint or finite attraction | Finite attraction permits departure at speed; unconditional adhesion deliberately prevents it |
| Leaving a surface | A spatial field remains active in air | Contact-follow orientation needs a retained or fallback airborne rule |
| Character posture | Shared root placement first | Separate boot placement needs sole metadata, stance phase, reach and row coverage |
| Held movement | Explicitly select the reference for an orbiting walk command | The existing press-time frame policy serves discrete flips; silently changing it changes that customer's controls |
| Camera | Fixed camera for the proof | A following camera needs per-view turn and input-reference choices |
| Deployment | A bounded optional scene | General NPC travel, room exits and arbitrary planet geometry are additional customers |

No choice in this table is made implicitly by using the word “surface.”

## What the current code actually provides

| Boundary | Source-backed state | Consequence |
| --- | --- | --- |
| Native surfaces | [`SurfaceChain`](../../../crates/ambition_platformer2d_core/src/world.rs) stores line segments, winding, kind, junctions, depth lanes and rigid motion | There is already a shared geometry object, but its tangent is segment-local |
| Curve authoring | [`AttachedLoop::route`](../../../crates/ambition_platformer2d_core/src/surface_loop.rs) samples cubic ramps, a circular revolution and a descent; [LDtk conversion](../../../crates/ambition_platformer2d_ldtk/src/conversion/entity_converters.rs) samples `SurfaceLoop` and `SurfaceRamp` | Curve-to-chain preparation is established; a second unrelated curve compiler is unnecessary |
| Ordinary support | [`material_lines` and `settle_on_line`](../../../crates/ambition_platformer2d_core/src/walkable_lines.rs) use the body material and select an admissible support under its extent | The supported contact can be uphill of the picture's middle |
| Body orientation | [`try_motion`](../../../crates/ambition_platformer2d_core/src/movement/kernel.rs) rights `Supported` toward frame down, and `Riding`/`Attached` toward the opposite support normal | An ordinary ramp does not rotate gravity or the physical robot to its tangent |
| Picture placement | [`BodyPoseView`](../../../crates/ambition_sim_view/src/pose_view.rs), [`place_sprite_posed_bodies`](../../../crates/ambition_character_sprites/src/posed_body.rs), and [`sync_visuals`](../../../crates/ambition_render/src/rendering/actors/mod.rs) carry body material, stance and authored art placement, without the selected slope patch | The promised picture-on-middle adjustment is not established by the current drawing path |
| Runtime limb adjustment | [`PosedParts::hold`](../../../crates/ambition_sprite_sheet/src/character/rigged/posed.rs) has a two-bone solve; [`planted_contacts`](../../../crates/ambition_sprite_sheet/src/character/anim.rs) selects ledge-grab hands | Useful rig math exists; terrain feet need a different target contract |
| Sanic | [`SurfaceMomentum`](../../../crates/ambition_platformer2d_core/src/movement/surface_momentum/mod.rs) tracks chain arc distance and tangent speed, projects acceleration onto the tangent, jumps along the normal and can shed support | This is a momentum policy, not a general planet walker |
| Crawler | [Adhesive movement](../../../crates/ambition_platformer2d_core/src/movement/adhesive_crawler.rs) owns its attachment and corner law | Adhesion already has an explicit policy owner; it should not become an accidental effect of all gravity |
| Frame resolution | [`FrameEnv`](../../../crates/ambition_platformer2d_shared_tangle/src/frame_env.rs) separates the reference basis, gravity response, external forces and carried acceleration | Extend this owner; do not make every force rotate the body |
| Gravity selection | [`GravityZones` and `gravity_dir_for`](../../../crates/ambition_platformer2d_shared_tangle/src/gravity.rs) select constant directional zones by body overlap and deterministic precedence | A radial or surface field needs a sampling rule in addition to eligibility |
| Held movement | [`HeldControlFrame`](../../../crates/ambition_platformer2d_actor/src/control/participant_input.rs) retains the press-time frame for interpreting the current stick through gravity turns | Keeping a local walk command while orbiting requires a different, explicit continuation choice |
| Ordinary sweeps | [Integration](../../../crates/ambition_platformer2d_core/src/movement/integration.rs) chooses gravity-X when `down.x != 0`; [collision semantics](../../../crates/ambition_platformer2d_core/src/collision_semantics.rs) use the dominant component | A continuous field exposes contradictory cardinal assumptions |
| Navigation | [`TraversalEnvelope::measure`](../../../crates/ambition_platformer2d_world/src/navigation/envelope.rs) rejects noncardinal frames | A player proof does not establish NPC navigation around a planet |

The movement policy split and frame ownership are already accepted in
[ADR 0024](../../adr/0024-frame-aware-unified-movement-kernel.md). This plan
extends those contracts; it does not replace them with a universal solver.

## One boundary can supply several different facts

The useful unification is a **named, prepared boundary with a source place,
orientation and accepted motion**. The consumers retain their meanings.

| Fact | Question it answers | Owner |
| --- | --- | --- |
| Authored boundary | What shape did the author mean, and which side is solid? | World or character source and preparation |
| Physical contact | Which actual material blocks this body, at which point and normal? | Native geometry and movement |
| Locomotion attachment | Does this body keep following that boundary, and when does it leave? | The selected motion policy |
| Reference up and acceleration | How are controls interpreted, and what accelerates the body? | The per-body frame resolver |
| Grounded visual pose | Where should the visible sole, legs and torso appear? | Presentation derived from accepted support |
| View up | How does this observer see and control the scene? | The selected view and input policy |

A terrain curve may feed all six. They are not six copies of one normal.
A wall can attract a flying body before it touches it. A crawler can attach
without attraction. A walker can stay upright on a hill. A camera can stay
fixed while the body goes around a planet.

A physical contact normal also differs from a smoothed visual tangent.
Smoothing an internal edge normal is not permission to accept material
penetration. Conversely, a harmless visual turn need not change collision.

## CV1 — Ground the picture from accepted support

**Candidate:** sharpen the existing slope presentation goal. This is
useful without any new curve or gravity capability. It is not a claim that
the maintainer's reported scene has one proven cause.

### Why a physically supported box can look airborne

For an upright rectangular body of width `w` on a straight slope of angle
`theta`, full-width support puts the uphill bottom corner on the slope.
The vertical gap under the bottom midpoint is:

```text
gap = (w / 2) * abs(tan(theta))
```

For width 30 and a 30-degree slope this is about 8.66 world units. This is
a geometric example, not a measured frame. A curved support requires its
actual geometry under the span; the tangent formula is not enough.

Putting the whole physical box lower penetrates the uphill part of the
terrain. A narrower foot span or rounded body can change this tradeoff,
but each changes collision behavior. A rigid flat sole also cannot touch
every point of a curved ground line. Exact visual contact everywhere is
not a property that one box or one image translation can provide.

### Preserve the accepted support through the pose boundary

After native motion and righting are accepted, provide a small
presentation description of the selected support: its valid local extent,
source occurrence, pose/time and the place relevant to the visual foot
origin. This may be an already evaluated target or a bounded patch; a new
global support graph is not required.

The present physical contact point alone is insufficient. It can be the
uphill extremity of the body. Recovering a line beneath the visual middle
requires the selected boundary and its extent. At an edge or fork:

- do not extend a segment beyond its endpoint;
- do not choose a different nearby floor to make the picture look planted;
- retain the native branch/boarding choice;
- use an explicit unchanged or limited-offset fallback when the selected
  support has no valid point beneath the chosen visual origin.

Apply the derived correction once with the authored quad/feet placement.
Do not give `ActorSpriteOffset` two competing writers. Baked sheets,
direct parts, composed sprites, held visual props and other body-owned
drawables must consume the same resolved presentation placement. Preserve
the [body-owned finalization boundary](render-animation-and-vfx.md#body-owned-drawable-geometry-has-one-finalization-boundary)
and the existing appearance-order work.

Body and support must be sampled at the same presentation cut. Interpolating
the body while reading an unrelated current support sample can create
drift even when both individual values are correct. Loss of support,
jump, drop-through, portal crossing and a source discontinuity need an
explicit visual cut or release rule. Do not let visual smoothing delay an
authoritative jump or invent a physical attachment.

Apply ordinary grounding only to the selected supported ground posture.
Derive it from accepted body placement and the neutral authored foot
origin, then compose the existing reaction and animation offsets.
[`PresentedPose`](../../../crates/ambition_sim_view/src/presented_pose.rs)
already lowers buried/crumpled art and carries hitlag presentation. Do not
resnap the final picture and cancel those choices. Ledge hands, capture,
saddle and other external pose owners keep their placement unless that
owner supplies an explicit grounding target. Unsupported rows retain
their declared fallback.

Bound the correction in the character's authored units and expose it in
the contact view. Visible art can otherwise move too far from its hurt
geometry. The existing [scale contract](scale-relative-gameplay.md) still
owns conversion to world units. At a portal, compose the correction in
the declared body/material appearance frame before the existing fragment
mapping; do not add the same world offset to every mapped fragment.

### Root placement, tilt and feet are separate choices

| Technique | What it can do | Limit |
| --- | --- | --- |
| Root translation | Put one declared visual foot origin onto the selected support | Cannot plant two boots at different heights |
| Bounded visual tilt about that origin | Follow a hill with baked or part art | Turns torso and both boots together; does not change physical clearance |
| Terrain foot placement | Set separate sole targets and solve the legs within reach | Needs stance phase, real sole metadata, fallback and pelvis limits |
| Physical body turn | Change the actual pose of the collision material | Must pass native swept righting and changes gameplay |

The robot already has authored leg joint triples. Its published foot
attachments mean ankles. Putting those directly on terrain would put the
wrong point on the ground. A later terrain-foot customer must name
sole/heel/toe offsets and which foot is in stance. Reuse the existing
small limb solver and part-pose road where suitable; do not require
`BodyRigAdmission` for a cosmetic experiment or require all characters to
have rigs. Some robot rows/views lack compatible rig coverage.

A planted foot on GNU is a place on GNU's material, not a permanent world
point. Toe-off, lost support and reach failure release it. These visual
plant/release choices do not replace locomotion attachment.

### GNU has a second, independent geometry question

The [giant author](../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/targets/characters/gnu_ton_boss/giant_art.py)
uses axial stretch for its barrel art. The native
[`BackLedge::slope_line`](../../../crates/ambition_mount/src/lib.rs)
fits an eleven-point hump to two authored endpoints by a uniform
similarity. These maps can agree at both ends and differ inside.

Decide whether the gameplay hump is a deliberate simplified contour or
must follow the full drawn contour within a stated tolerance. In the
second case, publish the support vertices or supported map from the same
offline recipe that draws the hump. Do not infer physics from image alpha.
[SUP1](slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion)
separately retains the motion of the chosen geometry over time. Correct
transport does not establish art agreement, and better art agreement does
not repair a lost motion interval.

## CV2 — Prepare authored curves into bounded native geometry

**Candidate:** select when a concrete hill or round-world authoring
customer needs it. First reuse existing loop/ramp preparation and chain
queries. The source curve, mechanical approximation and visual contour
must state their relationship.

### Keep one source recipe and explicit downstream products

A source recipe may be a line, a circular arc or a bounded polynomial
span. It needs orientation/solid side, explicit hard corners, and named
source spans. Topology is separate: coincident points do not establish a
junction. Existing loop depth lanes and `SurfaceJunction` already express
that distinction.

Select the geometric product as well as the source curve: a directed
route, a one-way boundary, or a filled collision region. Closedness joins
the endpoints; it does not fill the interior. Native
[`SurfaceKind::Solid`](../../../crates/ambition_platformer2d_core/src/world.rs)
currently expects a convex closed outline, and its material contact path
casts that whole outline as a convex polygon. Winding and the current
`SurfaceChain::validate` do not establish convexity. Preparation must prove
that precondition for the selected solid product. Use the convex product
for the first exterior-circle proof. A later concave filled product needs
an admitted volume representation with coherent union/contact behavior,
or an explicit unsupported result. Retain its exterior boundary and source
locations separately: internal decomposition faces are not authored routes,
support branches or gravity attractors. Merely creating independent convex
colliders does not establish that concave product. This is a capability
boundary, not selection of a general concave-collision project.

Prepare these products from the recipe:

- native segment geometry with adjacency and occurrence provenance;
- arc-length lookup for consumers that actually need distance;
- a map from each prepared segment to its source span and parameter range;
- conservative bounds and the owner's finite work/error declaration;
- optional visual geometry and placement queries with a declared relation
  to the mechanical boundary.

A segment number is not a stable material place after subdivision. Use
the source span and parameter inside the current content occurrence,
consistent with SUP1. This is needed for remeshing, contact continuity and
moving support now; it is not a saved-content versioning program.

Share pure curve evaluation, subdivision or distance lookup only where
the geometry and [PATH1](kinematic-world-objects.md#path1--prepare-one-spatial-path-law)
customers require the same operation. A motion path has no implicit
solid side. A collision boundary has no implicit patrol speed. Keep
routes, drive clocks, collision policy and field ownership out of a
generic curve object.

### Mechanical tolerance is independent of display quality

Use a fixed preparation rule for the selected mechanical product.
Changing camera zoom, texture quality or observer must not change the
floor or Sanic's release. Local scale and numeric budgets belong to the
existing geometry and material-scale authorities.

Position error and direction error are different controls. For a
nondegenerate minor circular arc of radius `R > 0`, approximated by one
chord spanning angle `a`:

```text
maximum chord deviation = R * (1 - cos(a / 2))
maximum tangent/chord angle difference = abs(a) / 2
```

A position-only criterion can leave visible turns or unstable response.
Conversely, making every segment very short increases query and
preparation work without fixing an incorrect corner rule. State a
physical feature-size limit and explicit sharp corners; do not make a
cusp claim to have one smooth normal.

Declare whether the prepared collider is an inscribed approximation, a
conservative enclosure, or the exact native gameplay chain within an
authoring error bound. Visual agreement and clearance depend on that
choice. For an exterior circular solid, inscribed chords leave the exact
circle outside the collider between vertices. This is another possible
contact/art gap, even with a perfect foot anchor.

The Sanic scripts already place objects on the **prepared painted
height**, rather than on the original ideal curve:
[Darkness author](../../../game/ambition_demo_sanic/tools/author_darkness_ldtk.py).
Preserve this principle in `place on surface` operations. An object
placed on an ideal curve while gameplay uses a different prepared contour
has two competing placement authorities.

### Native adjacency matters as much as sampling density

Internal vertices must retain their neighboring edges and solid-side
meaning. Treating each edge in isolation can produce a contact normal
that belongs to neither the intended contour nor its admissible side.
The primary [Box2D collision documentation](https://box2d.org/documentation/md_collision.html)
describes this connected-edge failure and its neighbor-aware treatment.
This is a geometric precedent, not a finding that Ambition uses Box2D or
a proposal to replace the native engine.

A smooth authored span and an intentional sharp corner need different
rules. Do not average normals across a cusp, a junction, a gap, a carved
opening or an edge with a different collision policy. A more accurate
curve compiler cannot by itself repair topology or contact policy.

### A support curve is not the path of a finite body

Let `C(s)` be a smooth support curve parameterized by its own arc length,
with unit tangent `t`, unit normal `n`, and signed curvature defined by
`dt/ds = k*n`. A circular rider of radius `r` on a regular offset branch
has center:

```text
Q(s) = C(s) + r*n(s)
dQ/ds = (1 - r*k) * t
center speed = abs(1 - r*k) * abs(ds/dt)
```

Thus support distance, center distance and visible gait phase are not
interchangeable. At `1 - r*k = 0`, that offset loses regularity. Tight
concavities can require multiple contacts or reject a body that cannot
fit. No spline representation removes the finite body's clearance.

Current Sanic increments the support-chain arc coordinate by tangent
speed times dt, seats its circle at the segment point plus radius times
normal, and reports tangent velocity. Replacing `frame_at` with a smooth
normal alone would leave the distance/velocity contract unresolved.
At an intentional convex corner a rider that remains in contact also
travels an arc around the vertex. A released rider can depart ballistically.
Neither path is a discontinuous normal offset.

The slope owner's existing phase D already proposes the circle offset
path and curvature-based release. Refine that work to state:

1. which distance the speed measures;
2. how accepted center displacement and published velocity agree;
3. which center-path curvature enters the support-load calculation;
4. how moving-support velocity is composed once;
5. what is invariant under refinement of the same authored curve.

Do not silently retune Sanic while introducing authoring syntax. Its
current per-vertex turn demand depends on segment angle and body radius;
arbitrary subdivision is not established as behavior-neutral.

### Reference and gameplay paths

The reference geometry evaluator can run offline over the source curve.
Use it to establish geometric error, source mapping and conservative
bounds. A simple exhaustive native query can also serve as the reference
for a prepared-chain query index.

These are different comparisons. An exact analytic curve and an
approximating chain do not have identical collision results. Differential
checks must compare the same declared model, or use its stated geometric
error. Calling the analytic result a reference does not authorize the
gameplay path to ignore its contact and motion contract.

Prefer straight/rigid cases, prepared lengths and local contact cursors.
Use the existing spatial-query foundations for candidate selection, but
do not assume chain broadphase is segment-local. Current
[`material_lines`](../../../crates/ambition_platformer2d_core/src/walkable_lines.rs)
visits every segment of each nearby chain; arc and projection helpers also
scan. A dense, level-sized curve can therefore make a local query costly.
If the selected workload needs finer preparation, index segment ranges or
chunks while retaining the original surface occurrence and source spans.
A query chunk boundary is not a new open end, junction or release point.
Moving bounds cover the same accepted interval as contact. Keep the direct
small-input path, and make frame and joint lookups share one prepared
distance and boundary convention. Measure before selecting an index or
threshold; the [data-structure owner](data-structure-kernels.md) supplies
candidate-query contracts, not a new surface topology.

Exact analytic casts remain a separate possible investment. Supporting
one exact circle is smaller than supporting arbitrary deforming curves,
but even that primitive needs coherent overlap, motion, normals, bounds
and contact response. Keep it out of the first hill presentation slice.

## CV3 — Make continuous down an honest walker capability

**Candidate and dependency:** needed for ordinary action-platformer
walking around a planet. A vector-valued frame is necessary but not
sufficient. This is the largest immediate engine cost in the proposal.

### The current limit is visible without running the game

For down approximately `(0.1, 0.995)`, integration treats X as gravity
because X is nonzero. Collision classification treats Y as gravity
because Y dominates. Replacing the first condition with a dominant-axis
test would align those branches, but would still switch world axes at
45 degrees. It would not produce continuous local-frame walking.

There is also a shape distinction. A 20-by-40 rectangle turned 45 degrees
and aligned with its down axis has a true half-extent of 20 along down.
The projection of its world AABB along the same axis is 30. The existing
[`AabbExt::feet_coord`](../../../crates/ambition_geometry/src/geometry.rs)
uses that envelope projection. Such an envelope is useful for candidate
selection, but cannot stand in for exact feet or prior-side admission on
a rotated body.

Current [collision code](../../../crates/ambition_platformer2d_core/src/movement/collision.rs)
has both useful material casts and world-axis response/repair. Its
one-way block rule reads previous AABB feet; parts of carry, velocity
cancellation and penetration repair still choose X or Y. Auditing only
the input vector misses these consumers.

### Adapt the existing law inside the existing kernel

The proposed direction is a motion leg with an explicit unit direction
and semantic role, such as side or gravity. Keep the current cardinal
specialization. The existing `MaterialCast` accepts a convex material
shape and arbitrary translation; `World::first_material_contact` and
`material_axis_contacts` already provide useful query foundations.
There is no need to copy or rotate the whole world for each body.

This is a direction to prove, not a claim that changing one enum finishes
the controller. Start with a fixed noncardinal frame, one ordinary body,
full-solid support, walk/jump/land and an obstruction. Its minimum contract
is exact material projections, coherent support/wall/head classification,
directional response/repair, the actual supported travel path and admitted
righting. A low ceiling must constrain the traveled body, not only its
endpoint. Keep accepted trajectories and the existing numeric authority.

Before enabling further mechanics in a continuously turning frame, audit
their own complete boundary: one-way prior-side/drop-through rules, edge
holding, wall abilities, ledges, body-to-body contact, floor/chain transfer
and portal admission. A small solid-planet scene may exclude those through
its content/capabilities. It must not advertise them as supported. Preserve
their ordinary cardinal behavior and existing owners; full generality is
not a prerequisite for deciding whether the planetary mechanic is useful.

Do not build a second whole-tick `GalaxyMover` beside `step_motion`.
Preserve the three policy owners. If the intended walking rule turns out
to require a distinct persistent constraint, decide that explicitly
inside the kernel; do not hide it in a renderer or frame modifier.

[`admit_body_righting`](../../../crates/ambition_platformer2d_core/src/movement/righting.rs)
already admits physical rotation against geometry. A down change does not
guarantee that a long robot can turn in a narrow passage. Keep the accepted
material orientation and the desired reference up distinct when the turn
is obstructed. Visual roll must not claim the physical turn succeeded.

A once-per-tick frame can also become too coarse when a body crosses a
large angle of a small planet in one tick. Begin with a bounded radius,
speed and angular-change envelope. Finer sampling is a separately selected
motion/frame change: one pure field snapshot, deterministic admitted
interval cuts and an explicit relationship to the tick-published frame.
Current `InputState` already contains interpreted local axes and action
edges. Repeatedly calling the whole step with fresh frames could reinterpret
a held command, replay an edge or advance a clock twice. Preserve the
[held-command policy](#held-input-needs-an-explicit-continuation-rule),
consume edges/ages once on their owner clock, and retain accepted interval
frames. Do not add competing ECS resolvers or read render-time orientation.

## CV4 — Prove one static circular world and choose its feel

**Candidate:** a human-controlled proof after CV3, or a clearly labeled
earlier experiment with Sanic's existing momentum semantics. Success with
Sanic does not complete ordinary walker support.

A small first scene needs one exterior solid, one influence region,
constant-magnitude inward attraction, a fixed view, and explicit spawn
and death bounds. Prove walking, stopping, one jump, release, return and
a tangent obstruction. Exclude moving planets, general field blending,
NPC route planning and portals in the first product proof. They retain
their existing owners and can be selected later.

The current LDtk `SurfaceLoop` samples normals toward the circle's
interior so Sanic rides **inside** a loop. It is not an exterior planet
recipe. Exterior support needs the intended winding and solid-side
semantics; the visual `filled` flag must not be mistaken for solid
collision.

### Spatial attraction and contact-follow orientation differ

| Rule | While attached | In air | Required authored meaning |
| --- | --- | --- | --- |
| Radial field | Down points from sample point toward a center | The same field continues to turn acceleration | Center, influence region, strength, center singularity and overlap rule |
| Attraction to a named boundary | Down points toward an admitted nearest place, or follows its declared normal | The field still evaluates inside its region | Target boundary/side, branch and endpoint rules, sample point and distance response |
| Orientation from accepted support | Down for control is chosen from the attachment normal | Retain last up or return to an environment rule | Source support, acquisition/release timing and airborne policy |
| Adhesive traversal | A movement constraint keeps contact | Detached motion resumes its own frame | Attachment permission, release conditions and force/load limits if any |

For an ideal circle, a center-directed field matches its exterior normal
line. A compiled circle's facets differ within its declared angular
approximation. For a general ellipse or hill, center direction does not
follow the surface normal. At the boundary itself,
`normalize(closest_point - body_point)` may be zero; a declared surface
side supplies a meaningful normal there. At the center of a radial field,
direction is undefined. Prevent entry, soften/disable the field with an
explicit fallback orientation, or author another behavior. Do not let a
zero vector quietly choose world-down.

Separate **field eligibility** from **where its vector is sampled**.
Current zones activate by body overlap. Retain that behavior unless the
product deliberately changes it. A radial field may then sample the
body's canonical reference point while the overlap region controls
eligibility. Averaging force over the full body would introduce a
different model, including possible torque; it is not required for this
platformer proof.

### Attraction is not adhesion

For a body following a circular center path of radius `R_center` at
center speed `v`, the required inward acceleration is:

```text
a_required = v*v / R_center
```

On the outside of a convex planet, ordinary contact can push outward but
cannot pull the body inward. If attraction is too weak, a physically
load-bearing rider must leave. Increasing attraction changes airborne
jump timing as well. A walking constraint can keep the body attached
deliberately; that is a gameplay choice, not a numerical repair.

Sanic projects gravity onto its tangent. If gravity is chosen exactly
opposite the current support normal, its tangential component is zero.
The hill ceases to provide Sanic's ordinary downhill acceleration.
Preserve that distinction when comparing “follow the normal” with a
gravity field. Neither normal alignment nor a smooth curve alone gives
the desired speed law.

### Keep the existing frame decomposition

Resolve selected orientation and gravity through `FrameEnv`. Preserve
the current body-response decomposition and follow SCALE-FEEL's selected
canonical gravity conversion when implemented; that conversion is not
complete in the inspected resolver. Keep non-orienting wind and other
forces distinct. Do not normalize the sum of all forces to obtain
up. At zero gravity response, the environment must still define controls
and reference orientation.

For the first field, use one explicit region and deterministic precedence.
Later fields may add accelerations or override orientation, but these
are different operations. A sum can cancel to zero even when each field
has a clear orientation. A generic blend graph would hide rather than
decide that policy.

Contact-follow orientation has a causal choice: the frame is resolved
before this tick's movement, while this tick's support is an output of
movement. Use a declared previous accepted attachment for the initial
frame, or a kernel-owned coupled interval rule. Do not query a guessed
nearest floor and then use the resulting gravity to justify attaching to
that floor. Any history-dependent selection belongs to native body state,
with existing rollback treatment, not a presentation cache.

Free [items](../../../crates/ambition_held_items/src/motion.rs) and
[projectiles](../../../crates/ambition_projectile_combat/src/systems.rs)
also read the shared gravity query directly, outside kernel-body frame
publication. Extending `FrameEnv` alone does not establish a new field law
for those bodies. A direction-only radial extension can share the query;
spatial magnitude, added-force composition or finer sampling needs those
consumers audited before claiming uniform behavior. The first proof may
omit them. Whole-body root sampling also does not define distributed force
or torque on portal-separated material.

### Held input needs an explicit continuation rule

Current [`hold_stick_through_turns`](../../../crates/ambition_platformer2d_actor/src/control/participant_input.rs)
retains the press-time control frame while interpreting the current held
stick. For an unchanged stick, this preserves its original world direction
until release, sufficient redirection or a control/mode change. Small analog
changes still steer within that retained frame; the policy does not freeze
the raw vector. This is an intentional comfort rule for discrete gravity
flips. Selecting body-relative input or a subject-following camera does
not remove it.

For a strict local-right command, retaining its original world direction
through a quarter-turn can leave no projection onto the new local side.
That is different from “keep walking clockwise while I hold right.”
The latter interprets the held locomotion command under the current
declared frame. Retaining the press-time reference and following the current
reference are two legitimate continuation policies.

The planetary proof must explicitly choose which meaning it wants. Extend
the existing [input continuation owner](participant-action-system.md#tentative-curved-world-held-direction)
if needed; preserve the discrete-flip behavior for its existing customer.
A policy change must rebase/end the held state at the admitted control cut,
and any retained state must follow the existing rollback/input ownership.
Camera easing is not that state. Keep aim and other actions under their
own policies rather than rotating every input with locomotion.

Include an uninterrupted full circuit, a release/repress on the opposite
side, and a discrete gravity-flip control case in future acceptance.
Automatic switching by “small versus large angle this frame” would make
the control meaning depend on speed and tick size; do not infer it that way.

## CV5 — Add a named surface field only for a selected scene

**Candidate:** after a circular proof establishes product value. A
generic nearest-terrain field is not the starting abstraction.

A surface attraction region names a boundary occurrence and a reachable
branch/domain. Preparation checks that the referenced geometry and
declared side exist. Query only its admitted candidate spans using the
existing bounds infrastructure. A field associated with a moving surface
must sample that surface at the same authoritative time.

The nearest point need not be unique. Concave valleys, two nearby limbs,
crossings and curve ends can have competing closest places. At the
equal-distance boundary, normal or direction can jump. More curve
samples do not make that selection continuous.

Choose one policy per concrete customer:

- restrict the influence region to a branch with a well-defined projection;
- use explicit branch/endpoint rules;
- retain the selected branch with a declared release/hysteresis rule;
- blend acceleration intentionally, while retaining a separately defined up.

A history-dependent choice becomes native state. A deterministic tie-break
gives repeatability, but does not by itself give smooth motion. Averaging
opposite normals can erase attraction and leave no reference direction.
The authoring inspector should show ambiguous regions and explain the
selected rule. Do not introduce a per-edge priority language to avoid
designing the scene's behavior.

A field attached to a moving curve also does not automatically carry a
body with that curve. Attraction changes acceleration; support transports
accepted material contact; adhesion constrains attachment. SUP1 and the
moving-frame owners remain responsible for those latter facts.

## Authoring should ask for intent, then expose the prepared answer

These are proposed operations, not APIs that exist at this head:

| Author's operation | Source facts | Prepared result to inspect |
| --- | --- | --- |
| Draw a hill | Curve spans, solid side, sharp corners, surface style | Native collision boundary, visual deviation and placement height |
| Put a door, actor or prop on it | Named boundary, source location, desired placement anchor | Actual support/clearance and any unsupported placement |
| Build a round world | Center/radius or exterior boundary, material and region | Exterior solid, field direction, spawn/death bounds |
| Make this region attract toward that surface | Typed source reference, side, influence and branch policy | Eligibility, sample vectors, ambiguity and endpoint behavior |
| Let this actor follow its ground | Existing motion policy plus explicit attachment/orientation choice | Jump/release behavior, physical turn clearance and unsupported cases |
| Adapt these boots | Rig row coverage, stance phase, sole offsets and reach | Visual-only root/foot targets and fallback |

Do not expose raw tessellation counts as the usual way to obtain smooth
gameplay. Keep an advanced inspection of prepared counts/error for
authors and agents. Existing fixed-resolution Sanic products need a
deliberate migration because current motion can depend on subdivision.

Extend the existing [LDtk and intent-level tooling](ldtk-authoring-and-world-tools.md)
and the character asset authoring packages. A curve authored by Python
and one edited spatially should lower through the same semantic recipe
and native validation. Keep source references typed and inspectable.
No new visual editor, universal scene graph or runtime script interpreter
is needed.

## Navigation and room rules need the field along the route

The current envelope rejects noncardinal frames and the navigation cache
keys a graph by a motion frame. In a varying field, two bodies with the
same initial down can encounter different future down along their route.
One frozen frame or a cache entry for each angle does not describe that
experiment.

If autonomous traversal becomes a customer, retain the current
[static geometry and native rollout direction](platformer-navigation-and-reachability.md#static-geometry-coverage--native-support-before-finite-preparation).
A candidate leg must evaluate the same bounded field description and
geometry at the same future positions/times as native movement. Separate
support/branch topology from traversal feasibility and cache only complete
experiments. A human-only proof does not need general AI first, and must
not claim to establish it.

A planet also needs an explicit room/death policy. The current
[`apply_world_hazard_gate`](../../../crates/ambition_platformer2d_core/src/movement/kernel.rs)
projects escape from rectangular room bounds onto the body's frame.
Fall always has a margin; rise and side are opt-in. Escape outward under
inward radial gravity is rise, so the default fall-only rule does not
define planetary escape. Choose enclosing bounds with explicit margins,
hazards or loss conditions for the first scene. Doors, camera limits, aiming,
knockback and projectiles need their existing frame contracts reviewed
before general content adoption.

A fixed camera is enough to decide whether moving around the planet is
fun. If the view later follows the actor's up, keep that choice per
observer under [multiview M3](multiplayer-and-multiview.md).
Screen-relative and body-relative controls remain controller policies,
not field properties.

## Performance contract and stopping points

No performance improvement is measured by this audit. These are
implementation constraints and future comparison cases:

- A room with no selected curve/field/grounding customer retains its native
  path. Do not install an all-body nearest-surface pass.
- Prepare curve samples, lengths, adjacency and bounds outside the hot
  body step. Static geometry retains prepared data; rigid movement uses
  the existing local-to-world path.
- Query a selected support or eligible field locally. Bounded refinement
  must state work and failure behavior; an unbounded “search until smooth”
  loop is not a gameplay algorithm.
- Reuse one accepted support result for the body's view and requested feet.
  Do not repeat world discovery independently in each renderer.
- Preserve constant-zone, straight-segment, cardinal and rigid cases.
  More expressive reference evaluation may remain offline.
- Mechanical sampling does not depend on rendering quality. Visual-only
  leg work can have a prepared fallback without changing gameplay.
- A moving/deforming curve must retain SUP1's actual motion law and bounds.
  Refitting a static index without the old interval is not sufficient.
- Measure query counts, preparation size, transient allocations and frame
  cost for the selected scene when execution is available. Do not claim
  that a BVH or analytic curve is faster without that evidence.

Stop after CV1 if hills read well and planetary mechanics add little to
the game. Stop after a static circle if it supplies the desired scenes.
Do not build CV5, moving planets, analytic arbitrary-curve CCD or global
navigation merely to complete an abstraction.

## Candidate sequence and future acceptance

These are planning gates, not work selected in [queue](../queue.md).

| Candidate | Smallest coherent result | Evidence required when implementation is selected |
| --- | --- | --- |
| CV1: visual grounding | One resolved support-derived root placement for the robot and its baked fallback | Flat, straight incline, crest, edge, jump release and GNU comparison at one body/support cut; visible and mechanical origins shown |
| CV2: curve preparation | One authored curve recipe with bounded native lowering and placement queries | Error/normal limits, source mapping, sharp corners and refinement semantics; unsupported shapes rejected before publication |
| CV3: continuous walker | Existing kernel handles a bounded range of continuous down without cardinal jumps | Fixed tilted full-solid walk/jump/land, exact support, rotation obstruction, low ceiling, release and existing cardinal behavior; extra mechanics admitted separately |
| CV4: circular proof | One human-controlled exterior planet with explicit attraction/attachment and held-command feel | Walk/stop/jump/land, uninterrupted circuit, release/repress, high-speed release choice, center/escape behavior and a fixed camera |
| CV5: named surface field | One region with defined target, projection and ambiguity behavior | Endpoints, competing branches, field entry/exit, rollback of retained choice and local query bounds |

Additional high-value comparisons:

- The same curve prepared at two admitted tolerances: stable source
  placement and bounded geometry differences; after phase D's law is
  corrected, path/release agreement within a declared behavior tolerance.
  Contact or launch-threshold cases can still differ between mechanical
  approximations and must state their permitted error.
- A wide box and a circle on the same boundary: their support and center
  paths legitimately differ.
- One radial field and one adhesive policy at the same speed: their
  release difference is intentional and visible.
- Zero gravity response plus wind: reference up remains defined and wind
  does not rotate the character.
- A stopped GNU, a length-changing hump and an interrupted source phase:
  pose grounding consumes the correct accepted shape and time.
- Mirrored/scaled material, crouch and part/baked rows: one appearance
  placement, bounded mismatch and no duplicate offset.
- A normal portal or a carved opening near a proposed curve: existing
  motion/contact authorities remain in force. General curved host
  placement needs its own flat-aperture/host-patch decision and is outside
  the first circular proof.

The important success criterion is an author who can state the scene's
intent and inspect the prepared answer without learning incidental
segment counts, render offsets or private mover state. It is acceptable
to support fewer curve and gravity products with clear contracts.
