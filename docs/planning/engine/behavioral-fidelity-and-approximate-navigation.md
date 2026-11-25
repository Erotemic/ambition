# Behavioral fidelity and approximate navigation

**State:** Proposed implementation direction, 2026-11-25. The maintainer
wants reusable abstractions that let full-fidelity and approximate behavior
work together. This document specifies a navigation-first implementation
and the boundaries for subsequent perception and deliberation work. It is
not a receipt for completed implementation or measured performance gains.

**Source basis:** `d49467d5b877` (the incremental source archive from
`7c0c36e2c6cd`), the subsequent actor/input naming changes, and the
maintainer's discussion of the Hall of Characters. The archive does not
include the newer ground-navigation work. Read the live implementation and
its current work receipts before making changes. The source observations
below identify starting points, not a requirement to restore old code.

**Execution:** Use [the queue](../queue.md) for work selection. This document
is an implementation design, not another global execution queue. The
maintainer can select it directly. Coordinate with the agent working on
ground navigation. Do not restart that work from the older snapshot.

## 1. Objective and maintainer direction

The engine must support a large population of characters, including on
Android. An actor that does not need a capability must not pay the full cost
of that capability. An actor that needs a cheap answer must not pay the
cost of a more accurate answer unless the task calls for it.

The Hall of Characters is an important scalability customer. Its population
will continue to increase. The final authored Hall will probably contain
mostly stationary characters. That does not make an all-walking population
an invalid benchmark: it separates ordinary locomotion costs from the cost
of intelligent movement and gives the engine a more demanding customer.

Approximation also has gameplay value. An NPC can estimate that a jump is
possible, try it, miss, and recover. An experienced NPC can prefer a large
landing area. A reckless NPC can attempt a marginal jump. A player should
not see every ordinary character execute the same precise edge-to-edge
trajectory. Navigation competence can differ even when two characters have
the same body and movement abilities.

The important goal is **composition of fidelity levels through a common
contract**, not a continuous numerical slider or identical behavior between
all implementations. A change in planning policy can change a future route.
It must not change the actor's existing physical state as a side effect.

The implementation must do more than optimize the full-fidelity algorithm.
Reduce the amount of work that the game requires, then reduce the cost of
that work. Preserve useful exact optimizations already in the engine.

## 2. Related contracts and scope

Read the following owners before editing their implementation:

- [Execution specialization and work selection](execution-specialization-and-work-selection.md): exact specialization, deterministic work admission, preparation and reference evaluation.
- [Platformer navigation and reachability](platformer-navigation-and-reachability.md): native movement experiments, geometry coverage, route costs, spatial identity and existing navigation customers.
- [Bounded perception and attention](bounded-perception-and-attention.md): evidence admission, target beliefs, retained memory, attention budgets and replay requirements.
- [Agentic character runtime](agentic-character-runtime.md): controller demand, observation and behavior boundaries.
- [One body, one path](../../concepts/one-body-one-path.md): one authoritative physical execution path for all actors.
- [Engine architecture](../../architecture/engine-architecture.md) defines the engine and game ownership boundaries.
- [Performance and iteration](performance-and-iteration.md): measurement and benchmark ownership.

This plan extends the above contracts. It does not supersede the native
collision, movement, replay or simulation-island contracts. Retain the exact
execution specialization work. Add an explicit distinction between an exact
implementation of an operation and a different, deliberately weaker
behavioral model.

The first implementation customer is navigation. Perception and deliberation
are subsequent customers. Do not block navigation on a generic framework
for all three. Do not introduce physical simulation level of detail as part
of this work.

### Exact optimization and behavioral approximation

| Kind | Required result | Example |
| --- | --- | --- |
| Exact specialization | Same observable result under the admitted contract | Retain a destination cost table instead of recomputing it. |
| Deliberate approximation | A result that satisfies a declared weaker behavioral contract | Estimate a traversal instead of running a full native movement rollout. |
| Work omission | No result because the consumer does not request it | An idle actor does not request a navigation graph. |
| Deferred work | No result yet, with a defined continuation | A bounded route search will continue on a subsequent simulation tick. |

An exact implementation must preserve the existing semantic and replay
contract. A deliberately approximate implementation need not produce the
same choice as the reference implementation. It must still obey its own
contract and produce deterministic results for the same admitted inputs.

Do not label a changed answer an exact optimization. Do not treat every
disagreement with a more expensive behavioral model as a defect.

A failure to find a route in a finite model is not a proof that no physical
route exists. A cached native rollout is not a proof of success in a changed
live world. A deferred query is not a negative answer.

## 3. Behavioral responsibilities and common boundaries

Keep these responsibilities distinct in the existing actor path:

```text
Perception      What information does the actor have?
Deliberation    What does the actor want to do?
Planning        What action or route might achieve that intention?
Execution       Which controls are necessary on this tick?
Simulation      What happens when those controls take effect?
```

The first three can use approximations. Execution can select a less precise
maneuver if that is a declared behavioral decision, but it still issues
ordinary actor controls. The last stage is authoritative physical truth.

The shared abstractions should be the questions, result types, and retained
state that cross these boundaries. Do not create an independent NPC body
simulation or give cheaper actors a separate collision model. An actor that
stands still remains an actor: it can be hit, displaced, addressed, or given
a new task through existing mechanisms.

Use existing Bevy systems and plain data. A small enum or function is
sufficient to select a planning method when there is no actual need for a
trait or provider hierarchy. Prefer explicit imports and readable
constructors. Do not create a universal `Fidelity` trait, service, scheduler,
registry, macro language, or new crate just to express this plan.

### Independent policy dimensions

Do not make one parameter control all the following properties:

| Property | Meaning | Example |
| --- | --- | --- |
| Physical capability | What the body can do | Jump, dash, climb, fly. |
| Activity | Whether the actor wants to move or act | Idle, local wander, pursue. |
| Competence | Accuracy of its estimates and execution choices | Prefer forgiving jumps or misjudge a gap. |
| Risk tolerance | Which uncertain outcomes it accepts | Avoid dangerous drops, attempt a marginal jump. |
| Planning effort | Which methods and how much work it uses | Local query, bounded approximate search, selected native rollouts. |
| Decision cadence | When the actor reconsiders an intention | Addressed event, periodic decision, every tick. |

