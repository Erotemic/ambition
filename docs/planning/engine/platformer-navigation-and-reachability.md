# Platformer navigation and reachability

**State:** OPEN strategic capability. The first slice landed 2026-10-09 (one
room, one body, walk/jump/drop). Advance from concrete movement/AI/world
customers rather than the current fighter rollout regression.

## Goal

Provide reusable, capability-aware answers to questions such as:

- can this body reach/support itself from here;
- which movement capability makes a route feasible or impossible;
- what support/landing surface is physically available;
- how should an autonomous actor reason across platforms, portals, gravity
  frames and moving geometry without using privileged game-specific shortcuts.

Navigation should consume the same movement/collision/body semantics used by
simulation rather than maintain a parallel "AI physics" model.

## Current foundation

The repository already has useful pieces:

- platformer geometry/collision/movement kernels;
- body-local gravity/reference-frame semantics;
- perceived world/terrain views for actor brains;
- support/floor queries;
- reusable recovery probing through the movement kernel;
- `RecoveryLens`, which evaluates body-specific recovery capabilities for fighter
  decision support;
- physical conflict/event reporting where mechanics report what happened and
  higher-level policy decides what it means.

These primitives are substrate. Their existence does not prove every consumer's
decision policy is correct.

Substrate locations: `RecoveryLens`
(`crates/ambition_combat/src/brain/fighter/recovery.rs`); support queries
`is_support_surface`, `support_face_separation` and `body_on_support_side`
(`crates/ambition_platformer2d_core/src/collision_semantics.rs`); and the
`CollisionWorld` questions `solids`, `carves_only`, `hostable_surfaces` and
`base` (`crates/ambition_platformer2d_world/src/collision.rs`).

**Navigation exists for one room and one body (2026-10-09).** The first
slice is below ("The first slice"). It is also the foundation
[`agentic-character-runtime.md`](agentic-character-runtime.md) needed.

**Explanation vocabulary.** For "why is this route shut", reuse `WhyNot { term,
subject, observed }` (`shared_tangle/src/authored_logic/mod.rs`), which
`GatedLockWallVerdicts` publishes per authored wall. It explains policy gates
only, not physical reachability. It has no production reader yet. So:

- do not invent a second explanation type; a physical "the body cannot make
  this" answer joins `WhyNot`'s vocabulary;
- bring a reader with the first slice;
- do not add a second producer (for example the encounter lock walls in
  `ambition_encounter_features/src/lock_walls.rs`) until something reads the
  first.

## Tentative spatial gravity: the field is part of the experiment

