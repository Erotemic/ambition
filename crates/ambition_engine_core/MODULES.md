# `ambition_engine_core` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_engine_core** — The pure, content-free movement/physics MODEL — the math the rest of the workspace builds on.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abilities`](src/abilities.rs) | Optional movement/combat capabilities. |
| [`body_clusters`](src/body_clusters.rs) | Authoritative **body** cluster types — the movement aggregate every actor carries, the player included (NOT player-specific). |
| [`cast`](src/cast.rs) | `cast` — the swept-primitive library (collision-and-ccd.md §2, CC1). |
| [`collision_semantics`](src/collision_semantics.rs) | Shared collision-semantics kernel: the gravity-relative support/surface truths every actor body agrees on. |
| [`combat_volume`](src/combat_volume.rs) | `CombatVolume` — a hit/hurt shape that can be an axis-aligned box, a rotated box (OBB), or a general convex polygon. |
| [`config`](src/config.rs) | Coordinate transforms and layer/grid constants. |
| [`control_frame`](src/control_frame.rs) | Device-agnostic per-frame control vocabulary. |
| [`frame`](src/frame.rs) | `frame` — the engine-level aperture vocabulary (collision-and-ccd.md §7, CC5). |
| [`geo_id`](src/geo_id.rs) | Durable geometry identity — `GeoId`/`GeoFaceRef` (collision-and-ccd.md §3.6). |
| [`geometry`](src/geometry.rs) | Bevy-native geometry helpers. |
| [`input_stream`](src/input_stream.rs) | **The input stream** (netcode N0.2) — the per-tick input artifact. |
| [`kinematic_path`](src/kinematic_path.rs) | Declarative movement paths for moving platforms, spike balls, patrol dummies, and scripted hazards. |
| [`ledge_grab`](src/ledge_grab/mod.rs) | Ledge grab probe, state, and movement-pipeline tick helpers. |
| [`movement`](src/movement/mod.rs) | One trusted, frame-aware movement kernel with swappable physics policies. |
| [`player_state`](src/player_state.rs) | Reusable player-state vocabulary. |
| [`reference_frame`](src/reference_frame.rs) | The gravity-relative reference frame and the transforms between Ambition's three frames. |
| [`volume_shape`](src/volume_shape.rs) | `VolumeShape` — an authored hit/hurt shape in LOCAL space. |
| [`world`](src/world.rs) | Generated sandbox room data. |

_18 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
