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
chooses. Roam uses it to keep near the player. A hostile brain does not use it
yet.

An author's page is [`docs/systems/npc-navigation.md`](../../systems/npc-navigation.md).

Build cost, measured by YardratAmbition 2026-10-09 on 76 shipped rooms and two
bodies, before any cut: about 0.25 to 0.3 ms for each rollout; 25 builds are
over 20 ms; the worst is `hall_of_characters` at 185 ms. A graph is built on
the first tick a navigating body is in the room. OPEN: cut the rollouts that
fall to the step cap, then decide if the build moves to room load.

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

Guards: `navigation::envelope::tests::a_gap_inside_the_envelope_is_crossed_and_one_outside_is_not`
(the envelope against the kernel) and
`navigation::graph::tests::a_body_that_follows_the_advice_arrives` (a body that
follows the advice arrives at each reachable surface, and a surface out of
reach is said to be unreachable).

Not modelled, each a seam:

- a drop through a one-way surface, an air jump, a dash, a wall verb, flight;
- a slope or a surface chain, a surface that moves;
- geometry that is not authored in the room (a gate, a breakable). A leg such a
  thing stops fails, and the brain plans again;
- a hazard in the air of a leg;
- a route to another room (the door graph below is a different graph);
- a gravity frame that is not axis-aligned;
- `WhyNot`: an unreachable goal is `NavNext::Unreachable` with no reason.

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
