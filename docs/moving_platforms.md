# Moving platforms

Moving platforms are authored in LDtk and consumed by the sandbox runtime. They
are intentionally still sandbox-side: they contribute temporary collision blocks
to the engine world each frame, but the reusable engine does not yet own moving
solid semantics such as carrying, crushing, or one-way moving platforms.

## Authoring contract

Use one or more `MovingPlatform` entities on the `Ambition` entity layer.

Each entity rectangle is one platform's starting AABB:

- `px` / entity position: top-left of the starting platform rectangle.
- `width` / `height`: platform size.
- `sweep_dx`: horizontal travel distance in world pixels.
  - Positive values sweep right first.
  - Negative values sweep left first.
- `speed`: travel speed in world pixels per second.

The runtime converts each entity into `MovingPlatformState::from_authored`. Each
platform ping-pongs between its start x-position and `start_x + sweep_dx`.

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

The current playable contract is deliberately simple: entity bounds plus
horizontal `sweep_dx`/`speed`. `KinematicPath` remains a generic path object and
is not the moving-platform authoring contract yet. Path-authored moving
platforms are still a likely future shape, but that should happen by promoting
`KinematicPath` into a typed runtime index rather than by reintroducing hidden
procedural platform placement.
