# `ambition_platformer2d_core` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_core** — The pure, content-free movement/physics MODEL — the math the rest of the workspace builds on.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abilities`](src/abilities.rs) | Optional movement/combat capabilities. |
| [`body_clusters`](src/body_clusters.rs) | Authoritative **body** cluster types — the movement aggregate every actor carries, the player included (NOT player-specific). |
| [`cast`](src/cast.rs) | `cast` — the swept-primitive library (docs/concepts/movement-collision.md). |
| [`collision_semantics`](src/collision_semantics.rs) | Shared collision-semantics kernel: the gravity-relative support/surface truths every actor body agrees on. |
| [`config`](src/config.rs) | Coordinate transforms and layer/grid constants. |
| [`confirmed_frame`](src/confirmed_frame.rs) | Which simulation frames are settled, and which are still a guess. |
| [`content_epoch`](src/content_epoch.rs) | **`ContentEpoch` — the app-local activation generation stamp.** |
| [`control_frame`](src/control_frame.rs) | Device-agnostic per-frame control vocabulary. |
| [`frame`](src/frame.rs) | `frame` — the engine-level aperture vocabulary (docs/concepts/movement-collision.md). |
| [`geo_id`](src/geo_id.rs) | Durable geometry identity — `GeoId`/`GeoFaceRef` (docs/concepts/movement-collision.md). |
| [`hit_response`](src/hit_response.rs) | **The hit-response kernel** — the pure math of what a landed hit does to a body: launch velocity, directional influence, and hitstun duration. |
| [`input_stream`](src/input_stream.rs) | **The input stream** (netcode N0.2) — the per-tick input artifact. |
| [`kinematic_path`](src/kinematic_path.rs) | Declarative movement paths for moving platforms, spike balls, patrol dummies, and scripted hazards. |
| [`ledge_grab`](src/ledge_grab/mod.rs) | Ledge grab probe, state, and movement-pipeline tick helpers. |
| [`motion_codec`](src/motion_codec.rs) | The movement-policy (`MotionModel`) rollback checksum codec — ADR 0024 §9. |
| [`motion_quality`](src/motion_quality.rs) | **Motion quality** — how a trajectory READS, as numbers. |
| [`movement`](src/movement/mod.rs) | One trusted, frame-aware movement kernel with swappable physics policies. |
| [`player_state`](src/player_state.rs) | Reusable player-state vocabulary. |
| [`snapshot`](src/snapshot.rs) | **The deterministic snapshot vocabulary.** |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`world`](src/world.rs) | Generated sandbox room data. |

_21 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
