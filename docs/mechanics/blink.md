# Blink

Blink is the short-range teleport / phase movement family. It is powerful enough to break collision and transition assumptions, so document policy separately from the generic ability list.

## Current policy

- Blink should never place the player inside solid collision.
- Destination search must respect body shape, collision layers, and room-transition boundaries.
- Post-blink grace affects safe-position recording; do not record a new respawn point during blink grace.
- Presentation owns blink VFX/audio; mechanics own eligibility and movement result.

## Important paths

- `crates/ambition_sandbox/src/engine_core/movement.rs` and related engine mechanics for movement vocabulary.
- `crates/ambition_sandbox/src/player/` for player ECS state and blink-adjacent player behavior.
- `crates/ambition_sandbox/src/app/update.rs` and focused systems for gameplay integration.
- `crates/ambition_sandbox/src/dev/trace/` for trace-backed validation of edge cases.

## Validation anchors

```bash
cargo test -p ambition_engine movement
cargo test -p ambition_sandbox blink
cargo test -p ambition_sandbox scripted_gameplay
```

When a blink bug depends on geometry, add or update a trace-backed reproduction instead of only checking the happy path.
