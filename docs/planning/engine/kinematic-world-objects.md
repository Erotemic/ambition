# Kinematic world objects

**State:** OPEN. First customer: Ambition moving platforms.

## Why this exists

Moving platforms work today, but they still expose transitional architecture.
The engine should treat deterministic moving world geometry as a first-class
world/simulation capability, not as a special feature hanging off the actor
monolith.

## Source-backed current state

The current implementation is more capable than some old docs imply:

- `MovingPlatformSpec` and `MovingPlatformState` live in
  `ambition_platformer2d_world::platforms`;
- LDtk emits `MovingPlatformSpec` and room preparation resolves optional
  `KinematicPathSpec` references;
- runtime state is serializable and participates in the live
  `MovingPlatformSet`;
- platforms advance once per frame and expose `last_delta` so riders/ledge
  contacts can be carried without advancing the platform per actor;
- collision composition inserts current moving-platform AABBs as solids;
- portals can attach to identified moving platform faces;
- the Bevy visual is a read-model projection of authoritative platform state.

But the module comment still calls the feature a design experiment, and several
boundaries are not engine-1.0 quality:

- the Bevy visual adapter remains under the actor monolith;
- provider lifecycle reaches through that monolith adapter to obtain platform
  state even though the world crate owns the model;
- `world_with_moving_platforms` builds a temporary collision world around the
  static room representation;
- motion authoring is an implicit precedence of `path > loop > sweep` across a
  bag of optional fields;
- the path relation is string-based even though LDtk `EntityRef` tooling exists;
- `KinematicPath` point authoring is a coordinate string;
- crush, one-way, passenger and moving-surface interaction policy is not yet a
  deliberate general contract.

## Target boundary

The engine needs a narrow concept such as **moving/kinematic world geometry**.
Do not generalize every dynamic object into it. A platform belongs here because
its authoritative identity is world geometry whose transform follows a
deterministic motion driver.

Conceptually:

```text
Authored moving solid
    stable spatial identity
    shape / collision policy
    motion specification
    presentation reference
        |
        v preparation
Resolved kinematic solid
    resolved path / typed refs
    validated motion
        |
        v simulation
Kinematic world state
    transform / previous transform / velocity-delta
        |
        +--> collision/contact query
        +--> passenger/ledge carry
        +--> portal host transform
        +--> sim-view / renderer
```

Exact type names should follow the carve, not precede it.

## Motion model

The current three useful behaviors are real product semantics:

- ping-pong sweep;
- path traversal (`Once`, `Loop`, `PingPong`);
- discontinuous wrapping vertical loop for paternoster/infinite-elevator effects.

Make them explicit variants or lower simple authoring sugar into a canonical
motion representation. Do not keep semantic precedence as "which optional field
happened to be filled in".

A discontinuous wrap is not necessarily the same thing as a continuously closed
path; preserve that distinction.

## Contact semantics to settle

### Riding / surface velocity

A supported body resting on a moving solid should inherit the surface delta via
one contact model. Existing carrying tests are useful evidence; consolidate the
behavior rather than adding actor-family exceptions.

### Ledge contacts

A latched ledge contact should remain attached to the same moving spatial
feature while valid. Prefer stable `GeoId`/feature identity over rediscovering by
position.

### One-way platforms

If moving one-way platforms are supported, one-way is a contact policy on the
moving surface rather than a second movement kernel.

### Crushing

Specify what happens when a kinematic solid closes space against another solid.
Possible rules include lethal crush, forced displacement where a legal path
exists, or capability-specific behavior. The engine must not silently depend on
pushout iteration order.

### Portals and attached objects

Portals already ride moving platform faces. Preserve this as a first-class
consumer of stable moving-geometry identity rather than a special name lookup.

## Ownership carve

1. Keep authored/runtime kinematic state in the world/simulation domain.
2. Move visual spawning/sync to presentation/render ownership when dependency
   boundaries permit it.
3. Remove actor-monolith dependency from provider/world lifecycle paths that only
   need world-owned platform state.
4. Give collision an explicit dynamic-geometry overlay/query rather than
   repeatedly treating moving geometry as an ad-hoc reconstructed static world
   if measurement shows that is the cleaner boundary.
5. Keep kinematic state scoped to its world/room partition so future multi-room
   residency can simulate the moving geometry needed by each resident partition
   without promoting one room's platform set to process-global authority.

## LDtk authoring slice

Use [`ldtk-authoring-and-world-tools.md`](ldtk-authoring-and-world-tools.md):

- native/typed path reference;
- visible path editing;
- explicit motion mode;
- validation for mode-specific fields;
- semantic room preview showing path and direction.

## Phases

### K1 — re-measure and write the contract

Inventory every production consumer of `MovingPlatformSet`, platform
`last_delta`, `world_with_moving_platforms` and platform identity. Record which
behaviors are authoritative versus presentation.

### K2 — typed authored motion

Replace ambiguous optional-field precedence with a validated motion spec and
resolve the path link during world preparation.

### K3 — isolate dynamic geometry ownership

Give collision/room simulation a direct moving-geometry seam and remove
unnecessary actor-monolith routing.

### K4 — contact completeness

Consolidate ride/ledge behavior and make crush/one-way policy explicit.

### K5 — authoring polish

Make the vertical slice pleasant in LDtk and the tooling, including visible path
and semantic diagnostics.

### K6 — second consumer test

Use at least one additional kinematic-world-object need before expanding the
abstraction. Candidates include a moving door/wall, conveyor-like solid, or
Smash stage platform. If its semantics differ materially, keep separate types
rather than forcing a false universal mover.

## Acceptance

- an Ambition level author can build a complex moving platform entirely through
  supported LDtk/tooling surfaces;
- runtime collision, passenger carry, attached portal behavior and rollback use
  one authoritative moving transform;
- no actor-family special case is needed to ride it;
- the world/provider path does not depend on the actor monolith merely to obtain
  moving-platform state;
- invalid path/motion authoring fails during preparation with useful provenance;
- the capability remains usable by another game without Ambition-specific code.
