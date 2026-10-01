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

Inspection baseline: `d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266` (2026-09-11).
The following source was inspected; runtime behavior was not rerun in that
review.

⚠ **FRESHNESS PASS 2026-09-20, AND IT SAYS EXACTLY WHAT IT CHECKED.** The
baseline is **1666 commits** behind `main` — an ancestor, verified with
`git merge-base --is-ancestor` rather than by the commit merely existing. All
eight cited paths still resolve. Two claims were re-read in full and hold:
`RoomSet` is still `{ rooms, active, start, .. }`, so one index still selects
one live room; and `PrefetchIdentity` is still
exactly `(content_epoch, session_scope, source_room_id)`, so a prepared plan
is still keyed by the world it was prepared for. **The other four rows'
claims were NOT re-read** and are carried forward on the baseline's authority,
which is what a row dated nine days and 1666 commits ago is worth. A reader
about to depend on one should re-read it; a reader about to CHANGE one should
update this note with what they found.

⭐ **AND ONE OF THEM WAS CHANGED THE SAME DAY, WHICH IS WHAT THAT SENTENCE IS
FOR.** `active` is no longer a public field. Five callers outside its crate
assigned it directly — two production (hot reload and definition
normalization) and three fixtures — and its setter CLAMPED an out-of-range index rather
than refusing — a session told to build room 7 of a set of two woke in room 1
wearing room 7's geometry, measured by accident 2026-09-14 and guarded on
exactly one road. It is now private behind `RoomSet::active()` /
`set_active()` / `set_active_by_id()`, both setters return `None` and write
nothing when the room does not exist, and both refusals are poison-verified.
⚠ **THIS DOES NOT ADVANCE OW1 BY ITSELF.** One index still selects one live
room; what changed is that the index can no longer be a value nobody checked.
An index that can be silently wrong cannot be promoted to an instance
identity, so this is OW1's precondition and not its first cut.
✅ Superseded by OW1 cut 5e (2026-09-29): the live selection is
`LiveRoomDefinition` on each live room root, and `RoomSet` keeps only the
prepared `activation`, behind `activation()` / `set_activation()`.

⛔⛤ **AND THE FIRST ATTEMPT AT THAT INVARIANT WAS HALF OF IT — CAUGHT BY
REVIEW THE SAME DAY.** Privatising the indices fixed their MUTATION roads and
left the CONSTRUCTOR building states they forbid: `from_parts` selected room 0
for a start id it did not hold (so a caller asking for room X ran a different
one) and built `active = start = 0` over an EMPTY `rooms`, an index naming
nothing that `active_spec()` and the room-set rollback checksum both
dereference. The commit had even argued the fallback was necessary because
`from_parts` has 61 callers — which is migration cost, not a contract.
⇒ `RoomSet::try_from_parts` returns `Result<_, RoomSetRefused>` and the
twelve production callers take it; `from_parts_or_panic` is the fifty
fixtures' road and enforces the SAME invariant, so no `RoomSet` anywhere holds
an index that names no room. `set_start_by_id` goes through `set_active`
rather than assigning the field beside it — one field, one mutation law.

| Locator | Current fact | Limit |
| --- | --- | --- |
| `crates/ambition_platformer2d_world/src/rooms/room_graph.rs`, `RoomSet` | One private `active` index selects a room, through one checked road, and no road builds an index that names no room (2026-09-20) | Multiple definitions in the graph do not mean multiple live instances |
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