Risk tolerance of zero does not in general mean that an actor cannot walk.
A cautious actor might walk safely along the floor, or spend substantial
work finding a safe route. For a stationary Hall actor, select an inactive
navigation demand or an activity policy that does not request locomotion.
The opinion that maps a cautious or sedentary character to that policy may
be convenient, but the engine must not infer an activity policy from risk
alone.

The engine does not need to expose six new authoring fields immediately.
Introduce only the policy distinctions needed by the first real customers.
Do not expose a numerical probability of success until there is a defined
model and a way to establish its meaning.

## 4. Existing implementation to reuse

The following source observations are from the supplied snapshot. The actor
crate had its old `_monolith` suffix there; use the renamed
`ambition_platformer2d_actor` in the live checkout. Verify all paths and
contracts after concurrent changes.

| Owner | Existing mechanism and opportunity |
| --- | --- |
| `ambition_platformer2d_world/src/navigation/surfaces.rs` | Standing spans, frame-relative support geometry and chain connectivity. Share these across methods. |
| `ambition_platformer2d_world/src/navigation/envelope.rs` | `TraversalEnvelope`, `ArcSample`, `EnvelopeProbe`: measure native movement in a prepared experiment. Reuse sampled arcs and motion facts to estimate candidate traversals. |
| `ambition_platformer2d_world/src/navigation/graph.rs` | `NavGraph`, `NavLink`, route search, reachability, candidate generation and native rollout. Preserve search and execution semantics where applicable. |
| `ambition_platformer2d_world/src/navigation/cache.rs` | Prepared motion and graph identities, retained values and geometry stamps. Extend the existing ownership and lifetime rules. |
| `ambition_platformer2d_core/src/navigation.rs` | `NavLeg`, `NavLegKind`, `LegFacts`, `follow_leg`, `RouteFollower`, `NavAdvice`, `NavNext`, `ErrandSight`. These are the execution and answer boundaries. |
| `ambition_characters/src/brain/mod.rs` | `Brain::navigation_request` and `NavigationRequest` express a brain's demand, but not yet the individual question it needs answered. |
| `ambition_platformer2d_actor/src/body/navigation.rs` | `advise_navigation` and `RoomNavigation` provide the read model. Localize work admission here and at the demand owner. |
| `ambition_characters::perception` and actor perception systems | `PerceptionRequirement::{None, TargetBelief, TacticalWorld}` and retained `WorldMemory`. Build on these for the later perception increment. |

### Source evidence and limits

In the supplied snapshot `advise_navigation` loops over bodies whose brain
returns a `NavigationRequest`. It first obtains a graph, then calculates
waypoints, a goal leg (when a goal exists), target placement and target
surface relations. It also answers pending errand or visit questions.
`Brain::navigation_request` returns a request for a roaming brain even when
its goal is absent. The graph lookup occurs before it is known whether the
current question needs a traversal graph. The query has no `Dormant`
filter in that snapshot.

`RoomNavigation::graph_of` builds a graph on first use and retains it. The
motion envelope is measured separately and reused. Do not claim that every
navigation advice tick runs all graph rollouts. Cold preparation, warm graph
access, route search, destination selection and per-body advice are distinct
costs.

`NavLeg` describes a concrete maneuver with `start`, `takeoff`, `land` and
`land_span`. `land_span` already accepts landing within a standing span.
`RouteFollower` retains a leg, phase and goal and has a `needs_route`
query. It can continue to execute a selected maneuver while the planner
does no new work. Inspect all callers of these types, including the seat
autopilot, directed actors, combat brains, errands and visits.

`NavGraph::route` now uses arrival state rather than a single best cost per
surface, according to the navigation owner. Do not undo that correction.
A region-based graph still needs the relationship between entry position,
next departure position and walking cost. Boolean connectivity alone is
not sufficient to claim an optimal route.

The graph and its envelopes have explicit coverage limits. The supplied
advisor uses authored-room geometry and does not model every live geometry
change. Some movement and gravity configurations are not represented.
Read the live coverage notes. Keep these limits visible; approximation is
not permission to conceal them or to silently substitute a different body.

## 5. Navigation target architecture

Use a common navigation request/result boundary. Within that boundary,
compose the necessary operations. A local ground request need not construct
a traversal graph. A difficult jump can use a native rollout without
requiring native rollouts for all other candidate jumps in the room.

```text
Retained intention or new navigation demand
                  |
         Select required question
                  |
       Shared support/geometry facts
                  |
       +----------+-----------+
       |                      |
   Local/ground         Traversal candidates
       |                      |
       |             Envelope-based estimates
       |                      |
       |               Route selection
       |                      |
       |             Optional native refinement
       |                      |
       +----------+-----------+
                  |
          Concrete executable leg
                  |
          Existing route follower
                  |
           Native actor controls
                  |
        Observed success or failure
```

Treat this as a decomposition of operations, not a requirement to implement
a separate planner object at every box. Reuse the existing types when they
can express the necessary contracts. An actor may use several methods in
one route or across successive requests.

### Methods and coverage

| Method | Work it admits | First customer |
| --- | --- | --- |
| No navigation | No navigation query or preparation. | Passive Hall character. |
| Local ground | Current support, nearby safe destination, direct walk. | Pedestrian on a floor. |
| Connected ground | Walk/slope connectivity and ground route search. | Ordinary room roaming. |
| Approximate traversal | Estimated jump/drop feasibility, then more abilities as needed. | Ordinary platforming NPC. |
| Hybrid | Approximate routes plus selected native validation. | Important companion or directed actor. |
| Simulation-backed | Existing graph construction with native movement rollouts. | Reference comparisons, autoplay, difficult tasks. |

These names are proposed labels, not required public API names. A method
can use a cheaper method for a sub-question. For example, a simulated
planner can answer a walk on the same support without preparing any jump
links.

