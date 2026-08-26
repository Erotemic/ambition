# `ambition_platformer2d_world` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_world** — Backend-agnostic authored world IR.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`collision`](src/collision.rs) | The composited collision world: the authored room folded together with the per-frame dynamic contributions a running sim adds to it. |
| [`debug_label`](src/debug_label.rs) | Generic room-object label for debug overlays and editor selection. |
| [`placements`](src/placements.rs) | Authored placement records on the room IR. |
| [`platforms`](src/platforms/mod.rs) | Authored moving platforms: the spec an editor writes, the motion it resolves to, and the runtime state the simulation advances. |
| [`ron_room`](src/ron_room.rs) | The `ron-room` loader: rooms as serialized world IR. |
| [`rooms`](src/rooms/mod.rs) | Room graph and authored room IR. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`world_manifest`](src/world_manifest.rs) | Which authored world documents a game ships, and where play starts. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
