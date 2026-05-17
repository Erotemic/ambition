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

`central_hub_main` is the spawn hub. The full set of authored rooms
lives in `assets/ambition/worlds/sandbox.ldtk` and grows as the
sandbox-first push covers more mechanics. As of 2026-05-07 the levels
are: `central_hub_main`, `central_hub_basement`, `scroll_lab`,
`vertical_shaft`, `square_arena`, `tiny_chamber`, `basement_hazards`,
`basement_enemies`, `basement_boss`, `basement_breakables`,
`basement_treasure`, `basement_npcs`, `mob_lab`, `water_world`. Each
basement room demos a single mechanic family; mob_lab and water_world
are larger composed labs.

Room layout has migrated to LDtk (see ADR 0009). The engine still
simulates a single `World` at a time; Bevy swaps the active `World`
when the player enters a `LoadingZone` LDtk entity. The runtime spine
in `crates/ambition_sandbox/src/ldtk_world.rs` keeps the engine
collision world synchronized with LDtk entities.

## Future work (resolved items struck through)

- ~~Promote room graphs into a serializable engine-side room/world model.~~ Resolved via LDtk + ADR 0009.
- ~~Make loading zones typed: door, portal, theorem gate, story transition, debug warp.~~ Partly resolved: edge transitions vs door interactions are distinguished (see `room_graph_data_model.md`); theorem gate / debug warp remain future work if the story arc needs them.
- Add per-room moving-platform specs instead of one generic time-reference platform.
- ~~Add tests for room transitions after the room graph moves into the engine.~~ Covered by `crates/ambition_sandbox/tests/repro_walls.rs` and the LDtk runtime tests in `ldtk_world.rs`; deeper transition fuzz coverage is still future work.

## Door interaction while flying

When fly is enabled, `Up` remains a flight direction. Door-style zones therefore require a deliberate double-tap-up gesture while flying. Edge exits are unchanged: moving through a wall opening still transitions automatically.
