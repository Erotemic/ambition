# Moving platforms

Moving platforms are authored in LDtk and consumed by the sandbox runtime. They
are intentionally still sandbox-side: they contribute a temporary collision block
to the engine world each frame, but the reusable engine does not yet own moving
solid semantics such as carrying, crushing, or one-way moving platforms.

## Authoring contract

Use a `MovingPlatform` entity on the `Ambition` entity layer.

The entity rectangle is the platform's starting AABB:

- `px` / entity position: top-left of the starting platform rectangle.
- `width` / `height`: platform size.
- `sweep_dx`: horizontal travel distance in world pixels.
  - Positive values sweep right first.
  - Negative values sweep left first.
- `speed`: travel speed in world pixels per second.

The runtime converts the entity into `MovingPlatformState::from_authored`. The
platform ping-pongs between its start x-position and `start_x + sweep_dx`.

## Runtime ownership

`LdtkProject::to_room_set()` lowers the first `MovingPlatform` entity in an
active area into `RoomSpec::moving_platform`. Startup, presentation spawn, room
sync, and sandbox reset now seed `SandboxRuntime::moving_platform` from the
active room spec via `platforms::moving_platform_for_room`.

`MovingPlatformState::time_reference` remains as a compatibility fallback for
unauthored rooms and unit tests. New gameplay rooms should place an LDtk
`MovingPlatform` instead of relying on that fallback.

## Current limitation

The runtime still stores a single moving platform per active area. If multiple
`MovingPlatform` entities are placed in one active area, only the first one is
used. The intended follow-up is:

1. Change `RoomSpec::moving_platform` to `Vec<MovingPlatformState>`.
2. Change `SandboxRuntime::moving_platform` to `moving_platforms`.
3. Update collision, carrying, trace, HUD, and visuals to iterate the vector.

## KinematicPath relationship

`KinematicPath` is a generic path object and remains separate from the current
`MovingPlatform` runtime. Path-authored moving platforms are the likely next
shape, but the current playable platform contract is deliberately simpler:
entity bounds plus horizontal `sweep_dx`.