No method is a proof of exhaustive physical reachability. Even the existing
native graph samples candidate movements, has a finite work limit, and
omits some capabilities and geometry. Use the name `simulation-backed` or
`refined` rather than `exact reachability` unless the stronger statement is
actually established.

## 6. First implementation: demand and the cheap ground path

Complete the ground-navigation work already in progress. Make it the first
part of the common architecture, not a second movement or route-following
system. Review the new code before deciding which details of this section
remain necessary.

### 6.1 Request the question that the consumer needs

Map each current read of `NavAdvice` to its reason for requesting it. Do
not merely replace `NavGraph` with a cheaper graph and continue constructing
all advice for every navigating brain on every tick.

Examples of distinct questions:

- Select a local or reachable roaming destination.
- Find the next leg toward a goal.
- Find a reachable place beside an existing target.
- Check whether an errand or visit has a viable approach.
- Validate a particular traversal proposal.

The final API may use separate request variants or a small set of explicit
request fields. Choose the form that makes the consumer's demand clear.
An actor can have more than one relevant question, but the system must not
answer unused ones by default. Do not make a new request registry.

| State | Necessary navigation work |
| --- | --- |
| No navigation intention | None. |
| Idle between roaming trips | None until the next destination decision is due. |
| Need a local destination | Support/ground query. |
| Executing a valid leg | Continue the existing follower. No routine route search. |
| Landed or completed the leg | Plan next leg if the goal requires one. |
| Goal or relevant context changed | Re-evaluate the affected question. |
| Failed or invalidated leg | Recover, replan, or report the failure to policy. |
| Required errand/visit with no current route | Answer that task's request, with appropriate coverage. |

Distinguish a new need from a retained answer. Keep the answer as long as
its inputs and the consumer's need permit. A moving target can invalidate a
retained approach at an authored or task-defined threshold. An actor's
current jump is not a reason to request a new jump plan every tick.

Do not omit per-tick execution facts needed by `follow_leg`, timers or
physical controls. A failed or canceled task must not retain an obsolete
route or issue its old controls. When a follower needs fresh information,
ensure that it can request it at the existing scheduling boundary.

A `Dormant` marker is not by itself sufficient evidence that all tasks can
be ignored. Audit admitted directed tasks, waking behavior and controller
requirements. Define the actual active demand; do not replace one broad
scan with a broad set of exceptions.

### 6.2 Ground before traversal preparation

Return a valid direct walk from shared support facts when the question can
be answered that way. Avoid acquiring an expensive graph first. Use the
native support and standing geometry contracts. Preserve slopes, body
clearance, one-way policy, gravity frames and geometry revisions within the
selected method's declared coverage.

The local path should require no jump-envelope measurement. The connected
ground path should require no room-specific jump rollouts. If cheap ground
connectivity is not enough for a request, either report the limitation or
admit a more capable method according to its policy.

Do not assume that two feet positions at the same height share connected
support. Do not treat a gap, wall or inaccessible slope as an ordinary
walk. The real movement kernel remains the final authority on actual
movement, but the ground model must have useful stated semantics.

### 6.3 Acceptance for this increment

- A passive population with no navigation demand constructs no navigation
  graphs and performs no per-actor traversal queries. Count the work, not
  just the number of nonempty answers.
- Local and ground-only navigation do not measure jump envelopes or run
  room-specific jump rollouts.
- A same-support walk can complete without building a traversal graph, even
  for a character whose policy permits native refinement.
- A valid retained leg does not cause routine waypoint selection, target
  approach calculation or route lookup when those answers are not consumed.
- Existing directed tasks, roaming, pursuit, and seat autopilot still have
  a way to request what they require. Run targeted behavior and replay checks.
- A live movement failure can cause a new request. Unsupported/deferred work
  is not a permanent errand or visit failure.
- Measure cold entry and warm steady state before and after the change.
  Report exactly which graph preparation and advisory work has been removed.

This is useful work even if approximate jumps are not ready. It should be
possible to stop after this increment with a coherent engine.

**First work admission slice, 2026-11-25.** Roam asks for destinations when
its rest ends. A retained leg gets live feet and can continue without another
route search. The advisor answers direct walks and approaches on one
standing span from the same `GroundMap` the native graph uses. One-support
roaming keeps its existing destination choice. The visit integration test
checks zero work for a passive actor, one prepared ground map for a direct
visit, and zero motion measurements and traversal graphs. The reference
graph test checks a walk and a gap. The existing
[Android measurements](../../../dev/benchmark-candidates/android-hall-contact-and-perception-work.md)
give the stationary and crossing baselines.

**Connected ground.** The standing map locates the supports at each end of
an authored slope. The planner checks only those slope walks with the
native follower. The hill-path test walks in both directions with zero
envelope measurements and zero jump rollouts. The seat keeps its measured
preparation path.

## 7. Approximate traversal proposals

### 7.1 Use native measurements as a compact model

`TraversalEnvelope` is a promising existing foundation. It measures the
native movement kernel under a controlled standing experiment and retains
sampled motion. Reuse this model before creating another set of movement
equations. Use it to propose traversals in new rooms without testing every
candidate with a complete native rollout.

The first approximate capabilities should be ordinary jumps and drops.
Support walk-off and one-way behavior where the current prepared model and
follower have the necessary information. Preserve unsupported cases as such.

Start with a conservative spatial candidate filter. For each candidate,
use the measured envelope and body footprint to estimate a set of feasible
launch and landing choices. Check geometry using a small number of swept
or sampled segments, preferably with existing native geometry queries.
Account for takeoff speed, acceleration, braking, gravity, body dimensions,
material effects, required actions, clearance, and relevant landing
conditions to the extent supported by the experiment.

A coarse sampled check can miss an obstruction. A conservative geometric
filter can reject a movement that a better controller would complete.
Both are valid limitations to measure in an approximate planner. Never
change actual collision or movement to make a proposal succeed.

Use spatial bounds and relevant capability families to avoid comparing all
surface pairs without a useful candidate filter. Keep work per candidate
bounded. Do not merely move the full rollout loop into an `Approximate`
function and measure an unchanged amount of work.

