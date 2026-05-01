# Room graph and loading-zone model

This is an early code-driven room graph. The next goal is to make the same
structure serializable from RON/JSON/TOML or a generated DSL, but the current
Rust structs already separate the important concepts:

- `RoomSpec`: one active simulated `World` plus loading zones.
- `LoadingZone`: rectangular non-colliding transition volume.
- `LoadingZoneActivation::EdgeExit`: automatic, intended for side-to-side room
  scrolling where the player walks out of one room into another. These preserve
  velocity to make the rooms feel connected.
- `LoadingZoneActivation::Door`: requires pressing up while inside the zone. Use
  this for elevators, doors, portals, or non-edge trigger volumes. Door
  transitions reset velocity because they are discrete interactions.

Design preference: use edge exits for left/right room continuity whenever
possible. Use doors for central hub entrances, vertical transitions, and anything
that should not trigger by accidental overlap.

Future data-driven shape:

```ron
RoomSpec(
    id: "central_hub",
    world: GeneratedWorld(...),
    zones: [
        EdgeExit(name: "to scroll lab", rect: (...), target: "scroll_lab"),
        Door(name: "to vertical shaft", rect: (...), target: "vertical_shaft"),
    ],
)
```

When we move this into `ambition_engine`, room transitions should emit pure
simulation events such as `RoomExitRequested`, leaving Bevy to perform fades,
camera pans, audio stingers, and visual effects.

## Edge exits versus doors

Automatic `EdgeExit` zones should live on the room boundary and should have a
matching opening in the solid wall. The player should read them as leaving
through a hole in the wall, not as colliding with an invisible trigger placed
inside the play space.

`Door` zones may be inside a room, but they must require an explicit interaction
(currently pressing Up). Door transitions can zero velocity and use a more
teleport-like effect. Edge exits preserve velocity and should feel like a
continuous side-to-side room transition.

Current layout convention:

- Hub left/right exits are edge exits with wall openings.
- Scroll Lab and Square Arena return through side-wall edge exits.
- Vertical Shaft and Tiny Chamber are explicit door-style transitions.
- Loading zones should avoid overlapping floors, shelves, and decorative blocks
  unless the overlap is intentional and readable.

## Room layout validation

The sandbox now has a lightweight `RoomSet::layout_warnings()` pass that reports
active fixtures overlapping loading zones. This is not a replacement for a real
room-spec validator, but it catches common hand-authored layout mistakes while
we expand the test room graph.

See `docs/room_layout_refactor.md` for the current conventions.


## Transition pairing convention

Room transitions should be authored as paired endpoints, not arbitrary spawn
positions. If the player exits through an edge opening in one room, the target
spawn should be just inside the matching edge opening in the next room. If the
player uses an interior door, the target spawn should be aligned with the paired
door volume in the destination room.

Room transitions are now stored as graph links between named loading zones rather than concrete `target_spawn` coordinates. The RON manifest expresses each edge with source and destination endpoint names, e.g.
`(from_room: "central_hub", from_zone: "east_exit", to_room: "scroll_lab", to_zone: "west_exit")`. `RoomSet` derives the arrival from the destination zone and then runs spawn validation.