[Curved surfaces and local gravity](curved-surfaces-and-local-gravity.md#navigation-and-room-rules-need-the-field-along-the-route)
does not select a new navigation program. Current `TraversalEnvelope::measure`
rejects noncardinal frames; retaining a whole `MotionFrame` in a cache key
does not establish a position-varying field along a future route.

When an autonomous planetary actor is selected, candidate legs must run
against the same bounded field and geometry as native movement, evaluated
at the body's changing positions/times. Equal initial down does not make
two such experiments equal. More per-angle cache entries cannot supply
the missing environment law. Keep support/branch connectivity separate
from feasibility, and retain complete experiment inputs under the existing
native rollout and static-geometry owners. A human-only circular proof can
precede this work and must not claim to establish general NPC navigation.

## Planned E4 — retain the state needed by route cost

**Source review, 2026-11-21 (`b91b29d35935`):** this is selected structural
work under [execution specialization and work selection](execution-specialization-and-work-selection.md).
The existing graph and kernel rollouts remain the foundation. This review
did not run a new navigation census or game. The numeric example below is
an arithmetic witness of the search reduction, not a shipped-room result.

**Follow-up at `f41218979625`, 2026-11-21:** the standing probe also drops
physical material scale. Correct its input contract before sharing its
envelope across more rooms or actors. This is source evidence, not a new
runtime failure reproduction or a timing result.

### A surface is a connectivity node, but arrival position affects cost

[`NavGraph::route`](../../../crates/ambition_platformer2d_world/src/navigation/graph.rs)
keeps one `best(cost, arrival_along, predecessor)` and one `done` bit per
surface. Its next-edge cost includes the walk from `arrival_along` to that
leg's takeoff. A cheaper arrival does not dominate a dearer arrival at a
different place. The cheaper prefix can require a much longer next walk.

For an abstract admitted graph with run speed 1, use these leg costs and
positions. Each intermediate leg starts where the prior leg in its path
lands, except for the stated walk on M.

| Route segment | Cost | Arrival or departure on M |
| --- | --- | --- |
| S to M | 1 | Arrives at 0 |
| S to X to M | 1.5 + 0.5 | Arrives at 100 |
| M to G | 1 | Departs at 100 |

The current surface reduction retains M at cost 1 and position 0, discards
M at cost 2 and position 100, and returns cost 102. Retaining both arrival
states finds cost 3. A separate arithmetic evaluation of this reduction
confirms the difference. `route` also stops when it reaches the goal surface,
before charging the final walk to the requested goal point.

This is a defect in the current graph cost model's fastest-route claim.
It is not a claim of fastest physical motion: leg costs come from rollouts
that start at rest, walks use distance/run speed, and graph construction
retains one preferred leg per surface pair. Keep those approximation limits
explicit. Fixing search state does not certify discarded leg variants.

**Fixed (2026-11-22).** `NavGraph::route` now searches arrivals: the start,
or the landing of one link (`links.len() + 1` states, with a binary heap
ordered by cost and then state index). It ends with the walk from the last
landing to the goal point, and it stops only when no open arrival can beat
the cheapest finished route. Witnesses in `navigation/graph/tests.rs`:
- the table above (route by X, cost 3; the direct way costs 102);
- a dearer link that lands by the goal beats a cheaper one 900 px away;
- on a five-surface graph, all 20 ordered surface pairs agree with an
  exhaustive enumeration of link chains.

Same-surface queries still return an empty route (the body walks). The
model limits above are unchanged: legs are measured from rest, and the build
keeps one leg per surface pair.

### Lift route state to movement anchors

Prepare takeoff and landing anchors on each standing span. Admitted legs
connect their exact anchors with their current costs. On a span that the
current graph treats as walk-connected, sorted adjacent anchors connect by
the same walk-distance cost. This represents the full pairwise walking
distance with a linear chain, rather than a dense walking clique.

Attach the current feet and clamped goal point at query time using the
existing surface-selection and arrival rules. Include the terminal walk.
Keep stable link/anchor tie rules; allocation order cannot select a route.
If a future cost reads another fact, such as arrival velocity or platform
phase, its state must retain that fact or declare the approximation. Do not
merge two states solely because they name the same platform.

The graph's boolean reachability may still use the surface projection under
its present walk-connectivity assumption. That cheaper projection is valid
for reachability even though it loses information needed by route cost.
This is the specialization boundary: share one admitted graph, then use the
smallest state that preserves the question being answered.

### Define the standing experiment before caching it

The current
[`NavGraphKey::probe_body`](../../../crates/ambition_platformer2d_world/src/navigation/cache.rs)
creates a default, identity-material body and assigns the key's `size` to
`canonical_size`. Both consumers supply a world size. The
[`advisor`](../../../crates/ambition_platformer2d_actor/src/body/navigation.rs)
uses the live stance size; the
[`autopilot`](../../../crates/ambition_platformer2d/src/autopilot.rs)
uses standing base size times material scale. Thus the probe keeps a box's
dimensions while changing the physical body those dimensions describe.

This changes the movement law. In
[`step_admitted_motion`](../../../crates/ambition_platformer2d_core/src/movement/kernel.rs),
material scale converts canonical control speeds, accelerations, jump speeds
and ability distances to world units. The reviewed runtime leaves ordinary
gravity response unscaled. A half-scale actor has half the authored jump
speed in world units;
the current graph probe has the smaller box but the unscaled jump speed.
The same kernel cannot make these inputs equivalent. The existing
[`material-scale fixture`](../../../crates/ambition_platformer2d_core/src/movement/kernel/tests.rs)
states the control/gravity distinction, and the shipped
[`80/40 portal pair`](../../../game/ambition_app/tests/portal_similarity_slice.rs)
is a current customer for a body that remains at half scale after crossing.
Those are source fixtures, not fresh test results from this review.

The maintainer now selects [the same timing at different scale](scale-relative-gameplay.md).
The frame owner will convert canonical body gravity response too. Keep this
new target separate from the current-source observation above: a resolved
world frame remains resolved, and world-authored fields retain their units.

**Decision:** navigation owns one explicit standing experiment description.
Construct both the cache key and the probe from it. It describes a reusable
question about this physical body at an admitted standing start. A separate
current-state continuation can use the live state when a customer needs it;
it cannot silently borrow the standing experiment's result.

| Input class | Standing experiment rule |
| --- | --- |
| Motion specification, abilities, canonical standing dimensions and required base-size facts | Retain the exact admitted values. Character name and world AABB are insufficient. |
| Material scale and required shape/control mapping | Preserve physical units. State the supported orientation and parity mapping; normalize only when the same experiment and its application are preserved. |
| Full motion frame and probe settings | Retain gravity separately from external acceleration, and keep the actual step/probe inputs. Equal net acceleration is insufficient. |
| Position, velocity, temporary stance and maneuver history | Set the declared standing start state. These resets define the question; they do not prove applicability from every live state. |
| Room geometry, observed feet and goal | Geometry selects the room graph. Feet and goal remain live query inputs, outside the reusable motion experiment. |

Keep unit conversion with each physical owner. Carry admitted material into
the scratch body so that the existing kernel converts canonical controls.
Use the shared frame derivation for canonical gravity response and retain
its decomposed result. Do not multiply a resolved frame, duplicate
`for_material` in navigation, or bake scaled parameters into a body that
the kernel will scale again. The same standing experiment drives key and
probe under the selected scale law.

The orientation boundary needs an explicit first slice.
[`TraversalEnvelope::measure`](../../../crates/ambition_platformer2d_world/src/navigation/envelope.rs)
currently admits cardinal frames and measures extents through `size()`,
which does not include material turn. Crawler support and righting can change
that turn during live motion. Copying every live angle into the graph key
could cause a new graph every tick. Admit a stated aligned standing geometry
and control mapping first. Normalize a transient orientation only under
that start contract; otherwise return a specific unsupported experiment.
Do not substitute identity material and return a success-shaped result.
The standing profile can remain cached while the live body is crouched or
righting. Only the existing physical authority admits the live shape or turn;
leg entry waits until the body satisfies its stated start conditions.

Profile equality is also not leg-entry proof. The current
[`NavLeg` follower](../../../crates/ambition_platformer2d_core/src/navigation.rs)
starts Commit from a grounded body near `leg.start`, with a bounded side
velocity. The graph rollout instead starts a fresh body at zero velocity,
settles it, and probes a finite set of start offsets. State and check the
standing-entry facts needed for a retained leg, including its admitted
geometry/control mapping. A normal live transient can wait or reapproach;
it does not by itself invalidate the reusable standing graph. These remain
bounded rollout witnesses under a stated controller, not a proof over every
velocity, ability history or input sequence.

Keep maneuver readiness at that live entry boundary too.
[`Grounded refresh`](../../../crates/ambition_platformer2d_core/src/body_clusters.rs)
can restore dash charges without clearing its cooldown, while a fresh probe
starts with no cooldown. A `DashHop` leg still needs the movement owner's
actual [dash admission](../../../crates/ambition_platformer2d_core/src/movement/abilities.rs).
Do not put the ticking cooldown in the graph key or
assume grounded means ready. Start with the verbs this graph uses and retain
existing wait/replan behavior; this does not require a new full-state proof
framework.

Migrate the advisor and autopilot together. Delete the separate `BodyTuning`
reduction, world-size-as-canonical-size construction, and competing stance
policies. The autopilot already uses standing size to avoid repeated builds
during crouch/drop-through; the advisor must use the same projection when it
asks the same standing question. Do not replace these reductions with a hash
of the whole scratch body or all ticking components.

### Prepare reusable questions at their own lifetime

[`NavGraph::build_on`](../../../crates/ambition_platformer2d_world/src/navigation/graph.rs)
remeasures `TraversalEnvelope` for each room graph, although the envelope
uses an independent probe world. Prepare it by the standing experiment
defined above: canonical geometry, physical units, movement law, full motion
frame and `EnvelopeProbe` inputs. Retain
kernel/follower semantic identity if an offline product is later stored.
A room geometry edit invalidates room legs; it need not repeat an unchanged
empty-world motion experiment. Never key this by character name, net
acceleration, or rounded movement parameters.

**First slice (2026-11-22).** `NavMotion` holds the traversal envelope and,
for a body that flies, its flight law. Both are measured in probe worlds of
their own. `NavGraph::build_measured` builds a room's graph from a measured
motion, and `build_on` is now measure-then-build. `NavGraphCache` keeps
motions by the graph key minus its room: the exact spec, abilities, size,
base size and whole motion frame, which is how the graph key is already
keyed. The advisor's `graph_of` uses `get_or_build_measured`, so a body
navigating several rooms measures its motion once. Witness:
`a_body_built_in_two_rooms_measures_its_motion_once` (both graphs equal a
fresh build; a body of another size is measured again).

**Seat autopilot (2026-11-24).** The autopilot still builds on a
background task, and now hands that task the motion its `NavGraphCache`
keeps (`kept_motion`). Only a body never measured measures in the task,
which returns the motion with the graph for the cache to keep
(`keep_motion`). A graph for a new room, or for the same room after a block
broke, no longer measures the body again. Measuring stays off the frame.
Witness: `a_second_rooms_graph_reuses_the_bodys_measured_motion` (two
rooms' graphs, one measure).

**Demanded reachability (2026-11-22).** `NavGraph::reach_from` searches a
surface's reach the first time it is asked for and keeps it with the graph
(one `OnceLock` per surface; the graph does not change once built).
`reaches(from, to)` answers membership from it. The advisor's waypoint,
placement and errand checks, `place_beside` and `waypoints` read the kept
answer, so they no longer allocate and search each time. `reachable_from`
remains, as a copy. Witness: `a_surfaces_reach_is_searched_once_and_kept`.
Seeded waypoint selection is reused (2026-11-24): `waypoints` keeps its
answer by the feet's surface and the seed (the last 32 pairs), so a body
asking every tick with a held seed no longer copies and sorts the reach
list. The answer is the one chosen fresh (`choose_waypoints`), and feet
anywhere on the surface read it. Witness:
`the_waypoints_are_reachable_and_the_same_for_one_seed`.

The [advisor](../../../crates/ambition_platformer2d_actor/src/body/navigation.rs)
asks `reachable_from` through waypoints, target placement and errands.
That routine allocates and traverses the graph each time. Retain ordered
reachability or membership for demanded origin surfaces in the immutable
graph. Reuse seeded waypoint selection only for the same graph, origin and
choice seed. Feet and goal observation remain live inputs.

**Lifted destination queries landed (2026-11-24).** `NavGraph::to_goal`
searches once from a goal over link landings (fixed points, so every edge
cost is fixed) and keeps each link's cost on to that goal, for the last
eight goals a graph was asked for. `next` reads it: of the links leaving
the feet's surface, the least walk to takeoff + leg + onward cost, with ties
going to the lower link. Only the walk from the feet is a per-tick input. A
body walking to a still goal (a waypoint, an errand item, a stand point)
asks the same table each tick instead of searching forward from its feet.
Every answer is the same with or without a kept table, so a rewind reads the
same answers. A moving goal is a new key each tick and costs one reverse
search, which is no more than the forward search it replaces. `route` stays
as the reference. Witness: `one_search_from_a_goal_answers_every_start`
(each link's kept cost equals the enumerated cheapest chain from its
landing; `next` starts a cheapest route for all 20 surface pairs; a second
ask reads the kept table).

The lifted cost graph permits reusable destination queries with fixed edge
costs. Start with the demanded goal set and exact dynamic attachments; do not
precompute an unbounded all-pairs table or enumerate every possible actor
loadout. Two actors share a goal result only when their admitted graph,
observed goal and route policy agree. A shared target entity does not prove
that their perception samples agree.

[`NavGraphCache`](../../../crates/ambition_platformer2d_world/src/navigation/cache.rs)
has a bounded FIFO. The advisor retains 16 graphs; a repeated ordered scan
of 17 distinct demanded keys can evict each one before its next use and
rebuild all 17 on each tick. Treat currently admitted demand separately from
the spare derived cache. Pin selected graphs for their live use, then use
bounded retention for inactive reuse. Name memory and activation bounds; a
larger constant alone does not establish a working-set policy.

**Done (2026-11-22).** `NavGraphCache::begin_demand` starts a cycle; a key
asked for through `get_or_build*` since then is demanded and is never
evicted. `capacity` now bounds the spare graphs: past it the oldest spare
graph goes, and with none spare the cache grows to its demand. The activation
bound is the number of distinct live keys; the memory bound for inactive
graphs is `capacity` (16 for the advisor). The advisor begins a cycle each
tick. The seat autopilot never begins one and keeps its plain FIFO. Witness:
`graphs_asked_for_each_tick_are_not_evicted_before_their_next_use` (17
keys at capacity 16 build once over two ticks; a key no longer asked for is
the one evicted).

### Delivery and evidence

1. Establish the standing experiment at both consumers. Use one owner-level
   scaled-body control/envelope witness and one actual graph/follower ledge
   case, with an unscaled control. Choose the ledge from kernel observations,
   not a continuous formula assumed to match the discrete simulation. An
   equal-world-size pair with different canonical size/material scale must
   not share a result merely because its boxes match. Check that an ordinary
   crouch/drop-through does not rebuild the standing graph, while a physical
   scale change selects the correct new experiment.
2. Extend the route owner with the two-arrival witness and a final-goal walk.
   Prepare the anchor graph and compare against an exhaustive small-state
   search over the same admitted legs. Keep the kernel/follower and explicit
   cost-model limits; delete the one-best-arrival-per-surface assumption.
   This search correction can proceed independently of the input correction.
3. Split envelope preparation from room graph construction and retain
   demanded reachability results. Compare cold and reused results for a
   changed room, two equal-net but different full frames, changed body size
   and a changed movement law. Include the physical-scale distinction from
   step 1. Keep the existing cache-drop replay witness. Count envelope
   preparations, graph builds and retained bytes for alternating stance,
   crossing scale, and two rooms that share the same experiment. Reject a
   design that adds graph misses for each ordinary support/righting change.
4. Retain actual live demand and share demanded destination work. Exercise
   the over-capacity ordered key set, a moving observed goal, retirement and
   graph replacement. Measure probes, builds, search work and retained bytes
   before choosing more elaborate reuse.

Derived preparation must not make worker completion an input to a brain.
Publish complete selected values at a deterministic readiness boundary or
use the synchronous exact query for this graph contract. Do not expose a
partly built graph or make cache eviction mean `NoRoute`. In-memory factoring
is the first slice. Portable graph serialization and whole-content artifact
admission are separate work; this packet does not lift their existing hold.

Keep refusal meanings bounded too. `NoRoute` can mean that the admitted
navigation policy found no route; it does not prove that no physical motion
exists. Unsupported experiment, incomplete preparation and an absent route
in a complete admitted graph are different facts. Preserve the distinctions
needed by the current consumer without adding an exact runtime search or a
new planner framework.

## Static geometry coverage — native support before finite preparation

**E4 follow-up at `00bf92c22142`, 2026-11-22.** This is a source audit,
with concrete geometry constructions below. No new native route, test or
timing run was performed. Preserve the landed arrival-state search, motion
reuse, reachability reuse and demanded-graph retention receipts.

The next boundary is upstream of route search. Native movement rollouts
cannot recover a support that candidate preparation removed. Nor can they
repair a flight occupancy map that omitted a native solid family.

### Three different interpretations of stationary geometry

| Consumer | Current source rule | Missing native distinction |
| --- | --- | --- |
| [`chain_walks`](../../../crates/ambition_platformer2d_world/src/navigation/surfaces.rs) | A flat requires `filled` or a painted/solid polygon beneath it. `solid_polygons` includes `earth`. | Presentation fill does not create mechanical support or a solid interior. |
| [`standing_surfaces`](../../../crates/ambition_platformer2d_world/src/navigation/surfaces.rs) | The whole polygon AABB becomes a final forbidden standing interval. | Bounds select possible obstacles; a non-box shape does not occupy every point in its bounds. |
| [`FlightMap::build`](../../../crates/ambition_platformer2d_world/src/navigation/flight.rs) | Blocks, exits and room boundaries populate clearance; chains are absent. | A stationary closed Solid chain still blocks native motion. |

[`SurfaceChain`](../../../crates/ambition_platformer2d_core/src/world.rs)
explicitly marks `filled`, `earth` and `hidden` as presentation data.
Native [material support](../../../crates/ambition_platformer2d_core/src/walkable_lines.rs)
uses line faces, outward normals, body material and contact. Full-solid chain
contact uses native shape geometry. A Ground chain's painted interior is not
that of a closed Solid chain.

The navigation module explains why it excluded bare lines: admitting stacked
disk tops made graph construction expensive. Its 23 ms / 4.9 s example is a
historical source comment, not a current measurement. Preserve finite work
selection. Express it as navigation policy instead of visual fill.

### The useful split: native candidates, admitted work, covered answer

1. Discover native support and occupancy under the current body experiment
   and stationary geometry scope. Retain the actual face direction, solid/
   one-way meaning and source shape. Presentation data cannot change this
   candidate set.
2. Select the finite supports, anchors, movements and local clearance work
   that this navigation profile prepares. Cheap support discovery need not
   schedule a native jump rollout between every pair.
3. Publish a complete result for that selected coverage. An endpoint or
   primitive outside it differs from a route search exhausted within it.
   Cache residency and worker completion order cannot select the meaning.

This is the concrete geometry customer for E4's existing distinction between
unsupported, incomplete and no route. Keep that distinction through
[`advise_navigation`](../../../crates/ambition_platformer2d_actor/src/body/navigation.rs):
it currently turns a missing target surface into `ErrandSight::NoRoute`.
[`settle_errands`](../../../crates/ambition_platformer2d_actor/src/body/errand.rs)
and [`settle_visits`](../../../crates/ambition_platformer2d_actor/src/body/visit.rs)
make that terminal. `NavGraph::next` also calls a missing goal
`Unreachable`.

**Local routes.** Autoplay uses the standing map when both endpoints resolve
to one surface. For two surfaces it measures links between them in
both directions. The existing kernel verifies each admitted link. If the
local graph cannot answer the request, or its route fails, autoplay requests
the full graph. The same follower runs both plans.
`a_continuous_walk_skips_link_rollouts` checks a walk, a jump across a gap,
and a request that needs the full graph.

A consumer may explicitly decline work outside its supported profile. It need
not wait forever or run exhaustive motion search. It must retain why it
declined instead of presenting an unexamined endpoint as an exhausted
admitted route. `NoRoute` remains a result within the stated policy, not a
proof that no physically possible motion exists.

Keep coverage narrow enough to use. Start with primitive families, stationary
scope, the selected body experiment, demanded endpoints/supports and completion
of the admitted preparation. Do not create a general proof language or a
universal spatial query service.

Finite demand may omit destinations, support anchors and candidate movements.
It must not make unchecked obstacles transparent along an admitted movement
or flight corridor. Within the declared static world, a relevant native
blocker must be checked, proven irrelevant by bounds, conservatively blocked
under the stated policy, or leave the query unsupported. An omitted support
and an unchecked obstacle are different coverage facts. Flight needs this
distinction even where no native movement rollout validates its grid route.

### Native clearance with cheap box paths

Retain the existing AABB interval and grid-marking fast paths, including their
declared margins and grid resolution. Exact shape membership at a queried
position does not by itself certify continuous travel between samples.
For an admitted non-box obstacle, use its bounds to find candidate spans or
cells, then refine against native shape occupancy before removing them.
Standing and flight can share that narrow geometry operation where the
questions agree. Their support, hazard margin and one-way policies remain
separate.

The first flight correction includes stationary closed Solid chains. Preserve
directional support rules for Ground/one-way surfaces; do not turn every line
into a full wall to simplify the grid. Declared clearance margins may remain
conservative. An undeclared entire bounding rectangle is not the same policy.
Do not run a full-world material cast for every empty cell.

The advisor's authored-room approximation and the seat autopilot's stationary
overlay remain explicit scopes. This packet does not add moving-platform,
dynamic-gate or portal-transition planning. In particular, replacing every
input with `solids_for(subject)` is incorrect: current holes, glue and gate/
contact permissions describe present contact state. Freezing them across a
hypothetical route does not supply future passability. A future transition
model must ask the native owner at the right state.

### First witnesses and deletion

Use one existing visit/errand or seat-autopilot customer. Establish these
small geometry/control pairs before broad room testing:

| Construction | Required distinction |
| --- | --- |
| A still open Ground chain from (40,200) to (360,200), empty earth, body 20×40; toggle only `filled` | Native support and navigation eligibility under the same declared policy stay the same. If omitted by policy, the reason is explicit. |
| Floor top y=300; body 20×40 with feet (120,300); Solid triangle (100,200),(200,200),(200,300) | The box x=110..130, y=260..300 is clear below-left of the diagonal. Replacing the triangle with its full bounding rectangle makes the same position blocked. |
| In an 800×600 room, a stationary closed Solid chain wall x=390..410, y=0..600; body half-size (10,20) | Flight clearance blocks the wall as native movement does. An equal-bounds block is the control. |
| A real target support deliberately outside selected preparation | The native request reports unsupported coverage or follows an explicit fallback; it does not become an unexplained terminal `NoRoute`. |

The triangle currently removes feet interval x=90..210 before any rollout.
The chain wall currently leaves its interior cells free. These are
source-derived expectations for the first implementation witnesses.

Keep an ordinary box-only room as the direct-path control. On the stacked-line
case, count discovered supports, admitted motion edges, native rollouts,
graph builds, retained bytes and cold/reused work. A correct support projection
must not restore eager all-pairs disk jumps. Choose a bounded demand policy,
then measure its actual cost rather than increasing cache constants.

**Delete after migration:** painted-earth/fill authority over support and
obstruction, polygon bounds as final shape occupancy, undocumented chain
omission in flight, and terminal outcome conversion that erases needed
coverage meaning. Keep current E4/K1 caches and keys unless an actual input
changes; this review established no stale-cache correctness defect there.

[World geometry](world-geometry-and-spatial-semantics.md) records the shared
query law. This refinement stays in navigation under EXECUTION-COST and
does not depend on completing portal or environment queries.

## The room graph is navigable by its doors

Runtime routes can use doors and gates in one search:
`GateNetworkCatalog::route` adds only links that `resolve_dial` permits under
the live gate facts. `RoomHop::dial` names the terminal action before entry.
The catalog owns addresses and availability; the route does not copy them into
a game-specific table. Autoplay opens the physical terminal, sends the normal
host intent, waits for the ring, and enters through the ordinary room transition.

`NavGoal::FlyInto` requires the body to enter an opening. `FlightMap::route_into`
searches for a free body-center position inside the entry region, and the shared
flight follower tests that region at arrival. A nearby grid point outside the
opening is not success. `LoadingZone::reached_by` is the shared body/path test
for the traveller and the room transition. Guards: the world tests for both
edges and a sealed box, and
`a_route_over_rooms::autoplay_uses_the_gate_network_and_returns_from_the_primeval_trail`.

`scripts/checks/check_world_graph_is_navigable.py` checks the shipped worlds: every
`LoadingZone` targets a real area, and no area can be entered but not left. The
engine's own response to a dangling target is a build-time `eprintln!` on a door
that then does nothing, so the script is the guard.

- An area is a level's `activeArea` (camelCase), falling back to the level id.
  The script verifies the key against `LdtkLevel::raw_active_area`.
- `RoomLink.bidirectional` is authored on most zones. Read LDtk through a
  structured `fieldInstances` parse; a Rust/RON text matcher cannot read it.
- The predicate is "a door is a `LoadingZone`". Portals are not modeled. No
  shipped portal pair spans two areas, and the script fails (with a control
  arm) if one does. When that happens, teach the check about portal pairs.
- The script reads the map submodule through symlinks. It exits 3 and skips
  when the submodule is absent, so it is not evidence between two machines.

Audit the predicate, not only the corpus. A corpus the matcher cannot read
looks like coverage.

## Important correction from fighter measurements

The earlier fighter `recovery_below` experiment exposed a decision/
integration defect, not a need for a private navigation engine. That
level-6 regression is recorded as closed in
[fighter brain](fighter-brain.md#level-6-recovery-integration-regression-is-closed).
Do not resume its old trace as current work.

The 2026-11-22 [REC1–REC3 review](fighter-brain.md#rec1--execute-the-recovery-after-the-move-starts)
identifies a different crossing: a forecast can select motion that the live
controller does not execute. Preserve E4's completed arrival-state search,
motion measurement and cache work. Supply actual live model/frame/state to
the existing recovery probe, preserve a usable forecast policy, and prepare
only demanded queries. A standing capability experiment and a falling-body
snapshot remain different inputs.

Preserve subject-specific geometry too. Fighter's recovery lowering drops
`PerceivedSolid::open_for_self`, so its body probe can collide with or stand
on a gate that body passes. Reuse the existing passability contract and
perception scope. The same gate can still block a projectile; that is not
the body's query.

For mounted flight, observe effective control routing and both bodies'
clearance. A snapped free grid cell does not certify the connector from the
actual start, and a carrier reaching a point does not establish a released
rider's support. REC1 uses existing sweeps, `route_into` and flight stopping
behavior for these boundaries. Its first return needs only the applicable
parts of REC2/REC3; it does not wait for a new general planner or all motion
families.

A body may recover through drift, jumps, flight, walls, ledges, recovery
moves, impulses, portals or grapples. Bounded search failure, unsupported
motion and physical impossibility must not become one answer.

## The first slice: a surface graph a brain can follow (2026-10-09)

Customer: the companion dog in the basement of the central hub. Jon,
2026-10-09: the dog must "navigate to random waypoints, and not jump in a fixed
pattern". Nothing in the slice is about the dog.

| Part | Where | What it is |
| --- | --- | --- |
| Traversal envelope | `ambition_platformer2d_world::navigation::envelope` | How far and how high one body's running jump and walk-off go. Measured by the kernel in an empty probe world. |
| Standing surfaces | `...::navigation::surfaces` | Where a body of a given size can stand in a room. Blocks that touch at one height are one surface. A wall, a low ceiling or a hazard takes its stretch out. |
| Surface graph | `...::navigation::graph` | The legs between surfaces. A leg is proposed from the geometry and the envelope, and kept only when a rollout in the room, in the kernel, arrives on the surface it names. Routes are a shortest-path search over the legs. |
| Leg contract | `ambition_platformer2d_core::navigation` | `NavLeg`, and `follow_leg`: the ONE rule that turns a leg and the body's state into input. The graph builder and the brain both call it. |
| Advisor | `actor_monolith::features::ecs::navigation` | A system in `ActorDecisionSet::Observe`. For each body whose brain navigates it writes a `NavAdvice`: places the body can reach, and the next leg to the brain's goal. Keeps the graphs (`RoomNavigation`). |
| Roam brain | `ambition_characters::brain::state_machine::roam` | Policy. Chooses a place, follows legs, rests. Catalog preset `Roam(...)`. |

The advice has three things: places in reach (seeded by the brain's own count
of choices, not a clock), the next leg to the brain's goal, and a place beside
the body's target when a route goes there (`target_place`). The last one is a
goal that moves: a brain that follows or chases asks for it each time it
chooses. Roam uses it to keep near the player. A `MeleeBrute` whose profile
says `navigates` uses it to go to a foe on another surface. No shipped profile
says so: no room has platforms a brute's jump reaches.

Both brains drive one `NavFollower`
(`ambition_characters::brain::state_machine::nav_follower`): the goal, the leg
and its clock. A brain owns where to go. The follower owns how the body gets
there.

Whom a body attends to: its combat target when it has one, and if not, the
nearest player in its room. A peaceful body has no combat target
(`ActorTarget.entity` is `None`), so a companion could not keep near anybody by
the target alone.

LIMIT, a perception leak: the place beside the target is computed from the
target's true position. A brute asks for it only while it chases, and it
chases only a foe it believes in, but the place is where the foe IS and not
where the brute believes it is.

CLOSED 2026-10-09, from a review of f0409fcc (two contracts that were not
what they said):

- The advisor kept a graph under the body's NET acceleration. A graph is
  built from the whole motion frame: which way is down, the gravity a jump law
  can scale, and the external acceleration it cannot. Two bodies with one sum
  and two frames shared the graph of the one that asked first. The key is the
  whole `MotionFrame` now. Guard:
  `navigation::tests::two_frames_with_one_net_acceleration_are_two_graphs_in_each_order`
  (actor monolith).
- A leg in the air "arrived" on any surface at the height of its landing.
  The graph's rollouts had a second test of their own (the surface the leg
  names); the live follower had none, took the leg as done, and forgot its
  misses, so it could not give a goal up. A leg carries the span of its
  landing surface now (`NavLeg::land_span`), and `follow_leg` asks for it: the
  one rule holds for the builder and for a live brain. Guards:
  `a_landing_at_the_right_height_on_another_surface_has_failed` (core) and
  `a_follower_that_lands_on_another_surface_counts_a_miss_and_gives_up_after_three`
  (characters). Rollback schema 338: two more `f32` for a leg in progress.

LIMIT, the checksum: a brute's follower is rewound (the `Brain` is stored by
clone) and is NOT in the brain's checksum cursor. `MeleeBrute` has no cursor
arm (`SnapshotCursor for Brain` writes tag 0 for it; its `mode` was not
written before this slice). Roam's follower is written. So two peers whose
brutes hold different legs agree on the checksum until the bodies move apart.
No shipped profile navigates, so each shipped brute's follower is idle. The
content that first sets `navigates` adds the arm, which is a payload schema
bump (three baselines).

An author's page is [`docs/systems/npc-navigation.md`](../../systems/npc-navigation.md).

Build cost, measured by YardratAmbition 2026-10-09 on 76 shipped rooms and two
bodies, before any cut: about 0.25 to 0.3 ms for each rollout; 25 builds are
over 20 ms; the worst is `hall_of_characters` at 185 ms. A graph is built on
the first tick a navigating body is in the room.

After the cut (YardratAmbition, `nav-census`, merged 2026-10-09): a rollout
that stands still on the ground in its run-up has failed (a walk-off into a
wall: these were most of the wasted steps), and a body below its landing and
falling has missed (`follow_leg`, so a live brain plans again sooner too).
Links are unchanged in all 152 rows; kernel steps 1,263,906 to 988,461; the
slowest build 184 to 112 ms; builds over 20 ms, 25 to 17. Rejected on
measurement: a tighter envelope prefilter (lost links in 6 rooms), and "keep
the first proposal that arrives" (total route cost +12.6 %). The census is
`game/ambition_app/tests/nav_graph_census.rs` (`--ignored --nocapture`).

OPEN, cost:
- 17 builds are still over 20 ms (the worst: `central_hub_complex` for the
  dog, 112 ms). A body that is placed in a room is advised on the room's first
  tick, so today the build is at room load already. A body that starts to
  navigate in the middle of play (a spawn, a brain change) pays it then, as a
  hitch. The graph is a pure function of authored geometry and a body's
  tuning, so it can be built when content is built and shipped as data. That
  is the direction; nothing builds it yet.
- CLOSED 2026-10-09: a body is born with a motion model that is not its own.
  The integrator writes the body's tuning into the model on each step
  (`step_body`), so the model is right from the first step on. The advisor
  does not advise a body the kernel has not stepped
  (`BodyGroundState::contact_initialized`), and one dog is one graph
  (`companion_dog::the_basement_dog_...` holds it). YardratAmbition found the
  cause by reading the integrator; two signals I tried first (`PosedBody`, the
  prepared character's motion model) were wrong.
- CLOSED 2026-10-09, found on the way (its own slice): the brain snapshot's
  movement law was the config's tuning alone, and the integrator prefers a
  body's `AuthoredMovementTuning`. Measured in the hall: of 138 bodies two
  author their feel (Mary-O and tall Mary-O), and both were told run 270,
  jump 520 and one air jump while they move at run 300, jump 450 and no air
  jump. The snapshot now resolves the law as the integrator does
  (`movement_law_of`). Guard:
  `a_brain_is_told_the_law_its_body_moves_by` (a patrol that asks Mary-O for
  100 px/s gets 100; it got 111.1 with the fix taken out).

The rules the slice holds:

- **No second physics, and no second follower.** A leg is in the graph because
  the body did it in the kernel with `follow_leg`. A brain follows it with
  `follow_leg`. A second copy of either makes the graph a guess.
- **A brain reads no room.** The advisor is perception: it writes a plain value
  into the `BrainSnapshot`. The brain owns the goal and the rhythm.
- **Intent, not position.** A leg becomes ordinary locomotion and jump intent in
  `ActorControlFrame`. The control gate and the body's abilities apply as they
  do to any brain.
- **The graph is derived, and built whole.** It is a pure function of the room's
  authored geometry, the body's tuning and the motion frame. It is not in a
  snapshot. It is never built a part at a time: an answer that depends on when
  the graph was asked for is different after a rewind.
- **The brain's state is rewound.** `RoamState` (goal, leg, phase, clocks) is in
  the `Brain` component, which is stored by clone, and in its checksum cursor.

These two rules are held by
`companion_dog::a_dog_that_navigates_resimulates_to_the_same_world_with_its_graph_dropped`
(2026-10-09): as much as 40 seconds of the basement under a sync test that rewinds and
replays each frame, while a system outside the timeline drops the graphs. The
dog goes by legs and the session stays healthy. Its control is a nudge of the
dog's body from outside the timeline, which is a mismatch.

Guards: `navigation::envelope::tests::a_gap_inside_the_envelope_is_crossed_and_one_outside_is_not`
(the envelope against the kernel) and
`navigation::graph::tests::a_body_that_follows_the_advice_arrives` (a body that
follows the advice arrives at each reachable surface, and a surface out of
reach is said to be unreachable).

An air jump is one leg, the double hop (2026-10-09): the follower presses
jump again on the first step the body falls, and the envelope measures that
arc in the kernel. The graph proposes a double hop only where no hop or drop
arrives, and keeps it only when its rollout arrives, so a body with no air
jump has none. Guard:
`navigation::graph::tests::a_body_with_an_air_jump_reaches_what_one_jump_does_not`
(a ledge between the two apexes: a double hop that arrives in the kernel; a
low step stays a hop; with no air jump the ledge is out of reach). Poison:
no second press, red at the double-hop link.

A wall verb is one leg, the wall climb (2026-10-10): a hop to the face under
a higher surface (with the air jump on the first falling step, when the body
has one), then hold into the face and up. The kernel does the rest: a cling,
a climb at its climb speed, the ledge grab, the pull-up. The follower judges
no early miss on it, because a climbing body is below its landing the whole
way up. It is proposed for a body that can cling and climb, only where no
hop, drop or double hop arrives, up to 600 px. `LegInput.up` carries the
hold-up (local -y) to the kernel, in the rollout and in the brain. Guard:
`navigation::graph::tests::a_body_that_climbs_gets_up_a_wall_no_jump_clears`
(a pillar three jumps tall: a wall-climb link that arrives in the kernel;
control, with no climb the top is out of reach). Poison: no hold-up, red at
the wall-climb link.

A wall jump is one leg (2026-10-12), for a body that can wall jump: a hop to
the face beside a higher surface, then a kick off each wall the body touches
below the landing, going on the way the kick sends it to the next wall, and
once the feet are over the landing's height, a steer onto it. So it goes up a
shaft of two walls. `LegFacts.wall` carries the wall the body touches (its
face's side), from the rollout's body, from a brain's `side_contact_normal`
and from the walker's `BodyWallState`. It is proposed after the climb and
before flight, only where nothing cheaper arrives, up to 600 px. No early
miss: the body is below its landing the whole way up. Guard:
`navigation::graph::tests::a_body_that_wall_jumps_gets_up_a_shaft_no_jump_clears`
(two walls three jumps tall: a wall-jump link from the shaft's floor to a
wall's top that arrives in the kernel; control, with no wall jump the top is
out of reach). Poison: no kick, red at the wall-jump link.

A dash is one leg, the dash hop (2026-10-12), for a body with a dash: jump
at the take-off, and at the top of the arc press the dash (the burst press)
straight toward the landing, so the body goes along at the height of the apex
for the dash's time, then steer to the landing. It is proposed after the
double hop and before the climb, only where no hop, drop or double hop
arrives, no higher than a jump, and up to `MAX_DASH_GAIN` (400 px) further
than the running jump; the rollout decides. `LegInput.dash_pressed` carries
the press to the kernel in the rollout, to the brain (`burst_pressed`) and to
the walkers (the burst key). Measured: for the test walker (270 px/s, a dash
at 760 px/s for 8 steps) a dash at the apex goes about 100 px past where a
level jump lands. Guard:
`navigation::graph::tests::a_body_that_dashes_gets_across_a_gap_no_jump_clears`
(a gap 50 px past the level jump, over a deep pit: a dash-hop link that
arrives in the kernel; control, with no dash the far ledge is out of reach).
Poison: no dash press, red at the dash-hop link.

Flight is one leg, the fly leg (2026-10-10), for a body that can toggle
flight: toggle at the start, rise to 32 px above the landing
(`FLIGHT_CLEARANCE`), go along to it, toggle off at half that height and fall
onto it. It is proposed last, only where nothing else arrives, and only up.
A flight rises straight from its start, so besides the hop proposals it
starts beside each end of a block it rises past: an opening in a ceiling
starts there. A flight that stops in the air for 15 steps fails. Guards:
`a_body_that_flies_gets_up_to_what_no_jump_reaches` (control: no flight, out
of reach; poison: no fly proposal, red at the fly link) and
`a_flight_goes_up_through_a_grate_from_under_it` (control: no gap, out of
reach; poison: no block-end starts, red at the fly link).

A leg whose body the kernel resets fails its rollout (2026-10-10): a hazard
in its air, or a fall out of the world. The rollout reads the kernel's own
reset flag, so a leg is never kept that the body survives only in the probe.
Guard: `navigation::graph::tests::a_leg_through_a_hazard_is_no_leg` (control:
the same gap with no hazard is a hop). For a body that a zone takes to another
room, `NavGraph::build_avoiding` takes the room's exits: a leg whose body
enters one fails, because that body is in another room then. Only a body a
player drives crosses by an overlap (an edge exit, a walk zone), so a brain's
graph has no exits. Guard:
`navigation::graph::tests::a_leg_through_an_exit_is_no_leg_for_a_body_the_exit_takes`
(control: no exits, the hop is there). Poison: the exit test off, red at the
exit assertion.

The walked route (2026-10-10):
`a_walked_route_of_the_persistent_world::the_player_walks_from_the_hub_to_alice_and_bob`
(`app_it`) walks the player from the hub to Alice and Bob through seven
crossings, and the playthrough walks the same road by keys in the shipped App
(`common::walk`, both drivers). In each room it builds the graph for the player's own body over
the room as the body collides with it (the authored blocks and each standing
gate solid), with the other overlap exits
avoided, and it gives the player the stick and jump that
`follow_leg` says. A door takes Interact, and the crossing is the shipped room
transition. No step puts the body anywhere. Before the exits were avoided,
legs in `intro_escape_shaft` and `drain_alley` went through the back edge exit,
and the body went back a room. With the exits avoided, `drain_alley` had no
route: its main street was solid across the room, and the door to
`under_town_pipes` is on the floor of the pipes layer below. The room's spec
says the player goes down through a grate in the street. Three 16 px cells of
the street over the door are now open (collision and tiles), and the body
drops through them. In `alice_relay` the lock wall of Alice's private return
stands on the floor before the exit to Bob until the survey (112 px tall), and
the walk goes over it with a double hop. Poison: the graph over the authored
blocks only, red at `alice_relay` (the body walks into the lock and stops).

A drop through a one-way surface (2026-10-12): `NavLegKind::DropThrough`.
The graph proposes it from a one-way surface to a surface under it, in their
overlap; the body stands there, holds down and presses jump until the surface
lets it go (`LegInput::down`, mapped by `leg_stick_y` for each driver), and
steers to the landing. The kernel rollout decides. Witness
`navigation::graph::tests::a_body_drops_through_a_one_way_floor_to_the_floor_under_it`:
a one-way loft between two walls, with no end to walk off. Control: the same
room with a solid loft has no road down. Poisons (no proposal; a stick that
ignores down): red at the link assertion.

Not modelled, each a seam:

- a second air jump, an air jump in a drop,
  a dash on the ground or up, a wall jump off one wall onto its own top,
  permanent flight, a pogo bounce. The pogo is a combat effect (a down-air
  hitbox on a `PogoOrb`, `pogo_moveset_off_world_orbs`), and a leg's rollout
  runs the movement kernel only, so a pogo leg needs the attack's hitbox in
  the rollout first (measured 2026-10-12);
- a slope or a surface chain, a surface that moves;
- geometry that is not authored in the room (a gate, a breakable). A leg such a
  thing stops fails, and the brain plans again;
- a route to another room (the door graph below is a different graph);
- a gravity frame that is not axis-aligned;
- `WhyNot`: an unreachable goal is `NavNext::Unreachable` with no reason.

## A player's seat that navigation drives (2026-10-16)

Customer: unattended profiling of a game in the hub (Jon, 2026-10-16: an
`AMBITION_AUTOPLAY=<game>` option for every game; "Let's add what we need to
navigation. No hacks, only elegant - efficient ... and correct design").

| Part | Where |
| --- | --- |
| One follower | `ae::navigation::RouteFollower`: goal, leg, phase, clock, misses; `needs_route()` says when it reads a route answer. A brain's `NavFollower` is it, with `DriveFollower` (advice in, `ActorControlFrame` out). |
| One cache | `world::navigation::cache`: `NavGraphKey` (geometry and exits stamp, tuning, abilities, size, base size, frame), `NavGraphCache`. The advisor's `RoomNavigation` wraps it, and keeps each room's stamp across ticks by its geometry revision (`GeometryStamps`). |
| The autopilot | `ambition_platformer2d::autopilot`: an input source outside the timeline. Graphs built on the async compute pool; the graph's inputs, compared whole (the room geometry's change tick, the still parts of its overlay, the exits), gate a recompose and a new key (`the_wanted_graph_follows_each_part_of_the_room_it_is_built_over`); a leg under way survives a geometry change; another room or form drops the graph. |
| Approach by law | `ae::navigation::ApproachLaw`, from the envelope's measured slide (`stop_distance`, `start_speed`): run, let go where the slide ends, creep (`CREEP_STICK`) inside the band a dash from rest overruns. |
| Recovery | `NavGraph::landing_in_reach`: a body in the air with no leg comes down on the nearest surface in reach, spending an air jump if it has one. |

Measured on standalone Mary-O (cosmic form): 1-1 with no death, 1-2 with
one; the test course to its pole on frame 220 (`mary_o_it autoplay`, which
also asserts that the autopilot had a graph and was going). Found on the
way: a stick of 0.077 is under the kernel's run threshold (0.1) and does not
move a body; the cosmic form at 620 px/s ran off a hop's ledge when the
approach was a fraction of top speed.

OPEN: a surface that moves (1-3 stops on a moving platform: a body on one
stands on no surface of the graph and waits). Design, not built: boarding,
riding and alighting legs, timed against the platform's deterministic motion
and rolled out in the kernel with the platform at its predicted positions.
OPEN: graph build cost on the phone (8,205 rollouts for 1-2); baking graphs at
content build stays the direction.

## Flight that stays in the air (2026-10-17)

Customer: Ambition's profiling itinerary (Jon, 2026-10-17: walk into the hall
of characters and out, then "navigate to and fly through the dinosaur level";
asked, he chose real flight as a navigation capability that NPCs can use
later, over walking the trail).

| Part | Where |
| --- | --- |
| The air of a room | `world::navigation::flight::FlightMap`: a grid (`FLIGHT_CELL`, 8 px) of the places a flying body's centre can be, clear of each block (a hazard by a wider margin), of the exits and of the room's edge, each with its chamfer clearance. Built with the room's `NavGraph` (`graph.flight`), in the same task and cache, for a body with `fly`. |
| Its law | `ae::navigation::FlightLaw`, measured in the kernel in open air: the speed a full stick holds, and the braking with the stick let go. |
| A route | `FlightMap::route`: A* over the free cells, each step weighted by how near a wall it is (`PREFERRED_CLEARANCE` 96 px), so a flight keeps to the middle of the air, over what walks on the floor. Then cut to straight parts, each as clear as the cells it stands for. Reuses a `FlightSearch`'s buffers. |
| The follower | `ae::navigation::follow_flight`: the stick asks for a velocity along the route, at the speed from which the body can still slow, by its braking, for each turn and stop at the end. `LegInput::axis_down` carries the analog vertical stick. |
| The autopilot | `NavGoal::Fly(centre)`: take off (a toggle press at most each `TOGGLE_FRAMES`), plan when the goal or room is new or the flight has come no nearer for `FLIGHT_STALL_FRAMES`, fly. A `Stand` goal lands first, unless a fly leg of the surface graph is under way. |

Measured: the tests in `flight/tests.rs` fly the route in the kernel (over a
wall; across an open room, clear of the floor). The itinerary in
`game/ambition_content/src/autoplay.rs`, run through `capture_scene --route
ambition_gameplay`, looped three times in 9,000 frames: the six trail rooms
flown both ways, the tyrant hall crossed.

NOT MODELLED: a route is not rolled out in the kernel before it is flown (a
stuck flight plans again instead); the bodies in the air (the swallows); a
block that moves; water. OPEN, not flight: in the hub, a Drop leg off the
hall's floor catches on a wall face in the air and stalls about 2 s before
the follower plans again.

## The cross-room slices: a character fetches an item from another room (design, 2026-10-09)

The queue's NAVIGATION row asks for a character that sees an item in another
room, decides whether it can reach it, goes there with its real movement, and
does a typed action. What the tree has, measured 2026-10-09:

- A route over rooms: `RoomSet::route` (fewest rooms, by authored zones).
- Native room admissions accept a body roster, including bodies that no
  participant drives. The shared room application retains rooms with
  unselected claims and moves selected bodies and their custody together.
  A character driver can use this admission seam. The old second-seat return
  still uses direct transit; PARTY-TRAVEL-POLICIES will route that return through
  native room admission.
- A body left in a room that is not live is rebuilt there from its `Placed`
  whereabouts row (`a_character_left_elsewhere_stays_there`).
- A held item is taken only by a driven body, on an Attack press
  (`pickup_held_item_system`). `ActionRequest` has no take.
- Goals: absent ([agentic character runtime](agentic-character-runtime.md)).

The slices, each with its own witness:

1. **A goal given from outside, in one room.** A deterministic goal on a
   body (fetch the item with this `SimId`), rollback state, that a navigating
   brain honours: it goes to the item by the in-room graph and ends the goal
   as done or refused, with the reason (no route, the item is gone). The take
   is a typed request, not a press, so a brain that attacks near an item does
   not take it. Acceptance arm in one room: a reachable item is fetched, an
   unreachable one is refused, and a body without the jump it needs refuses
   the first.
   **Done 2026-10-09.** `Errand` (actor monolith, `features::ecs::errand`)
   names the item by `SimId`; the advisor writes what it sees of the item
   (`ErrandSight`: at a place, no route, gone) into the body's `NavAdvice`;
   the `Roam` brain goes to the place before it rests or roams;
   `settle_errands` ends the errand after the press pickup, by the one take
   of a ground item (`ambition_held_items::take_ground_item`, which the press
   now calls too). Witnesses in `a_dog_sent_for_an_item`: the basement dog
   fetches the hub's Blink (control: from the same start with no errand it
   never comes within 96 px of it); a Blink put on a surface out of reach is
   refused `NoRoute`; a Blink put where only a jump gets to is fetched, and
   the same dog under a keyed ceiling with no jump refuses it; and under a
   sync test the errand ends with the Blink held. Poisons: the brain ignores
   the errand, red at "the dog did not fetch the Blink"; the advisor never
   judges reach, red at both refusals; the errand not rewound, red with the
   errand done and the Blink back on the ground. Found: an errand given from
   outside the timeline is undone by the first rewind past it, so a giver
   must be a system in the simulation; and the advisor reads the verbs the
   body had on the tick before (the ability projection runs after it).
2. **A crossing into a live room.** A body whose goal is behind a zone goes to
   the zone (the first hop of the route) and, in it, moves into the live room
   of the destination by the second-seat road: custody closure stamped,
   `transit_body` to the zone's arrival. Only into a room that is live, so no
   room is opened or retired for a body no slot drives.
   **Done 2026-10-09.** The advisor sees an item in another live room as
   `ErrandSight::Door(place)`: a route of live rooms (`RoomSet::route`, each
   room on it live) starts at a zone of the body's room, and `place` is the
   feet point under that zone when the body can get to it (else `NoRoute`).
   The `Roam` brain goes there as it goes to an item. `cross_on_errands`
   moves a body that stands in the zone into the next live room by the
   second-seat road, and gives a body with durable whereabouts its `Placed`
   row in that room (`AuthoredOccurrences::admit_crossing`, a third way for a
   live occurrence to enter the ledger, beside custody and a mint). A body on
   an errand crosses a Door zone with no press: the press is how a player
   says "go through", and the errand has said it. Witnesses in
   `a_dog_sent_for_an_item`: Alice carries the Blink to `basement_npcs` while
   Bob holds the hub, and the dog, from the far end of the basement, goes
   through the door and takes it there (control: from the same start with no
   errand it never comes within 200 px of the door); after that Bob leaves,
   the hub retires, Alice walks back, and the hub built again has no second
   dog; and under a sync test with two players the crossing and the take
   resimulate to the same world. Poisons: no crossing, red at "the dog did
   not fetch the Blink from the other room"; `Roam` deaf to `Door`, red at
   the same line; no ledger row, red at "the bodies of the dog's identity, by
   room" (two dogs); a crossing made only on the first run, red at "the
   replayed world differs" (a checksum mismatch at frame 196). Found: from
   the dog's authored start the roam goes through the door by chance, so the
   arm needs a start with the door behind the dog.
3. **A crossing into a room that is not live.** The body leaves through the
   ledger: despawned, with a `Placed` row in the destination at the arrival.
   Only for a body whose whereabouts are durable.
   **Done 2026-10-10.** The advisor finds the item's room from its ledger
   row, the dog walks to the door, its row is written in `basement_npcs` and
   it is despawned. That room builds the dog beside the Blink, and the hub
   does not build it again (`a_dog_sent_for_an_item_in_a_room_nobody_holds_leaves_for_it`,
   control: no errand, the dog stays). It was blocked on 2026-10-09: under a
   sync test it desynced at frame 182, because bevy_ggrs spawns a despawned
   entity again from its snapshot only, and four derived components
   (`BodyEnvironmentContact`, `BodyMotionFacts`, `ResolvedMotionFrame`,
   `SurfaceUpright`) did not come back. Road (b) was taken: the body requires
   them (`#[require]` on `BodyKinematics`, and the gravity plugin registers
   the frame), so the insert that restores a body inserts them too. Measured
   first: in 7 rooms each `BodyKinematics` carrier had all four and none had
   some, so an ordinary spawn does not change. Witness
   `a_dog_that_leaves_for_a_room_nobody_holds_resimulates_to_the_same_world`;
   poison (no requirement): red at "the replayed world differs", the
   mismatch at frame 182 again.

Not in these slices: enemy navigation and a baked graph (Jon's open
decisions). The air jump, the climb, the dash and the wall jump have legs
(above).

**A door that needs Interact (re-measured 2026-10-12).** The basement's
`npc_door` is a `Door` zone (Interact), and the crossings above go through
it: the errand is the press. What was missing was the gate. The player's
road refused a gate portal that was not `On`, and the errand road did not
ask, so a dog could go through a portal that was shut for the player. Both
roads now ask `GatePortalRegistry::lets_through`. Witness
`a_dog_sent_for_an_item::a_dog_on_an_errand_waits_at_a_shut_gate_portal`: the
door made a portal with its switch off, the dog waits in its zone in the
hub; switched on, it goes through and takes the Blink. Poison (the errand
road does not ask): red at "the dog went through a gate portal that is
shut".

## Architecture direction

### Use real body capabilities

Reachability is conditional on the body and its current state. A useful query
must know the capabilities relevant to the question rather than answer for a
fictional universal platformer body.

### Share movement/collision truth

Where possible, navigation/recovery probes should call or lower into the same
pure movement/geometry kernels as runtime simulation. Approximation is allowed
for search cost, but it must be explicit and validated against the real kernel.

### Report physical facts; keep policy above them

Reusable mechanics should report facts such as:

- support exists/does not exist;
- route is blocked by a capability or geometry constraint;
- no legal position exists;
- a transition/portal/path is reachable under stated capabilities.

Game/brain policy decides what those facts mean for goals, risk, aggression or
quest behavior.

### Stable spatial identity and provenance

Persistent/open-world navigation eventually needs stable room/surface/portal
identity that survives unload/reconstitution. Do not use Bevy `Entity` as the
long-lived route identity.

Use the durable spatial model in
[`../../architecture/spatial-model.md`](../../architecture/spatial-model.md).

## Near-term customers

Promote focused work from one of these:

1. a fighter/actor decision trace proves a missing reusable physical query;
2. persistent/open-world actors need room-to-room route reasoning;
3. portal/gravity/moving-platform traversal exposes duplicated reachability
   logic;
4. authoring/inspection needs to explain why a route is unreachable. The
   answer exists: `body.can(verb)` and `body.fits(height)` are published
   conditions, `gated_by` is an authored condition line, and
   `GatedLockWallVerdicts::why_standing(wall)` returns `WhyNot` per standing
   wall of the live room. Nothing in production reads it, and
   `AgentObservation` carries body state only. The missing piece is a surface,
   not a planner. Which surface is an open question on
   [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md).
   Do not add an `AgentObservation` field without a consumer;
5. a second game needs the same capability-aware query.

Do not build a universal navmesh/path planner merely because these customers may
exist later.

## Acceptance for a promoted slice

A navigation slice should:

- state the body capabilities and world facts it consumes;
- use stable identity where results outlive one ECS instance;
- agree with representative real movement outcomes;
- explain failure in semantic terms useful to policy/authoring tools;
- remain headless and deterministic;
- delete a demonstrated duplicate/heuristic road when it replaces one.

## Do not do yet

- no genre-specific death/fall heuristic in reusable navigation;
- no universal navmesh before a customer requires it;
- no second collision/movement implementation for AI;
- no path identity based on raw ECS entity order/ids;
- no claim that the current fighter rollout failure is a navigation-kernel
  failure until the decision trace demonstrates that.

## Spatial reuse without a universal world context

Navigation consumes spatial/body-motion facts and proposes movement; accepted
body control and the existing motion kernel execute it. Do not move actor live
mutation into a path service because AI and player motion share geometry.

Navigation and obstacle queries read the subject's own live room
(`LiveRooms::of`, `CollisionWorld::room`), as every pairwise query does. A9's minimal profiles should not require navigation to
step an otherwise self-contained body. Preserve deterministic motion-policy and
shape assumptions in reachability tests; a new spatial index requires measured
cost rather than a decomposition target.