One-time native envelope measurement is allowed. Distinguish it in the
counters from room-specific validation. The unrefined approximate path
must not secretly run full native traversal rollouts for each edge.

### 7.2 Extend the shared vocabulary only as needed

**Initial jump and drop implementation.** The planner indexes the standing
spans, takes nearby supports in a stable order, and checks bounded sweeps
along the measured arc. Each launch sample has its own landing and travel
cost. The test executes an estimated jump and walk-off in the native
kernel. It also places a wall across the gap and checks that the planner
withholds the route. Preparation runs zero room-specific rollouts.
Running jumps and walk-offs on still blocks have coverage. Chains, moving
blocks, water, and exits along an estimated trajectory cause a refusal.

| Capability | Candidate model (subject to actual coverage) |
| --- | --- |
| Walk / slope | Existing support connectivity. |
| Jump / drop | Sampled native envelope and clearance. |
| Double jump / air dash | Joined or capability-specific measured arcs. |
| Wall jump / climb | Contact regions and capability-specific reach estimates. |
| Ledge grab | Reachable support edge and body clearance. |
| Flight | Bounded free-space connectivity, with native execution. |
| Blink | Supported displacement and destination constraints. |
| Morph / body mode | Geometry and capabilities of the relevant body form. |
| Portal / room transition | Existing directed spatial connections and transition policy. |

This is an extension direction, not a list of capabilities that must all
land in the first jump implementation. Do not make a false supported claim
for variable gravity fields, moving platforms or portal transformations.
Preserve the existing high-fidelity route for such cases where it actually
supports them. If neither method covers a case, return an honest result.

### 7.3 Regions, realizations and evidence

A navigation proposal is a suggested movement from one support/spatial
region to another, not a promise that one exact trajectory will succeed.
A concrete realization provides the parameters the existing follower needs.

Distinguish these concepts:

| Concept | Necessary meaning |
| --- | --- |
| Traversal proposal | Source and destination, movement kind, admissible launch/landing choices, estimated effort, evidence and coverage. |
| Concrete realization | Start, takeoff, landing target and acceptance region, with the action sequence required by the follower. |
| Evidence | Which method produced the estimate, and which prepared conditions it covers. |
| Outcome | What the real actor did when it executed a realization. |

The current `NavLeg` is a good candidate for the concrete realization. Do
not replace it and `follow_leg` just to support approximate candidates.
Extend `NavLink`, use a compact set of alternatives, or introduce an
upstream proposal type if necessary. Do not mandate a final type before
examining how search and existing followers consume it.

In particular, a takeoff interval and a landing interval are not generally
independent. For a given launch point and motion, only some landing points
are feasible. An implementation that stores two independent intervals and
allows every Cartesian product can invent many invalid proposals.

A practical first representation could retain a small set of sampled
realizations or a bounded relation between takeoff and landing intervals.
Choose based on measured cost, fidelity and the existing graph interfaces.
Keep route costs consistent with the selected realization. Account for the
walk to its departure position and the walk from its landing to the final
goal. Do not undo the arrival-state correction in the navigation owner.

A useful evidence classification might distinguish geometric, envelope-based
and native-rollout-backed proposals. An actual success in live movement is
additional evidence, not a permanent certificate of validity. No calibrated
probability field is necessary for the first implementation.

### 7.4 Remove surgical movement as a default objective

**Landing preference.** A roaming profile can ask for more space between
a landing and the edge. The preference adds a cost to the concrete leg.
It changes route selection and leaves the physical capability and the
follower unchanged. The route test selects a wider landing when the
profile asks for it.

Current trajectories can look excessively precise because a planner chooses
an exact edge-adjacent takeoff or landing that minimizes its estimated cost.
Do not optimize only for traversal duration when generating NPC routes.

Prefer an appropriate landing margin, comfortable takeoff region, tolerance
to imperfect input and a route that leaves room for recovery. A short route
with a very narrow successful launch window may be worse for an ordinary
NPC than a longer, robust route.

Keep physical constraints separate from character preference. A wide
landing area must not be rejected just because its center is not the
fastest reachable point. A capable actor may still deliberately choose a
narrow route for a sufficiently important goal.

Use the current `land_span` semantics as appropriate. Verify what it
actually accepts: a landing on another support at the same height must not
count as success. Record accepted support identity where the live spatial
contract provides it. Do not assume that arriving at one coordinate is the
only success condition.

### 7.5 Sharing across actors

Separate room geometry and candidate discovery from evaluation for a body.
Share immutable room facts and movement experiments when their semantic
inputs match. Avoid building the same surface relation for hundreds of
characters simply because the characters have different nonphysical
properties.

Some character differences do affect physical reachability. Keep exact
body/motion keys for native validation. An approximate model may use a
conservative or explicitly representative capability class; define how the
class bounds or approximates size, motion tuning and abilities. Do not
silently reuse a representative's validated success as proof for every
member of a heterogeneous class.

Start with measured reuse from actual bodies in the Hall and a platforming
room. Do not create a second global geometry cache or an arbitrary catalogue
of body-size buckets without evidence that the existing preparation owners
cannot provide the reuse.

## 8. Hybrid refinement

The hybrid path should not build the full simulation-backed graph and the
full approximate graph for every query. It should reuse inexpensive
proposals and refine only selected ones.

A first useful algorithm:

1. Discover and evaluate cheap candidates.
2. Select a provisional route or the next required traversal.
3. If the request's policy requires stronger evidence, select an uncertain
   realization or a short part of the route for native validation.
4. Run the existing native rollout on the selected experiment.
5. Accept the realization, adjust it, reject it, or continue searching
   according to the result and the deterministic work allowance.
6. Execute the admitted concrete leg through the existing follower.
7. Observe the outcome. Recover, continue, or replan.

The first hybrid customer can validate the next important link rather than
perform full route validation. If a later link is invalid, the actor can
choose another route from its new state. For a task that requires more
assurance, validate more of the route under the stated conditions.

