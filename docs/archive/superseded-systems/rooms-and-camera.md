# Archived: rooms-and-camera.md

Superseded by current LDtk/data-driven ECS docs and concept pages.

Original path: `docs/systems/rooms-and-camera.md`

---

# Rooms, camera scrolling, and loading zones

This prototype now tests more than a single fixed screen.

## Current room graph

The Bevy sandbox owns a small `RoomSet` in `crates/ambition_sandbox/src/rooms.rs`:

- **Room 1: Scroll Lab**
  - Uses the active RON-backed `RoomSet` world from `ambition_sandbox::rooms`.
  - Widened to `3200 x 900` so the camera must scroll horizontally with the player.
  - Contains the original movement sandbox plus a right-side scroll wing.
  - Has a loading zone near the far-right door area.

- **Room 2: Disconnected Archive Chamber**
  - A separate `World` created by the sandbox adapter.
  - Intentionally disconnected from the main lab.
  - Has a return loading zone on the left side.

## Why room loading is sandbox-side for now

`ambition_engine` still simulates one `World` at a time. That keeps the pure
engine focused on movement/collision and avoids prematurely committing to a
final room-graph format.

The Bevy adapter handles:

- active room index,
- loading-zone triggers,
- despawning old room visuals,
- spawning new room visuals,
- camera follow/clamping,
- resetting runtime state for the new room.

Once this feels right, the room graph can move into the engine as a testable
`WorldGraph` / `RoomGraph` model.

## Camera behavior

`rendering::camera_follow` converts the player position from Ambition Engine
coordinates into Bevy camera coordinates, then clamps the camera so it does not
show outside the current room bounds.

This is intentionally simple: it follows the player directly rather than using
spring smoothing or dead zones. Those can be added later once the scrolling room
feels correct.

## Loading zones

Loading zones are non-colliding AABBs. When the player body intersects one,
`sandbox_update` swaps the active room and respawns room visuals. The zones are
shown as translucent cyan rectangles.

For now loading zones reset enemies, moving platform state, and player position.
Future versions may preserve more state per room.

## Transition safety notes

Loading zones now use explicit destination spawn points plus a short real-time
transition cooldown. The cooldown prevents the player from spawning into or near
a destination loading zone and immediately bouncing back to the previous room.

The main-lab return spawn is intentionally placed left of the archive entrance
zone rather than inside it. Moving-platform visuals are tagged as room visuals,
so they are despawned and respawned with the active room instead of persisting
across transitions.

## Hub-and-test-room layout

The current room graph starts in a central hub. The hub has loading zones to:

- the original large horizontal scroll lab,
- a tall vertical shaft,
- a large square arena,
- a compact tiny chamber.

Each room has a return loading zone back to the hub with an explicit destination spawn point and transition cooldown. This keeps the first implementation simple while still testing camera clamping, scrolling in both axes, and disconnected-room transitions.
