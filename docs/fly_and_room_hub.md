# Fly ability and room hub draft

This pass adds a deliberately overpowered sandbox-only traversal mode and a larger room graph for testing scrolling/loading.

## Fly ability

`AbilitySet::fly` enables a free-flight mode. The Bevy sandbox maps the toggle to the preset utility key:

- default classic preset: `D`
- WASD/J/K/L preset: `U`
- gamepad target: `Y / Triangle`

When flying is toggled on, the player no longer uses normal gravity/ground acceleration. Directional input applies acceleration toward a terminal velocity, so flight should feel like drifting/steering a body rather than moving a cursor. With no vertical input, the engine applies a small sinusoidal hover target so the character bobs subtly up and down.

The ability remains engine-level rather than Bevy-only so it can be tested headlessly and eventually disabled by story/progression state.

## Current room graph

Room `0` is now the central hub. It connects to four test spaces:

1. Scroll Lab: the previous long horizontal sandbox.
2. Vertical Shaft: a tall room for camera scrolling, wall/fly/blink tests, and vertical routing.
3. Square Arena: a large square room for two-dimensional camera movement.
4. Tiny Chamber: a much smaller room for scale, camera clamping, and tight movement tests.

The room graph is still sandbox-side. The engine simulates a single `World` at a time; Bevy swaps the active `World` when the player enters a loading zone.

## Future work

- Promote room graphs into a serializable engine-side room/world model.
- Make loading zones typed: door, portal, theorem gate, story transition, debug warp.
- Add per-room moving-platform specs instead of one generic time-reference platform.
- Add tests for room transitions after the room graph moves into the engine.

## Door interaction while flying

When fly is enabled, `Up` remains a flight direction. Door-style zones therefore require a deliberate double-tap-up gesture while flying. Edge exits are unchanged: moving through a wall opening still transitions automatically.