A rollout only establishes success or failure of the particular tested
experiment. It does not establish the entire proposed region. A rejected
realization need not reject every other launch point or every movement kind
between the same two supports.

Refinement can alter route choice and cost. Update or invalidate the
relevant search result before using its old evidence. Keep the live
follower and preparation identities consistent with the admitted result.

### Retained refinement

Reuse validation when the complete experiment and geometry inputs are
equivalent. The existing navigation owner defines key requirements for
material scale, standing state, motion frame and probe. Preserve those
requirements. Include any new realization parameter, coverage or model
revision that the validation reads.

Avoid an expensive key reconstruction on each idle body tick. Retain
prepared inputs and their revision according to the existing owner.
Do not create another cache merely because the plan distinguishes evidence
levels. A stored negative result has the same scope limits as a stored
positive result.

Cache warming, eviction, background preparation and the order of thread
completion must not change the authoritative choice or the time at which an
answer becomes available in simulation ticks. See section 11.

### Hybrid acceptance

**Selected validation.** The hybrid query validates the first link of
the cheapest route. It searches again with the measured cost or a rejected
link. The cache keeps both results by the complete graph key. The test
offers an infeasible jump before a useful realization. Cold and warm
queries choose the same alternative and admit the same number of
experiments. The warm query runs zero rollouts.

- The approximate mode executes no room-specific validation rollouts.
- A hybrid request can use a mixture of ground, estimated and refined legs
  through the same follower.
- The refinement work is bounded independently of the number of all
  hypothetical graph links.
- A native-refined realization uses the real motion experiment and does
  not become an unconditional guarantee of live success.
- A selected failure causes deterministic continuation, a different
  realization, or a truthful incomplete result.
- The existing simulation-backed method remains available for comparison
  and customers that need its coverage. Do not duplicate it.

## 9. Failure and fallible NPC behavior

**Result contract.** The same route and task answers distinguish an absent
request, an unsupported request, an incomplete request and absence of a
route in the model. Visits and fetches record exhausted execution retries
as a separate failure. A canceled task clears its retained leg. A visit
outside the standing model stays pending in the headless integration test.

An approximate planner is only useful if its consumers can tolerate its
predictions being wrong. Specify the result contract before changing
errand and visit semantics or promoting approximate navigation to a default.

The existing `NavNext` and `ErrandSight` are relevant boundaries. They may
need a small extension. The following table specifies meanings, not a
mandatory enum that combines all request and execution states.

| Result | Meaning | Typical response |
| --- | --- | --- |
| No request | No answer was asked for. | Do no navigation work. |
| Not applicable now | For example, airborne while a ground query requires support. | Finish the physical maneuver or wait. |
| Proposal available | A supported method suggests a traversal. | Choose and execute it if policy accepts. |
| No route in model | Search completed for the admitted representation and found none. | Consider another goal or a more capable method. |
| Unsupported / insufficient coverage | The model does not represent a required operation. | Escalate if the task permits; otherwise decline. |
| Deferred / budget exhausted | The search is incomplete. | Retain or retry the request under an explicit policy. |
| Arrived | The actor actually reached its intended destination. | Complete the task. |
| Execution failed | The actual maneuver did not achieve its required outcome. | Recover, reconsider, retry or give up. |

Do not make every actor escalate on failure. That could make approximate
navigation as expensive as simulated navigation in a busy room. Escalation
is a choice of task policy. A low-cost NPC can accept a missed opportunity.
A required story movement can request additional work, report an explicit
failure to the story system, or follow its authored recovery policy.

Let mistakes result from bounded knowledge, estimated reachability and
imperfect realizations. Do not find a native-proven jump and then insert a
random failure purely to simulate incompetence. A deterministic choice
among admissible alternative movements can provide variation. Any deliberate
imprecision must be a behavioral parameter or policy, not a change to
physical motion rules.

A failed attempt should not cause infinite repeated planning or a new
native rollout on every tick. Use the existing follower's failures and
retry state. Add limited failure memory only if a concrete customer needs
it. Scope remembered failure to the relevant proposal, body capabilities,
geometry revision and time or task context. A failure caused by a temporary
obstruction is not permanent proof of impossibility.

A character can learn from actual outcomes. Do not make a shared cache of
one NPC's observations become knowledge that every NPC has. Distinguish
shared physical preparation from individual belief.

## 10. Policy selection and switching

**Production policy.** The grazing preset uses connected ground. The
townsfolk roaming preset uses estimated traversal. The default roaming
profile and required visits and errands use simulation-backed navigation
for questions beyond a direct walk. Hybrid is selectable for roaming.
The follower keeps the current leg when its planner setting changes.

The engine owns the methods, requests, result semantics, and execution
boundaries. Shared opinions select useful configurations. Each game owns
its authored actor/room defaults, exceptions, and task priorities. The
engine must not name the Hall or particular story characters to choose a
planner.

Illustrative profiles, not an approved public schema:

| Customer | Observation | Navigation | Deliberation |
| --- | --- | --- | --- |
| Hall display actor | None or addressed events | None | Idle or event-driven. |
| Local pedestrian | Task-relevant local facts | Local ground | Periodic. |
| Ordinary wandering enemy | Retained target belief | Approximate | Periodic, with relevant interruption. |
| Important companion | Task-relevant rich facts | Hybrid | As needed. |
| Required scripted task | Explicit target and task facts | Hybrid or simulation-backed | Task-dependent. |
| Autoplay / reference customer | Existing requested facts | Simulation-backed | Existing required cadence. |

A game can choose a different policy. Do not require these profiles to be
a new serialized struct spanning unrelated systems. Keep navigation policy
at its owner and perception policy at its owner. An opinions package can
compose them with normal data and functions once multiple customers exist.

A task can temporarily request more or less planning effort than the
actor's default. When the task ends, restore the appropriate existing
policy. Do not reinitialize the actor or reset its body state merely because
the method changes. Apply a new planner to the next admitted question.

Preserve across policy changes:

- Stable actor identity, body position, velocity, support and physical mode.
- Current goal and task identity, unless game policy explicitly cancels it.
- Relevant retained observations and task history, subject to normal expiry.
- The active physical maneuver and its follower state, unless it is
  explicitly invalidated or safely interrupted under existing rules.

