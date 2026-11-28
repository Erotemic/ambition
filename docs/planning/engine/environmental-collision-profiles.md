# Environmental collision profiles and ground support (ECP)

**State:** proposed engine track, 2026-11-27. The maintainer prefers a hybrid
system: retain rectangles, investigate fighter ECBs and paired support for
quadrupeds first, and leave rounded shapes available for later customers.
The art must communicate collision and hits. Prefer axis-aligned platforms
and jumpable steps in level design, but make correctly authored ramps a
reliable engine capability. This document proposes delivery gates; it does
not select work in [the live queue](../queue.md).

**Evidence:** static source archive `52878392a290`, the maintainer's Hobford
companion-dog report and the related-work references below. The map-authoring
submodule is not in this archive. The exact snag is not reproduced. No build,
playtest, benchmark or new collision implementation is claimed.

## Goal and limits

A body can walk, stop, turn, land and replan on admitted terrain. Its collision
profile describes how it occupies and contacts space. A single accepted
mechanical pose supplies gameplay geometry and the corresponding art. Players,
companions, enemies and fighters use the same native movement operations.

Do not require every body to adopt a new shape. Do not require a new rigid-body
engine, a skeleton physics solver, continuous gravity or a universal curve
solver. Ordinary rectangular rooms must remain inexpensive. Use the current
movement law unless a selected character needs a different law. Preserve
intentional game behavior, not incidental limits of the rectangular code.

The first useful delivery is a repaired Hobford route and reliable static
ramp joins. The first profile delivery adds a fixed ECB fighter and a dog
with paired support. Later deliveries cover navigation, pose adaptation and
dynamic profiles. Each delivery must be useful without the later ones.

## Current evidence and boundaries

