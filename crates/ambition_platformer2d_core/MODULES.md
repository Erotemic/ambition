# `ambition_platformer2d_core` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_core** — Content-free deterministic movement and body-state foundation.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abilities`](src/abilities.rs) | Optional movement/combat capabilities. |
| [`body_clusters`](src/body_clusters.rs) | Authoritative movement-state components shared by every actor body. |
| [`cast`](src/cast.rs) | Shared swept collision primitives. |
| [`collision_semantics`](src/collision_semantics.rs) | Gravity-relative collision classification and geometry shared by actor movement. |
| [`config`](src/config.rs) | Coordinate transforms and layer/grid constants. |
| [`confirmed_frame`](src/confirmed_frame.rs) | Host-published boundary between confirmed and speculative simulation frames. |
| [`content_epoch`](src/content_epoch.rs) | `ContentEpoch` — the app-local activation generation stamp. |
| [`control_frame`](src/control_frame.rs) | Device-agnostic per-frame control vocabulary. |
| [`frame`](src/frame.rs) | `frame` — the engine-level aperture vocabulary (docs/concepts/movement-collision.md). |
| [`geo_id`](src/geo_id.rs) | Durable geometry identity — `GeoId`/`GeoFaceRef` (docs/concepts/movement-collision.md). |
| [`hit_response`](src/hit_response.rs) | Carved down from `ambition_damage` (FB6b, fighter-brain.md §12.3 route 1) so ONE formula answers both callers: |
| [`input_stream`](src/input_stream.rs) | Versioned per-tick input recordings for replay, trajectories, and desync analysis. |
| [`kinematic_path`](src/kinematic_path.rs) | Declarative movement paths for moving platforms, spike balls, patrol dummies, and scripted hazards. |
| [`ledge_grab`](src/ledge_grab/mod.rs) | Ledge grab probe, state, and movement-pipeline tick helpers. |
| [`motion_codec`](src/motion_codec.rs) | The movement-policy (`MotionModel`) rollback checksum codec — ADR 0024 §9. |
| [`motion_quality`](src/motion_quality.rs) | Numeric diagnostics for the shape of an authoritative per-tick trajectory. |
| [`movement`](src/movement/mod.rs) | One trusted, frame-aware movement kernel with swappable physics policies. |
| [`player_state`](src/player_state.rs) | Reusable player-state vocabulary. |
| [`sim_random`](src/sim_random.rs) | RANDOMNESS THAT SURVIVES A REWIND — and it survives by not existing. |
| [`snapshot`](src/snapshot.rs) | Backend-neutral deterministic snapshot vocabulary. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`world`](src/world.rs) | Generated sandbox room data. |

_22 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