Allow new answers, different route preferences and different observation
or deliberation cadence. It is acceptable for behavior to have a
discontinuity. It is not acceptable to teleport, reset velocity, manufacture
success or lose a pending story task as an incidental effect of a switch.

For initial implementation use deterministic authored and task-specific
selection. Do not introduce a dynamic quality manager. In particular, do
not select a weaker planner merely because a device is slow, a frame is
late, or the cache is cold. A host can select an explicit session policy
before admission if the game permits it; that becomes a shared simulation
configuration, not an unannounced per-peer preference.

## 11. Determinism, rollback and work limits

A behavior approximation is part of simulation semantics if it affects
controls, events, beliefs or future decisions. Different peers must use
the same admitted configuration and deterministic selection inputs. Couch
and network sessions use the same gameplay rules modulo transport.

Use stable actor/spatial identities, deterministic request order, stable
candidate ordering and tie-breaking, and simulation ticks for cadence and
expiry. Derive any variation from existing deterministic randomness with
appropriate scope and state. Do not use wall-clock time or unordered
iteration to choose a route or an actor's decision tick.

Start with small method-specific work limits. Candidate evaluations, graph
expansions, rollout steps, and refinement attempts are possible units. Use
the units that correspond to the actual algorithm. A local query needs no
global budget service. Add a room-level work allocation policy only if
measurements show the need, and specify fair/stable admission and
continuation rules first.

Do not confuse a logical work allowance with CPU time. If a warm cache can
answer a query physically faster, it must not make that query logically
eligible for an answer on a different tick from its cold counterpart. A
cache hit and miss must agree under the selected method. An asynchronously
prepared result must not become visible because a particular computer
happened to finish it first.

Classify new state before implementation:

| State | Expected treatment |
| --- | --- |
| Immutable geometry/envelope preparation and derived search index | Reconstructible or retained under its current owner; not automatically rollback state. |
| Current intention, admitted policy, active leg and retry history | Canonical state if it influences future controls and cannot be derived. |
| Retained perception and individual learning | Canonical history when it affects behavior. |
| Pending planning request and partial progress | Canonical or deterministically reconstructible continuation, with explicit lifetime. |
| Counters, timings and debugging views | Non-authoritative diagnostics; never select behavior. |

Do not add every preparation to a snapshot. Do not omit state that changes
a future answer. Rewind and resimulation must reproduce the chosen behavior,
not merely the eventual destination in a convenient test case.

Scope requests and preparations to the proper live room/simulation island
and existing authority. Preserve navigation task identity across allowed
transitions. Do not make a frozen island accrue deliberation, retry or
planning time through a separate wall-clock loop.

## 12. Subsequent customer: bounded perception

**Room demand.** OBS3 and the native brain declare which live rooms need
peer rows or projectile rows. A passive room with no reader prepares no
rows. Resolved hits can still supply evidence from the striker. Roam gets
company and destinations from navigation advice. During a committed
pursuit leg the brain can use its retained belief. This pause in sight
does not refresh or decay that belief. A module that asks for a fresh
observation still gets one.

Do not wait for a complete new perception architecture before implementing
navigation. The existing `PerceptionRequirement` and bounded-attention
contract provide useful independent work. Continue those plans rather than
adding a second observation API.

Extend the demand boundary from the type of brain to the actual question or
need of the current behavior. A passive actor does not need an environmental
scan. A body executing a route can read self facts and movement outcomes
without building a tactical view. An addressed interaction or resolved hit
can provide relevant information directly through existing event mechanisms.

Keep the distinction between acquiring a target and retaining a belief.
Do not continuously search for a better target if the current task can use
a retained one. Establish when the target becomes invalid, when the actor
should reconsider it, and when more evidence is required.

Distinguish absence of evidence from evidence of absence. An unobserved
target is not automatically dead, gone or visible. A lower cadence must not
accidentally refresh stale memory or decay memory as if a new full scan had
occurred. Preserve the existing time and evidence admission semantics or
change them explicitly as a behavioral choice with replay coverage. In
particular, preserve the receipt that hit-induced memory updates happen
before rollback snapshots.

A bounded output is not a bound on work. In the reviewed snapshot a rich
view can collect and sort many candidates before truncation. The retained
attention and cheap-belief paths have different semantics. Measure offered,
examined, admitted, retained and reported information. Bound candidate
acquisition or amortize shared spatial summaries where the selected model
permits it. Do not merely decrease the returned row count while keeping an
exhaustive per-observer search.

A fighter can use exact facts about its immediate opponent, a small watchlist
of task-relevant actors, and an approximate summary of the crowd. Preserve
the existing bounded-perception owner's exact/watchlist/aggregate direction.
An ordinary Hall actor should not pay for that representation.

## 13. Subsequent customer: retained intentions and deliberation cadence

**Local cadence.** Roam rest and look intervals advance by the body's
simulation step. Grazing checks for company every 0.25 seconds. Route
followers advance each active tick. A zero step neither spends the leg
timer nor emits a movement edge. The roam test pauses before takeoff and
observes the jump press on the next active step.

An actor should be able to decide to walk somewhere at a low frequency
while its controls and physical motion continue every simulation tick.

Audit the brain/controller seam before choosing a design. Locate the
stateful decisions, one-tick action edges, continuous movement signals,
route-following state and timers. The current `Brain` implementation may
mix some of these responsibilities; do not simply run it every N ticks and
hold its complete last output.

Retain a goal or intention that has a clear lifetime and interruption
policy. Execution interprets the retained intention into appropriate
controls on each required tick. It must not repeatedly emit a jump press,
attack edge or toggle because the deliberator has not run again. It must
also be able to complete or abort a physical maneuver while no new decision
is due.

Use task-relevant interruption where it is necessary: damage, invalidated
goal, directed command, changed authority, traversal outcome, or other
admitted event. Respect current controller ownership and control holds.
Do not require every event to wake every actor.

