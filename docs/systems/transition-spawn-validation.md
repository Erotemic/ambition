# Transition spawn validation

Room transitions now treat authored arrival positions as intent, not as trusted
coordinates. When a loading zone switches rooms, the sandbox calls
`rooms::validated_spawn()` before constructing the new player state.

The validator:

1. clamps the target center inside the destination room bounds;
2. checks the player body against solids, blink walls, one-way platforms,
   hazards, and rebound pads;
3. searches upward first, then sideways, for the nearest clear position;
4. emits room-layout warnings at startup when an authored transition would need
   repair.

This is meant to prevent the common iteration bugs we saw while moving doors:
spawning partly inside the floor, spawning outside a vertical room, immediately
falling or snapping to geometry, and feeling briefly frozen after a transition.

## Authoring convention

- Edge exits should spawn just inside the corresponding edge opening in the
  target room.
- Door exits should spawn at the corresponding door footprint, with the
  validator ensuring the player stands just outside collision.
- If a warning says a transition repairs its arrival by a large amount, the room
  data should be cleaned up. The validator is a safety net, not a substitute for
  clear room layout.

Longer-term, loading zones should use symbolic links such as
`target_zone_name = "hub_to_shaft"` rather than storing raw target coordinates.
Then the destination spawn can be derived entirely from the paired zone and its
entry direction.
