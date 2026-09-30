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

Do not require one particular map/chunk/COW implementation before measurement.
Do require FI9 to show that adding dormant records does not add an all-world walk
to an unrelated active step. M2 measures actual retention, promotion, restore and
resimulation costs, including sparse/dense changes and candidate peak memory.

## Implementation cuts

These cuts refine A8 and existing owner work. They are not another global queue.

| Cut | Work | Required evidence |
| --- | --- | --- |
| OW1 | Two instances of one room; audit selection/identity/query/teardown paths | Same local IDs, separate contacts/observations, no cross-despawn; one-instance profile remains one path. ⭐ **A LIVE ROOM HAS AN IDENTITY AS OF 2026-09-20**: `LiveRoomInstance` (`crates/ambition_platformer2d_world/src/rooms/instance.rs`), an ordinal of this session's room publications, minted by `apply_world_replacement` — the one road that seats a session in a published room — and rollback state (`root.live_room_instance`, schema v202). Witnessed on the shipped Mary-O lap: 1-1 → 1-2 → 1-3 → 1-1 returns to index 0 and reaches instance `#3`, so the room she comes back to is not the room she left. ⚠ It lives on the SESSION ROOT because that is where the one live room lives; two simultaneous instances move the carrier, not the ordinal. ⚠ And residency is still UNKEYED — `RoomScopedEntity` says an occurrence dies with *a* room, never with *which* — so the teardown sweep is the next thing OW1 has to key. ⭐ **OW1 HAS AN INSTRUMENT AS OF 2026-09-20**: `[census] rooms` prints every session root's `active` INDEX beside its authored id, plus the live crossing, so the moment an index stops identifying one live instance is visible rather than inferred. It is derived and read-only; it owns nothing. |
| OW2 | Accepted body/custody transfer and prepare/publish between instances | Refused transfer retains state; successful transfer preserves identity and exactly one writer |
| OW3 | Dormant durable records and active-state handoff | Save/load and promotion preserve occurrences; active step excludes unrelated dormant records |
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
still in the room it leaves (`another_player_stays`). Only a driven
subject opens a room: a body no slot drives (the d71 crossing body) is
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
- Not measured: whether the session-wide rebase at Alice's crossing resets
  Bob's history, and what `GoverningRules` and the mode scope answer for
  the room Bob is in.

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
