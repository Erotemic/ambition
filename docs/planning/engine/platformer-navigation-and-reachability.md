# Platformer navigation and reachability

**State:** OPEN strategic capability; advance from concrete movement/AI/world
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

> **RE-MEASURED against `5fdf977db` (2026-09-03): the substrate list is accurate, and
> the layer above it is still empty.**
>
> Spot-checked the two most specific entries and one the kinematic campaign
> added since: `RecoveryLens`
> (`crates/ambition_combat/src/brain/fighter/recovery.rs:79`) exists and is
> consumed by the fighter rollout; support/floor queries are
> `is_support_surface`, `support_face_separation` and `body_on_support_side`
> (`crates/ambition_platformer2d_core/src/collision_semantics.rs:82`, `:130`,
> `:136`); and `CollisionWorld` answers the four questions the game asks —
> `solids`, `carves_only`, `hostable_surfaces`, `base`
> (`crates/ambition_platformer2d_world/src/collision.rs:96`, `:126`, `:148`,
> `:159`).
>
> ⛔ **AND THERE IS STILL NO NAVIGATION.** No reachability type, no nav graph, no
> pathfinding of any kind anywhere in `crates/`. So the split this section draws
> — substrate present, policy unproven — is now sharper than "unproven": there is
> no navigation consumer to prove or disprove.
>
> ⚠ **Which makes this page the single remaining gate on another program.**
> [`agentic-character-runtime.md`](agentic-character-runtime.md) says to wait for
> actor/navigation/world-fact foundations; world facts and observations/memory
> both exist now, so navigation is the only one of its three still missing. That
> is worth knowing before this page is deprioritised again: it is not only its
> own capability, it is somebody else's blocker.

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
4. authoring/inspection needs to explain why a route is unreachable;
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
