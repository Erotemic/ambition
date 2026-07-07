# Player ECS migration — compact archive

**Status:** complete. The old `ae::Player`, `PlayerMovementAuthority`, and `PlayerBody` aggregate paths are gone. The player entity now carries explicit cluster components and companion resources; current state lives in `docs/current/state.md`.

## Durable lessons

- Cut authority boldly; do not preserve a large compatibility aggregate.
- Query the narrow components a system actually needs.
- Keep movement/control timers explicit instead of hidden behind an aggregate update call.
- Preserve behavior with focused movement, collision, and trace/replay tests.
- Let tests use player-cluster scratch fixtures instead of reconstructing the deleted aggregate.

## Current anchors

- `crates/ambition_engine_core/src/player_clusters.rs`
- `crates/ambition_actors/src/player/`
- `docs/current/state.md`
- `docs/systems/character-ai-refactor.md`

Use git history if the full phase-by-phase plan is needed.
