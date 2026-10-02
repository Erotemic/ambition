# Kinematic world objects

**State:** RESTING. K1-K4 and K6 are closed. K5 is closed except for one item
with no customer. Reopen only for a real kinematic customer. The sole kinematic
customer is Ambition's moving platforms (K6 census).

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
  resource. `advance_moving_platforms` advances every live room's platforms once
  per frame.
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

Three behaviors are real product semantics:

- ping-pong sweep;
- path traversal (`Once`, `Loop`, `PingPong`);
- discontinuous wrapping vertical loop (paternoster / infinite elevator).

A discontinuous wrap is not a continuously closed path. Keep the distinction.

The motion driver (`KinematicPath` / `PathMotion`) is separate from the
moving-solid contact contract. It has three consumers (moving platforms, damage
volumes, enemy patrol brains). Its only authored customer is enemy patrol
(`EnemySpawn.path_ref`).

## Target boundary

```text
Authored moving solid
    stable spatial identity, shape/collision policy, motion spec, presentation ref
        | preparation
Resolved kinematic solid
    resolved path / typed refs, validated motion
        | simulation
Kinematic world state (per live room)
    transform / previous transform / delta
        +--> collision/contact query
        +--> passenger/ledge carry
        +--> portal host transform
        +--> sim-view / renderer
```

Do not generalize every dynamic object into this concept. A platform belongs
here because its identity is world geometry whose transform follows a
deterministic motion driver.

## Open work

| Item | Work | Trigger |
| --- | --- | --- |
| K5 platform path relation | `MovingPlatform.path_id` is a string (`convert_moving_platform` reads `field_string(entity, "path_id")`; the entity contract has no `path_ref` for it). No shipped platform authors a path | Author a path-following platform, and migrate the field to an `EntityRef` in the same change. The tooling (`LdtkEntityCtx::kinematic_path_ref`, the path index) exists |
| `KinematicPath.points` | A semicolon-separated `x,y` string parsed by `parse_points`; `SurfaceChain` authors the same way | Has authored customers; do this first. See [`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md) |
| Belt / conveyor | Split `Block::velocity` (see K6) | A belt is authored |

## K6: the second-customer census

`MovingPlatformState::as_collision_block` is the only site that originates a
non-zero `Block::velocity`. Every other dynamic solid (lock walls, falling sand,
breakables, portal carves) toggles existence or kind at a fixed place.
Everything that moves per frame (patrolling hazard volumes, the cut-rope anvil,
gravity zones, hosted portal apertures) is not solid. Geometry can move without
being kinematic.

Rejected candidates: lock walls appear, they do not slide; no conveyor is
authored; the Smash stage is one static block. Falling sand is a field, not a
body, with no tile identity across frames.

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
