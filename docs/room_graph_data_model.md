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
