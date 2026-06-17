# Blink

Blink is the short-range teleport / phase movement family. It is powerful enough to break collision and transition assumptions, so document policy separately from the generic ability list.

## Current policy

- Blink should never place the player inside solid collision.
- Destination search must respect body shape, collision layers, and room-transition boundaries.
- Post-blink grace affects safe-position recording; do not record a new respawn point during blink grace.
- Presentation owns blink VFX/audio; mechanics own eligibility and movement result.

## Important paths

- `crates/ambition_engine_core/src/movement/mod.rs` and related engine mechanics for movement vocabulary.
- `crates/ambition_gameplay_core/src/player/` for player ECS state and blink-adjacent player behavior.
- `crates/ambition_app/src/app/sim_systems.rs`, `crates/ambition_gameplay_core/src/player/`, and focused systems for gameplay integration.
- `crates/ambition_gameplay_core/src/dev/trace/` for trace-backed validation of edge cases.

## Validation anchors

```bash
cargo test -p ambition_gameplay_core --lib engine_core::movement
cargo test -p ambition_gameplay_core blink
cargo test -p ambition_app --test scripted_gameplay --features "rl_sim portal"
```

When a blink bug depends on geometry, add or update a trace-backed reproduction instead of only checking the happy path.
