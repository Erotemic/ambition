# Kinematic world objects

**2026-10-26 distinction:** A persistent placeable/carryable world object requires stable
identity, custody, room location, save disposition and reconstitution *regardless of whether* it
uses static placement, simple deterministic motion or optional Avian. This resting kinematic
work does not prove a generic movable-prop capability has shipped. See [ADR
0038](../../adr/0038-persistent-objects-and-optional-physics.md) and [architecture
reassessment](architecture-reassessment-and-composable-multiplayer.md).
**State:** K1-K4 and K6 retain their closed receipts. K5's platform path
reference stays customer-gated. PATH1/PATH2 below are planned from static
source review at `00bf92c22142`, after planning commit `e389726ea716`
(2026-11-22). Existing moving hazards and authored actor routes provide
customers for the shared path law and its native consumers. No new build,
test, gameplay run or cost measurement is claimed.

⛔ Before any conveyor-like solid, split `Block::velocity` into displacement and
surface drag (see K6).

## Current shape

Moving platforms are ordinary deterministic world geometry.

- `MovingPlatformSpec` and `MovingPlatformState` live in
  `ambition_platformer2d_world::platforms`. The module doc states the contract:
  authoritative state is in the world crate, and the Bevy visual is a read-model
  projection.
- `MovingPlatformSet` is a component on each live room root
  (`RoomInstanceRoot`, rollback row `root.moving_platform_set`), not a process
  resource. `platforms::install_moving_platform_simulation` installs one world
  step for every live room. Runtime places it between body rescue and contact
  snapshots. Lifts use each grounded body's room, collision plane, and frame.
- Platforms expose `last_delta`, so riders and ledge contacts are carried
  without advancing the platform per actor.
- `CollisionWorld::room(Option<&InRoomInstance>)` composes one live room's
  geometry, platforms and collision overlay off one root, as `RoomCollision`
  (`solids`, `carves_only`, `hostable_surfaces`, `base`). It never falls back to
  another room. The unkeyed `CollisionWorld::solids()` and siblings read the
  sole live room and are the named one-room debt
  ([open-world residency](open-world-runtime-and-residency.md)).
- No consumer composes a collision world by hand. `world_with_moving_platforms`
  still returns an owned extended world for its remaining callers.
- Motion authoring is classified once: `AuthoredPlatformMotion::classify`
  produces a validated `MovingPlatformMotionSpec` and refuses conflicting
  fields by name (two motions, an anchor without a shaft, a zero shaft, a speed
  beside a path).
- Crush is reported, not resolved: `AxisConstraintConflict`
  (`ambition_platformer2d_core::collision_semantics`, documented in
  `movement/events.rs`); displacement or crush immunity is the owner's policy.
  One-way is `BlockKind::OneWay`. Rider carry is pinned by
  `a_wrapping_platform_carries_a_rider_by_its_travel_not_by_its_teleport`.
- Portals attach to identified moving platform faces.
- Visuals: `ambition_render::rendering::moving_platforms` reconciles visuals
  from `MovingPlatformSet` (spawn, retire, move). A pure reconcile has nothing to
  remember, so it cannot clobber a restored set.

## Motion model

The current platform owner has distinct motion policies:

- a simple ping-pong sweep;
- path traversal (`Once`, `Loop`, `PingPong`);
- discontinuous vertical and horizontal wrapping;
- a rider-triggered lift and a ferry;
- optional plank spin/swing composed with the selected translation.

A discontinuous wrap is not a continuously closed path. Keep the distinction.

The path law is separate from moving-solid contact, damage and actor
steering. It is not yet one shared evaluator: combat's `PathMotion` and
world's `advance_path_position` duplicate it. Shipped damage volumes use
inline paths. Two shipped actors name a path, but their native advance
discards its result; they are not proof of working route following.

## Target boundary

| Boundary | Owned facts |
| --- | --- |
| Authored moving solid | Spatial identity, shape/collision policy, motion spec and presentation reference |
| Preparation | Resolved references, admitted geometry and motion law |
| Native room simulation | Current continuation and proposed motion; accepted transform and continuation commit together |
| Native readers | Collision, carry and hosted portals read accepted geometry/motion; presentation reads the published result |

