# `ambition_world` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_world** — Backend-agnostic authored world IR.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`collision`](src/collision.rs) | The composited collision world: the authored room folded together with the per-frame dynamic contributions a running sim adds to it. |
| [`debug_label`](src/debug_label.rs) | Generic room-object label for debug overlays and editor selection. |
| [`placements`](src/placements.rs) | Authored placement RECORDS on the room IR — the [W-b] shape (decomposition.md, W-track ruling; architecture.md §4b). |
| [`platforms`](src/platforms/mod.rs) | LDtk-authored moving-platform runtime helpers. |
| [`ron_room`](src/ron_room.rs) | The `ron-room` loader: rooms as serialized world IR. |
| [`rooms`](src/rooms/mod.rs) | Room graph and authored room IR. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
