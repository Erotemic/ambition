# Open-world runtime and residency

**State:** planned expansion beyond the current single-active-room model.
The maintainer's persistent, systemic platformer and separated multiplayer actors
are the customer. A8's two-instance proof no longer waits for another demo.
[The queue](../queue.md) selects execution. This page owns residency/activity
semantics; construction, custody, spatial facts and rollback keep their owners.

## Goal

Support a persistent world larger than its current live simulation. An actor,
item, encounter or quest must not cease to exist because a camera moved. Human,
AI, possessed and uncontrolled actors use the same body/domain rules. A one-room
profile and a multi-room profile use one implementation at different populations.

Keep these axes separate:

```text
persistent existence != prepared data != live ECS population
                     != simulation activity != local view visibility
```

The target is not a new generic world engine beside Bevy. Existing room records,
typed construction, occurrence/custody facts and load coordination are the basis.
Rooms are the first residency unit because source and constructors already use
them. Sub-room chunks or region hierarchies need a measured consumer; do not build
a universal partition tree before the first two-room proof.

## Source baseline, not completion claims

Inspection baseline: `d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266`.
The following source was inspected; runtime behavior was not rerun in this review.

| Locator | Current fact | Limit |
| --- | --- | --- |
| `crates/ambition_platformer2d_world/src/rooms/room_graph.rs`, `RoomSet` | One `active: usize` selects a room | Multiple definitions in the graph do not mean multiple live instances |
| Runtime `room_transition/prefetch.rs`, `PrefetchIdentity` and `RoomConstructionPlanPrefetch` | Prepared plans are keyed by epoch/session/source room | A cached plan is data, not a simulated room |
| Shared `lifecycle/markers.rs` and `lifecycle/continuity.rs` | Residency/custody and occurrence continuity have existing homes | Do not replace them with an extension-owned world mirror |
| Actor monolith `features/ecs/dormancy.rs` | Existing dormancy concerns actor work within a live room | It is not an off-room world simulator |
| Runtime `session_world.rs` and `room_transition/commit.rs` | Prepared source and live session state are distinct | Multi-instance publication and rollback membership need additional work |
| Runtime `external_effects.rs` | Speculative outputs are replaced and released under confirmation | Durable delivery/retry is not guaranteed by the in-process journal alone |

Historical checks at `baebe16f3` and `ba111c995` distinguished asset residency from
room residency and exercised authored occurrence continuity. They do not establish
current multi-room support, a room budget or background simulation. Reuse existing
canonical reconstitution/checkpoint tests as controls; add the new instance case.

## Ownership table

| Responsibility | Owner | Required output |
| --- | --- | --- |
| Authored room/region definition | World provider and preparation | Immutable definition and stable references |
| Live spatial instance | Spatial/world authority | Scoped geometry, membership and semantic entity resolution |
| Need for a room to be active | Gameplay/lifecycle policy using declared interests | Deterministic required set and reasons |
| Read/prepare/prefetch work | Existing source/load coordinator | Candidate data with identity and readiness |
| Construction and publication | Existing lifecycle plus typed domain constructors | One admitted live population and timeline boundary |
| Actor/item/quest durable facts | Existing corresponding domain owners | Pinned revision or accepted state handoff |
| Visual residency and quality | Asset/presentation owners | Device readiness independent of simulation truth |
| Rollback population/history | Existing registrar and GGRS host | The admitted active state under one generation |

Do not centralize all these facts in a new `WorldContext` resource. A coordinator
combines owner results; it does not become a second item ledger, physics engine,
checkpoint router or query system.

## Identity and coordinates

A room definition can be instantiated more than once. A live query must identify
the instance whose geometry and occurrences it interprets. A construction attempt
and a ContentEpoch are not durable occurrence identities. A view ID is never the
identity of the simulated room it happens to show.

Start A8 with two instances of one prepared definition and identical local
placement IDs. Trace the current source paths that lose the instance distinction.
Qualify only those domain references/keys, through the owning spatial/lifecycle
contract. Do not append a universal WorldId to every unrelated engine value.

Where an occurrence must survive save/load, the world owner supplies its stable
instance/location identity. A temporary reload attempt cannot mint a second
persistent occurrence namespace. Placement, runtime spawn, custody and definition
IDs retain their distinct meaning. The runtime-to-durable mapping is explicit.

Spatial requests name units, coordinate frame and instance. A cross-instance
operation requires an installed transform/transfer service. Copying coordinates
or querying the globally active room is not that service. A portal may map spaces;
a camera looking through it may not transfer simulation ownership on its own.

## Activity policy and deterministic inputs

The simulation-required set follows gameplay: controlled bodies, unresolved
contacts/projectiles, active encounters and declared cross-room mechanisms. It
cannot follow only local cameras, rendering quality, memory pressure or which IO
request completed first. Optional prefetch and visual detail can remain best-effort.

The initial background policy is explicit suspension for domains that permit it.
A mechanism requiring time evolution declares a supported rule instead: a scheduled
logical event, deterministic elapsed-time reconstruction, or full active simulation.
Do not silently invent approximate combat or resource production when a room leaves
view. A paused domain does not claim continuous simulation.

Event-driven/elapsed-time processing uses an admitted logical clock, input facts
and wake conditions. Persist/rewind its counters and next-event state at the owning
lifetime. A domain with nonlocal physical interactions must activate the required
geometry/population or supply an explicit valid coarse model. Do not ship a second
inconsistent physics solver for dormant rooms.

