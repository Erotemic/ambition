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

## The room graph is navigable by its doors

`scripts/check_world_graph_is_navigable.py` checks the shipped worlds: every
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

The fighter `recovery_below` experiment does **not** validate the rollout/recovery
integration.

With the shipped rollout enabled, level 6 fails the controlled fixture 45/45;
with rollout disabled it succeeds 45/45. `RecoveryLens` did not change that
outcome. The current next step is a fighter decision trace, owned by
[`fighter-brain.md`](fighter-brain.md).

Do not respond by redesigning generic navigation or by adding a Smash-specific
"committed fall means dead" heuristic. A body may still recover through drift,
jumps, flight, walls, ledges, recovery moves, impulses, portals or grapples.

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
gate solid, `world_with_gate_solids_and_carves`), with the other overlap exits
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
- A body moves from one live room to another only as a crossing's subject (a
  driven body) or in its custody. The player road
  (`RoomTransitionIntent` and the room transaction) is not for a body that no
  slot drives: with no participant, `another_player_stays` is false and the
  crossing would retire the room the body leaves, and the road runs the room
  load. The second-seat return (`sandbox_reset.rs`) moves a body between two
  live rooms directly: it stamps the body's custody closure with the
  destination's `InRoomInstance` and calls `transit_body`.
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