Use deterministic simulation-tick scheduling, stable staggering when
needed, and explicit elapsed time for stateful decisions. Timing differences
are an intended behavioral change, so validate them as such. Do not reduce
the physics tick rate or make a slow brain lose events as a side effect.

Start with one passive/local-roaming customer and one active pursuit or
directed customer. Preserve higher-frequency fighter, scripted or autoplay
behavior until the new cadence contract demonstrably supports it. Do not
rewrite all brains in one increment.

## 14. Measurement plan

Performance matters at increasing populations and on the intended platforms.
A change that saves time on one snapshot of the Hall may not solve its
scaling problem. A change that reduces planning quality may be worthwhile
if the measured cost and gameplay are appropriate for that policy.

### Baseline and test scenes

Use the existing measurement owner, scripts, fixtures and measurement
submodule. Identify the current ground-navigation baseline before editing.
Use a native platforming fixture or room with meaningful jumps and obstacles
as well as the Hall. The Hall alone may contain too few difficult traversals
to evaluate approximate jumping.

Suggested population sweep: 100, 250, 500, 1000 and 2000. Use supported
sizes up to available memory and CPU limits. Report where a run cannot
complete. Do not spend the first implementation increment running every
combination on every platform.

| Population behavior | Question |
| --- | --- |
| Passive, no navigation/perception | What is the unavoidable baseline cost per resident actor? |
| Walking, no route search | What is the cost of actual active physical locomotion? |
| Local/ground wandering | How much work does destination selection and ground navigation add? |
| Approximate platforming | What do candidate discovery, search and failed maneuvers cost? |
| Hybrid platforming | How many native refinements does a real workload need? |
| Fighting/tactical | How does perception, targeting, deliberation and contact density scale? |
| Mixed | What cost and behavior do the intended authored profiles produce? |

Do not compare a passive new population against an active old population
and call the entire difference an exact performance speedup. Report both
same-workload implementation comparisons and different-behavior policy
comparisons. Separate the number of active actors from the number of
resident actors, and record the geometry, movement mix and device/build
configuration.

Measure cold preparation and warm steady state separately. Hold simulation
rate, room, input trace, seed, relevant build flags and policy fixed for
comparisons. Record raw measurement locations, source revision, hardware,
run length, and repeat/variation information. Use the current project's
measurement practices, not a new general benchmark harness.

### Required counters and timings

| Area | Measurements |
| --- | --- |
| Demand | Actors resident, active requests by kind/method, retained legs, dormant/passive actors, answered/deferred/unsupported requests. |
| Preparation | Shared support preparations, envelope measurements, graph builds, cache reuse, geometry invalidation, preparation elapsed time. |
| Search | Candidate surface pairs examined, proposals admitted, expansions, destination queries, route queries, selected realizations. |
| Refinement | Native rollouts and kernel steps, successful/failed/deferred refinements, reused validations. |
| Execution | Traversal attempts, successes, failures, retry count, recovery time, completed tasks. |
| Perception (later) | Candidate acquisition cost, examined/offered/admitted/retained rows, target changes, report delivery. |
| Deliberation (later) | Decision invocations, intention lifetime, interruptions and executed controls. |
| Runtime | Simulation phase time, rendered frame time separately, p50/p95 or other existing summary statistics, memory and cold entry latency. |

Include work counters so a benchmark can prove that an entire computation
was omitted. A lower wall time alone does not establish that no native
rollouts occurred or that inactive actors paid no navigation cost.

Estimate scaling in milliseconds per additional actor over the measured
population range. Separate fixed room preparation and warm per-actor cost.
Report deviations from a linear fit; dense contact or geometry variation
can change the curve. Include population-size-dependent preparation,
publication and memory costs when they become significant. On Android,
record cold entry, sustained frame/tick time and thermal conditions where
hardware is available. Do not claim Android gains from desktop timing.

### Behavioral comparison

For the same admitted geometry and body experiment, compare approximate
proposals with native-rollout results for selected realizations:

- Accepted by both.
- Accepted by the approximate model but failed by the tested native
  realization.
- Rejected or not discovered by approximation but found by native search.
- Unsupported or incomplete on either side.

Treat the native method as a bounded comparison, not an exhaustive truth
oracle. Compare actual execution too: the time and support margin of a
successful landing, repeated failures, completion rate, path choice and
recovery behavior. Inspect rendered NPC movement. A perfectly accurate
estimate with consistently surgical jumps can still be bad gameplay.

Keep deterministic correctness distinct from acceptable approximation
quality. Do not assert a universal success percentage before measuring
representative customers. Set policy-specific targets with evidence:
passive/local behavior has different requirements from a required story
task or autoplay. Publish failed examples and the chosen tradeoff.

## 15. Implementation increments and acceptance

These are local dependencies and deliverables, not a new execution ledger.
Use the queue and the maintainer's selected work. Each completed increment
should be usable before the next. Adjust the order if the latest ground
implementation makes a different boundary cheaper, but preserve the
objectives and contracts.

| Increment | Deliverable | Acceptance |
| --- | --- | --- |
| A. Reconcile and measure | Read current ground work, current owners, source/measurement baseline. Identify actual consumers and required answers. | Record current active and cold costs and what is already implemented. Do not redo existing work. |
| B. Work admission | Actual navigation demand, retained leg execution, cheap local/ground path before costly preparation. | No navigation demand does no navigation work; ground-only work performs no jump rollouts; existing real tasks remain functional. |
| C. Result contract | Explicit incomplete/unsupported/failed distinctions and consumer handling. | A cheap planner cannot prematurely declare a required task impossible. A failed jump has a supported continuation. |
| D. Approximate traversal | Shared support facts and envelope-based ordinary jump/drop candidates. | No room-specific native rollouts in this mode. Native movement still executes the proposed leg. Compare representative feasible, obstructed, and missed traversals. |
| E. Regions and competence | Practical proposal-to-realization relation; landing margins and route preferences. | Ordinary NPCs do not require the same edge-perfect trajectory. Physical constraints and actual outcomes remain authoritative. |
| F. Hybrid refinement | Selective native validation and retained evidence using existing preparation. | A route uses cheap and refined legs together. No duplicate complete graph required. Work scales with selected refinements, not all hypothetical links. |
| G. Integration and policy | At least two real actor/task policies, deterministic transitions, profile ownership. | Switch planning method without resetting body, goal or active maneuver. Establish passive and platforming customer performance. |
| H. Perception | Use demand, retained belief and bounded evidence acquisition through existing observation owners. | Passive actors incur no environmental scan; measure input work as well as output size; preserve replay semantics. |
| I. Deliberation | Retained intentions and periodic/event-driven decisions with existing continuous execution. | Actors continue walking/jumping at full simulation rate. No repeated edges, dropped required events or cache-dependent scheduling. |
| J. Scale and promote | Population benchmarks and gameplay review; select defaults by actual customer. | Document quality, cost, remaining bottlenecks and rollout policy. No claim of universal Android suitability without evidence. |

