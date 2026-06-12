# Player ECS phase 3 — compact archive

**Status:** complete and folded into the full player ECS migration.

Phase 3 rebuilt movement feature clusters after authority moved out of the old aggregate. The important rule remains current: keep each feature's state/query narrow instead of recreating a compatibility mega-player.

Use `crates/ambition_engine_core/src/player_clusters.rs`, `crates/ambition_sandbox/src/player/`, and `docs/current/state.md` for current references.
