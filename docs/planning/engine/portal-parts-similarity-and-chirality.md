# Portal parts: exact partitions, similarity transforms and numerical failures

**State:** PARTLY IMPLEMENTED; structural corrections selected, 2026-11-21.
**Reviewed source:** `b91b29d359358ea417353afce4f472dc7fab15ec`.
**Execution:** [PORTAL-SIMILARITY](../queue.md#portal-similarity--canonical-parts-and-scaled-mirror-portals).
This is the delivery owner for the portal system. The supporting pages below
specify individual contracts; they do not create competing queues.

The review traced production code and existing witnesses. It did not run a
Rust build, a game or a device benchmark. The broad-phase arithmetic example
below was evaluated separately from the Rust engine. Source deductions and
proposed acceptance must not be reported as reproduced gameplay failures or
completed repairs.

## Structural decision

Keep the existing canonical material and `Sim2` model. Complete the contracts
between its consumers. The missing common object is an **accepted spatial
operation with its scope, selected topology, time interval and result**.
Several readers currently recover some of those facts from a final pose,
a reusable channel or a camera rectangle. That reconstruction loses meaning.

Four changes follow:

1. A portal relation has directed occurrences and admitted topology edits.
2. Body motion retains the accepted trajectory and chart transitions.
3. Spatial queries prepare conservative candidates and reuse their actual
   immutable inputs. Fast cases are specializations of that contract.
4. Presentation consumes a complete chart for each observer and admits the
   image work needed to show it.

Use existing owners and pure geometric values. This is not a request for a
universal scene graph, an ECS service locator, a new physics engine or one
solver for every movement law. Each change must remove a competing decision
or repeated computation at a demonstrated consumer.

The shared contract does not require one large record. Topology owns accepted
relations; motion owns its bounded trajectory; presentation retains only the
crossing projection and image facts it needs. Share compact immutable values
at these boundaries instead of making every reader depend on every owner.

## Product contract

A live portal is physically real. Bodies, limbs, projectiles, attacks and
supported world interactions act on its far side while material remains on
its near side. A portal does not create a second logical body.

- Keep each endpoint's full authored aperture. For half-lengths `h_a` and
  `h_b`, the directed scale is `h_b / h_a`; the reverse is its inverse.
  The map convention belongs to the admitted connection. Mixed authored
  rotation/reflection pairs cannot be interpreted by one global convention.
  A developer override is an explicit admitted experiment, not the source
  of each authored pair's meaning.
- A canonical primitive is partitioned into source subsets with disjoint
  interiors and complete coverage. Each spatial image has one composed map
  and an ordered itinerary. Finite opening boundaries remain finite.
- Preserve scale, rotation and reflection in accepted material. Facing,
  chirality, controller intent and observer orientation are separate facts.
- Exact collision, hurt and strike domains retain their own shape. Art can
  use a different domain under the same chart/cut contract. An AABB is a
  broad-phase bound when the exact source is not a box in that chart.
- Whole bodies must fit. A refused continuation cannot escape as an uncut
  successful part. Independent grants, attacks or contacts do not multiply
  merely because their material has several images.
- The world owns gravity direction, zone selection and world-authored fields.
  The selected [scale-relative gameplay contract](scale-relative-gameplay.md)
  converts canonical body gravity response with material scale, together
  with controls, to keep normal action timing at the new size. A resolved
  world frame or already mapped vector is not scaled again. Geometric scaling
  does not imply conservation of mass, kinetic energy or authored damage.
- Ray reach stays in its initial chart. Each world segment consumes its
  distance divided by cumulative scale. An exit nudge cannot spend authored
  reach or skip the first nearby hit; keep the implemented exact-face start.
- Physics, deterministic refusal and logical outcomes are the same headless
  and visible. View count, pixel density and capture capacity cannot change
  them.
- The selected scope is one live simulation room/rollback authority.
  Cross-island continuous portals and automatic history merging remain
  outside this work. Loading zones keep their own committed transfer road.

The [material-domain page](portals-a-body-in-two-places.md) defines motion,
owned reach, visibility and source-domain distinctions. The
[causal world plan](causal-world-commit.md) applies when a consequence leaves
an independently revisable history.

**Follow-up selected at `f41218979625`, 2026-11-21:** ordinary scaled
behavior is now an explicit product target, beyond geometric transit.
[SR1–SR3](scale-relative-gameplay.md) complete consumer units through existing
owners. [Presentation order and optional 2.5D rendering](presentation-order-and-25d-rendering.md)
adds cart membership and logical appearance order, then a second Bevy render
consumer. P2/P3 retain motion/contact authority; P5/P6 retain portal observer,
capture and readiness authority. These customers do not create a second
portal implementation queue.

These packets preserve current overlap contact for melee strikes and guards.
[Q172](../maintainer-decisions.md) (2026-11-26) keeps overlap contact for
melee until a gameplay need for continuous contact is shown.
Projectile/item continuous contact is in scope.

## What is already useful

| Established mechanism | Source or bounded receipt | Preserve |
| --- | --- | --- |
| Canonical material, similarity and typed numerical refusal | [frame](../../../crates/ambition_platformer2d_core/src/frame.rs), [Sim2](../../../crates/ambition_platformer2d_core/src/frame/similarity.rs), [partitions](../../../crates/ambition_platformer2d_core/src/frame/thread.rs) | One exact geometric vocabulary, including scale and parity |
| Recorded moving apertures and temporal material cells | [ApertureStep](../../../crates/ambition_platformer2d_core/src/frame/motion.rs), [MaterialCast](../../../crates/ambition_platformer2d_core/src/frame/material_cast.rs) | Full winding, contact time and material that exists between endpoint samples |
| Canonical ray reach through scaled hops | [Aperture trace](../../../crates/ambition_platformer2d_core/src/cast/aperture.rs), `a_ray_spends_initial_chart_reach_after_scaling_and_starts_on_the_exit_face` | Initial-chart distance, exact exit-face start and deterministic first hit |
| Playable unequal-width pair and repeated replay | `fddf6084a`; [portal_similarity_slice](../../../game/ambition_app/tests/portal_similarity_slice.rs) | Persistent material, reverse crossing, partial contact and authored lab customer |
| Simultaneous and chained limb contact | `2c4bd3822`, `99d51bc9f`, `6ae7a653a` | Independent limb roots with one logical owner and logical hit identity |
| Stable living channel assignments | [link resolver](../../../crates/ambition_portal2d/src/link.rs) | Adding another pair does not renumber the surviving pair |
| General drawable carry and retained storage | [carry](../../../crates/ambition_portal2d_presentation/src/carry.rs), [ClipQuadPool](../../../crates/ambition_portal2d_presentation/src/clip_material.rs); `ae8f2b07a`, `1d551594b` | Ordinary drawable coverage, independent families/depth and storage reuse |
| Conservative query preparation already attempted | `811150d6f`, `07d0356fb`, `d984a1c20` | Local queries and bounded work; complete the exclusion proof below |

The historical 80/40 slice proves its stated population. It does not prove
moving-exit root continuation, the target's retained path after rebase, two
observers in one room, or arbitrary candidate pruning. The former long P01–P10
and G1–G6 case files are replaced by the contracts and bounded packets here.
Git history and the [November 10 review](../../../dev/reviews/portal-correctness-and-cost-review-2026-11-10.md)
retain the investigation record.

## Spatial facts with distinct identities and clocks

| Fact | Identity and lifetime |
| --- | --- |
| Pair/end binding | Source/control slot, connection incarnation and placement revision are distinct; continuous motion keeps its occurrence, renewed physical bindings cannot alias old history |
| Topology revision | Which relations exist and are admitted at a publication cut |
| Aperture pose/motion sample | The accepted endpoint motion and the instant or interval being read |
| Material source | The logical owner, canonical primitive/region and its geometry revision |
| Query itinerary | Ordered directed occurrences plus the subset and time interval that took the path |
| Presented view | Observer/view identity, presented pose/projection and selected sample |
| Ready image | Source content revision, capture chart, pixel extent and completion state |

These are conceptual facts. Reuse existing scoped IDs, components and values
where they carry that meaning. Do not create one global identity registry.
A dense `u32` link index remains useful inside a selected query snapshot; it
is not a durable pair identity after the snapshot is replaced.

A simulation phase is an observation cut, not automatically a fraction of
physical time. A late held-port request cannot change a completed motion
interval. A topology publication names its effective cut; a motion record
names the geometry and time interval it actually used. Rendering chooses an
accepted presentation sample explicitly and can interpolate admitted motion.
It must not equate every pose change with a new pair occurrence.

Give each topology edit an effective physical instant as well as a publication
phase. A post-integration opening first exists at the declared end boundary;
a later system that evaluates an old projectile over the whole tick cannot
use it over that earlier interval. Clip each leg to the topology valid there.
If a mechanic needs an earlier opening, request/admit it before that motion,
or use an explicit split interval. Consuming a held request this tick does
not make its geometry exist retroactively. See the pair plan's phase policy.

A view consumes transitions only through the pose/time it actually presents.
Retain a compact ordered crossing projection until its observer cursor advances
or a declared scene/branch cut discards it. This can outlive contact queries
without retaining every full query scene. Bound that retention; an observer
that falls outside it takes an explicit cut, not a partially reconstructed fold.

## Topology has occurrences and atomic edits

The [pair and lifetime contract](portal-pair-identity-and-aperture.md) owns
the full proposal. The structural change is to prepare additions, removals,
replacement and occupied-end consequences as one portal edit.

Current pressure is concrete:

- A room replay's scope is erased by the unit `ClearPortals` bridge.
- A held recast can test overlap against its own pending-despawn predecessor.
- A transferred caster can retain a held record whose ends are dead or still
  resident in the old room.
- A reused channel or matching exit position does not identify which pair
  occurrence an action used.

The portal owner evaluates the resulting candidate relation set. It keeps
the old accepted relation and pose data while a closing connection drains or
an admitted eviction settles its material. Only then may it retire the old
binding. A voluntary failed edit retains the previous accepted binding.
Compulsory caster/host/room retirement needs a portal-owned drain/settlement
that does not depend on its dead issuer. Retain source residency until that
settlement finishes, or refuse a travel before its lifecycle commit. Never
half-commit a move and leave old material attached to a retired room. No
outcome reports an open record without admitted endpoints.

Authored assemblies bind pair names in their local definition and then to a
placement occurrence. Two instances of one mechanism must not accidentally
join four same-named ends. Palette choice, gun controls and the live relation
are distinct data. A gun pair's ownership policy must state item, capability
or explicitly shared network ownership instead of inferring it from a number.

## Motion commits a trajectory and a final state together

The [moving-frame plan](portals-are-moving-frames.md) contains the source
traces and equations. Three existing paths currently give different answers:

1. Temporal material casts follow moving maps over a recorded interval.
2. Free item/projectile movement can stop at a crossing and continue the
   remaining interval in the exit chart.
3. Generic kernel transit maps an already integrated end position using the
   map sampled at the crossing, then collapses `SweepSample` to arrival.

For a translating exit with velocity `u`, crossing at time `tau` in a tick
of duration `T` requires the exit's contribution over `T - tau`. Mapping the
old end position with the frozen crossing map omits `u * (T - tau)`. A static
map test cannot distinguish these results. Removing the completed path also
makes a later projectile query see a crossing victim stationary at its exit.

The accepted operation must retain:

- The source state and proposed movement law over the declared interval.
- The admitted geometry/restrictions and ordered contact/crossing events.
- Motion legs with their actual material charts, intervals and source turns.
- The accepted final state and its next-step seed.

A moving host is also a source of changed geometry. Admit its proposed
trajectory, hosted aperture motion and affected material before committing
the host. The current host adapter reads already published motion and has no
admission result back to that writer. Refusal retains an accepted combined
state or uses a separately admitted close/eviction policy; it cannot leave
the portal behind a host that has already moved. Continuous motion keeps
the connection incarnation but still requires this physical admission.

Reconciliation changes next-step contact/attachment facts. It does not erase
history that later consumers in the current tick need. Discrete arrival has
an explicit discontinuity; no straight collision segment joins remote points.

Bind root transfer to this result. The kernel already has a refusal recovery
that closes an unbacked aperture and continues ordinary falling. The later
unrestricted portal pass must not rediscover that refused crossing and apply
it. The existing `a_motion_refusal_steps_with_portals_closed_and_does_not_replay_an_old_sweep`
fixture is a starting customer; extend it through production transit.

Contact response uses the same point/time/path witness as detection. The
axis sweep currently tests approach with a static mapped displacement while
material contact can include a moving exit. Reuse `ContactVelocityFrame` and
`MaterialCollider::velocity_in_root` as the geometric moving-frame authority,
with body, support and aperture rates first expressed in the
[same comparison clock](portals-are-moving-frames.md#one-chronology-does-not-give-rates-the-same-unit).
The helpers do not convert a caller's room-effective seconds automatically.
Keep axis, momentum, crawler and bounce/slide policy differences explicit.

### Canonical aperture contact replaces post-step snag repair

[snag_straddlers](../../../crates/ambition_portal2d/src/snag.rs) projects the
world AABB into the mouth frame, then writes a positional carry after motion.
That envelope can reject a rotated body which actually fits. Its correction
also needs admission against other obstacles.

A source-derived example uses a 20-by-2 rectangle at 45 degrees. Align the
mouth normal with its long axis, give the opening half-length 12, and offset
the body 4 along the tangent. Actual material spans tangent coordinates 3 to
5. The projected world AABB has half-span 11 in both mouth axes. As the root
moves from normal distance 12 to 1, the envelope's plane section reaches 14,
so the snag pushes a physically fitting body back.

Use the canonical body's aperture-rim/contact constraint in the accepted
motion operation. Keep envelope rejection as conservative broad phase.
Delete the second post-step seam-position authority once that constraint
covers its customer. Closing an occupied portal remains a separate explicit
topology operation; this does not authorize general geometry-repair pushout.

## Query preparation must preserve every reachable material image

The existing performance correction filters room links before expensive cell
construction. Keep that direction. Its current proof is incomplete.

[MaterialCast::reachable_links](../../../crates/ambition_platformer2d_core/src/frame/material_cast.rs)
marks each directed link admitted once. Later frontier regions do not pass
through an already admitted link. But the partition explicitly permits
revisiting a link with a different region and transform. A visited endpoint
is not a visited spatial state.

### Curved support includes the pivot distance

The same candidate builder maps bounds at the two endpoint poses and pads
by the resulting box diagonal times total turn. That does not bound an
image rotating far from a pivot: small shape size is not orbital radius.

Use a tiny stationary source at `(-1,1000)`. A's entry at `(0,0)`, normal
`-X`, half-length 2000 turns through a full revolution; its fixed exit at
`(10000,0)`, normal `+X`, has the same size. Under Rotation convention,
quarter-turn material lies near `(11000,1)`. Both endpoint maps place it
near `(9999,1000)`. Padding by that small endpoint box's diagonal misses
the intermediate image. C has entry `(10990,1)`, normal `-X`, and exit
`(20000,1)`, normal `+X`, both with half-length 3. Its entry can be reached
there but excluded from the candidate set. Its reverse entry stays outside
the propagated bounds. C's entry also lies outside the rotating entry's
swept disk, so this is not an overlapping-opening example.

The entry sweeps the source front-to-back; the full-interval temporal query
supports the carried material. Not every source is a free body whose query
ends at the first root event: posed material and full-tick queries also use
this API. A later dynamic rebase does not establish a universal query-prefix
invariant that would justify these bounds.

Bound curved support using distance from the relevant pivots, recorded turns,
translation and composed-path motion. Endpoint equality does not remove full
winding. Establish conservative bounds independently of the exact solver,
which cannot discover a link the broad phase removed.

### A finite repeated-link counterexample

All ends below are fixed vertical apertures under Rotation convention. Entry
normals point left and exit normals point right. Include both directions of
each pair. The source box is `x=[-72,70], y=[-0.5,0.5]`, rooted at `(-1,0)`,
with zero translation and A as the current first link.

| Link | Entry center / half-length | Exit center / half-length | Forward point map |
| --- | --- | --- | --- |
| A | `(0,0) / 10` | `(100,0) / 10` | `(x+100,y)` |
| B | `(110,0) / 1` | `(-10,3) / 2` | `(2x-230,2y+3)` |
| C | `(200,3.8) / 3` | `(1000,3.8) / 3` | `(x+800,y)` |

Material point `(69,0.4)` takes A to `(169,0.4)`, B to `(108,3.8)`, then A
to `(208,3.8)`. It is now through C's finite opening. On that second A image
it misses B's narrow tangent interval, so B does not recapture it first.
Use sufficient part/hop budget and finite backing hosts that do not obstruct
the later beside-B route.

The current bound-propagation loop, including its one-unit padding and
inverse links, selects A, B and their inverses, then stops. C is absent because
A's later region is not propagated. A separate arithmetic evaluation confirms
that exclusion; an engine fixture must establish the exact resulting contact
or refusal. This is not evidence of an executed production tunnel.

### A conservative query plan

Candidate propagation carries the source image bounds and interval, with the
remaining hop/scale allowance. A previously visited link can be skipped only
when the earlier propagated envelope subsumes the new relevant state. A
bounded worklist can merge conservative envelopes and revisit a link when
its envelope, time coverage or remaining traversal allowance grows, or retain
path-specific states. The owner must establish
which rule conservatively contains every admitted continuation.

If candidate preparation cannot prove an exclusion within its work bound,
include the unresolved candidates or return a truthful preparation refusal.
Do not silently close an otherwise reachable connection to make the search
cheap. The exact solver still applies finite apertures, material budgets,
ordering and numerical refusal.

Preserve these distinctions:

- Candidate bounds may overestimate; exact cuts/contact decide the result.
- `max_hops`, `max_pieces` and scale policy bound supported material meaning.
- A numerical/work refusal means the operation was not established. It is
  different from a proved miss and from a gameplay-disabled portal.
- An improved algorithm may resolve an old work refusal. It may not convert
  an unproved route to an accepted miss or bypass the body's declared policy.

### Reuse preparation and specialize execution

Follow the [execution specialization and work selection contract](execution-specialization-and-work-selection.md).
The general material meaning and the cost of obtaining an answer are separate
design concerns. Keep full declared expressiveness available for reference
evaluation; make ordinary gameplay select a proven cheaper execution path.

The existing implementation already filters reachable links, caches fixed
aperture poses, specializes affine cells, uses primitive contact for an uncut
cell and queries separate cell envelopes through spatial indices. Preserve
those gains. The [November 12 correction](../../../dev/reviews/portal-stall-handoff-2026-11-12.md)
also stopped repeated expensive refused motion; adding accepted history must
not undo that containment.

#### Three responsibilities

| Responsibility | Required behavior |
| --- | --- |
| Material semantics | Canonical shape, admitted topology and convention, finite openings, time/winding, chart/path identity, contact ownership, overlap rules, body budgets and typed refusal. |
| Reference execution | Expensive unpruned relation evaluation and general geometry for offline/debug comparison; permitted to report an unresolved numerical result. |
| Production execution | Conservative local selection, proven specializations and bounded remaining general work under the same material contract. |

The reference is not an always-on shadow query or an automatic unbounded
fallback after a gameplay work limit. An offline higher-precision calculation
can diagnose a mathematical or conditioning error; it does not silently redefine
the engine's operational rounding, stable tie order or boundary ownership.
Compare production specializations with the operational reference under those
same rules. Cache warmth or eviction cannot decide whether an operation
commits, defers or exhausts its work allowance. Cold and warm paths need the
same deterministic publication/continuation policy; retain prepared inputs
required by admitted live state under their owner's lifetime contract. A new
algorithm may resolve an old work refusal; it may not turn an unresolved
route into an accepted miss or bypass authored material limits.

#### Select work before exact root and cell queries

Prepare admitted relation coefficients and conservative swept-aperture bounds
once per changed topology/motion snapshot. Index those bounds. Root trajectory
and finite material queries select their candidates from this shared input,
then retain their distinct exact crossing/fit and partition predicates.
Full turns and remote pivots need their complete conservative envelopes.

Today [`MaterialCast::motion_admission`](../../../crates/ambition_platformer2d_core/src/frame/material_cast.rs)
searches every supplied link before `reachable_links` narrows cell work.
Axis collision calls this search before
`cells` can call it again. Keep the prepared root result explicitly, including
**searched and no crossing**, with its interval and relation generation.
Pass the result to cell construction and the accepted motion operation.
Delete repeated endpoint sorting, partner lookup and later root rediscovery
as those consumers migrate to the same admitted relation/root witness.

A query with no local aperture candidates performs no exact root solves for
distant portals. Distant aperture motion cannot make its cell program curved;
the source's own turn or collider motion may still require curved contact.
A conservative revisit remains a new image/interval/allowance question; an
index is not permission to restore the one-visited-bit rule. If uncertainty
retains a genuinely broad candidate set, charge its work to that operation's
bound. Do not expand every ordinary body's work to all room links just because
one operation is difficult.

#### Keep specializations inside the common material authority

| Proven query class | Execution |
| --- | --- |
| Uncut primitive with linear relative motion | Inline single material image and native primitive contact. |
| Static/translation or affine cuts and maps | Existing analytical builders/contact, including instant snapshots of turning apertures. |
| A proved shared rigid motion | Factor out that motion where the remaining geometry/cuts become static or affine. |
| Genuinely curved relative motion | Existing bounded continuous solver with full winding and conservative bounds. |

Choose from admitted inputs and active dependencies, not player kind, room
name, capability installation or a tolerance that treats a real turn as zero.
A common-frame specialization needs a shared admitted motion law or exact
algebraic proof. Matching velocity at one instant is insufficient. The source
trajectory, relevant cuts and ordering predicates, and queried collider must
all have suitable residual laws in that frame before contact becomes static
or affine. Host-local opening geometry alone does not establish that for a
free body. Start with aperture/face relations already recognized by
[`line_parts` and `excludes_rigid`](../../../crates/ambition_platformer2d_core/src/frame/material_cast/arcs.rs);
preserve each body's eligibility and path mask. Keep the uncarved solid
authoritative and independent nearby obstacles intact. Transform query
geometry and contact derivatives without changing the owner's force law.

The curved evaluator should also follow the expression it evaluates.
[`Scene::states`](../../../crates/ambition_platformer2d_core/src/frame/material_cast/arcs.rs)
currently samples every selected link even for a map-only request. Prepare
the cell's path, active cuts, holes and ordering predecessors
once. Map/velocity evaluation needs the path; cut evaluation also needs those
predecessors. Share their results within one time/interval evaluation instead
of repeatedly building all-link pose vectors. Keep fixed-pose caching and
existing whole-interval exclusions. Do not accumulate an unbounded cache of
adaptive sample timestamps.

#### Prepare a leg once and retain compact history

| Reusable input | Required identity / invalidation |
| --- | --- |
| Validated canonical shape and local bounds | Source geometry revision. |
| Relation coefficients and aperture envelopes | Scoped topology version and accepted motion sample. |
| Collider index and host-local opening geometry | Owning geometry/motion revision and admitted relation identity. |
| Prepared source leg, root result and cells | Exact material/source curve, interval, relation selection, overlap mode and body budget. |
| Observer cut/image | Observer, drawable/sample and ready source image. |

World contact, recipient contact and payload interpretation should borrow the
same prepared leg. Projectiles prepared source cells for recipients and
rebuilt them inside [`query_world_collision`](../../../crates/ambition_platformer2d_shared_tangle/src/projectile/collision.rs).
**Removed 2026-11-21 (E1):** the projectile system passes its leg's source
cells to `query_world_collision_in`. The two builds were the same cast: the
canonical box, the material map, the line from the leg start, the interval,
the prefix and reported initial overlap. `query_world_collision` keeps
building its own cells for callers without prepared ones. No time was
measured: the removed work is one source-cell build (with its reachable-link
search) per projectile leg.
Items rebuild the accepted prefix after a world sweep. Return the admitted
prepared prefix with its contact/witness where the inputs match.

A restriction keeps the original curve and clock, closed event strata,
initial-overlap behavior and prefix-only budget admission. A refused suffix
must not prevent an earlier valid solid stop. First reuse successful prepared
prefixes under the existing search; incremental temporal-frontier reuse needs
its own proof before it replaces that search. A root rebase creates a new leg.

Retained motion stores compact geometry provenance, source curves, intervals,
accepted path/maps and relation generations. Keep the ordinary one-leg case
inline. Heavy cell/scene graphs are transient, bounded query products shared
at an exact reader cut; they are not copied into permanent per-body rollback
history or retained for each observer. Views keep compact crossing projections
through their cursor. A rollback or topology edit must not change the meaning
of an input still borrowed by a query.

#### Preserve bounded, truthful refusal continuation

Work allowance bounds algorithmic effort; hop, piece and scale policy bound
supported material meaning. Preserve the owner's deterministic admitted
prefix/restriction or refusal recovery, and make later readers consume it.
Do not restore repeated full-speed attempts of the same refused operation on
every tick. Reusing a refusal requires identical relevant inputs; changed
source, host motion, topology or budget asks a new question. A numerical
failure does not itself prove a wall or authorize a shared topology mutation.
Preserve the movement owner's named per-operation narrowed recovery: its
accepted restriction belongs to that operation and its history, and later
readers must not reopen it or mistake it for a global pair edit.

Host/aperture admission uses accepted occupancy/path dependencies and swept
envelopes to find affected bodies. It must not rebuild every body's cells for
each moving host. No affected material is a cheap result, with room identity
and body/relation occurrence preserved. This is an index over the same
accepted material, not a second occupancy authority.

Verify contact/path identity, rounding/tie rules, typed admission and rollback
alongside cost. Use no-portal, local static, genuinely curved, revisiting-chain,
remote-pivot and repeated-refusal cases. Adding moving links outside the
conservatively reachable query geometry must not increase exact root/cell
work; world plus recipient queries must prepare an unchanged source leg once.
Measure matched shipped-host timing
separately from geometry work counts and offline reference time. Extend the
existing owner fixtures and receipts; do not add a permanent shadow solver or
generic benchmarking/guard framework for this work.

## Presentation is a query of one complete observer chart

The [view/capture plan](portals-one-transit-for-every-mover.md) owns the details.
Current restrictions are concrete: one portal eye per room, a host sample
that drops camera rotation, channel-derived private layers, and capture
allocation before view demand/readiness is known.

Retain one presented sample per observer. It includes the persistent camera
chart, final projection after shake/finishing, actual viewport, and the
visibility eye/facing chosen by policy. That eye can follow the body while
the camera center eases or clamps elsewhere. Preserve both facts.

For a root transition `H`, persistent world-to-screen projection `P` satisfies
continuous chart handoff when `P_after * H = P_before`. Compose transitions
through the same presented body sample in their actual order. Apply transient
finishing to obtain the final footprint used for capture and clipping; do not
feed shake/finish zoom back into persistent chart transport. Subject scale
already affects the ordinary focused camera; consume that fact once.

A reflection changes determinant sign. No continuous invertible camera
transform can ease that sign back to positive. Genuine continuous transport
retains the reflected view chart. An explicitly unmirrored view uses a named
cut/fade/reframe policy. Camera policy does not rewrite body chirality or
input interpretation. The mode must state which continuity it promises.

One room can have multiple observer queries. Captures are reusable only when
their complete source/projection/sample meaning is equivalent. Room, channel
or matching image size alone cannot prove equivalence. Parallax belongs to
the selected view/source room; it cannot inherit every live room's backdrop.

Admit visible capture work before allocating its targets. Retain explicit
ready-image state. An open live window updates each presented frame; a count
limit cannot justify a stale or uninitialized opaque pane. Retain existing
quad/mesh/image pools and independent drawable family depths.

For a complete, already prepared image source, reuse its texture/UV/bounds
with the same cut geometry. Arbitrary draw families retain capture. This
removes a repeated raster pass where equivalence is established without
requiring each gameplay effect to learn about portals.

## Numerical refusal and authored feedback

Preserve validated finite positive scale, orthogonal orientation, invertible
composition, representable bounds and precision limits. Unsupported gameplay
scale is a policy refusal, not a false floating-point overflow.

An actual numerical fault uses the existing bounded Floating-Point Exception
effect: IEEE 754 letters and reason-appropriate NaN, infinity, zero or bit
fragments. An out-of-policy scale can use the separate EXPONENT BIAS accent.
These are ordinary presentation projections of typed outcomes; no IEEE special
value enters accepted simulation state.

The operation/occurrence and fault lifetime identify the event. Replaying a
tick or viewing it twice does not create another logical failure. A continuing
rejected attempt is not a fresh fault merely because the tick advanced.
Changing pair occurrence or the attempted operation can create a new result.

## Implementation plan

The queue selects bounded packets below. Preserve current product work and
existing static/affine paths. Run the first failing production witness at
its owner before claiming a source-derived scenario repaired.

### P1 — Scoped pair edits and lifetime

**Owner:** portal relation/admission; native reset and held-port adapters.
Start with a manual reset in one of two live rooms and same-position held
recast. Use the current public phase/installers. Evaluate removals and
replacements before candidate overlap; publish actual accepted/refused state.
Then close or explicitly rebind the lease on room transfer.

**Delete:** erased reset scope, self-overlap against retiring ends, stale
held-open records and geometry-based pair-use matching as each path migrates.
**Accept:** the other room's pair survives; replacement at the same place has
one defined outcome; old and new occurrences cannot alias through channel
reuse; a moved caster never edits foreign-room endpoints.

**First slice done (2026-11-22).** `ClearPortals` carries the reset's room
(`None` only for a New Game, or a reset whose room cannot be told), and
`clear_portals_on_reset` clears the gun pairs and transit cooldowns of that
room alone. The content bridge names the room of the replay's subject, else
the sole live room. Witnesses: `a_reset_of_one_live_room_spares_the_other_rooms_pair`
(portal core) and `a_manual_reset_clears_the_room_of_its_subject` (bridge).
Same-place replacement: the held-pair lowering judged an asked pair against
ends it had just released, because a despawn is a command and the queries
still held them; the replacement fizzled and stayed spent. The ends it closes
this tick, and a held pair that fizzles, now leave the overlap set first, so
the defined outcome is that the replacement opens
(`a_pair_asked_where_a_released_pair_was_opens_the_same_tick`, with the
still-held control fizzling). Two held pairs that trade places in one tick
still resolve in submission order.

**Second slice done (2026-11-22).** A recast slot reuses its link
(`held_pair_link` is per owner/module/slot), so the new pair resolves the old
channel; the only per-body record that named the old occurrence by channel was
`PortalTransitCooldown { pair }`. A pair the lowering closes now ends the
cooldowns of its room that name it or its partner
(`a_released_pair_ends_the_cooldowns_that_name_it`, a foreign cooldown as
control); a straddled pair is never closed, so `PortalTransit.straddling`
cannot alias. The lease is of the holder's room: a holder that moved to the
other live room releases the pair it left there and its ask opens a pair in
its own room, so a moved caster never edits another room's ends
(`a_holder_that_changes_rooms_leaves_its_pair_behind_closed`). A pair a body
still straddles in the old room is kept, unedited, until it is empty.

**Remaining:** publish the accepted/refused outcome of a held request to its
module (a reply port: opened, fizzled, refused for no identity/room), so a
module stops re-asking a spent slot by its own bookkeeping.

### P2 — Accepted motion, root transitions and retained history

**Owner:** core motion authority with portal adapter and free-mover consumers.
Extend the existing refusal fixture through the real post-movement transit.
Add a translating exit at a nonterminal crossing time and a projectile hitting
a target before/after that target crosses. Preserve one shared world clock,
each native clock's mapping onto it, the remaining interval and domain-specific
force/contact policy. Event chronology alone does not normalize velocity.
Retain compact leg provenance and crossing facts, with the ordinary single
leg inline; heavy cell/scene products live only through their query readers. Preserve
bounded refusal continuation and bind any named narrowed recovery to the
accepted operation instead of retrying an unchanged expensive refusal forever.

**Delete:** independent crossing rediscovery and final-pose history collapse
for migrated continuous motion. Retain the root result, including a searched
absence, so construction and later readers do not repeat that work. Keep
discrete arrival distinct.
**Accept:** refused routes stay refused; static maps retain their behavior;
moving maps include residual motion; later readers see the completed legs;
multiple crossings, late aperture birth, zero-progress bounds and replay keep
ordered outcomes. A completed path retains its selected topology intervals.
A driven host gets refusal before its aperture would create invalid
occupancy; host, aperture and affected material cannot commit separately.
Find affected material through admitted occupancy/path dependencies rather
than rebuilding every body's cells for each host. Views retain compact
crossing projections without keeping full collision programs resident.

### P3 — One physical contact witness through response

**Owner:** material contact and movement constraints.
Route approach classification and response through point/time/path-relative
velocity. Integrate exact aperture-rim fit into the same admitted operation.
Use the rotated thin body and an exit moving into a stationary obstacle.

Also use a stationary body with an admitted moving obstacle. Zero source
displacement does not justify endpoint-only repair when relative geometry
moves. Preserve the separate hitlag/frozen-body clock policy.
[SUP1](slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion)
owns the changing support's source trajectory; P3 consumes that witness
instead of reconstructing lost local motion from displacement and turn.
The detailed [M3 owner](portals-are-moving-frames.md#m3--resolve-contacts-through-the-complete-velocity-witness)
retains the response law. The local support correction can start before
P2, while a portal-hosted case requires P2's accepted interval and admission.
Changing portal hosts also require the
[material attachment and point trajectory](portals-are-moving-frames.md#driven-aperture-motion-is-admitted-before-the-host-commits);
current world arc distance cannot identify the same material under stretch.

**Delete:** the static-vector reconstruction of moving-frame relative motion
and the post-step AABB snag writer once their replacements cover the case.
**Accept:** an exact-fitting rotated body passes; a truly oversized body is
blocked; moving-frame contact cannot be discarded by a contrary static sign;
contact correction cannot push the body through another obstacle.

### P4 — Conservative query planning and proven fast cases

**Owner:** core material query preparation and the room geometry adapter.
Two bounded cuts can start independently. First share the projectile's
prepared source leg between world and recipient contact, removing the
demonstrated duplicate without changing candidate selection. Separately turn
the full-turn and repeated-A examples into owning Rust witnesses; correct
curved bounds and state propagation while preserving local work selection.
Compare the selected query with the complete eligible link set for its
supported result, including the contact/refusal difference. The exhaustive
mode tests candidate selection, not the shared narrow phase's correctness.
Retain the no-portal, static and affine cases throughout. Then compile exact
cell dependencies and admit further relative-motion specializations only
where their conditions hold. Host-local opening preparation does not itself
prove that a free body's contact is affine.

**Delete:** endpoint-only visited exclusion, duplicate world/recipient source
cell construction, repeated immutable/root preparation and irrelevant
all-link pose evaluation at the migrated consumers.
**Accept:** alternate paths and revisits cannot hide a reachable opening;
adding moving portals outside the conservatively reachable query geometry
does not force a local straight query into the curved solver or exact root
search; one unchanged source leg serves world
and recipient contact. A valid prefix remains admissible before a refused
suffix. Source/clock/topology changes invalidate the right value; cold/warm
preparation preserves deterministic results and refusal continuation. Use
offline reference checks and production work counts separately, including
repeated refused traffic; no always-on general shadow query is introduced.

### P5 — Observer chart and ordered handoff

**Owner:** camera snapshot/response and portal view extraction.
Use two observers in one room, a rolled rectangular viewport, two transitions
between presented frames, and a reflected asymmetric scene. Retain current
subject-scale adoption and authored camera priority.

**Delete:** first-eye-per-room selection, axis-aligned reconstruction of a
rolled view and last-event overwrite of a composed camera transition.
**Accept:** each eye sees its own aperture footprint; ordered maps compose;
continuous mode meets the screen equation at the selected sample, including
several fixed ticks per render; shake/finish is not compounded. Parity
restoration uses its named transition policy and never an impossible
interpolated matrix.

### P6 — Demand, ready pictures and one source rasterization

**Owner:** portal presentation with body-image and parallax providers.
Select observer/path demand and private view bindings before target allocation.
Readiness gates exposure or dependent execution; admitted targets can be needed
to produce that ready image in the same render graph. Keep the open-window
freshness rule and existing window pixel density; its tier cap controls count.
The carry's resolution/proxy fallback is a separate budget policy. First share a
complete composed-body source image under an explicit equivalent-image proof;
retain capture for the other families.

**Delete:** room-wide demand for every animated root, duplicate capture of
an already complete source, cross-room parallax copying and unready live panes
at the selected consumers. Preserve existing pools.
**Accept:** same-channel rooms remain isolated, two eyes receive correct
pictures, capacity has a visible defined fallback, and ordinary sprites,
meshes, text, children and independent limbs retain portal coverage/depth.

### Material consumers at the existing sample

The `00bf92c22142` follow-up selects two bounded continuations under this
delivery owner:

- [T3 publication coverage](portal-pair-identity-and-aperture.md#t3-refinement--a-publication-replaces-only-its-declared-population)
  preserves foreign room geometry during partial replay. Its first native
  witness needs no complete connection-ID migration or new motion solver.
- [Present material occupancy](world-geometry-and-spatial-semantics.md#present-material-occupancy)
  makes discrete water contact and physical switch touch use admitted body
  parts. The result retains its native region/target and map for response,
  facing and preference. Keep ordinary exact-box paths and reuse the existing
  core partition; do not reconnect copied/glued world authority.

These queries can use current supported snapshot-local admission. They do
not wait for all P2–P6 work, nor redefine samples inside an accepted interval.
P2 still owns chronology; P3 owns moving contact; this follow-up owns the
missing interpretation at the existing sample. Navigation's independent
static-geometry correction remains with E4.

P1's scope repairs and P4's exclusion correction can start independently.
P2 establishes the chronology consumed by P3 and camera handoff; the rolled
viewport and view-identity portions of P5 do not need to wait for it. P6 uses
the view contract. These are dependencies of meaning, not a requirement for
one large cross-crate commit.

## Evidence and measurement

Use existing owning tests and the authored portal labs. Extend the smallest
behavioral witness for each corrected boundary; do not add a source-text
policy, registry census or exhaustive combination suite.

Preserve the 80/40 pair, reflection/return, independent limbs, a chained third
link, free projectile and held-item cases. Check the ordinary portal-free
body and an independently selected portal-free composition where the changed
boundary applies. Exercise replay at the actual commit/read cut.

The [November 12 performance review](../../../dev/reviews/portal-desktop-performance-2026-11-12.md)
contains useful historical evidence of fixed-pose reuse. It is not a fresh
baseline for this tree. New measurements must separate:

- Cost per simulation tick from the number of ticks per presented frame.
- Candidate discovery, cell construction, narrow contact and caller repetition.
- View/carry CPU work from GPU capture work, uploads and target allocation.
- Source images, finishing images, carry targets and window targets in bytes.
- No-portal, fixed pair, moving pair, alternate/repeated paths and refused work.

Compare matched inputs on the same build/device. Report the mechanism removed
as well as time and memory. This review environment has no runnable Cargo
toolchain or GPU proof; the review makes no new runtime or compile-time claim.

## Steering the next implementation

Start from the current checkout, recheck the source facts against the reviewed
baseline, and read the selected packet plus its contract page. Preserve the
full-aperture, material/chirality, frame-aware and ordinary-drawable rules.
Pick a packet that removes one current competing authority. Reproduce its
named case through the real writer/consumer pair; make the accepted operation
carry the fact the consumer was reconstructing; remove the replaced road.

Do not flatten all movement models, add a per-character portal exception,
recover a continuous trajectory from end positions, key a lifetime by color,
close a connection to hide an unproved candidate exclusion, or call a picture
current before its producer is ready. Compress the receipt when the bounded
packet is complete.