| Site | Source-backed state | Remaining question |
| --- | --- | --- |
| [`walkable_lines.rs`](../../../crates/ambition_platformer2d_core/src/walkable_lines.rs) and [`integration.rs`](../../../crates/ambition_platformer2d_core/src/movement/integration.rs) | A walker settles onto a line after its box sweeps; full-width foot support, boarding and downhill settlement exist | Can the lateral sweep reject a legal supported continuation before settlement? Inspect the actual Hobford contact before changing this order. |
| [`SurfaceChain`](../../../crates/ambition_platformer2d_core/src/world.rs) and [SUP1](slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion) | Native chain geometry, junctions, rigid and changing shape intervals | Which block/chain joins are legal support continuations? Keep source identities and material motion. |
| [`SupportFact`](../../../crates/ambition_platformer2d_core/src/movement/kernel.rs) | Native movement reports support semantics | Retain the accepted source place, extent and time needed for subsequent placement queries. |
| [`BodyMaterial`](../../../crates/ambition_platformer2d_core/src/frame/material.rs) and [`MotionModel`](../../../crates/ambition_platformer2d_core/src/movement/model.rs) | Material transformation and movement policy are distinct | Select environmental profile independently of controller kind; preserve the existing frame, scale and portal contracts. |
| [`StandSurface` and `GroundMap`](../../../crates/ambition_platformer2d_world/src/navigation/surfaces.rs) | Standing spans have a constant top; slope pieces join flat spans | A slope interior is not a general standing location. A chord between flats is not a compound hill. |
| [`NavGraph`](../../../crates/ambition_platformer2d_world/src/navigation/graph.rs) | Native rollout admits traversal legs; slope legs connect flats | Keep rollout and follower truth. Extend standing and travel domains, not a second physics implementation. |
| [Actor navigation](../../../crates/ambition_platformer2d_actor/src/body/navigation.rs) | `slope_exit` directs a body on an unrecognized slope to a flat end before planning | This workaround does not supply arbitrary slope destinations or general replanning. Remove it when continuous support queries cover its customers. |
| [CV1 and rig evidence](curved-surfaces-and-local-gravity.md#cv1--ground-the-picture-from-accepted-support) | Art placement and native support do not yet have the complete common grounding handoff | Prefer admitted physical pose and shared rig geometry to moving the whole picture independently of combat. |

The report can have more than one cause: a real protruding block corner, an
internal collision face, or an AI route that cannot start on its current
support. Do not diagnose all three as one failure.

## Related work and choices

Rivals of Aether II is the primary modern fighter reference. Its public
[1.2.0 update (2025-04-02)](https://steamdb.info/patchnotes/17957950/)
introduced animation-dependent environmental collision boxes (ECBs). The
change affected landing time, low aerial attacks and hitstun. Its
[1.2.1 update (2025-04-15)](https://steamdb.info/patchnotes/18114736/)
corrected several ECB interactions. Treat a change of environmental geometry
as a change of gameplay, not an art-only improvement.

The [Rivals II character template guide](https://rivals2.com/workshop/knowledge-base/character-creation/create-your-character-mod/)
associates the hurtbox physics asset with both ECB and hurtbox configuration.
The [hurtbox guide](https://rivals2.com/workshop/knowledge-base/character-creation/setting-up-hurtboxes/)
uses joint-attached capsules for hurtboxes. This supports common authored
geometry and separate contact semantics. It does not establish that Rivals
uses a capsule controller or any particular internal sweep algorithm.

| Reference | Useful technique | Limit for Ambition |
| --- | --- | --- |
| [Melee collision source](https://github.com/doldecomp/melee/blob/master/src/melee/mp/mpcoll.c) | Fixed and joint-derived ECBs, previous and current ECB, directional terrain contact and retained floor | An ECB is a gameplay contact proxy, not a guarantee that every limb stays outside material. Do not assume Rivals II has the same implementation. |
| [Sonic Physics Guide](https://github.com/trentbrew/sonic-physics-guide/blob/main/docs/06-slope-collision.md) | Directional sensors, terrain-follow state and ground-relative movement | Paired sensors do not by themselves establish quadruped occupancy, and momentum travel is not the default walker law. |
| [Box2D: Ghost Collisions](https://box2d.org/posts/2020/06/ghost-collisions/) | Neighbor-aware edges reject invalid internal contact normals | One-sided chains are not all of Ambition's solid and one-way geometry. Keep volume queries. |
| [Box2D geometric character mover](https://box2d.org/documentation/md_character.html) | Shape casts and contact-plane resolution outside rigid-body dynamics | A useful bounded experiment for corner resolution; its capsule implementation is not a reason to replace all profiles. |
| [Surfacer: platform graph](https://devlog.levi.dev/2021/09/building-platformer-ai-part-4-platform.html) | On-demand travel along supports and sparse inter-surface anchors | Keep Ambition's own discrete native rollouts and controller playback, not the example's independent trajectory correction. |
| [Johansen: locomotion system](https://runevision.com/thesis/) | Stance-aware adaptation of walking and running animation to terrain | Full locomotion synthesis is not necessary for the first dog or fighter. |
| Rain World | Physical core with creature-specific articulation | An interesting later model for serpents or multipart creatures. No compound-body solver is selected. |

The references motivate the design. They do not measure the cost of the
proposed implementation in Ambition.

| Model | Benefits | Costs and failure modes | Decision |
| --- | --- | --- | --- |
| Axis-aligned rectangle | Cheap axis sweeps, predictable clearance, simple occupancy | Wide base floats at slope midpoint; corners and steps can snag | First-class default; preserve specialized path |
| Fighter ECB | Directional floor/wall/ceiling semantics, animation correspondence, established fighter precedent | Corner coverage, limb penetration, support and landing changes | First new contact profile; fixed before dynamic |
| Paired support | Natural pitch for long bodies, two local ground samples on a simple grade | One or zero supports, crest penetration, rotation through walls, gap bridging | First new grounding configuration; use admitted occupancy |
| Circle/capsule | Smooth support and useful round-body approximation | Different ledge behavior, less precise angular fit, new casts | Later, from a real character need |
| Convex polygon | General bounded shape queries and explicit solid envelope | Rotation, contact ordering, greater implementation and tuning cost | Consider a shared low-level primitive if the pilots need it |
| Compound articulation | Expressive multipart bodies and material attachment | More state, constraints, navigation and authoring work | Defer until a concrete creature requires it |

Paired support is not itself a shape. ECB and capsule are not mutually
exclusive with an independent locomotion law. Do not expose all combinations
of possible settings. Admit and test a small number of coherent profiles.

## Design: one movement kernel, selected environmental profiles

Separate these responsibilities in the design. This table is not a request
for six traits, six components or a plugin registry.

| Responsibility | Initial choices |
| --- | --- |
| Environmental contact features | Existing rectangle, fixed fighter ECB |
| Physical occupancy and bounds | Existing rectangle; bounded authored material appropriate to each new profile |
| Support configuration | Full span, central ECB foot, front/rear contacts |
| Locomotion and orientation | Existing lateral walker; gravity-upright fighter; admitted paired-support pitch; existing momentum/crawler policies |
| Geometry source | Fixed prepared metadata, state-dependent then animation-dependent metadata |
| Resolution | Existing specialized axis sweeps; admitted directional sweep and corner operations for new profiles |

Select the profile in the body definition, not by character name, AI/player
ownership or game mode. Scout in Adventure and Smash uses the same body
capabilities. A transformation may request a different profile. Shape changes
are admitted by movement; changing the rendered row cannot silently resize a
solid body. Preserve [one body, one path](../../concepts/one-body-one-path.md)
and the [unified movement kernel](unified-movement-kernel.md).

Start with these supported combinations:

- **Rectangle:** existing full-width support and native rectangular movement.
- **Fighter:** fixed authored bottom/top/side ECB features, gravity-upright
  pose, and an explicit occupancy/clearance contract. Scout is the pilot;
  Alice and Bob check reuse before adoption.
- **Quadruped:** two longitudinal support anchors, a simple admitted solid
  profile, and a bounded pitch rule. The companion dog is the pilot. Reuse
  the operations for other long bodies only when their needs agree.

Do not equate an ECB with generic diamond polygon intersection. Its contact
features can answer distinct questions. A fixed ECB may have a central foot,
wall extent at a selected height and an upper contact. The sweep must still
cover relevant sides/corners and fast motion; four independent endpoint rays
alone do not prove nonpenetration. If separate occupancy limits would reject
all intended ECB placements, revise the authored solid profile rather than
retain a contradictory rectangular solid.

Use body-local canonical coordinates and existing `BodyMaterial` mapping for
scale, orientation and reflection. Preserve the distinction between material
pose, gravity/reference axes, motion policy and observer view. Directional
features follow their declared frame. Initial walking proofs may use the
existing cardinal frame domain. Do not silently advertise arbitrary down.

A small prepared profile and bounded native operations are preferable to a
heap-allocated polymorphic physics object per body. The exact Rust interface
follows ECP3/ECP4 evidence. Keep the hot path specialized; extract common
operations when the two new customers demonstrate the common need.

## Terrain connectivity and contact admission

Keep authored geometry, prepared exposed material boundaries, and legal
support continuations distinct. A shared source may supply all three.
Coincident endpoints alone do not establish a junction. Respect solid sides,
one-way pass/boarding/drop rules, depth lanes, explicit junctions, hazards,
portals and moving-material identity.

Prepare adjacency and bounds once for stable geometry. An internal solid
face covered by neighboring material must not block legal travel. A real
exposed step or wall still blocks. Do not weld different moving occurrences
by proximity or erase geometry identity to make an anonymous floor. A dynamic
join must be valid at the relevant time and use the actual material interval.
Begin with compatible static block/line and line/line joins; do not require
arbitrary polygon union.

For a supported walker, propose the continuous grounded path in the selected
movement frame, including its change in height. Validate it against relevant
obstacles and admit only valid motion. This is not permission to snap through
ceilings, skip a step, erase a real impact, or ignore walls near an endpoint.
A branch needs its native boarding rule. Preserve ordinary lateral walking,
walkable angle (default 50 degrees), no sliding below the limit, crest
continuation and intentional momentum launch behavior. These are existing
defaults, not universal settings. Make surface-distance or momentum speed a
separate choice and charge navigation cost according to that choice.

For a continuous height profile `h`, measured against down, a body with local
lower boundary `b(u)` has the following minimum nonpenetrating root height
for this support alone:

```text
H(x) = max over valid u of (h(x + u) - b(u))
```

The full-width rectangle is the special case `b(u) = 0`. On a straight line
its midpoint clearance gap is `(width / 2) * abs(tan(angle))`. A support point
or narrower base can reduce that gap. This equation is not a general collision
solver: it assumes a single compatible height profile, known orientation and
applicable lower boundary. It does not establish overhead or side clearance,
one-way eligibility, branch choice, time of contact or occupancy at an edge.
Use local analytic segment queries when applicable. Let native motion admit
or reject the full candidate placement.

The result retains the supporting occurrence, source segment/place, contact
normal, applicable extent, accepted pose and motion time. Reuse SUP1's
material-place and time-sliced geometry. Loss of support, jumping, dropping,
portals and destruction have explicit release semantics. Do not independently
search for a prettier floor in navigation or rendering.

For simultaneous constraints, start with bounded directional sweeps and
coherent rechecks. If contact ordering still gives false blocking or missed
contacts, compare a small contact-plane solver with the specialized path.
Use a fixed work bound, deterministic tie rules and an explicit failure
result. Do not replace the rectangular kernel to complete a general solver.

## Spatial correspondence: one accepted mechanical pose

The [art-derived geometry rule](the-art-says-where-a-body-stands.md) applies to
all supported profiles. Preserve separate semantics:

| Geometry | What it answers |
| --- | --- |
| Environmental contact | How a body lands, walks and is blocked by terrain |
| Material occupancy | What solid body space must fit; spawn, crush, portal aperture and overlap decisions |
| Hurt/strike geometry | What can receive or deal a hit, with state and team rules |
| Broad-phase envelope | Conservative candidates, not a proof of contact or occupancy |
| Visual pose | How the accepted body, rig and attachments appear |

These are different projections of common authored geometry and accepted
placement. The same visible hand or weapon need not block terrain and deal
damage. A coat flap, hair or particle can be decorative. A visible solid
torso, planted sole, shield and active weapon must communicate their gameplay
location within declared tolerances. Do not infer gameplay shapes from image
alpha or render pixels. Author named features and intentional simplifications.

Use a bounded evaluation order. This is a logical order to reconcile with
existing phase ownership, not a new whole-tick ECS scheduler:

1. Evaluate the requested mechanical state/pose and the proposed local
   profile (fixed data first). Retain the prior accepted profile and phase.
2. Native movement admits the displacement and geometry change, including
   support and orientation. Retain the accepted physical trajectory.
3. Use the accepted support to propose bounded terrain-dependent rig changes
   when a profile requests them. Admit any change that affects material
   occupancy or contacts, or use the last valid/base pose. Do not iterate
   physics and animation until convergence. An obstructed mechanical shape
   change has a declared state-transition or constrained-pose result.
4. Publish one authoritative pose/geometry result for combat, attachments,
   body-owned surfaces and presentation at the declared cut. Preserve native
   contact/event timing and existing body-owned finalization. If geometry
   changes after the contact cut, it must not retroactively rewrite the path;
   schedule or admit it at the appropriate native boundary.

Do not move the complete art below the physical box as the general ramp
solution. A modest cosmetic correction for non-gameplay details may remain,
with a declared bound and fallback. If it moves a gameplay feature, derive
its hurt/strike/attachment placement from the same accepted adjustment. For
baked rows without suitable metadata or articulation, retain a compatible
mechanical placement and declared fallback. Do not introduce competing sprite
offset writers. Interpolate all gameplay-significant display geometry at the
same body/support cut, including hitlag, reaction, portal fragment mapping
and pose ownership. Presentation may smooth or interpolate between accepted
poses; it cannot make a simulation decision or shift gameplay independently.

Reuse the existing named rig anchors, body pose clock, mechanical geometry
publication and two-bone limb operations. Store/evaluate only necessary
mechanical features at simulation time; do not evaluate render meshes or full
skeletons merely to find fixed contact points. Publish true sole offsets,
stance/swing phase and reach limits if adapting feet. A planted foot can keep
a source place on moving material until toe-off or release. A leg adjustment
that cannot fit has a bounded fallback, not an unbounded solver.

### Paired support rules

The dog has front and rear contacts in its longitudinal frame. On one straight
ramp, their supported configuration can determine height and pitch by simple
geometry. On mixed segments, solve for a bounded admitted pose: rotating the
body changes the world locations of its anchors, so two height samples taken
at an old pose alone do not prove the final contacts. Check actual transformed
anchors and material clearance, including the swept rotation. A sharp crest
may intersect the torso even when both feet have valid targets.

Define two-contact, single-contact and unsupported states. A lone contact
must have a declared pivot/retained-orientation and release rule. It is not
automatic permission to bridge a gap or stand on a second unrelated object.
Do not average arbitrary nearest floors. Begin with same/static compatible
support, then admit more complicated moving and branching cases with evidence.
A stable flat-ground pose needs no pitch solve. Use explicit fallback for
unreachable contacts or invalid orientation. Do not simulate four independent
rigid legs.

### Dynamic ECB rules

After fixed profiles are useful, admit state-dependent changes before fully
animation-dependent changes. Prefer prepared collision tracks keyed by named
mechanical rows and simulation phase for the first fighter. Selected rig
anchors can supply tracks or evaluate equivalent compact geometry later.

Sweep actual previous and proposed geometry, not just the character root.
A stationary root may have a moving foot. A rising root may still have a
bottom point that crosses a floor. Specify floor eligibility for jump ascent,
shape changes, downward crossing, one-way platforms and moving material. Do
not let an animation manufacture a landing, teleport the root or bypass an
intentional drop. Landing and tech timing are deliberate tuning decisions.
Crouch/stand and morph transitions need clearance and explicit rejection or
constrained-state behavior. Keep previous profile/phase for resimulation and
contact intervals, including interrupted actions and hitlag. Coordinate with
[collision and CCD](collision-and-ccd.md), not a separate animation collider.

## Navigation: support locations, not flat platforms alone

ECP5 extends existing [navigation E4 and standing experiments](platformer-navigation-and-reachability.md).
Use a stable source occurrence and source place for a support position.
Separate geometry from body-dependent admissible standing/travel intervals:
profile, material, locomotion, frame, clearance, boarding and hazards can
split one geometric chain into multiple legal intervals. A continuous chain
is not necessarily one traversable span. Keep the original segment geometry,
not the chord joining its endpoints.

An arbitrary valid point on a supported span can be a start or goal. Use
actual accepted support when it is available; a request to project a target
onto terrain needs an explicit reachable projection rule. Never silently
replace an occupied upper slope with a floor underneath it. Connect adjacent
admitted positions along supports on demand (or by the existing sparse anchor
chain). Keep junctions, discontinuities and useful takeoff/landing anchors.
Do not create one graph node per tessellation vertex or a dense walking clique.

Travel cost includes the first and last walk and follows the selected motion
law: lateral length for constant lateral speed where applicable, arc length
for constant surface speed, or a declared rollout approximation. Retain E4's
arrival state and stable tie rules. Do not merge arrival states if future cost
depends on their different positions or velocities. Use exact native standing
experiments and rollout/follower for traversal links; do not duplicate profile
physics. Generalize starts, landings and recovery on slopes after ordinary
walking. Preserve the supported/unsupported and estimated/native distinctions.

Cache the prepared geometry by occurrence/revision and the admitted standing
experiment by relevant physical capability (including profile and mechanical
phase policy where necessary). Do not key only by name or current box size.
Do not build an all-phase dynamic graph for every fighter. Bound the selected
phase/stance used by a graph; verify at execution and replan or report
unsupported when the physical assumptions change. Moving support navigation
requires its own time/arrival model; static ramps can ship first. Cache drops
and rebuilds cannot change authoritative movement or route choice.

## Delivery gates

These are candidate milestones, not a second execution queue. Revalidate
HEAD, choose a concrete customer and move only the next executable slice into
[queue](../queue.md). ECP numbers identify scope, not a requirement to finish
all work before further game development.

| Gate | Deliverable | Evidence before promotion of dependent work |
| --- | --- | --- |
| ECP0 — baseline and Hobford | Reproduce direct input vs companion brain in the actual room with actual profile. Inspect contact/route facts and geometry. Fix a confirmed local defect. Create one reusable compact terrain course. | First failing frame and cause (or explicit unreproduced status); a focused regression if fixed; baseline timings. |
| ECP1 — connected terrain | Shared adjacency/exposed boundaries and valid supported motion for existing boxes. Preserve one-way semantics and SUP1. | Correct block-to-line, line-to-block and line-to-line travel both ways; real obstacles block; flat legacy rooms unchanged. |
| ECP2 — profile boundary | Separate prepared profile, material, occupancy, contact and bounds. Inventory rectangular assumptions. Keep native gateway and AABB fast path. | Existing rectangular fixtures and measured baseline; explicit unsupported paths; a usable seam for ECP3/ECP4. Extract only required shared operations. |
| ECP3 — fixed ECB | Scout pilot with fixed directional geometry, stable root and coherent corner/clearance law. Alice and Bob reuse check. | Same body in Adventure/Smash; ledges, one-way landing/drop, walls, tight passages, high-speed sweeps, combat alignment and relevant portal/scale cases. |
| ECP4 — quadruped | Dog pilot with paired support, accepted pitch and occupancy. | Grade, crest, valley, ledge release, overhead obstruction, jump/land and correct shared visual/combat pose. Use ECP3's terrain facts, not a new movement engine. |
| ECP5 — navigation | Continuous support starts/goals and on-demand travel, then jump/drop anchors from and onto slopes. First AABB customer may start after ECP1. | Stop, reverse, roam, replan partway up a compound hill; blocked branch rejected; native follower arrives; cache and rewind equivalence. Remove obsolete `slope_exit` path only after coverage. |
| ECP6 — pose adaptation | Shared accepted root/pitch from ECP3/ECP4, then optional stance-aware sole/limb adjustment and compact rig geometry. | Actual collision, hit/strike and attachments correspond to art; baked fallback; moving support, hitlag and release use same time cut. Root/angle correspondence is already required in the pilots. |
| ECP7 — dynamic profiles | State-dependent ECB, then one animation-track fighter experiment. | Previous/proposed profile sweeps, blocked expansion, apex/landing/tech/hitstun and one-way timing; intentional move retuning; deterministic rollback. |
| ECP8 — adoption and tools | Profile authoring, preview and diagnostics; migrate only where useful. | Per-character gameplay acceptance, target performance and no silent unsupported behavior. Rounded/compound profiles require a new customer and separate gate. |

Dependency intent: ECP0 -> ECP1 -> ECP2 -> ECP3/ECP4. ECP5 can deliver static
rectangular support earlier; extend it for each admitted profile. ECP6 shares
mechanical pose work with the pilots; richer feet are optional. ECP7 requires
the profile boundary, fixed fighter and common mechanical pose. There is no
requirement to wait for full dynamic ECB navigation before shipping a static
dog route. ECP8 runs incrementally. Do not migrate the full roster at once.

## Cross-owner audit before a profile is enabled

| Consumer | Required answer |
| --- | --- |
| Native movement, ledge grab, wall jump | Valid feature/anchor, sweep and release semantics; no whole-tick fork |
| Spawn, teleport, portals, blink, crush, solid overlap | Actual admitted occupancy, material and path; broad-phase AABB is not proof. Retain portal charts, clipping, event time and source identity. |
| Hazards, projectile contact, push and combat | Select the correct gameplay geometry and actual path; preserve strict-overlap versus touch policy and hit identity. |
| Mounts and moving/deforming surfaces | Material places and actual time intervals; no double carry or world-space approximation of local geometry. |
| Native scratch bodies and navigation | Same selected profile/stance and motion law; explicit unsupported or approximation result. |
| Authoring, rollback, views | Immutable prepared definition and necessary authoritative state; stable ordering; shared accepted cut; inspectable fallback. |

See [world geometry](world-geometry-and-spatial-semantics.md),
[scale-relative gameplay](scale-relative-gameplay.md),
[portal delivery](portal-parts-similarity-and-chirality.md),
[simulation determinism](simulation-authority-and-determinism.md) and
[rigged animation](runtime-rigged-sprite-animation.md). Do not enable a profile
in a live world that requires an unimplemented physical query. A bounded
prototype may reject unsupported operations with a diagnostic. Do not silently
switch its physical shape to a rectangle in another subsystem.

## Authoring and inspection

Prefer axis-aligned platforms and jumpable steps in ordinary levels. Use a
ramp for an intentional continuous grade, such as a hillside road or mount.
Do not approximate a correctly authored ramp with many invisible solid steps
just to avoid engine bugs. Keep existing stair-as-slope customers. Add seam
and angle diagnostics to the prepared geometry path, with the source IDs,
exposed face and legal continuation. Do not silently snap endpoints or join
incompatible materials. The level author can inspect the actual boundary.

Use one shared developer course with flat blocks, correct and bad ramp joins,
crest/valley, a one-way fork, a narrow passage and moving support. Include
character switching and current frame-step tooling. Extend the existing
geometry inspection view to show contacts, selected material place, admitted
profile, occupancy, broad-phase bounds, combat geometry and named anchors.
Show the prior and proposed profile if motion rejects a change. Do not add
an independent editor/visual physics layer. Show geometry of the accepted
simulation cut, not a guessed reconstruction from the picture.

## Cost and acceptance

Retain the rectangular fast path. Static terrain preparation and profile
metadata have their own lifetime; reuse them for native queries and navigation.
Use small fixed-size data, local candidate lookup, no heap allocations in
small contact solves where practical, bounded work, and deterministic ties.
Flat, unsupported and unchanged poses skip terrain adaptation. Query the
accepted support once for the required features. A quadruped does not search
the entire room for every foot. Use broad-phase envelopes that include the
actual admitted geometry interval, not only the root's displacement. Evaluate
mechanical poses once per needed simulation cut, not for every observer.
Do not let view distance or render detail change the mechanical pose.

Before changing the kernel, measure representative workloads with fixed
seeds/input, build profile, tick rate, room/body counts and device. Use the
existing measurement tools. Report CPU tick median and tail, relevant work
counts (candidates, contact solves, pose evaluations, allocations), navigation
build/query cost and rollback resimulation. Compare a dense flat AABB room,
ECB fighters in combat, mixed hill-follow companions and moving-support riders.
Measure on desktop and Android before broad adoption. No numeric time or
speedup target is asserted without a baseline. A material regression in the
rectangle-only workload blocks broad migration until explained and addressed.
Bound work per new profile; compare quality and cost before adding a generic
solver or index. The documented Hanoi graph blowup is a warning against
admitting every line as a standing surface without a meaningful work domain.

Prefer a few high-value behavioral fixtures over a combinatorial test matrix:

- Continuous flat/ramp/flat travel and compound hill both ways; no false
  blocking, unintended airborne tick, or penetration. Bad joins and genuine
  walls remain obstacles.
- One-way fork/boarding, drop-through, narrow ceiling, ledge and high-speed
  corner. Existing native collision, mount and Sanic witnesses still pass.
- Dog with direct input and brain; arbitrary slope start/goal, stop/reverse,
  crest clearance and loss of one contact. Verify actual Hobford after the
  general course; do not require a particular root cause before reproduction.
- Fixed fighter in Adventure and Smash; pose/geometry overlay for a planted
  stance and attack; dynamic-profile change while stationary or in flight
  when ECP7 is selected. Record intentional landing/attack timing changes.
- Selected moving and deforming support, reflected/scaled material and a
  90-degree frame; portal and crush/teleport checks before live enablement.
- Rollback with prepared navigation cache discarded; repeated runs have the
  same authoritative result for identical inputs. Use existing supported
  reconstitution/rollback evidence instead of creating a second harness.

ECP0 may be a local same-day repair. ECP1/ECP2 and the two new customers are
a multi-week design/implementation track, with useful smaller gates. More
precise effort estimates require the first reproduction and prototype. Stop
when the delivered profiles cover actual game needs; generic convex, capsule,
compound, continuous local gravity (CV3–CV5), planetary AI and universal IK
are not completion requirements. SUP1 remains its own motion-interval work.