C can be developed with B or D where existing result types already support
part of the distinction. A first approximate realization can be a concrete
leg; do not block D on a fully general interval representation. E must
remain an explicit deliverable, not an unbounded research program. H and I
are subsequent work after navigation demonstrates the common-contract
approach; do not wait for a universal behavioral profile framework.

### High-value verification

Use existing fixtures and tests where possible. Do not add a large number
of tests that duplicate implementation details. Prefer a small set of
witnesses that span the contract:

- No-demand and ground-only counters demonstrate omitted work.
- A retained follower executes multiple ticks without new route queries.
- Approximate open jump, blocked jump, missed jump with recovery, and a
  ground-plus-jump route execute through the native controls.
- Region choices accept more than one useful realization without claiming
  that every pair of interval endpoints is feasible.
- A hybrid route refines a selected uncertain traversal and handles a
  rejected realization without constructing every native link.
- Unsupported/deferred results do not cause a false terminal errand or
  visit result; a required task can explicitly request refinement.
- Switch a policy during an active maneuver and compare physical state and
  replay, including cold/warm preparations and stable work ordering.
- Observe a passive actor waking for an addressed interaction or damage
  without continuous perception or navigation when that work is absent.
- Exercise per-tick execution under reduced deliberation cadence when that
  later increment is implemented, including action edges and interruptions.

Use an existing replay/integration fixture when checking rollback. An
isolated pure-function test cannot establish that a new authoritative
scheduler behaves identically after a real rewind. Use the project's
cheapest-sufficient-check policy, and build/check the assembled application
at appropriate boundaries. Follow `AGENTS.md` and its build/submodule rules.

## 16. Prohibitions and out-of-scope work

- No new parallel physics, NPC body execution, or navigation-specific
  collision truth. The planner proposes; native simulation executes.
- No universal fidelity manager, second ECS scheduling layer, or new
  plugin hierarchy for hypothetical future customers.
- No full-graph construction for a local walking query or passive actor.
- No mandatory eager warm-up of all expensive representations for all
  characters in the Hall.
- No hidden fallback from a cheap planner to a full graph on each ordinary
  failed attempt. Escalation is explicit and bounded by task policy.
- No behavior changes caused by elapsed CPU time, worker completion order,
  platform speed or cache warmth.
- No claims that approximate search failure proves physical impossibility,
  or that a successful bounded rollout proves general reachability.
- No arbitrary forced misses to make a native-perfect route look human.
- No per-character parallel physical truth or deletion of meaningful actor
  interactions to achieve a favorable benchmark.
- No special-case Hall logic in reusable engine code. Select the desired
  actor population policy in game data or shared opinions.
- No mandatory all-ability approximate coverage in the first increment.
  Preserve explicit limitations and add capabilities for actual customers.
- No requirement to solve physics sleeping, collision optimization,
  presentation scalability, or every Android bottleneck in this navigation
  task. Measure their costs and hand findings to their existing owners.
- No new compatibility path that survives indefinitely after its replacement
  is established. Remove obsolete demand and interpretation code.

## 17. Agent handoff and completion report

The immediate assignment is to continue the ground-navigation work under
this broader direction and implement the navigation increments in order of
useful dependency. Do not stop at another plan. Begin with the current live
state and concrete measurements. The desired endpoint is an interchangeable,
demand-driven navigation contract with inexpensive approximate traversals
and selective native refinement, not three unrelated planners.

First establish how the ground work already fits the shared contract.
Then remove unnecessary demand, introduce the approximate jump/drop path,
and make it usable by existing followers. Use the Hall for scaling and a
real platforming customer for quality. Work toward hybrid refinement and
robust, less surgical movement choices. If an increment reveals an actual
problem in the proposed abstraction, revise the plan and implementation at
the relevant owner. Do not preserve a bad interface just because it appears
in this document.

For each completed increment record a concise receipt in the owner plan:
what changed, relevant commit, observed behavior, work counters/timing,
guard, and remaining limitation. Reconcile the existing navigation,
execution-specialization and perception documents with the new semantics
when their implementation changes. Link this plan from the current planning
index without overwriting unrelated concurrent edits. Keep historical
investigation in Git and the measurement journal.

Report at handoff:

- What the prior agent's ground work already solved and what was missing.
- The actual request, proposal/evidence, realization and execution boundaries.
- Which calculations no longer occur and for which actors/questions.
- Cold/warm costs, scaling, refinement counts and behavior comparisons.
- Which approximation errors remain, their recovery behavior, and why the
  selected policies accept or reject them.
- What was verified by source inspection, execution, replay, and rendered
  observation. State any unavailable hardware or unexecuted tests.
- Decisions made autonomously, unresolved contracts, and the next bounded
  work item.

Work in the assigned checkout. Do not create or switch worktrees. Commit
only owned changes with correct attribution. Follow the live queue and the
repository's coordination/build policies.

The final criterion is that an actor can receive a useful answer at the
cost appropriate to its current question, use the same execution path as
any other actor, and fail or recover according to its own behavior policy.
A human should be able to find and understand where that policy is selected.
The engine should not require a full simulation-backed prediction merely
because the body is capable of a movement.
