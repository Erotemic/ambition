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

Substrate locations: `RecoveryLens`
(`crates/ambition_combat/src/brain/fighter/recovery.rs`); support queries
`is_support_surface`, `support_face_separation` and `body_on_support_side`
(`crates/ambition_platformer2d_core/src/collision_semantics.rs`); and the
`CollisionWorld` questions `solids`, `carves_only`, `hostable_surfaces` and
`base` (`crates/ambition_platformer2d_world/src/collision.rs`).

**No navigation exists** (checked 2026-09-05): no reachability type, nav graph
or pathfinding in `crates/` or `game/`. Search hits for `navigation` are menu
navigation, and `a_star` and `reachability` hits are substrings. This page is
also the one missing foundation for
[`agentic-character-runtime.md`](agentic-character-runtime.md).

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

## ✔ THE ROOM GRAPH IS NAVIGABLE, measured 2026-09-05 — a POSITIVE verdict

⭐ **Written down because an unmeasured row and a satisfied one look identical in
a summary.** Across the four shipped worlds:

```text
areas (activeArea, falling back to the level id)   72
directed room edges                               150
LoadingZones with a target                        151
   of those, authoring `bidirectional`            122
doors targeting a room that is not an area          0
areas you can ENTER but not LEAVE                   0
```

⇒ Every authored door leads to a real area and nothing is a one-way trap. Guarded
by `scripts/check_world_graph_is_navigable.py`, because the engine's own response
to a dangling target is `eprintln!("room graph warning: unknown target room …")`
— a build-time warning nobody reads, on a door that then silently does nothing.

⛔⛔ **AND `RoomLink.bidirectional` IS NOT DORMANT — the dormant-mode census could
not read LDtk.** 122 of 151 zones set it. The census matcher is Rust/RON syntax
(`field: true`) and LDtk is JSON with the name and value in DIFFERENT KEYS, so
`sandbox.ldtk` mentions `bidirectional` 124 times and matched ZERO times. Fixed
with a structured `fieldInstances` parse. ⇒ **A corpus that is present but
unreadable by the matcher is worse than an absent one: it looks like coverage.**

⚠ **THE KEY IS `activeArea`, camelCase, and getting it wrong is invisible.**
Keying on the level identifier invents areas and reports their cross-area doors
as dangling; keying on `active_area` matches nothing and falls back to
identifiers everywhere, which looks the same as working. I produced BOTH false
findings before the script existed, which is why it verifies the key against
`LdtkLevel::raw_active_area` rather than commenting it.

⚠ This gate reads the map SUBMODULE through symlinks, so it exits 3 and SKIPS
when the submodule is absent, and **cannot be evidence between two machines** —
two boxes at the same commit can hold different worlds (#62).

### ⭐⭐ THE VERDICT'S PREDICATE IS "A DOOR IS A `LoadingZone`", AND IT IS NOW ASSERTED

**MEASURED 2026-09-05.** The claim above is exactly as large as what it counted,
and what it counted is loading zones. A PORTAL is also a way between two places
and this check does not model one, so the honest form of the verdict is *"no area
is a trap **by the doors**"*.

That bound is complete today, and the number says why rather than asserting it:

```text
authored Portal entities                            14
portal groups (keyed by the `link` field)            7
   of those, groups spanning MORE THAN ONE area      0
   of those, falling back to `iid` (ungrouped)       0
```

⇒ Every group is a real PAIR — none is a singleton that could never span — so all
seven are genuine candidates and none crosses an area. The predicate has no blind
spot in the shipped content.

⛔⛔ **But that is a fact with an expiry date, so the script FAILS on it rather
than commenting it.** A cross-area portal breaks the verdict in BOTH directions at
once: an area whose only exit is that portal reads as a trap it is not, and a
connection the player can really use is missing from the graph. The new arm names
what to do (teach the check about portal pairs) instead of just going red, and it
ships with the control arm — without one, the guard would fire on every world that
authors a portal at all, and all four do.

⭐ The general shape, which cost two false findings on this page already: **audit
the PREDICATE, not only the corpus.** `bidirectional` above was a matcher that
could not read its corpus; this is a corpus the matcher never asked for.

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
   ⭐⭐ **THIS CUSTOMER ACQUIRED ITS EVIDENCE 2026-09-04, and the interesting
   part is that the ANSWER exists while the QUESTION has no asker.** A route can
   now be closed by a body capability — `body.can(verb)` and `body.fits(height)`
   are published conditions and `gated_by` is an authored condition line — and
   when a wall stands, the domain that refused it states why:
   `GatedLockWallVerdicts::why_standing(wall)` returns the structured
   `WhyNot { term, subject, observed }`, derived and keyed by wall id.
   ⛔ **And nothing in production reads it.** Measured the same day: that
   resource has no production reader, and `AgentObservation`
   (`ambition_sim_harness/src/observation.rs`) carries body state only —
   position, velocity, ability charges, health — with no world-gate field at
   all. ⇒ An agent driving the harness cannot learn that a wall is standing, let
   alone why, which is exactly the product criterion
   [`../game/open-world-roadmap.md`](../game/open-world-roadmap.md) still marks
   `▢`: *"navigate enough of the world that AI and agent tooling can reason
   about routes."*
   ⇒ **So the navigation slice this page is waiting for is smaller than a
   planner:** the reachability facts a tool needs are already computed and
   already structured; what is missing is a surface. ⚠ **NOT built here, and
   deliberately** — which surface is an open design question on
   [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md)
   (*"In-process query API versus trace/report artifacts?"*), and adding a field
   to `AgentObservation` with no consumer would be the dormant-cluster growth
   this project refuses. What is recorded is that the customer is now REAL and
   the input already exists, so whoever answers that design question can cut
   this without re-deriving any of it.
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
body control and the existing motion kernel execute it. The
[responsibility map](architecture-responsibility-map.md) does not move actor live
mutation into a path service merely because AI and player motion share geometry.

A8's instance-isolation witness must include navigation/obstacle queries when that
capability participates. A9's minimal profiles should not require navigation to
step an otherwise self-contained body. Preserve deterministic motion-policy and
shape assumptions in reachability tests; a new spatial index requires measured
cost rather than a decomposition target.