Current platforms commit their requested step directly. Proposal/admission
before a coupled host/aperture commit is the selected
[M2 follow-up](portals-are-moving-frames.md#driven-aperture-motion-is-admitted-before-the-host-commits),
not a claim that current platforms already use physics-limited integration.

Do not generalize every dynamic object into this concept. A platform belongs
here because its identity is world geometry whose transform follows a
deterministic motion driver.

## Open work

| Item | Work | Trigger |
| --- | --- | --- |
| PATH1 shared path law | Prepare geometry once and evaluate phase/intervals in core; migrate platform and hazard consumers | Current duplicate evaluators admit a cyclic nontermination case; see below |
| PATH2 actor route intent | Consume the authored route through ordinary control and accepted arrival | Two shipped `EnemySpawn.path_ref` customers do not currently steer from the route |
| K5 platform path relation | `MovingPlatform.path_id` is a string (`convert_moving_platform` reads `field_string(entity, "path_id")`; the entity contract has no `path_ref` for it). No shipped platform authors a path | Author a path-following platform, and migrate the field to an `EntityRef` in the same change. The tooling (`LdtkEntityCtx::kinematic_path_ref`, the path index) exists |
| `KinematicPath.points` | A semicolon-separated `x,y` string parsed by `parse_points`; `SurfaceChain` authors the same way | Has authored customers; do this first. See [`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md) |
| Belt / conveyor | Split `Block::velocity` (see K6) | A belt is authored |

## K6: the second-customer census

The original K6 census selected moving platforms and found no belt.
Current mount backs also publish typed ledge blocks with nonzero accepted
displacement. Lock walls, breakables and carves can change existence or kind
without a motion driver; moving hazards can follow a path without being
solid. The common fact is spatial motion, not one world-object lifecycle.
GNU's changing-length line is a separate
[SUP1 support customer](slopes-feet-ride-surfaces.md#sup1--keep-a-changing-supports-material-motion),
not surface drag and not evidence for a conveyor.

Rejected candidates: lock walls appear, they do not slide; no conveyor is
authored; the Smash stage is one static block.

**The falsifier.** `Block::velocity` means both the solid's per-frame
displacement and the drag it gives a rider. Only a belt (zero displacement,
non-zero drag) tells them apart. `ledge_grab::ledge_carry_for_frame` selects a
carrier by `velocity != ZERO` and recovers the previous pose as
`block.aabb.translated(-block.velocity)`, so a belt authored as
`Block { velocity: drag }` would be a ledge carrier with a previous pose it never
had. The day a belt is authored, split `velocity` into `displacement` and
`surface_drag` before any new `BlockKind` or authoring field.

`WorldDelta` does not exist in code (only the reserved `GeoSource::Delta`
variant). The runtime model is the immutable authored base plus per-frame
recomposition through the collision overlay. Do not plan against a delta-op road.

## PATH1 — Prepare one spatial path law

### Evidence and dependency direction

[`KinematicPath`](../../../crates/ambition_platformer2d_core/src/kinematic_path.rs)
lives in core. Its evaluator lives in
[`ambition_combat::path_motion`](../../../crates/ambition_combat/src/path_motion.rs),
while [platforms](../../../crates/ambition_platformer2d_world/src/platforms/mod.rs)
copy that evaluator as `advance_path_position`. Combat already depends on
world. Making world call combat would create a dependency cycle. Put the
shared geometry and traversal operations beside the existing core path
vocabulary. No new utility crate or generic collection API is needed.

Both copies admit a static counterexample: two coincident waypoints,
positive speed, and `Loop` or `PingPong`. Each zero-distance target changes
the cursor without consuming remaining distance. The guard detects one
unchanged cursor, not a cycle between cursors. The update therefore has no
finite result. LDtk and `KinematicPath::is_valid` admit the input. This was
not executed. The existing two-point loop regression uses distinct points
and does not prove this case.

Preparation also accepts `start_offset_seconds`, but neither evaluator
consumes it. No nonzero shipped offset was found in this audit. It is an
inert advertised parameter, not a measured level failure. Current moving
hazards in `noodly_belfry`, `basement_hazards` and Mary-O provide real path
customers; the platform path road exists but has no shipped placement yet.

### Shared geometry, different authority

| Consumer | Shared path fact | Native authority retained |
| --- | --- | --- |
| Prescribed moving hazard | Phase, position and requested travel interval | Hazard clock, enabled state and damage/contact policy |
| Path-driven platform | Proposed host motion and continuation at an accepted cut | Host admission, committed pose, carry, attached portals and refusal policy |
| Actor route follower | Waypoints, source segment identity and route target | Control intent, collision-limited movement and progress from actual arrival |

The core evaluator does not own an ECS tick, mechanism activation, control
or a global clock. Current platform and hazard callers pass room-scaled
time. Keep that choice in the callers. A lift deciding when to depart is
not the same operation as sampling a spatial path.

### Small native contract

1. **Prepare geometry.** Establish finite points, positive finite speed,
   nonnegative finite initial offset and positive finite length. Remove
   zero-length traversal legs while retaining source waypoint provenance;
   reject a moving path with zero total length.
   Compute ordered segment lengths and cumulative distance. Do not merge
   revisits to the same point: a self-crossing route has distinct ordered
   occurrences. `Loop` includes the closing last-to-first leg.
2. **Seed a new prescribed mover.** Select pose and continuation together from
   `start_offset_seconds`. `Once` clamps at its end; `Loop` reduces over
   the closed circuit; `PingPong` selects both position and direction over
   the outward-and-return cycle. This is initial placement, not travel
   before the object existed. Initial carry/contact history is empty.
   Restore uses its native continuation and does not apply the offset again.
   An actor route instead selects its initial target and progress while
   preserving the actor's admitted placement; see PATH2.
3. **Propose without mutation.** From the continuation and effective
   duration, retain a compact requested interval with path identity,
   original time domain and unwrapped travel. The only API must not be a
   mutating advance straight to the requested endpoint.
4. **Sample or cut the proposal.** Return an endpoint, the next leg boundary,
   or pose and continuation at a selected time. The native owner commits
   its accepted continuation and pose together. A refused host does not
   secretly advance a drive clock and catch up later unless its policy
   explicitly asks for that behavior.

Define ties at exact endpoints and reversal once. Use cycle reduction and
prepared segment lookup for endpoint evaluation, rather than subtracting
small leg lengths from a possibly huge floating remainder. Work to sample
an endpoint must not grow with the number of complete laps. A cumulative
distance table is justified by this operation; it is not a request to
cache every polyline in the engine.

Reject nonfinite requested travel and cumulative-length overflow through
the existing numeric policy before publication. Do not let checked source
values produce an unchecked phase through multiplication or accumulation.

Keep the full interval meaning after reducing its endpoint phase. Equal
endpoints after a lap do not mean no motion. Represent periodic travel
compactly instead of allocating all crossed legs. Contact/admission may
ask for chronological legs and return a truthful accepted prefix or refusal
under its native work allowance. An arbitrary loop cap that silently drops
travel is not a valid finite-work contract.

Current hazard contact uses the new AABB after `HazardRuntime::update`.
Sharing an evaluator does not establish continuous moving-hazard contact.
That consumer can continue its declared sample policy; a future continuous
consumer must consume the actual path. Likewise preserve platform wrapping:
`VerticalLoop` and `HorizontalLoop` reposition discontinuously and exclude
the teleport from carry. They are not closed-path `Loop`, and
`new_pos - old_pos` cannot replace their accepted `last_delta` blindly.

### Native migration and deletion

- Route named LDtk paths and lifted inline hazard paths through the same
  preparation. Report source level/entity/field provenance. Native Rust
  constructors must admit the same geometry, not bypass it with `is_valid`.
- Give path-driven construction an explicit prepared pose/phase seed.
  `HazardRuntime::new` currently keeps its supplied AABB center while
  `new_with_paths` selects the path's first point. Do not conceal this
  difference by projecting an arbitrary spawn onto the nearest segment.
  A different off-path start needs an explicit consumer policy.
- Migrate hazard and platform path evaluation, then delete both old
  evaluators and their duplicated cursor arithmetic. Share immutable
  preparation; keep each occurrence's continuation small and native.
- Preserve the existing [hosted continuation](hosted-prediction-contract.md)
  contract. M2 can adopt proposal/cut semantics without a second path
  model. This slice need not finish all host admission work first.

Use the existing native path fixtures for degenerate input, closed-loop
closure, reversal, a nonzero seed offset and a step across multiple laps.
At least one acceptance case must reach real LDtk-to-hazard publication;
that is where authored preparation and initial pose join. Restoring native
continuation must preserve the next result. These are future checks, not
tests run by this audit. No new harness or source-text gate is required.

The supported efficiency claim is bounded endpoint evaluation and one
implementation of the path law. No compile, binary-size or frame-time
saving was measured. `ambition_body_seed` has other combat dependencies;
moving this type does not by itself remove that dependency closure.

## PATH2 — Let an authored route produce ordinary intent

[`convert_enemy_spawn`](../../../crates/ambition_platformer2d_ldtk/src/conversion/entity_converters.rs)
lowers `EnemySpawn.path_ref` to `CharacterBrain::Patrol` and rejects a
simultaneous nonempty legacy `brain` field. It separately accepts
`brain_profile`; it does not establish every policy combination here.
The seed attaches `ActorMotionPath`, but the only live advance in
[`body/movement.rs`](../../../crates/ambition_platformer2d_actor_monolith/src/body/movement.rs)
is `let _ = motion.advance(self.kin.pos, dt);` after body integration.
The result is discarded. The
[patrol brain](../../../crates/ambition_characters/src/brain/state_machine/mod.rs)
uses a lane and wall contact; its
[build context](../../../crates/ambition_characters/src/control/brain_binding.rs)
explicitly does not consume the path.

Two shipped routes make this a current authoring gap:

| Room | Actor/reference | Authored path points |
| --- | --- | --- |
| `basement_enemies` in `sandbox.ldtk` | Goblin `EnemySpawn-0140`, `KinematicPath-0139` | `(520,650)` to `(820,650)`, speed 95, PingPong |
| `combat_calibration_lab` in `intro.ldtk` | Puppy Slug `EnemySpawn-104851`, `KinematicPath-104852` | `(256,712)` to `(528,712)`, speed 110, PingPong |

The seed fixture checks that the motion component exists. The wall fixture
injects rightward intent. Neither establishes that an authored route drives
the actor. Do not fix this by assigning a sampled path position to the body.

The currently selected native patrol policy must select its route target
before control interpretation, publish normal intent through existing
controller arbitration, and advance its route continuation from accepted
arrival/contact facts. Attached path state alone cannot authorize another
intent writer over human possession or a different selected brain. A blocked
body does not reach a waypoint because its nominal travel time expired.
Keep the ordered route
occurrence at crossings; do not recover progress from the nearest geometric
point after a knockback or interruption.

Start with the two grounded routes above. State their requested speed in
the existing resolved movement units, subject to body permissions and
ordinary collision. Keep chase/attack interruption and resume policy in
the brain. Use the existing [navigation owner](platformer-navigation-and-reachability.md)
for a route leg that requires a jump, climb or flight; a polyline alone
does not grant those capabilities. Report unsupported route use instead
of accepting it as a working patrol or flattening it silently.

Share PATH1's prepared geometry and waypoint semantics, not its authority
to advance physical position by elapsed time. The actor keeps its admitted
placement. The Puppy Slug's authored spawn x=400 is inside a route whose
first point is x=256; starting patrol must not relocate it to that point.
The patrol owner selects its initial ordered target/direction separately
and reaches it through ordinary movement. No new spawn-policy format is
needed for these routes. The native control/brain owner retains pause/resume
and route progress. Delete the discarded post-movement `advance` when that
owner consumes the route.

Acceptance is one native authored patrol that reaches and reverses at its
real route target, then remains on the correct route occurrence when a wall
blocks it. The normal body kernel must remain the movement writer. A new
planner framework, general task graph or combat-brain rewrite is not needed.

## Acceptance

- An Ambition level author can build a complex moving platform through
  supported LDtk and tooling surfaces.
- Collision, passenger carry, attached portals and rollback use one
  authoritative moving transform.
- No actor-family special case is needed to ride it.
- Invalid path/motion authoring fails during preparation with provenance.
- Another game can use the capability without Ambition-specific code.
- Kinematic state stays scoped to its live room.

## World-object authority instead of a feature bucket

`features` is a historical container. Kinematic motion, destructible transition,
interaction verb, loot policy and presentation do not become one
`world_objects` catch-all. Co-locate each object's transition state with its
accepted writers. Share the spatial/body-motion substrate where the semantics
match.

A falling chest may use path motion, receive an accepted hit and expose an
interaction; none of these makes its lifecycle combat-owned. Breakables, chests
and falling chests do not share a transition authority, so there is nothing
duplicated to consolidate. Preserve stable geometry identity,
moving-host portal behavior and phase visibility when moving an object out of
combat or features. An animation effect must not become a second motion
authority.
