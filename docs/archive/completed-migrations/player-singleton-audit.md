# Player singleton audit — compact archive

**Status:** complete. The audit fed into the player ECS migration and universal-brain work.

Durable outcome: avoid player-singleton assumptions when adding gameplay systems. Systems that need player data should query explicit marker/component sets and stay compatible with future multi-actor/player control where practical.

Current references:

- `docs/current/state.md`
- `docs/planning/universal-brain-interface.md`
- `crates/ambition_engine_core/src/player_clusters.rs`
- `crates/ambition_characters/src/`

Use git history for the original long audit.
