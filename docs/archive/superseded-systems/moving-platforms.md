# Moving platforms

Moving platforms are authored in LDtk and consumed by the sandbox runtime. They
are intentionally still sandbox-side: they contribute temporary collision blocks
to the engine world each frame, but the reusable engine does not yet own moving
solid semantics such as carrying, crushing, or one-way moving platforms.

## Authoring contract

Use one or more `MovingPlatform` entities on the `Ambition` entity layer.

Each entity rectangle defines one platform's size and, for fallback sweep mode,
its starting AABB:

- `width` / `height`: platform size.
- `path_id`: optional `KinematicPath` id/name. When set, the platform follows
  that path's points, speed, and mode, starting at the path's first point.
- `sweep_dx`: horizontal travel distance in world pixels when `path_id` is
  empty. Positive sweeps right first; negative sweeps left first.
- `speed`: fallback sweep speed in world pixels per second when `path_id` is
  empty.

The runtime resolves each entity through `RoomSpec::kinematic_paths` when
`path_id` is authored; otherwise it builds the simple horizontal sweep state.

## Runtime ownership

`LdtkProject::to_room_set()` lowers every `MovingPlatform` entity in an active
area into `RoomSpec::moving_platforms`. Startup, presentation spawn, room sync,
room transition, hot reload, trace, debug overlay, and sandbox reset seed or read
`SandboxRuntime::moving_platforms` via `platforms::moving_platforms_for_room`.

There is no procedural compatibility fallback in gameplay code. If an active
area has no `MovingPlatform` entities, the runtime has no moving platforms for
that area.

## Runtime behavior

During the sim phase, every active platform advances once per scaled tick. The
first platform currently detected as being ridden carries the player by its frame
delta before engine collision resolution. Collision worlds are rebuilt with all
active moving-platform blocks so player control, blink preview/resolution,
feature hazards, trace snapshots, and debug gizmos agree on the same temporary
solids.

## Current follow-ups

Path-authored moving platforms are now the rich movement contract. The simple
`sweep_dx`/`speed` mode remains useful for quick horizontal platforms, but do not
reintroduce hidden procedural platform placement. Future follow-ups can move
continuous moving-solid semantics (carrying, crushing, one-way moving platforms)
into `ambition_engine` once the sandbox behavior is covered by tests.