The precise amount of background life is game policy. The ownership, clock and
handoff contracts are DO. Scheduling algorithms and CPU/memory limits are MEASURE.
Numeric platform budgets remain Q94; lack of numbers does not justify missing
accounting or camera-owned simulation.

## Promotion and demotion

Use the existing prepare/admit/commit road. At a supported confirmed boundary:

1. Pin content generation, source instance, target instance and the required
   durable/occurrence revisions. A stale plan cannot resolve from whichever
   mutable ledger happens to be current at commit time.
2. Prepare the destination population through owning constructors. Resolve
   retained bodies, custody and relationship endpoints without duplicating them.
3. Obtain lifecycle authorization and validate all prerequisites. Partial asset
   loading may affect reveal under its declared policy, never collision truth.
4. Transfer write authority once. Live state becomes the simulation owner; pinned
   dormant facts are not left as a parallel writable truth. Demotion writes the
   settled disposition through its domain's accepted policy.
5. Establish the active rollback baseline before simulation resumes. Retire only
   the departed instance/attempt, not every room with the same definition ID.

Failure before accepted publication retains previous ownership. IO completion
publishes candidate readiness, not speculative state. Required records influencing
future ticks join the rollback model or a pinned deterministic input revision;
placing them outside snapshots is not an exemption.

The current room transition rebases the timeline. The first multi-instance design
retains that explicit policy at session scope: changing active membership must
preserve unaffected instances while producing a coherent whole-session baseline.
Do not clear another actor's world state merely to reset history. Independent
per-room rollback clocks are not introduced by this plan. A later transport design
must coordinate the same membership/generation decision across peers.

## Bounded work and memory

Partition immutable definitions, dormant product records, active mutable state,
derived indexes and presentation assets. An active tick traverses the required
active set, not every saved occurrence in the world. Maintain indexes at the owner
that can update them correctly; derived indexes are rebuilt on restore.

Measure counts and bytes by class: prepared definitions, candidate materialization,
active records, retained snapshots, dormant records, device assets and pending
work. A stored budget number with no admission/eviction consumer is not a policy.
Reject or pause explicitly when required simulation cannot be admitted; do not
despawn mechanical actors, shorten rollback history arbitrarily or reduce physics
accuracy in response to local device pressure.

Residency requests have owner-scoped reasons and release paths. A departed actor,
canceled transition or removed view must release its own claims even if its
producer no longer ticks. Other owners' claims remain. Expose why each room is
prepared/live/active and which owner prevents retirement.

Do not require one particular map/chunk/COW implementation before measurement.
Do require FI9 to show that adding dormant records does not add an all-world walk
to an unrelated active step. M2 measures actual retention, promotion, restore and
resimulation costs, including sparse/dense changes and candidate peak memory.

## Implementation cuts

These cuts refine A8 and existing owner work. They are not another global queue.

| Cut | Work | Required evidence |
| --- | --- | --- |
| OW1 | Two instances of one room; audit selection/identity/query/teardown paths | Same local IDs, separate contacts/observations, no cross-despawn; one-instance profile remains one path |
| OW2 | Accepted body/custody transfer and prepare/publish between instances | Refused transfer retains state; successful transfer preserves identity and exactly one writer |
| OW3 | Dormant durable records and active-state handoff | Save/load and promotion preserve occurrences; active step excludes unrelated dormant records |
| OW4 | Owner-scoped interest/budget accounting and diagnostics | Cancellation/re-entry release only the right claims; supported absence does not freeze unrelated work |
| OW5 | One concrete background mechanism requiring logical time | Deterministic events/reconstruction under replay and room return; no camera/device dependence |

OW1 is the first scope proof, not completion of streaming. OW2 reuses A1/A10 and
existing construction; it does not add another lifecycle state machine. OW3 can
share I5's state handoff fixture. OW4 chooses budgets from measurements. OW5 follows
a real mechanic, not a speculative universal offscreen simulator.

## Existing repairs and standing lessons

| Historical receipt | Preserve |
| --- | --- |
| `ef3e864de`, room identity/caption correction | One authority defines aliases; consumers do not invent their own OR chains |
| `3c9d5d149`, prop identity correction | A caption/art key is not semantic occurrence identity |
| `f0274d38f`, cut-rope trigger witness | Exercise the trigger's real consumer; do not retain a stale deferral after closure |
| `d472f516b`, duplicate retractor correction | One owner retracts the fact; two fallback retractors conceal each other's defects |
| `a1f8a75`, prefetch identity repair | The producer publishes identity with the plan; the consumer does not fabricate it later |

Old module/reference counts and long investigation narratives remain in Git
history. Recompute a source graph only for a selected ownership change. The current
`LoadCoordinator` centralizes successful plan-change reporting; do not reopen the
older seven-call-site story without a current counterexample.

## Choices still requiring evidence or product policy

The storage/index layout, residency budgets, preparation concurrency and eventual
chunk granularity require measurements. Which offscreen mechanics should advance,
and which saves remain compatible across product versions, require game policy.
Two-instance identity, one writer during handoff, generation-aware preparation,
and no camera-owned simulation are settled architecture. Do not send those back
to the maintainer as open design questions.