✅ **OW4, first slice, landed 2026-10-01: each live room names the owners
that hold it live.** A live room is held by each driven body stamped into
it, with the slot that drives it. That is the rule a crossing reads to
choose whether the room it leaves stays whole or is retired
(`another_player_stays`), and it is now one function,
`rooms::residency::claims_on`. `live_room_claims` derives from it the
answer for every live room, and `[census] rooms` prints it
(` holders=[#0:slot1 #1:slot0]`; `-` is a room nothing holds). A claim is
not stored: it is read from the driven bodies and their stamps, so a
departed body's claim is gone with it even when its producer no longer
ticks, and a refused crossing, which moves no body, releases nothing.
Witnesses: `a_departing_player_releases_only_their_own_claim` (Bob, slot 1,
holds `switch_lab`, #0; Alice, slot 0, holds the hub, #1; Alice walks back:
Bob's claim on #0 stands at each sample, the hub retires with Alice's claim,
and both hold #0; the census prints ` holders=[#0:slot0,slot1]`) and the
refused arm of `a_crossing_into_a_room_another_player_holds_joins_it` (a
stale join: slot 1 still holds #0 and slot 0 still holds #1). Poisons: with
the claim rule counting a driver in any room, the first arm of the
departure witness failed (both slots held both rooms), and
`a_player_who_comes_back_joins_the_room_the_other_player_holds` failed (the
hub stayed live): the crossing reads the same rule. With the stale-join
check removed, the refused arm failed at its refusal assertion, before its
claims assertion. ⚠ The door walk hides its ticks, so "at each sample" is
before the crossing and on each of the 30 ticks after it. ⚠ Only driven
bodies are owners in this slice: views and pending transitions hold no
claim yet, and no budget is stored, because a budget with no consumer is
not a policy.

Do not require one particular map/chunk/COW implementation before measurement.
Do require FI9 to show that adding dormant records does not add an all-world walk
to an unrelated active step. M2 measures actual retention, promotion, restore and
resimulation costs, including sparse/dense changes and candidate peak memory.

## Implementation cuts

These cuts refine A8 and existing owner work. They are not another global queue.

| Cut | Work | Required evidence |
| --- | --- | --- |
| OW1 | Two instances of one room; audit selection/identity/query/teardown paths | Same local IDs, separate contacts/observations, no cross-despawn; one-instance profile remains one path. ⭐ **A LIVE ROOM HAS AN IDENTITY AS OF 2026-09-20**: `LiveRoomInstance` (`crates/ambition_platformer2d_world/src/rooms/instance.rs`), an ordinal of this session's room publications, minted by `apply_world_replacement` — the one road that seats a session in a published room — and rollback state (`root.live_room_instance`, schema v202). Witnessed on the shipped Mary-O lap: 1-1 → 1-2 → 1-3 → 1-1 returns to index 0 and reaches instance `#3`, so the room she comes back to is not the room she left. ⚠ It lives on the SESSION ROOT because that is where the one live room lives; two simultaneous instances move the carrier, not the ordinal. ⚠ And residency is still UNKEYED — `RoomScopedEntity` says an occurrence dies with *a* room, never with *which* — so the teardown sweep is the next thing OW1 has to key. ⭐ **OW1 HAS AN INSTRUMENT AS OF 2026-09-20**: `[census] rooms` prints every session root's `active` INDEX beside its authored id, plus the live crossing, so the moment an index stops identifying one live instance is visible rather than inferred. It is derived and read-only; it owns nothing. |
| OW2 | Accepted body/custody transfer and prepare/publish between instances | Refused transfer retains state; successful transfer preserves identity and exactly one writer. ✅ **The accepted arm between live rooms is witnessed (2026-09-30)**: the crossing's publication re-stamps the crossing body and its custody closure (`InCustodyOf`: what it holds, rides or wears) into the room it enters, for an opened room and a join alike (`publish_pending_world_replacement`). `an_item_carried_out_of_a_room_another_player_holds_crosses_whole`: Bob holds `blink_run` (#0); Alice carries its authored item to `portal_bridge` (#1): one occurrence of its `SimId`, held, stamped #1, #0 still live; thrown down, it lies in #1; when Alice joins #0 again, it retires with #1 and #0 has no copy. Poison (only the body moves): the item stayed stamped #0, fell into #0's world, and outlived #1 as a stray in Bob's room. ✅ The refused arm with two live rooms (2026-09-30): `a_crossing_into_a_room_another_player_holds_joins_it` stages a join into a live room that is not there; it is refused as `StaleJoinedRoom`, both live rooms and their bodies stand, and nothing is minted (poison, the stale-join check removed: it published, and #0 was retired with both its bodies). The one-room refusal is `a_room_staged_for_a_stale_live_room_is_refused`. ⚠ Witnessed at the publication, not through a shipped crossing: the app has no road that makes a crossing stale while two rooms are live. |
| OW3 | Dormant durable records and active-state handoff | Save/load and promotion preserve occurrences; active step excludes unrelated dormant records. ✅ **First slice (2026-10-01): a runtime mint left in a room that is not live is a dormant record**, kept by the save's minted rows while the occurrence ledger places it, and a mint enters the ledger when it is minted, not when it is first carried; see "OW3, first slice" and "second slice" below. ✅ **FI9 (2026-10-01): dormant records add no all-world walk to an idle tick**: the custody projection reads a custody index, and the two save mirrors walk the dormant rows only when an input changed; see "OW3 / FI9" below. ⚠ Not yet: the rollback frame's costs (a snapshot clones the whole save and ledger, and the peer checksum folds every ledger row; M2 measures them), and the actor dispositions a retired room loses (an enemy's HP, a fight in progress). |
| OW4 | Owner-scoped interest/budget accounting and diagnostics | Cancellation/re-entry release only the right claims; supported absence does not freeze unrelated work |
| OW5 | One concrete background mechanism requiring logical time | Deterministic events/reconstruction under replay and room return; no camera/device dependence |

OW1 is the first scope proof, not completion of streaming. OW2 reuses A1/A10 and
existing construction; it does not add another lifecycle state machine. OW3 can
share I5's state handoff fixture. OW4 chooses budgets from measurements. OW5 follows
a real mechanic, not a speculative universal offscreen simulator.

### OW1 carrier decision, 2026-09-29: one entity per live room instance

**Decided** under [autonomous decision-making](../../concepts/autonomous-decision-making.md);
the customers are the persistent world and Alice/Bob separated multiplayer.

A second live instance of a room needs a home for the values that are
singular today: `RoomGeometry` and `LiveRoomInstance` on the session root, and
two process-wide resources, `MovingPlatformSet` (`world/src/collision.rs`) and
`FeatureEcsWorldOverlay`, that `apply_world_replacement` writes for "the" room.
`CollisionWorld` reads all three and its queries take no instance.

| | (A) a `RoomInstanceRoot` entity per live instance | (B) one coordinate space, rooms at disjoint offsets |
| --- | --- | --- |
| instance in a query | a named value on the reader (`InRoomInstance`) | implicit, a distance |
| authored coordinates | unchanged, room-local | every spawn, edge, camera bound, path and zone translated |
| identity and despawn | still to key (cuts 2, 5) | the same collisions, unsolved |
| pairwise proximity | keyed explicitly (cut 4) | separates for free |

⇒ **(A).** (B) hides the instance in a distance, which this page's "Spatial
requests name units, coordinate frame and instance" forbids, and solves none of
the identity or teardown collisions. The session root keeps the `RoomSet`
definition graph and the ordinal counter; each instance root carries its
geometry, platforms, overlay, selected definition and `LiveRoomInstance`.
Spatial entities carry `InRoomInstance(LiveRoomInstance)`, a value and not an
`Entity`, so it snapshots without entity mapping. **The one-room profile is the
same implementation:** a session always has exactly one instance root, and every
geometry read resolves through the reader's instance, with no `Single` fast path
and no fallback to "the live room".

| Cut | Work | Witness (control) | Deletes |
| --- | --- | --- | --- |
| 1 ✅ | Stamp room-scoped entities with the instance their plan was built for (`SessionSpawnScope` carries it, as it carries visibility) | staged occupants carry the pinned instance after publication (none before) | "which room" inferred from the moment of the sweep |
| 2 ✅ | Mid-room spawns inherit the source's instance; the transition roster is `RoomResident` of the departing instance | a resident of `#7` survives a publication that retires `#0` (the same entity stamped `#0` retires) | the unkeyed whole-world roster |
| 3 (3a ✅ 3b ✅ 3c ✅ 3d ✅) | Geometry, platforms and overlay move onto the instance root; `CollisionWorld` takes the instance | a body in `#1` collides with `#1`'s wall (in `#0` it passes) | the session-root geometry field, `MovingPlatformSet` as a resource |
| 4 (4a ✅ 4b ✅) | Pairwise queries (contacts, hits, perception, projectile victims) keyed by instance | identical local positions in two instances never touch (one instance does) | unkeyed body-contact vectors |
| 5 (5a ✅ 5b ✅ 5c ✅ 5d ✅ 5e ✅) | Live identity and room selection per instance; rollback rows instance-qualified; the save maps to a durable location key, never the ordinal | a second construction of one room in `#1` succeeds (a duplicate inside `#1` is still refused) | `RoomSet`'s private active index |
| 6 (6a ✅ 6b ✅ 6c ✅ 6d ✅ 6e ✅) | The Alice/Bob proof: two instances, two driven bodies; retiring `#1` leaves `#0` whole | the one-room profile runs the same systems | the sole-room reads on the crossing road |
| 7 (7a ✅ 7b ✅ 7c ✅ 7d ✅ 7e ✅ 7f ✅ 7h ✅; 7c and 7e reviewed ✅) | The simulation systems a second live room freezes read their subject's own room (`LiveRoomOf<T>`) | the system runs in both rooms, each entity against its own room (one room: the same answer) | one `SoleLiveRoom*` parameter per system |

✅ **Cut 1 landed 2026-09-29.** `LiveRoomInstance` moved down to
`ambition_platformer2d_shared_tangle::lifecycle` beside the new
`InRoomInstance` (rollback row `scope.room_instance`, schema 264).
`SessionSpawnScope::in_room` carries the instance, and `apply_to` stamps it
on every entity spawned under the scope. The stamp is in `apply_to` and not
in the room helpers, because most occupants get `RoomScopedEntity` inside a
bundle passed to `insert_session_scoped`. The activation plan carries
`#0`. `replace_live_world` takes `publishes_as`, the root's instance
advanced once, and stamps the staged room with it. The verifier refuses a
stale pin (`StaleRoomInstance`) before anything is torn down. Witnesses:
`a_staged_room_belongs_to_the_live_room_its_publication_mints` (the
control stages the same plan unpinned and stamps nothing),
`a_room_staged_for_a_stale_live_room_is_refused`, and
`every_room_occupant_belongs_to_the_live_room_it_was_built_for`
(`switch_lab` → hub → back reads #0, #1, #2, with no stray).

✅ **Cut 2 landed 2026-09-29.** Measured first: over every shipped room
(72), after load and 240 ticks of running and attacking, the only unstamped
simulated residents were death drops and a projectile. The home body now
starts in #0 (the activation scope carries it). A publication moves what the
sweep left standing in the replaced instance to the minted one, within its
own session, so the crossing body and what it holds follow without a road
remembering to. Drops, blasts and split offspring take the dead body's room
(`FeatureHitWriters::spawn_scope_from`), a projectile its owner's, and a
minted throw its thrower's. Both replacement roads build their roster with
`InRoomInstance::leaves_with`: a resident of another live room stays. ⚠
**An unstamped resident still leaves with any departing room**, which is
exact for one live room. The roads that still spawn one are named on that
function (portal shots, match and world items, presentation `RoomVisual`s),
and `every_room_resident_carries_its_live_room_after_combat` holds the
simulated population at zero over 72 rooms (66 stamped residents; the sample
is running and attacking, headless). Witnesses:
`a_resident_of_another_live_room_stays_when_this_one_is_replaced` (a #0
control is swept, a #7 subject stays in #7) and the crossing body's room in
`every_room_occupant_belongs_to_the_live_room_it_was_built_for`.

✅ **Cut 3a landed 2026-09-29: the live room is its own root.** A session
now has a `RoomInstanceRoot` entity beside its `SessionRoot`, carrying
`RoomGeometry` and `LiveRoomInstance` (rollback rows `root.room_instance`
and `root:room_instance`, schema 265; the two components keep their
`root.geometry` and `root.live_room_instance` rows). The session root keeps
the `RoomSet`. `PreparedPlatformerSource::instantiate_live_room` builds it;
the direct and the candidate session spawn it in their own scope, so a
hidden candidate's room is hidden with it. A publication names the live room
it REPLACES (`PendingWorldReplacement::replacing`), finds that root in its
own session (`live_room_root_for`), advances its instance and writes its
geometry. A publication that names no room, or a room its session no longer
has, is refused before anything is torn down (`StaleRoomInstance`). The
crossing names the room its subject stands in (`InRoomInstance`), and the
sole live room only when nobody crosses. Deleted: the `geometry` and
`live_room` fields of `PlatformerSessionWorld`, and "advance whatever
instance the session root holds".

⚠ **THE READERS ARE NOT KEYED YET, AND THAT IS THIS CUT'S NAMED DEBT.** 77
system parameters in 61 production files read `SoleLiveRoom<T>` (79 after 3b, 80 after 3c)
(`Single<Ref<T>, With<RoomInstanceRoot>>`), the direct successor of their
`SessionWorldRef<RoomGeometry>`: with two live rooms a `Single` matches
nothing, and the system skips. That is the decision's "`Single` fast path",
kept on purpose for one cut, because each reader must be keyed by the
instance of what it reads (a body, a camera, a view), and that is a separate
change per reader. `check_alias_census_agrees_with_source.py` holds the
count to the `alias-split` marker in `consolidation-plan.md`, so it cannot
move unseen; it does not force it down. Cut 3b moves
`MovingPlatformSet` onto the root, 3c the overlay, and `CollisionWorld` then
takes the instance. Witness:
`a_publication_writes_only_the_live_room_it_replaces` (a session with live
rooms #0 and #7: replacing #0 gives that root #1 and the candidate's
geometry, and #7 keeps both of its own).

✅ **Cut 3b landed 2026-09-29: the moving platforms are the live room's.**
`MovingPlatformSet` is a component on the room root (rollback row
`root.moving_platform_set`, schema 266), not a process resource.
`advance_moving_platforms` advances every live room's platforms. Setup writes
the first room's set onto its own session's room root, hidden with a
candidate, so the candidate no longer carries the set out as data for its
adoption to install. A publication inserts the published room's set on the
root it writes the geometry to. `CollisionWorld` reads the geometry and the
platforms in ONE query off one root. Deleted: the resource and its
`init_resource`, the session teardown's clear (a room root leaves with its
session), `SimulationWorld::moving_platforms`,
`PreparedCandidateSession::moving_platforms` and its adoption write, and
`StagedWorldViolation::NoPlatformStateToPublishInto` with its preflight and
its two tests (a room root that has geometry always takes the platforms).
Witness: `every_live_rooms_platforms_advance` (live rooms #0 and #7 each
hold a platform; both advance one tick's travel). Smash's respawn platform
writes through `SoleLiveRoomMut`, the write twin of the named debt; a room
that authored a `respawn_platform_` id would still collide with it.

✅ **Cut 3c landed 2026-09-29: the collision overlay is the live room's.**
`FeatureEcsWorldOverlay` is a component on the room root, not a process
resource (its rollback row stays `derived`: it is rebuilt every tick). The
engine's rebuild clears every live room's overlay and sends each breakable
and pogo contribution to the overlay of its own entity's room
(`RoomOverlays::for_room`), and so do the encounter lock walls. The room
publication retracts the overlay on the root whose geometry it replaces, in
the same step, so every publishing road retracts it (the room commit's own
retract, and its `feature_overlay` field, are deleted). `CollisionWorld` and
the projectile collision world read geometry, platforms and overlay off ONE
root. Deleted: the resource, its `init_resource`, and Mary-O's and Sanic's
plugin-build inserts of it. Witness:
`a_breakable_contributes_to_its_own_live_rooms_overlay` (a crate in #7 is in
#7's overlay and not in #0's). ⚠ The content contributors (authored gated
walls, falling sand, portal carves, GNU-ton's gate, Mary-O's bricks and
hidden blocks, Sanic's monitors) write `RoomOverlays::sole()`, the same
named debt as `SoleLiveRoomMut`: they are one-room content today.

✅ **Cut 3d landed 2026-09-29: a body collides with the live room it is in.**
`CollisionWorld` queries every live room root and answers for the room a
reader names: `room(Option<&InRoomInstance>)` gives that room's
`RoomCollision` (geometry, platforms and overlay off one root), and no other.
A named room that is not live, or that two roots share, gives `None`. It
never falls back to "the" room. The body step (`integrate_sim_bodies`) and
the brain tick (`tick_actor_brains`) read each body's stamp. They compose
each live room once per tick, and they skip a body whose room is not live.
Deleted: the `Single` in `CollisionWorld`, and the body step's "no room, so
return" early exit for all bodies. Witnesses:
`a_reader_collides_with_the_live_room_it_is_in_and_no_other` (with #0 and
#1 both live, each reader composes only its own room's walls; with one room,
the unkeyed reader gets it) and `a_body_moves_only_against_the_live_room_it_is_in`
(the driven body stamped #7 does not move under a held run, and it runs in
both directions in its own room). ⚠ The shorthand readers `solids()`,
`carves_only()`, `hostable_surfaces()` and `base()` still name no room: 23
calls in 19 files (traversal, damage, held items, projectiles, the boss
tick, tether, trace). Each must be keyed by its subject. Cut 4a keyed the
projectile world.

✅ **Cut 4a landed 2026-09-29: two bodies meet only in one live room.** One
rule says which live room an entity is in: `LiveRooms::of` (shared_tangle)
gives its own `InRoomInstance`, or else the sole live room. With two live
rooms, an entity with no stamp is in neither, and it meets only another
entity whose room is also unknown. Keyed by it:
- the body-contact snapshot (`field_for` gives only the boxes of the
  body's room);
- melee strikes (`apply_hitbox_damage`: the strike's room is the volume's
  stamp, else its owner's);
- the projectile stepper (the shot's room picks its collision world, and its
  victims, breakables and bosses);
- perception (peers and shots are grouped by room, and `peers_in(room)`
  borrows one room's run, so there is still no copy per viewer);
- target selection (`select_actor_targets`), and through it contact damage,
  which strikes only the target;
- crew calls.
`ProjectileCollisionWorld` is no longer a `Single`. Deleted: the unkeyed
contact vector, and the whole-population peer and shot slices.
Witnesses, each with a one-room control:
- `bodies_in_two_live_rooms_do_not_meet`;
- `a_swing_does_not_reach_a_body_in_another_live_room`;
- `a_shot_does_not_reach_a_body_in_another_live_room`;
- `a_viewer_perceives_only_the_bodies_of_its_own_live_room`;
- `an_actor_does_not_target_a_body_in_another_live_room`;
- `a_crew_call_is_not_heard_in_another_live_room`.
⚠ Cut 4b still owes:
- `HitTarget::Volume` events (blink, dive, mark-recall, projectile splash,
  wielded AOE). They carry no room, and `apply_feature_hit_events`
  resolves them by overlap. Keying them adds a field to a rollback-coded
  message, which is a schema bump.
- the steering and crowd indices;
- footstool;
- pickups and interactions;
- the 23 shorthand `CollisionWorld` readers.

✅ **Cut 4b landed 2026-09-29: a hit that names no victim, a footstool and
a pickup stay in one live room.**
- A `Volume` broadcast reaches only the actors, bosses, breakables and
  primary player of the live room the hit is in: its own `HitEvent::room`,
  else its attacker's (`HitEvent::live_room`). See "Repaired after review"
  below.
- The pogo refresh matches a breakable in the attacker's room only. Two
  instances of one room have the same crates at the same places.
- `apply_feature_hit_events` and `apply_player_hit_events` ask it with no
  new `HitEvent` field and no schema change.
- `claim_footstools` pairs only bodies of one room.
- Pickups follow the same rule: magnetize, collect, world items and
  held-item pickup.

Witnesses, each with a one-room control:
- `a_broadcast_hit_does_not_reach_a_body_in_another_live_room`;
- `a_body_does_not_stand_on_a_head_in_another_live_room`;
- `a_body_does_not_collect_an_item_in_another_live_room`.

✅ **Repaired after review, 2026-09-29.** Two of cut 4b's readers were keyed
by room, but their writers did not supply the room. Now:
- **A world item is in a room from its spawn.** `spawn_world_item` and
  `spawn_moving_world_item` take the item's live room as a parameter and
  stamp `InRoomInstance`. Before, they inserted only `RoomScopedEntity`,
  so with two live rooms no body could collect the item. Mary-O's block
  pays out into the room of the body that struck it.
  - Witness: `a_body_does_not_collect_an_item_in_another_live_room` spawns
    through both production helpers, with two live room roots. The
    control is an item in the body's room, and an unstamped item fails it.
- **A hit carries its own room.** `HitEvent::room` is the live room of a
  hit whose writer knows it apart from the attacker. `HitEvent::live_room`
  is the one rule: the hit's room, else its attacker's, else the sole
  live room. The bomb blast gives the bomb's room, and a shot's landing
  splash gives the shot's room. Both consumers (`apply_feature_hit_events`
  and the player-hit `Volume` branch) ask it. The staged player-hit
  checksum includes it (schema 268).
  - Witness: `a_blast_with_no_attacker_hits_only_its_own_live_room`. The
    shipped `tick_bomb_fuses` detonates a bomb in #0 with #0 and #1 live.
    The control enemy in #0 is hit, and the enemy at the same place in #1
    is not.
- ⚠ Still owed: a bomb's own room comes from the room build that
  constructed it. This change does not check which room a bomb that a
  body carries into another live room is in. That is custody transfer
  between instances (OW2).

⚠ Deferred to cut 5, because the key is the identity: the crowd and
steering indices are maps keyed by the authored body id, and two
instances of one room share those ids. A room key there is part of the
per-instance identity. Also still owed: interactions (the nearest
interactable is a view-side affordance) and the 23 shorthand
`CollisionWorld` readers.

**Cut 5, measured 2026-09-29 before it was cut into slices.**
- An occupant's identity is `placement:{authored id}`, so a second instance
  of one room mints the same identities as the first.
- Construction compares `SimId` alone. A second construction of a room
  would SUPERSEDE the first instance's bodies (`transaction.rs`,
  "superseding"), not be refused, because the baseline is scoped to the
  session and not to the live room.
- The GGRS carrier order sorts by `SimId` and falls back to the local
  spawn order on a tie.
- Crowd facts and `WorldMemory` are keyed by the authored id string.
- Saves name places by room id. Nothing durable stores the ordinal, which
  already matches the plan.

⇒ A live identity is the pair (live room, `SimId`). The `SimId` string stays
the authored one, because every lookup built from an authored id
(`SimId::placement(id)`) must keep working.

The slices:
- **5a**: one minting counter for the session.
- **5b**: construction's baseline, supersession and duplicate checks keyed by
  the pair.
- **5c**: the carrier order and census keyed by the pair.
- **5d**: the crowd, steering and memory maps keyed by the pair.
- **5e**: `RoomSet`'s active index becomes each live room's own definition.
  It has 141 `active_spec()` call sites in 76 files, so it is the largest.
  Design, decided 2026-09-29: the index is two facts under one name.
  - **The live fact** (which definition this live room instantiates) moves
    onto the live room root as `LiveRoomDefinition`, beside its geometry.
    Publication writes the replaced root's definition, not the session's.
  - **The prepared fact** (which room a prepared world activates into, which
    hot reload normalizes and which is not `start`) stays on `RoomSet`,
    renamed `activation`. Only activation reads it.
  - A reader with a subject reads the subject's root. A reader without one
    uses the one-live-room read, named as the debt `SoleLiveRoom` is.

✅ **Cut 5a landed 2026-09-29: the session mints each live room from one
counter.** A publication used to advance the root it replaced
(`LiveRoomInstance::advance`). With live rooms #0 and #1, replacing #0 gave
it #1 too: two live rooms with one identity. Now:
- `RoomSet` carries `next_live_room`, the one counter on the session root
  that the decision record names. It is in the room set's checksum (schema
  267).
- Staging pins it (`LiveRoomSuccession { replaces, mints }`).
- The verifier refuses a pin that another publication already minted
  (`StagedWorldViolation::StaleMint`) before anything is torn down.
- Application moves the counter past the pin, and seats the replaced
  root as the pinned instance.
- A hot reload that replaces the set keeps the counter
  (`inherit_live_room_counter`), so it cannot mint an identity twice.

Deleted: `LiveRoomInstance::advance` and "the next instance is the replaced
one's next". Witness: `a_publication_mints_the_sessions_next_live_room`.
With #0 and #1 live, replacing #0 seats it as #2, #1 keeps its identity,
and the counter is at #3. The control is a room pinned to #1 as the old rule
pinned it, and it is refused with `StaleMint`.

✅ **Cut 5b landed 2026-09-29: a room transaction's world is the live room
it replaces and the one it mints.** Construction's baseline and the three
verifier gathers were scoped to the session. With two live rooms, a
planned identity found on the other room's body was declared SUPERSEDED,
and publication despawned it. Two live room roots with one identity were a
duplicate, and the baseline refused the room. Now:
- `TransactionRooms` (shared_tangle `lifecycle`) names the pair. An entity
  is in a live room by its `InRoomInstance` stamp, or a root by its own
  `LiveRoomInstance`. An entity with neither is in every transaction's
  world.
- `transaction::open` reads the pair from the publication's
  `LiveRoomSuccession`. `TransactionBaseline::capture_for_session` keeps
  only the pair's entities and records the pair.
- The three gathers (`transaction::close`, the actor lane and the
  capability lanes) read it from the baseline (`TransactionBaseline::rooms`),
  so the verifiers see the world the baseline saw.

Witness: `a_publication_leaves_the_other_live_rooms_occupants_standing`.
Live rooms #0 and #1 both have the shared root identity. A body wearing the
candidate's authored identity stands in #1, and replacing #0 publishes and
leaves it standing. The control puts the body in #0, and it is superseded.

✅ **Cut 5c landed 2026-09-29: the GGRS carrier order and its census key
by the live identity.** `rebase_rollback_carrier_order` sorted by `SimId`,
so two instances of one room tied and this App's spawn order decided their
place in the peer-compared checksum. The sort key is now (`SimId`, live
room), from `live_room_of`, the one rule `TransactionRooms` also asks. The
populated-timeline census requires the pair to be unique. One live room
orders as before (schema 269).
- `live_room_of` does not put an unstamped entity in the sole live room,
  as `LiveRooms::of` does: an identity must not change when a second room
  goes live.
- Witness: `two_instances_of_one_identity_order_by_their_live_room`. Two
  carriers named `alpha`, in #0 and #1, rebase to #0 then #1 whichever
  spawned first. The control spawns them in room order.

The roots all still carry `session:room_instance`; with the pair key they
no longer collide in construction or in the carrier order.

✅ **Cut 5d landed 2026-09-29: a crowd is one live room's bodies.** The
crowd observation keyed its requests, factions, targets and outputs by
actor id. Two instances of one room have the same ids, so their fighters
were one crowd: a body was pushed by a body in another room, and the last
writer of an id won its nearest neighbour. Now `CrowdObservation` keeps one
`RoomCrowd` per live room (the body's `InRoomInstance`), derives each room
alone, and keys `CrowdFacts` and `ActorSteering` by the body that reads
them.
- ⛔ Corrected after review: `WorldMemory` DID need a change. Perception
  shows a body only its own live room's peers, but the memory outlives a
  body that changes live room, and its positions are the old room's
  coordinates. Repaired below ("Tactical memory is room-local").
- Witness: `a_body_is_crowded_only_by_its_own_live_room`. Rooms #0 and #1
  each hold fighters `a` and `b`, together in #0 and far apart in #1. #1's
  pair is not crowded and `a`'s neighbour is #1's `b`. The control is #0's
  pair, which crowds each other.

✅ **Cut 5e landed 2026-09-29: a live room names its own definition.**
`RoomSet::active` was one index on the session root. It answered two
questions: which room a prepared world activates into, and which room the
session is in now. With two live rooms, the second question has one answer
for each room, and the one index moved for all of them. Now:
- **The live fact** is `LiveRoomDefinition` on each live room root, beside
  its geometry. A publication writes it onto the root it replaces
  (`apply_world_replacement`). It is a canonical rollback row
  (`root.live_room_definition`, schema 270), so a rewind across a
  publication returns the root to the room it was.
- **The prepared fact** stays on `RoomSet`, renamed `activation`. The
  session construction (`LiveRoomWorld`) and hot reload read and write it.
  The `RoomSet` checksum hashes the activation room.
- Readers with a subject ask `RoomSet::spec(definition)`, and
  `transition_for_player` and `nearby_zone_hints` take the definition.
  Readers without one take `SoleLiveRoomSpec` (the set and the sole live
  room's definition), named as the one-live-room debt beside `SoleLiveRoom`.
  64 production uses in 45 files at 5e; `check_alias_census_agrees_with_source.py`
  counts them.
- Fixtures seat a live room with `insert_room_set` or
  `seat_sole_live_room_by_id`. A set with no seated live room answers no
  "which room is this" question, so a `SoleLiveRoomSpec` system does not
  run.

Deleted: `RoomSet::active`, `active()`, `active_spec()`, `active_world()`,
`active_metadata()`, `active_loading_zones()`, `active_props()`,
`set_active()`, `set_active_by_id()` and `neighboring_room_indices()`.
Witness: `a_publication_seats_only_the_live_room_it_replaces_in_its_room`.
Live rooms #0 and #1 both instantiate room `n`, and the candidate replaces
#0. The replaced root reads `candidate` and #1 still reads `n`. The control
is the two roots before publication, which both read `n`. With one index,
#1 read `candidate` too.

✅ **Repaired after review, 2026-09-29: an operation on one body names its
live room.** Commands and intents named their body by `SimId` alone, and two
live instances of one room hold the same ids. So a dialogue action, a pet or
a crossing reached the first body with that id. Now one type names one live
occurrence: `LiveBodyId` (the `SimId` and the live room it is in, from
`live_room_of`: its `InRoomInstance` stamp, `None` for a session-level body).
`LiveBodies` resolves it both ways (`id_of`, `entity_of`).
- Producers capture it where they hold the entity: the four yarn commands
  (`challenge`, `pet`, `use_brain`, `restore_brain`), the transition
  detector, the checkpoint resume and startup resume, the level departure,
  the replay admission and the death retraction.
- It rides in `BrainCommand.target`, `ReleaseProvocation.target`,
  `ChallengeRequested`, `PetRequested`, `PetBeat.petted`,
  `RoomReplayAdmitted.subject` and `RoomTransitionIntent.subject`.
  `TransitBodies::subject_entity` resolves the exact occurrence and gives
  no substitute from another live room. Schema 272: the crossing's subject
  is in the codec and the checksum of `resource.pending_lifecycle_commit`.
- ⚠ One reader runs AFTER the commit: the checkpoint restore's verification.
  The commit carries the subject into the live room it publishes, so the
  recorded room is the room the body left. The verifier looks for the body
  in the produced room. (It first read the recorded room, refused every
  restore, and left the game paused.)

Witnesses: `a_brain_command_reaches_only_the_body_in_its_own_live_room`
(two `puppy` bodies in #0 and #1; a command for #1 switches #1, and the
control is #0 still on its default), and
`a_missing_transition_subject_resolves_to_none_never_a_substitute` (a
crossing whose subject is in #1 resolves the #1 body, the #0 duplicate
resolves only when named, and #2 resolves nothing), and
`retraction_removes_only_the_crossing_owned_by_that_body` (the duplicate's
death in #0 does not retract #1's crossing).

✅ **Repaired after review, 2026-09-29: tactical memory is room-local.** A
body that remembered a hostile at P in live room #0 and was then retagged to
#1 pursued P in #1, a place where no foe was seen. `WorldMemory` now records
the live room its memories were formed in (`room()`, the `LiveRoomInstance`
ordinal), and `enter_room` forgets them when the body is in another live
room. `tick_actor_brains` scopes each body's memory to its own stamp before
it hears or folds, and a body whose memory is of another room makes no crew
calls. The key is the stamp and not `LiveRooms::of`, which puts an unstamped
body in the sole live room and would change answer when a second room goes
live. Durable social memory (grudges, fates) is a separate store and is not
changed. Schema 273 (`actor.perception_memory` carries the room). Witness:
`a_memory_from_another_live_room_is_not_a_pursuit_target` (remembered at P
in #0, retagged to #1, sees nobody: no target; the control stays in #0 and
pursues P; #1's own `x`, seen at Q and lost, is pursued at Q).

✅ **Cut 6a landed 2026-09-30: a crossing reads the crossing body's own
live room.** The transition detector read the loading zones of "the" live
room (`SoleLiveRoomSpec`), so with two live rooms it did not run at all, and
a body could not leave either room. Now it takes `LiveRoomSpecs`: the room
set, and for an entity the definition of the live room it is in
(`LiveRooms::of`, then that room's root's `LiveRoomDefinition`). One live
room answers as the sole read did. Witness:
`a_crossing_reads_the_crossing_bodys_own_live_room` (live room #0 is room
`a`, #1 is room `b`; a body in #1 standing in `b`'s zone records a crossing
to `a`; the control, the same body at the same place in #0, records none,
because `a` has no zone there).

✅ **Cut 6b landed 2026-09-30: a crossing leaves its subject's own live
room.** Cut 6a recorded a crossing from a second live room, and it waited:
the readiness took its source room from `SoleLiveRoomSpec` and did not run
with two live rooms, and both commits compared the transaction against the
sole root's definition. Now the room a crossing leaves is one answer,
`LiveRoomSpecs::left_by(subject)`: the live room the intent's subject was
recorded in (`LiveBodyId.room`), and the sole live room only for a crossing
with no subject or a subject in no live room.
`live_room_definition_left_by` is the same answer at an exclusive-world
boundary. The readiness (`source_room`), the eager commit's staleness check
(`RoomTransitionApplication::definition_left_by`) and the confirmed host's
authorization all read it. The staging already took the departing residents
from the subject's room (5b). Witness:
`a_crossing_leaves_its_subjects_own_live_room` (live rooms #0 `a` and #1
`b`; a subject recorded in #1 leaves `b` in both reads; the control, a
subject in #0, leaves `a`; a crossing with no subject leaves no room while
two are live).

✅ **Cut 6c landed 2026-09-30: a crossing opens a live room when another
player stays.** Until now every crossing replaced the room it left, so the
session never had a second live room unless a test built one. A publication's
`LiveRoomSuccession` is now `Replace { replaces, mints }` or
`Open { leaves, mints }`, and `for_crossing` decides: the crossing opens a
room when another player's body (a different `DrivingParticipant` slot) is
still in the room it leaves (`another_player_stays`). Only a crossing
some slot drove when it was accepted opens a room (see the review note
after cut 6e): a body no slot drives (the d71 crossing body) is
sent across by the session, which follows it and replaces the room as
before. The subject's own slot does not count, so one player still has one
live room. The first lane found the second rule: three app tests (the d71
crossing body, and two possession crossings in `carried_item_crosses_rooms`)
opened a room and then failed on the harness's sole-room read; with the
rule, they pass unchanged. An opened room's transaction world
is the minted room alone (`TransactionRooms::opening`): the room it leaves
is not in its baseline, a body there is not superseded, and nothing is
retired. Publication spawns a new live room root for it and moves only the
crossing body and its custody closure (`InCustodyOf`) into it. Witnesses:
`a_publication_that_opens_a_live_room_leaves_the_room_it_leaves_whole` (#0
`n` with three bodies, one wearing the candidate's identity; opening #1
leaves #0 `n` with all three, and #1 `candidate` with its occupant; the
control, replacing #0, leaves one live room and nothing of #0), and
`a_room_stays_live_for_another_player_and_not_for_the_subjects_own_bodies`
(the decision). Both room roots wear the one `session:room_instance`
identity (cut 5b scoped the baseline, so that is accepted), and a room that
opened is never retired when its last player leaves: retiring an empty live
room is owed.

✅ **Cut 6d landed 2026-09-30: Alice and Bob, end to end in the app.** In
the shipped app, Alice (the primary slot) goes through the `switch_lab` door
to the hub while Bob, a body slot 1 drives, stays. Afterwards two live rooms
are simulated: `switch_lab` (#0) with Bob in it, and the hub (#1) with Alice
in it. Thirty ticks later, Bob's slot still runs his body in #0. The one
sole-room read the crossing met was the sim harness's observation. It now
reads the observed body's own room (`live_room_spec_of`, the room of its
`InRoomInstance` stamp), and a body stamped into a room that is not live
observes no room rather than another room. Witness:
`a_door_crossed_by_one_player_leaves_the_other_players_room_live` (the
control is the same body driven by no slot: the crossing replaces the
room, the hub is the one live room, and the body is retired with
`switch_lab`; Bob's run before the crossing is the control that slot 1
reaches his body in this harness).

✅ **Cut 6e landed 2026-09-30: a crossing into a room another player holds
joins it.** After 6d, Alice coming back to `switch_lab` built a second live
room of `switch_lab` beside Bob's. So two players in "one room" stood in two
worlds, and neither saw what the other did. A third succession,
`Join { leaves, joins, retires }`, is chosen when a live room of the target
room holds a body of another slot (`joined_room`). A join mints nothing and
builds nothing: its publication verifies an empty roster
(`RoomFeatureConstructionPlan::emptied`, the same transactions with no
entities). It writes nothing onto the joined room's root (definition,
geometry, platforms, collision overlay), because that room is the other
player's live world. When no other player stays in the room it leaves
(`retires`), that room's roster is retired as a replaced room's is, what
is left there crosses with the body, and its root is despawned. Otherwise
only the body and its custody closure move. The first app run found that
a retiring join's transaction world held both live room roots, which wear
one identity, and the baseline refused the duplicate. A join's world is
now one room (`TransactionRooms::only`): the room it retires, so its
departures are still declared, or else the room it joins. Witnesses:
`a_crossing_into_a_room_another_player_holds_joins_it` (retiring: #1 is
the one live room, with its body and without the candidate's occupant;
keeping: #0 and #1 both stand; nothing minted; the control, opening from
#0, builds a second live room of `candidate`; a join staged for a room that
is not there is refused `StaleJoinedRoom`; root #1 wears the production
root identity), `a_crossing_joins_the_live_room_of_its_target_that_another_player_holds`
(the decision), and in the app
`a_player_who_comes_back_joins_the_room_the_other_player_holds` (one live
room, both players in it, and Bob's slot still runs his body). The
`door_to` test helper now reads the walker's own room.

✅ **Review of cut 6, 2026-09-30: the crossing's participant is recorded
when the crossing is accepted.** Open and Join ask "another participant",
and the commit read who that is from the subject's `DrivingParticipant`
at commit. Commit is frames after detection, and possession, a death or a
handoff can move the slot in that window. The crossing then read as
nobody's, and it replaced the room another participant was in.
`RoomTransitionIntent.participant` is now written by detection and by the
level-exit departure. It is in the pending commit's checksum and snapshot
(schema 275), and both commit roads (eager and rollback) give it to the
decision. Witnesses: `a_crossing_is_the_participant_it_was_accepted_for_when_it_opens_a_room`
and `..._when_it_joins_a_room` (Alice's slot is taken off her body between
acceptance and commit). A poison that reads the subject's slot at commit
again made both fail: Bob's room was replaced, and a second live room of
`switch_lab` was built. The controls are the two cut 6d/6e app tests.

⚠ Owed after cut 6d, found while writing it:
- No production road seats a second player in ordinary play. Bob is a
  harness body with `DrivingParticipant(PlayerSlot(1))` inserted by the
  test. Match seats (`character_runtime/match_activation.rs`) and actor
  spawns build their `SessionSpawnScope` with `for_optional_active_session`,
  which names no room, so their bodies are unstamped, and an unstamped body
  does not keep a room live. A join road must stamp its body into the live
  room it joins.
- ~~An opened room is never retired when its last player leaves it.~~
  Answered by the successions: the last player to leave a room replaces it
  or, by a join, retires it (6e).
- Two live room roots wear one identity (`session:room_instance`). A join
  works around it with a one-room world. The root is a peer-compared
  rollback carrier, so a per-instance identity moves the checksum and is
  its own cut.
- Both players share one camera and one observation. The per-player view is
  P5 (multiview).
- Measured (cut 7h, 2026-09-30): under a GGRS sync test with two seats,
  Alice's crossing and Bob's run afterwards resimulate to the same
  checksums, the two live rooms stand, and Bob's slot still runs his body
  in #0 (`two_players_in_two_live_rooms_resimulate_to_the_same_world`; the
  instrument's control: a system that nudges Bob by a counter a rewind does
  not restore, once two rooms are live, reads as a checksum mismatch). A
  sync test has one peer, so it does not say what the rebase at the
  crossing costs a remote Bob's rollback window; that needs a two-peer
  session. ✅ Measured since (the review of the per-game heavy-hit rule,
  below): `GoverningRules` read THE live room, so with two rooms live it
  answered the rules of no room, in both rooms.

✅ **Cut 7a landed 2026-09-30: the systems a second live room froze, first
two.** `SoleLiveRoom<T>` and `SoleLiveRoomSpec` are `Single`s, so a system
that takes one does not run at all while two rooms are live, in either
room. After 6d that is the Alice/Bob world. The measured list of simulation
systems it stops (read from the parameter lists, 2026-09-30):
`step_item_motion` ✅, `update_ecs_falling_chests` ✅,
`prepare_authored_switch_commands` ✅ (7b), `drive_wave_encounters` ✅ (7c),
`update_boss_encounters` ✅ (7e; its scripted road in the review of 7e), `heal_save_shrine_system` ✅ (7d),
`sync_encounter_reward_chests` ✅ (7d), `retire_rewards_for_rearmed_encounters` ✅ (7c),
`record_placed_ground_items` ✅ (7d), `physics_spawn_debris_messages` (presentation, see 7m) and
`tick_npc_idle_barks` ✅ (7m), and the content bosses (`cut_rope` ✅ in the review of 7e, `gnu_ton` and the
flying spaghetti monster ✅ in cut 7j). ⚠ This list was not complete. A
second reading at cut 7k (every non-render function that takes a sole-room
parameter) also found `push_room_entered_quest_events` ✅ (7k),
`auto_trigger_room_cutscenes`, the two boss specials
`spawn_overflow_flood_from_special_messages` and
`spawn_apple_rain_from_special_messages`, the portal gun's
`portal_projectile_step` and `sync_portal_host_depths` ✅ (7l), the falling-sand
room ✅ (7m), and the demos' one-room systems (Mary-O, Sanic, Smash). The
presentation readers in `ambition_render` are P5's (a view per player). The
new reader is `LiveRoomOf<T>`: `T` of the live room an entity is in, by the
rule of `LiveRooms::of`. Item motion and falling chests now step each
entity against its own room's geometry, and an entity in no live room does
not move. Witnesses: `each_pickup_falls_onto_the_floor_of_its_own_live_room`
and `each_chest_lands_on_the_floor_of_its_own_live_room` (live rooms whose
floors differ by 200; each lands on its own; the control is one room).

✅ **Cut 7b landed 2026-09-30: a switch is pressed and answered in its own
live room.** Two defects, one road. The interact loop did not compare the
switch's room with the pressing body's, so a body in #0 pressed a switch at
the same place in #1 (cut 4 keyed the other pairwise reads, and this one was
missed). And the authored verbs were prepared for the sole live room, so
with two live rooms no switch verb ran. Now a body presses only a switch in
its own live room (`LiveRooms::of` on both), `SwitchActivated` names the
switch's room, `AuthoredSwitchCommands` holds each live room's verbs by room
id, and the request asks the activation's room
(`LiveRoomSpecs::definition_named`). Witnesses:
`a_body_presses_only_the_switch_in_its_own_live_room` (Alice in #0 and Bob
in #1 at one place, a switch there in each room; each presses only its own,
and the activation names the room; the one-room control is
`two_driven_bodies_each_flip_their_own_switch`) and
`each_live_rooms_switch_asks_for_its_own_rooms_verb` (two rooms author one
switch id with two verbs; each press rings its own room's; the one-room
control is `pressing_an_authored_switch_asks_for_the_verb_the_level_named`).
~~⚠ The encounter road the same activation feeds (`SwitchActivationQueue`,
`drive_wave_encounters`) does not read the room yet.~~ It does since 7c.

✅ **Cut 7c landed 2026-09-30: every live room's encounter runs.** The wave
driver read the sole live room, so with two live rooms no encounter
started, advanced or reset, in either room. It now reads every live room
(`LiveRoomSpecs::live_definitions`): an in-flight encounter resets when its
room is not live, the first encounter of each live room starts when a
player in that room enters its trigger, and each live room's waves run.
The room of a press reaches the drain: `SwitchActivationQueue` holds
`QueuedSwitchActivation { activation, room }`, the room is in its checksum
(schema 274), and `ResolvedSwitchActivation.room` carries it on. A re-arm
that names no encounter targets its switch's own room's, in the driver and
in the reward retire. Witnesses:
`a_player_in_either_live_room_starts_only_that_rooms_encounter` (a player
in #1 starts #1's encounter; the control, the same player at the same place
in #0, starts nothing), `an_unnamed_rearm_retires_the_reward_of_its_own_rooms_encounter`
(two rooms, two encounters, two chests; the press in #1 retires only #1's)
and `the_room_of_an_activation_moves_the_checksum`. ~~⚠ Still sole-room on
this road: a player's death fails every in-flight encounter, in every
room.~~ Done in cut 7f.

✅ **Cut 7d landed 2026-09-30: a chest, a shrine and a put-down item, each
in its own live room.** Three more systems that a second live room froze.
The reward chest sync now runs for every live room
(`LiveRoomSpecs::live_rooms`), and it spawns each cleared encounter's chest
stamped into its own room. A body rests only at a shrine in its own live
room (`LiveRooms::of` on both), and the checkpoint names the resting body's
room, not "the" room. The ground item placement records each item that
comes to rest in the room it is in, and republishes each room's placements
on its own. Witnesses: `a_cleared_encounters_chest_stands_in_its_own_live_room`,
`a_body_rests_only_at_a_shrine_in_its_own_live_room` and
`each_item_put_down_is_placed_in_its_own_live_room`. The controls: one
live room for the chest and the item, and for the shrine, Alice pressing
at the same place in the other room. Poisoned together (first live room only, no room
filter, first definition), each failed on its own assertion: no chest in
#1; a rest at the other room's shrine; both items placed in `hall`. ~~The
chest sync finds an existing chest by encounter id across rooms.~~ Since the
review of 7c it finds it in the occurrence's own live room.

✅ **Cut 7e landed 2026-09-30: a boss fights and drops its chest in its
own live room.** The boss driver read the sole live room's geometry for
its reward chest, so while two rooms were live no boss woke, fought, died
or dropped a chest, in either room. It now reads each boss's own room
(`LiveRoomOf::room_of`), and each room's cleared bosses drop their chests
in that room, on its floor, stamped into it. The sim harness staged a
scenario actor into "the" live room, and panicked with two. It now stages
into the primary player's own live room. Witness:
`a_boss_in_one_of_two_live_rooms_fights_and_drops_its_chest_in_its_own_room`
(Alice in #1 with a mockingbird, Bob in #0: the boss wakes and its music
plays, and killed, it is recorded cleared, with one chest in #1). With the
old driver restored, the boss did not wake. The control is the one-room
fight in `boss_lifecycle`. ⚠ The boss music is still one track for the
session: the first boss fighting in any room is heard by both players (P5).

✅ **Cut 7f landed 2026-09-30: a death ends its own room's attempt.** The
wave driver failed and reset every in-flight encounter on any player's
death, so Bob's death in one room ended Alice's fight in another. Now a
death fails only the in-flight encounters of the room its victim is in
(`LiveRoomSpecs::definition_of` on `ActorDiedMessage::victim`). A death
whose room cannot be told ends nothing. Witness:
`a_death_fails_only_the_encounter_of_its_own_live_room` (the encounter of
#1 is in flight; a death in #1 fails it, and the control, a death in #0,
does not). With the old rule restored, the control failed.

✅ **Review of cuts 7c and 7f, landed 2026-09-30: an encounter is an
occurrence, not an authored id.** 7c and 7f made the driver read every live
room, but under it the encounter runtime still knew an encounter only by
its authored id. A spawn request named no room, so a wave's mobs were in no
live room and, with two rooms live, chose no foe. The lifecycle reducer
applied a command to every entity with the id, so two live rooms of one
room started, failed and reset as one. An authored encounter is its
definition id. A live encounter occurrence is the pair (live room, authored
id). `ambition_encounter::occurrence` holds the one rule: an occurrence's
room is its `InRoomInstance` stamp (`LiveRooms::of`); a message's room is
the room it names, else the sole live room. `EncounterCommand`,
`EncounterEventMsg`, `EncounterGate` and `SpawnCommand` name the room.
`project_live_encounter_occurrences` builds one stamped occurrence of each
encounter of each live room, from the rooms that are live; it replaces
`populate_encounter_registry` and its session latch, and
`EncounterRegistry` is deleted (schema 276). The driver, the reducer, the
cleanup, the cleared list, the reward chest and the authored `encounter`
verb each address an occurrence by its id and room. The mob spawn server
stamps each mob into the room its request names. The rollback identity of
an occurrence keeps `SimId::encounter(id)`: the carrier order sorts by
`(SimId, live room)`, so two occurrences are two rows. The save keeps one
fate per authored id: the occurrences fold to cleared over failed over
untouched. Witnesses: `a_wave_spawns_its_mobs_in_the_live_room_that_started_it`
(Bob in the hub, #0; Alice in `goblin_encounter`, #1, in its trigger: every
mob is stamped #1 and targets Alice, not Bob put beside it; with the stamp
removed from the spawn server, the mobs were in no room and targeted
nobody) and `two_live_rooms_of_one_room_run_their_encounters_apart` (two
live rooms of `goblin_encounter`: Alice starts only #1 and Bob only #2;
Alice's death ends #1 while #2 stays in flight; every spawn names #2's
room; with the reducer keyed by id only, Alice's entry started #2 too).
Superseded: 7f's per-room-id death set is now per occurrence. The first
draft of this repair looked the spawner's room up by authored room id; it
was not landed, because two live rooms of one room are one authored id.
✅ The driver's per-occurrence mob liveness lookup is witnessed on its own
(2026-10-01): `an_occurrence_reads_the_liveness_of_its_own_rooms_mobs` (two
live rooms of `goblin_encounter` ask for mobs of the same ids on one tick;
each request is served into its room, the mobs of one room killed: each
occurrence holds its own room's liveness, in both arrangements; with the
room filter removed from the lookup, #1 read #2's live mobs).
⚠ Still keyed by authored id: the music intent (`music/intent.rs`) and the
encounter camera zoom, which are views (P5), and the symmetry attunement
content encounter (✅ it reads every live room since cut 7i, below).

✅ **Review of cut 7e, landed 2026-09-30: the scripted boss fight runs in
its own live room.** 7e made the boss driver multi-room, but the scripted
road under it stayed sole-room. The wrap read the sole live room's props,
so with two rooms live it had none and the cut-rope script could not be
prepared. The dropped hazard was unstamped, the hazard fell against the
sole live room's geometry (so, with two rooms, not at all), and the rope
detector, the flavor and the victory NPC read the sole live room. A boss
wrap is now an occurrence like a wave encounter: the boss's placement id in
the boss's live room. `sync_boss_encounter_entities` covers bosses by
(id, room), prepares the script from the props of the boss's own live
room, stamps the wrap into that room and makes it room-scoped (it retires
with its room, as the boss does; it no longer outlives a room change).
`update_encounter_progress` resolves a member by id in the wrap's room.
`tick_encounter_scripts` gives a script only the gates fired in its own
room, and stamps what it drops into that room. `tick_falling_hazards` reads
each hazard's own room's geometry (`LiveRoomOf`) and fires the impact gate
in that room. In content, the cut-rope arena state is one arena per live
cut-rope room; the rope detector takes a hit's live room
(`HitEvent::live_room`) and fires `rope_cut` there; the flavor mirrors each
room's own hazard and boss; the victory NPC is released per room and stamped
into it. The hazard keeps `SimId::spawned(encounter, n)`: two occurrences'
hazards are told apart by the room in the carrier order, as the wraps are.
Witnesses: `the_cut_rope_fight_runs_in_its_own_live_room` (Bob in
`hall_of_bosses`, #0; Alice through its door into the arena, #1: the wrap is
stamped #1 with its script; a hit on #1's rope fires `rope_cut` in #1; the
behemoth is sent to #1's anvil and walks toward it; the hazard is stamped #1
and falls; `cut_rope_impact` fires in #1; the behemoth dies and is recorded
cleared; its victory NPC is in #1. Before this repair the wrap was
unstamped with no script. With the hazard unstamped, it never fell, and no
impact or death followed),
`two_live_rooms_of_one_boss_room_wrap_their_bosses_apart` (with coverage by
id alone, the boss that woke second in #2 had no wrap) and
`a_gate_fired_in_one_live_room_advances_only_that_rooms_script` (with every
script hearing every gate, both rooms' bosses died). The behemoth authors no
reward chest; its reward is the victory NPC. ⚠ Still session-wide, as views
(P5): the gameplay banner, the encounter music claims, the HUD, and the
prop visuals, which draw the sole live room's arena. ⚠ A room replay is of
the sole live room. The arena state is not rollback state, and its
`rope_cut` latch decided whether a hit fired the gate, so a rewind across a
rope cut would have resimulated the hit against a latch already set (the
resource-memory guard flagged it once the latch was written through a map).
Now every hit on the rope fires the gate, and the script's cursor, which is
rollback state, is the one record that the rope is cut; the arena gates only
effects. `a_replay_lets_the_rope_be_cut_again` counted gates, and a fixture
that wrote `RoomReplayAdmitted` rebuilt nothing, so it passed while the
spent script would drop no second anvil. It now asks for the replay
(`RoomReplayRequested`) and counts the anvils dropped, by (`SimId`, live
room): one before the replay, none from a second slash, and one more after
the replay rebuilds the room, its behemoth and a fresh wrap.

✅ **Cut 7i landed 2026-09-30: the Noether attunement starts in either
live room.** Its driver read the sole live room, so with two rooms live the
puzzle never started. The attunement is one puzzle for the session (an
unstamped authority, remembered by a save flag), so it now starts when any
live room is the chamber. Its switches' signals already reach it: an
unstamped occurrence and a signal that names no room agree under the
occurrence rule with one live room and with two. Witness:
`the_attunement_starts_when_the_chamber_is_one_of_two_live_rooms` (the
control, `hall` live alone, starts nothing; with the sole-room read
restored, neither case started it).

✅ **Cut 7j landed 2026-09-30: GNU-ton and the flying spaghetti monster
fight in their own live rooms.** Their conductors measured the hall against
the sole live room's geometry, so with two rooms live they did not run, and
the bosses stood still. The GNU-ton ladder gate read the sole live room's
geometry and overlay, and asked whether ANY GNU-ton was dead. Each conductor
now measures the hall against its boss's own live room (`LiveRoomOf`). The
ladder gate runs for each live room root: it hides that room's ladders
until the GNU-ton in that room is dead, and writes that room's overlay.
Witnesses: `gnu_ton_measures_its_hall_in_its_own_live_room` and
`the_fsm_measures_its_hall_in_its_own_live_room` (Bob in `hall_of_bosses`,
#0; Alice through its door to the arena, #1: the boss is in #1 and its hall
is measured; with the sole-room read restored, the boss was in #1 and no
hall was measured) and `each_live_arena_gates_its_ladder_by_its_own_boss`
(two live arenas, the boss of the second dead: the first keeps its ladder
hidden and its floor gate; with the room filter removed, the dead boss
opened both).

✅ **Cut 7k landed 2026-09-30: a room that becomes live beside another is
entered.** The quest producer pushed `RoomEntered` when the sole live
room's id flipped, so while two rooms were live it did not run, and the
room Alice entered while Bob held another was never entered: a quest step
"reach X" did not advance. Its memory (`LastQuestRoom`, rollback state) is
now the set of live room ids, and each id that becomes live is entered
once, in id order. A second live room of an id that is live already is not
entered again. With one live room the set has one member and flips as the
one id did. Schema 277 -> 278. Witness:
`a_room_that_becomes_live_beside_another_is_entered` (`hall` live, then
`cellar` beside it, then a second `hall`: `hall` and `cellar` entered once
each; with the sole-room read restored, `cellar` was never entered). The
one-room control is `restoring_the_last_room_makes_the_producer_announce_the_room_again`.
⚠ Not changed: `auto_trigger_room_cutscenes` has the same shape, but a
cutscene is one session-wide playback. Whether a cutscene that one player's
room starts also stops the other player is a product question, not a
keying one: multiplayer.md files it as "which story interactions pause
only one participant versus the whole party".

✅ **Review of the per-game heavy-hit rule, landed 2026-09-30: each live
room plays under its own game's combat rules.** `project_combat_rules`
resolved `CombatRules` and `StrikeWeightRules` through `GoverningRules`,
which reads `CurrentRoom`, which is the one-live-room read. With Alice and
Bob in two Ambition rooms it answered `NoRoom`, `UntaggedRooms` did not
govern it, Ambition's heavy line (3) was gone, and every robot hit played
the light cue. The projection now resolves the rules of each live room
(`LiveRuleRooms::of`, from that room's mode tag) and puts them on the room's
root as `RoomCombatTuning` (derived, schema 278 -> 279). The
`ResolvedCombatTuning` resource is now the rules of no room. Every combat
reader reads its subject's room through `CombatTuningOf::of`: the hit
resolver and the projectile stepper read the strike's room, the clash
arbiter each contender's, the body victim drain the hit's, the player
drain the struck player's, the grab, the throw and the escape the captor's
or captive's, and the pogo, the footstool, the clank rebound, the edge
cancel, the special turn, the move trigger and the brain's launch law the
body's. A stocks match reads the room its seated fighters share, and the
rules of no room when they do not share one. Four pairwise reads had no
room key and now have one: the grab, the clash, the ledge trump and the
contact harm (a body in #0 could grab, clank with, trump or harm a body at
the same place in #1). Witnesses:
`a_heavy_robot_strike_stays_heavy_while_another_room_is_live` (control: the
strike with one live room is heavy; with the one-room projection restored,
the strike beside Bob's room was light),
`two_live_rooms_of_two_games_hold_their_own_heavy_lines` (an Ambition room
heavy at 3 and a Smash room heavy at 12 at once; restored, both had no
line), `a_grab_does_not_reach_into_another_live_room`,
`attacks_clash_only_in_their_own_live_room_under_its_rules`,
`the_same_anchor_in_two_live_rooms_is_two_edges` and
`it_does_not_harm_a_body_in_another_live_room` (each red with its room key
removed). The strike is a written `HitEvent`; the swing that writes it is
room-blind on weight. ⚠ A root seated during a tick has no
`RoomCombatTuning` until the next `WorldPrep`, so for that part of a tick
its bodies read the rules of no room (before, they read the previous
room's rules for the same window). ⚠ Not witnessed reader by reader: the other
`CombatTuningOf` readers are one call each and read the same component.
~~⚠ Still the one-room read (`CurrentRoom`): the mode gates `in_mode`,
`in_base_mode` and `in_rules_scope`, and the other `GoverningRules` readers:
`death.rs`, `wallet_shield.rs`, `dormancy.rs`, `starting_character.rs`,
`control_prompt.rs`, the Smash limit meter and the TwinTrack participants.~~
Done in the next paragraph.

✅ **Rules per live room for the remaining readers, landed 2026-10-01.**
A reader with a subject now reads `RulesOf<T>`: the rules of the live room
its subject is in (`LiveRooms::of`; a subject in no live room reads the
rules of no room, as `CombatTuningOf` does). Death opens the victim's room's
beat, and a closing beat asks its body's room whether the level goes back.
The wallet shield, the dormancy rule, the driven techniques, the prompt
naming and the Smash limit fill read their body's room. A gate with no
subject (`CurrentRoom::in_scope`, so `in_mode`, `in_base_mode`,
`in_rules_scope`, the shell's menu suppression and the TwinTrack panes) is
open while its scope governs ANY live room, and the gated systems key by
subject. With one live room every answer is the same as before. Dormancy
also had no room key on its observers: a player in #1 woke an actor at the
same place in #0. Each observer now sees only its own room, and an actor
whose room holds no observer stays awake, as a world with no observer did.
Witnesses (unit, two live rooms of two games, the hall untagged and the
stage `smash`): `each_subject_reads_the_rules_of_its_own_live_room`,
`a_scope_governs_while_any_live_room_is_its_own` (controls: the one-room
arms), `a_death_holds_the_beat_of_its_own_live_room`,
`a_closing_beat_replays_by_its_own_live_rooms_rules`,
`an_actor_sleeps_by_its_own_rooms_rule_and_observers` and
`a_wallet_absorbs_by_its_own_rooms_rule_beside_another_live_room`. Each is
red with its reader on THE live room again, and the dormancy witness is
also red with the observer room key removed. ✅ Witnessed reader by
reader (2026-10-01), each red with its reader on THE live room again
(`GoverningRules::get`):
`the_driven_body_wears_the_techniques_of_its_own_live_room` (the gate: the
body on the Smash stage did not spin),
`the_prompt_follows_the_rules_of_the_subjects_own_live_room` (naming
poisoned: "Swat" on the stage; driven poisoned: no Special slot) and
`a_meter_fills_by_the_rule_of_its_own_live_room` (the stage seat gained 0
in one second, not 0.5). ⚠ Not changed: `project_room_rule` (a
resource for a crate that cannot see rooms) and the mode owners
(`despawn_departed_mode_entities`, `follow_mode_owner_rooms`) still read THE
live room. A mode owner is one entity for each mode, so its two-room meaning
(which room it follows) is a design question. The hosted demos are one-room
profiles today. ⭐ **Decided 2026-10-01 (autonomous-decision-making): a mode
owner belongs to one live room.** It holds one game in progress (Mary-O's
flag and timer, Sanic's act), so two live rooms of one mode are two games in
progress. The target is one owner per (mode, live room), born in the room
it governs and retired with that room. Until that lands, the sweep and the
follow read keyed facts: `despawn_departed_mode_entities` retires a
mode-scoped entity only when no live room is governed by its mode, as
`CurrentRoom::in_scope` does. `project_room_rule` stays a one-room
projection whose readers move to `RulesOf`.

✅ **Cut 7l landed 2026-09-30: the portal gun fires in its own live room.**
The shot step and the host-depth measure read the sole live room's
geometry, so while two rooms were live neither ran: a shot hung in the air,
and the depths of the last one-room frame stayed. The carve bridge wrote the
sole live room's overlay, so no wall was carved. A `PortalFireIntent` now
names the room it is fired in (the firer's, by `LiveRooms::of`). The shot
and the portal it opens carry that room as their `InRoomInstance`. Each
shot steps against its own room's solids, and each portal's depth is
measured in its own room. Witness:
`a_portal_shot_opens_its_portal_in_the_live_room_it_was_fired_in` (Bob in
`switch_lab`, #0; Alice in the hub, #1, fires down: the portal is in #1 and
has a finite depth; with the sole-room read restored in the shot step, no
portal opened in 60 ticks; restored in the depth measure, the depth was
`None`).

✅ **Cut 7l, second part, landed 2026-10-01: a portal pair is two portals of
one live room.** Portal core paired, carved and transited with no room
filter. A blue in one room paired with an orange in the other room, and a
body in one room crossed a portal of the other room at the same coordinates.
A shot closed the portal of its channel in every room. Now portal core groups
the placed portals by live room (`PortalsByRoom`, by `LiveRooms::of`):
transit, the free-body teleport, the carve, the straddle eviction, link
groups and the aperture equalize see only the portals of their subject's
room. Each carve names its room (`PortalCarves::holes`), and the bridge
writes it to that room's overlay. Host depths are filed by room
(`PortalHostDepthsByRoom`, schema 283 -> 284). A shot replaces its channel
only in its own room. A projectile threads only the portals of its own
room. A portal finds and follows its host face in its own room
(`CollisionWorld::room`): with two live rooms, the sole-room read did not
attach or carry any portal. Witnesses, each poisoned with the room-blind
form restored (the failure is in brackets): in `ambition_portal2d`
`rooms::tests`: `a_body_crosses_only_a_pair_of_its_own_live_room` (the body
in #0 came out at x=380), `a_blue_and_an_orange_in_two_rooms_are_not_a_pair`
(x=380), `a_carve_is_cut_only_for_a_body_of_the_pairs_room` (a hole for #1),
`a_closing_portal_evicts_only_the_bodies_of_its_room` (the body in #0 moved
from y=290 to y=279), `one_link_in_two_rooms_is_a_pair_in_each` (all four
ends closed). In content: `a_shot_replaces_the_portal_of_its_channel_in_its_own_room_only`
(one blue remained), `each_carve_goes_to_the_live_room_it_was_cut_in` ([3, 0]
for [1, 2]), `a_portal_finds_its_host_face_in_its_own_live_room`
(`Unattributed`). In the monolith:
`a_shot_threads_only_the_portals_of_its_own_live_room` (the shot in #0 came
out at x=1217). ⚠ A transit between two rooms is not a crossing: a pair split
by a player who crosses rooms is two lone portals. ⚠ Not changed (P5
presentation): the portal visuals, view cones, far-side panes and the host
camera continuity still read every placed portal.

✅ **Cut 7m, first part, landed 2026-09-30: an NPC barks at the cadence of
its own live room.** The ambient bark ticker read the sole live room's spec
to ask if the room is a gallery. While two rooms were live it had no spec,
so it ran, but every NPC barked from its `Idle` pool at the idle cadence,
the pedestals of the Hall of Characters too. Each NPC now reads the spec of
its own live room (`LiveRoomSpecs`). Its bark clock is keyed by its live
room and then its id, because two instances of one room hold the same ids.
Witness: `a_gallery_pedestal_barks_at_its_own_rooms_cadence_beside_another_live_room`
(Bob in the hub, #0; Alice in the hall, #1: no hall bark in the first 24 s,
some by 60 s; with the sole-room rule restored, the hall barked 80 times in
the first 24 s).

✅ **Cut 7m, second part, landed 2026-09-30: the falling-sand room runs
beside another live room.** Ten falling-sand systems asked whether the sole
live room was the sand room. While two rooms were live, none of them ran:
no settled sand reached any collision overlay, and the room's swim loan
stayed on a player who had left. They now read `LiveSandRoom`, the live
room that instantiates the sand room. The settled sand and the particle
projection write that room's overlay, and the swim loan goes to each
player whose own live room it is. Witness (feature `falling_sand`):
`the_falling_sand_room_runs_beside_another_live_room` (Bob in the sand
room, #0; Alice in the hub, #1; the sand spout opens in #0: the settled
sand reaches #0's overlay and Alice has no swim loan). Poison: with
`LiveSandRoom` answering only for one live room, no sand reached the
overlay. With the swim loan's sole-room parameter restored, Alice kept the
loan. ⚠ Not changed: the sand world (grid, ledger, particles) is one set of
resources, so two live instances of the sand room would share it. That
does not happen today, because a player who comes back joins the instance
held by another player (6e). If it does happen, the lowest instance has the
sand.

✅ **Cut 7n landed 2026-10-01: a boss fights in its own live room beside
another live room.** Two readers stopped every boss while two rooms were
live. The boss brain tick and the boss body step each read the walls of the
sole live room through the unkeyed `CollisionWorld::solids()`. With two
rooms live that is `None`, and each system returned before its loop. Thus
no boss in either room chose an attack or moved: the 7j witnesses saw the
conductors measure their halls, but the bosses they conduct did nothing.
Each boss now reads `collision.room(its InRoomInstance)`, as the actor step
does. The two boss specials that size their volley by the room, GNU-ton's
apple rain and the overflow flood, read the width of their boss's own live
room (`LiveRoomOf<RoomGeometry>`) and not the sole live room's. Witnesses
(Bob in the Hall of Bosses, #0; Alice in the arena, #1):
`the_overflow_boss_swoops_in_its_own_live_room` (the boss moves more than
10 px in 120 ticks), `gnu_ton_s_apple_rain_falls_in_its_own_live_room` (the
apples are in #1) and `the_overflow_flood_fills_its_own_live_room` (the
boss at half health, so in phase 2: the flood columns, 24 by 28 with no
visual, are in #1). Poisons, each one arm: with the body step on the sole
room, only the swoop witness failed. With the brain tick on the sole room,
all three failed. With a special's sole-room parameter restored, only its
own witness failed. ⚠ The first flood witness matched any shot with no
visual, and the overfit volley that comes before the flood in phase 1 has
no visual too. It passed with the flood spawner poisoned, because the boss
never reached phase 2 and the shots it found were the volley's. ⚠ The
other unkeyed `CollisionWorld` readers (pogo, the damage safe point, the
held items, the traversal abilities, the tether, the portal host, the
trace) still answered only while one room was live. Cut 7o keys the
traversal abilities; cut 7p keyed the held items, pogo, the damage safe
point and the tether; the trace cut keyed the trace and the blink
reticle.

✅ **Cut 7o landed 2026-10-01: a body's traversal reads the walls of its
own live room.** Blink, dive, grapple, the authored teleport, the trapdoor
and the body-mode driver read the walls through the unkeyed
`CollisionWorld::solids()`. With two rooms live that is `None`. Blink,
dive and teleport then went the full distance through any wall, the
grapple fizzled, the trapdoor left her under the boards, and the body-mode
driver returned before any body, so nobody could crouch. Each now reads
the room of its body (`InRoomInstance`). The systems that share one
composed world across bodies (teleport, trapdoor, body mode, and the actor
step, whose private `composed_room` is deleted) use one cache,
`ComposedRooms`: it composes each live room once per run, on the first ask.
`spawn_live_room` (shared_tangle) spawns a second live room root for
focused tests. Witnesses, each with #0 empty and the wall in #1 where the
body is: `a_blink_stops_at_a_wall_of_its_own_live_room`,
`a_dive_stops_at_a_wall_of_its_own_live_room`,
`a_grapple_catches_a_wall_of_its_own_live_room`,
`a_teleport_stops_under_a_ceiling_of_its_own_live_room`,
`she_surfaces_through_the_floor_of_her_own_live_room` and
`a_body_crouches_in_its_own_live_room_beside_another`. And the app
witness, also `a_blink_stops_at_a_wall_of_its_own_live_room`, in
`two_players_two_live_rooms.rs`: Bob in `blink_run` (#0), and Alice in
`portal_bridge` (#1) with its blink, blinks at a wall of #1. Poisons (each
reader back on the sole room): every witness failed at its own assertion.
The app witness failed with Alice 149 px on, through the wall.

✅ **Cut 7p landed 2026-10-01: held items, the pogo, the damage step and
the smash tether read the walls of their subject's own live room.** Each
read the sole live room through the unkeyed `CollisionWorld` and returned
while two rooms were live. Thus no item fell or rode its platform, no
down-air bounced off an orb, no player took a hit or had a safe point
remembered, and no tether bit. Each now reads
`collision.room(LiveRooms::of(subject))`: the item, the striker, each
player, and the fighter. The item physics builds each room's composed
solids once per tick (`SolidsByRoom`). A flying item now strikes only a
body of its own room: the strike read the bodies of every room.
Witnesses, each poisoned with the sole-room read restored (the failure is
in brackets): `an_item_falls_onto_the_floor_of_its_own_live_room` (it
stayed at y=200), `a_settled_item_rides_the_platform_of_its_own_live_room`
(x stayed at 200), `a_flying_item_strikes_only_a_body_of_its_own_live_room`
(with the victim room filter removed, the body in #0 was struck),
`a_down_air_bounces_off_an_orb_of_its_own_live_room` (the striker in #1 did
not bounce), `a_tether_bites_a_ledge_of_its_own_live_room` (the throw:
no bite in #1; the reel: released on tick 5, not on the one-room tick),
and `a_safe_point_is_remembered_in_the_players_own_live_room` (Alice in
the hub, #1, beside Bob's `switch_lab`, #0: her safe point stayed 131 px
behind her). ⚠ The first draft of the last witness used Bob with no slot.
That is the one-room control, because Bob's room retires when Alice
crosses, so it passed under the poison. It now asserts that two rooms are
live.

✅ **Landed 2026-10-01: the dev traces and the blink reticle read their
subject's own live room.** The player trace, the actor OOB trace and the
blink reticle read the sole live room, so while two rooms were live no
trace row was recorded and no reticle showed. The player trace reads its
player's room (`LiveRoomOf`, `LiveRoomSpecs`). The actor OOB trace judges
each body against its own room's world: a frame holds each live room a body
was in (`RoomTraceSnapshot`: area, envelope, solids), and each body names
its room. The reticle reads the controlled subject's room. Witnesses:
`the_traces_record_each_body_in_its_own_live_room` (Alice in the hub, #1,
beside Bob's `switch_lab`, #0: the player trace keeps recording in the hub;
the actor frame holds both rooms and tags Alice with the hub; poisoned
alone, the player trace recorded no row, and the actor frame held no room)
and `the_blink_reticle_reads_the_walls_of_its_subjects_own_room` (#0
walled, #1 open; poisoned, the reticle was inactive). `SoleLiveRoom`
69/49 -> 68/48, `SoleLiveRoomSpec` 39/31 -> 37/29.

✅ **OW1 Cut A landed 2026-10-01: a replay, a checkpoint reset and a level
departure serve the live room of the player they move.** Each read the
sole live room, so while two rooms were live each one stopped. The replay
admission drained the request and lost it, so no death replayed its room,
and the return to spawn read the sole room's geometry. The checkpoint
resume kept the reset owed and never served it. The departure driver
returned before any departure, so no level could end. Each now keys on its
subject's `LiveBodyId.room` (`LiveRoomSpecs::definition_named`, and
`LiveRoomOf<RoomGeometry>` for the return to spawn). A subject with no
stamp is in the sole live room, as before. Witnesses in
`two_players_two_live_rooms.rs` (Alice in the hub, #1, beside Bob's
`switch_lab`, #0; for the first two, Alice is hurt and 60 px or more from
the hub spawn): `a_replay_beside_another_live_room_replays_the_players_own_room`,
`a_checkpoint_reset_beside_another_live_room_is_served_in_the_players_own_room`,
`a_level_that_ends_beside_another_live_room_sends_its_player_on` (Alice
joins #0 and the hub retires) and
`a_replay_of_the_cut_rope_arena_beside_another_live_room_hangs_the_next_heavy_object`
(Alice in the arena beside Bob's Hall of Bosses: the cut-rope arena reset
also reads the replay's subject's room). Poisons, each one parameter back
on the sole room, each failure predicted before the run: the replay admission
(Alice stayed at (1447, 1928), not at the hub spawn (950, 883)); the
return to spawn (she was at the spawn by the transition's arrival, but her
health stayed 1 of 60); the checkpoint resume (the session was still owed
`LastCheckpoint`); the departure (two rooms stayed live, and the hub did
not retire); the arena reset (the heavy object cycle stayed at 0).
`SoleLiveRoom` 68/48 -> 67/47, `SoleLiveRoomSpec` 37/29 -> 33/27. ⚠ Not
changed: the checkpoint's verification after the commit
(`verify_restored_domains`) still reads the sole live room for its room
check and its presence check. With two rooms live, both checks are weaker,
but they do not fail a correct restore. No witness can make them fail, so
they wait for a cut that needs them.

✅ **OW1 Cut C, two readers, landed 2026-10-01: a room-entry cutscene and
a map visit are recorded for a room that becomes live beside another.**
The cutscene trigger (`auto_trigger_room_cutscenes`) and the map's visit
tracker (`track_room_visits`) read the sole live room, so while two rooms
were live neither ran: a room Alice entered beside Bob's room queued no
cutscene and was not marked on the map. The trigger now remembers the
sorted ids of every live room (`LastCutsceneRoom` is a list, as
`LastQuestRoom` became at cut 7k) and queues the cutscenes of each id that
becomes live, in id order. A second live room of an id already live
queues nothing. The visit tracker flags each live room, because a room is
live while a player is in it. Schema 286 -> 287. Witnesses in
`two_players_two_live_rooms.rs`, each with a one-room control arm (Bob
not driven): `a_room_cutscene_plays_when_its_room_becomes_live_beside_another`
(a test cutscene bound to the hub) and
`the_map_records_a_room_visited_beside_another_live_room`. Poisons, each
reader back on `SoleLiveRoomSpec`, each failure predicted before the run:
each witness failed in its two-room arm, and its control arm passed.
`SoleLiveRoomSpec` 33/27 -> 31/25.

✅ **Cut 7q landed 2026-10-01: each live room keeps its own gated lock
walls, and the gnu's back is ground in the giant's own room.** Two overlay
contributors wrote to the sole live room. With two rooms live, both wrote
nothing. The gated lock walls cached one room's walls (the room the set
named while one room was live) and pushed them to `RoomOverlays::sole()`,
so no room had a gated wall and anyone could walk through a locked door.
The gnu's back platform went to the same sole overlay, so nobody could
stand on the giant. The cache (`GatedLockWallCache`) now holds walls by
room id. It refreshes only when the room set or the catalog changes, a
cached room is gone, or a live room is not cached. Each tick, each live
room root gets the standing walls of its own room in its
`FeatureEcsWorldOverlay`. The gnu's back goes to the overlay of the
giant's own room (`RoomOverlays::for_room`). Witnesses, each poisoned with
the sole-room write restored (the failure is in brackets):
`each_live_room_keeps_its_own_gated_walls` (`drain_alley` #0 with no
walls, `alice_relay` #1 with one gated wall; walls per room (0, 1), then
(0, 0) after the flag is set; poisoned (0, 0) at the first assertion) and
`the_gnu_s_back_is_ground_in_its_own_live_room` (Bob in
`hall_of_bosses` #0, Alice in `gnu_ton_arena` #1; poisoned, no room held
the back). The census markers do not change: neither reader was a
`SoleLiveRoom` marker.

✅ **Landed 2026-10-01: each player's trail reads the walls of the
player's own live room.** `update_player_trail` read the sole live room,
so while two rooms were live no trail saw a wall, and a loop drawn around
a wall was erased as empty. It now reads each player's room
(`LiveRoomOf<RoomGeometry>`). `render_player_trail` is a view and is not
changed (P5). Witness: `a_trail_keeps_a_loop_around_a_wall_of_its_own_live_room`
(#0 open, #1 walled; the loop is kept in #1 and erased in #0, the
control). Poison (the sole-room read restored): the #1 arm failed, and the
loop was erased. `SoleLiveRoom` 67/47 -> 66/47.

✅ **Landed 2026-10-01 (customer 2): a crossing resets only what it leaves
behind.** The crossing's commit (`apply_crossing`) despawned every live
projectile, put the ambient gravity back to its default and asked for the
sim clock to be reset, for any player's crossing. With Bob's room live,
Alice's door deleted Bob's shots, unflipped the gravity of his room and
cancelled his bullet time or hitstop. The staged crossing now records what
it leaves standing (`CrossingScope`: the room it leaves, whether that room
retires, whether another live room stays). With no other live room
standing, the crossing replaces the world, and all three are reset as
before. While another live room stays, only the shots stamped into the
room left go, and only when that room retires. The developer preset flash
is a view and is not changed.

⭐ **Decision: the sim clock and the ambient gravity are ONE fact each for
the whole world.** Every live room steps on the one `ClockState` (the
time-control code says the same: per-player clocks wait for the
multiplayer regimes), and `BaseGravity` is one resource that a gravity
switch in any room flips for every room. So a crossing does not reset
them while another live room stays: a reset there would be one player
changing the other's world. A clock or an ambient gravity per live room is
a later design decision, not this cut.

Witness: `a_crossing_resets_only_what_it_leaves_behind` in
`two_players_two_live_rooms.rs`. A shot is stamped into `switch_lab` and
gravity is flipped before Alice leaves. Each tick of the door walk is read
for the clock reset request. With Bob driven (two rooms): the shot stays,
no reset is asked for, gravity stays flipped. The control, Bob not driven
(one room): all three are reset. Poisons, each one condition forced open,
each failure predicted before the run: every shot despawned (the shot was
gone), the clock reset always asked for (it was asked for), and gravity
always reset (it was put down). ⚠ The first draft planted a half-speed clock
target and read it after the crossing. The plant was overwritten before
the commit, because a crossing's game-mode change (playing ->
room-transition -> playing) freezes the clock and then asks for the
default speed. So that arm read 1.0 with the gate working. The witness
now reads the request.

⚠ Found, not changed here: that game-mode change is session-wide, so every
live room freezes for the frames of one player's crossing. It is OW4's own
evidence ("supported absence does not freeze unrelated work"), and the
OW4 work takes it.

✅ Same day, the replay too: a replay (reachable with two rooms since Cut A)
reset the shared clock (`reset_sandbox`) and the ambient gravity
(`reset_gravity_on_room_reset`) for every live room. Both now keep them
while another live room stays (more than one live room root), by the
decision above. Witness:
`a_replay_keeps_the_worlds_clock_and_gravity_while_another_room_is_live`
(Alice replays the hub with gravity flipped: with Bob driven, no clock
reset is asked for and gravity stays flipped; the control, Bob not driven,
both are reset). Poisons, each gate forced open: the clock reset was asked
for; gravity was put back down.
The hazard respawn (`safe_respawn_player`, a pit or a spike that sends a
player back to the safe point) asked for the same reset for every live
room, and now asks only while its room is the one live room. Witness:
`a_hazard_respawn_keeps_the_worlds_clock_while_another_room_is_live` (a
`SafeRespawn` hazard hit on Alice: she respawns in both arms, and the reset
is asked for only in the one-room control). Poison (the writer always
passed): the reset was asked for with two rooms.

✅ **Cut 7r landed 2026-10-01: a mode lives while any live room is in its
scope, and its owner follows a room of its own mode.** The mode sweep
(`despawn_departed_mode_entities`) and the owner follow
(`follow_mode_owner_rooms`) read the sole live room. With two rooms live,
the sweep swept nothing and the follow did not move. Thus a mode that no
player was in kept its entities, and an owner born on the tick a second
room opened kept its `First` arrival: Sanic's act and Mary-O's lap start
over on each arrival, so they started over on every tick. The sweep now
asks whether any live room is in the mode's scope, the same question
`CurrentRoom::in_scope` asks for the mode's systems. It runs when the room
set is replaced, a live room gets another definition, or a live room
retires; a retirement that did not change the room set did not wake it
before. The owner stays in its room while that room is live and in its
scope, else it goes to the first such room in instance order; while none
is, its visit does not change and the sweep retires it. One owner per mode
still follows one room: an owner per (mode, live room) is the later cut
that the mode-owner decision above names. Witnesses in
`ambition_platformer2d_runtime/tests/mode_scope.rs`:
`a_mode_owner_follows_its_own_room_beside_another_live_room` and
`a_mode_ends_when_no_live_room_is_in_it`. Poisons: with both systems back
on the sole room, the owner stayed at `(a, First)` beside an untagged room,
and a mode that no live room was in (`mary_o`, beside `a` and `b`) kept its
entity. With only the retirement trigger removed, mode `a` kept its entity
after Bob's room retired. `project_room_rule` (the portal camera rules)
stays on the sole room: it is presentation, P5.

✅ **OW3, first slice, landed 2026-10-01: a runtime mint left in a room
that is not live is still there when the room is live again.** A runtime
mint (a boss's dropped gauntlet) has no authored record. Two facts rebuild
it: the occurrence ledger's `Placed` row (where) and a minted description
(what). Before this slice, three defects lost the description of a mint
that lay in a retired room:

1. The save mirror (`persist_minted_item_horizon_to_save`) described only
   live mints, so the row left the save the tick its room retired.
2. The door build read the description from the checkpoint baseline
   (`MintedItemBaseline`), which a checkpoint replaces with the live mints.
   A shrine rest in another room forgot the gauntlet, and a mint that no
   checkpoint saw was never described.
3. A load's rebuild refused the whole room: the plan requires a dynamic
   row's parent to be planned or live, and the boss that dropped the
   gauntlet is in no room of a fresh process.

The one record of a dormant mint's description is now the save's minted
rows. The mirror keeps an earlier row while the ledger places the
occurrence (`with_dormant_mints`) and drops it when the ledger does not.
The door build reads those rows. A checkpoint restore still reads the
checkpoint's baseline, and a load adopts the file's rows. The plan does not
ask a reinstated row for its parent
(`ConstructionPlan::prepare_reinstating`, with the outlook's
reinstatements); a summon still needs its summoner. Witnesses in
`a_save_remembers_where_you_left_things.rs` (the gauntlet is put down in
the hub and the player walks to the shaft):
`a_gauntlet_left_in_a_room_outlives_a_checkpoint_taken_in_another`,
`a_gauntlet_no_checkpoint_saw_is_still_where_it_was_left` and
`a_gauntlet_left_in_a_room_outlives_a_save_taken_in_another` (the file
holds the row; a fresh process loads it and the gauntlet lies where it
fell). Unit witness: `a_reinstated_row_does_not_need_its_parent`. Poisons:
with the door on the checkpoint baseline, the checkpoint and no-checkpoint
witnesses failed; with the parent rule on every row, the load witness
failed after the file had the row; with the mirror on live mints only, all
three failed and so did `a_gauntlet_left_in_a_room_is_rebuilt_when_the_room_is`.
⚠ The door witnesses cannot see the parent rule: the harness stages its
boss as room content, so the rebuild plans the boss again.

✅ **OW3, second slice, landed 2026-10-01: a runtime mint that nobody
carried is still where it fell.** The ledger admitted an occurrence only
through custody ("an object nobody ever carried has no relocation to
remember"). That is true for an authored object, whose record rebuilds it.
It is false for a runtime mint, which no record describes: a boss's
gauntlet left on the floor had no row and was gone when its room was live
again. A runtime mint (`SpawnOrigin::Dynamic`) now enters the ledger where
it lies, in the tick it appears (`AuthoredOccurrences::admit_mints`, fed by
`record_placed_ground_items`). An id that already has a row keeps it.
Witness: `a_gauntlet_nobody_carried_is_still_where_it_fell` (the boss dies
in the hub, the gauntlet stays on the floor, the player walks to the shaft
and back). Unit witness: `a_mint_enters_where_it_lies_and_a_known_id_keeps_its_row`.
Poison (nothing admitted): the witness failed, and the hub was rebuilt
without the gauntlet.

✅ **OW3 / FI9 landed 2026-10-01: dormant records add no all-world walk to
an idle tick.** Three systems walked every ledger row or every saved
minted row on each tick, so the cost of a tick grew with the dormant
records of rooms that are not live. Measured first (dev profile, one
system at a time, 10,000 dormant mints in a room that is not live, against
none): the custody projection +70 µs a tick (`in_custody()` built a set
over every row), the save's occurrence mirror +6.1 ms (it built every
`PersistedOccurrence` to compare it with the save), and the save's minted
mirror +24 ms (it converted every minted row of the save). All three
together: 0.14 ms a tick with none, 0.63 ms with 1,000, 4.7 ms with
10,000. The cut:

1. The ledger keeps a custody index (the `InCustody` ids), kept by each
   mutator and built again by `adopt_rows`. A clone, which is the rollback
   snapshot, carries it with its rows. The custody producer compares and
   republishes the carried set through the index.
2. Each save mirror walks the dormant rows only when an input changed
   since it last ran: the restore latch, the ledger, the save, or its live
   inputs (the restorable set; the live mint descriptions). The live
   inputs are read from live bodies each tick and kept in a `Local`, a
   cache and not state. A rollback restore marks the ledger and the save
   changed, so the rows are mirrored again after it. ⚠ Under a sync test,
   which restores every frame, the gate opens every frame: correct, and
   no faster.

After the cut, 10,000 dormant mints cost an idle tick what none cost (each
system and all three: within noise). Witnesses:
`dormant_rows_add_no_walk_to_an_idle_tick` (an idle tick with 10,000
dormant rows takes less than three times the tick with none, plus
100 µs), `a_changed_dormant_row_still_reaches_the_save`,
`a_replaced_save_is_mirrored_again`,
`a_dormant_mint_taken_up_leaves_the_minted_rows` and
`the_custody_index_is_the_custody_rows_after_every_mutation`. Poisons,
each failure predicted before the run, and each failed only its own
witness: the index not kept by a put-down (the index test, "after one put
down"); both gates forced open (the idle tick took 18.6 ms against
0.13 ms); the save mirror's gate deaf to the ledger (the moved row did not
reach the save); both gates deaf to the save (the replaced save stayed
empty); the minted gate deaf to the ledger (the taken-up mint stayed in the
minted rows). The two `Local`s are adjudicated as a seventh mechanism, A
CHANGE GATE, in `check_sim_schedule_memory_is_adjudicated.py`. ⚠ Not changed: a rollback
snapshot still clones the whole save and the whole ledger, and the peer
checksum still folds every ledger row. Those are costs of the rollback
frame, which M2 measures, not of the active step.

⚠ **`physics_spawn_debris_messages` is presentation, not simulation, and is
not changed.** Its Avian debris bounces off static colliders that are
built with the room visuals, and both are placed through `world_to_bevy`
with one room's geometry. The room visuals are built for the sole live
room (P5's view work), so with two rooms live the second room has no
colliders and no visuals. A room-aware debris system alone would throw
debris into a room with nothing to land on. It moves with the room visuals
to P5.

⚠ Still owed from cut 5: `outlook_for(room: &str)` is keyed by definition;
the `SoleLiveRoomSpec` readers each need a subject before a second live room
is simulated (cut 6 finds which ones a two-player world reaches).

⚠ Risks carried forward: every cut that changes a snapshot value bumps the
schema; instance roots must be re-creatable by a rewind across a publication;
`outlook_for(room: &str)` is keyed by definition and will merge two instances;
and the session-wide rebase on a transition would reset Bob's history when
Alice crosses, which OW1 surfaces but does not solve.

⚠ Owed when multi-room hot reload is real (review of cut 5e): a
`LiveRoomDefinition` is an index into the CURRENT `RoomSet`, not a
generation-stable definition identity. Today hot reload is gated to one live
room (`SoleLiveRoomSpec`), and the replaced room is seated from the new set,
so no survivor holds an old index. When that gate goes, replacing the set
while another root survives would give that root's generation-N population a
generation-N+1 definition under the same index. The cut that removes the
gate must choose: re-prepare every resident root atomically with the set, or
make the root's definition reference name its prepared generation. An old
index must not silently take a new generation's meaning.

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
