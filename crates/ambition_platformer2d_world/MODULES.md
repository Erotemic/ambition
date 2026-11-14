# `ambition_platformer2d_world` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_platformer2d_world** — Authored room data and live world geometry.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authored_room_commands`](src/authored_room_commands.rs) | Runs the command line a room authors in its `while_live` level field. |
| [`collision`](src/collision.rs) | The composited collision world: the authored room folded together with the per-frame dynamic contributions a running sim adds to it. |
| [`debug_label`](src/debug_label.rs) | Generic room-object label for debug overlays and editor selection. |
| [`gated_lock_walls`](src/gated_lock_walls.rs) | Authored condition-gated lock walls. |
| [`geometry_observers`](src/geometry_observers.rs) | The collision world's geometry, for observation (GOBS-1, `docs/planning/engine/geometry-observability-and-f1.md`). |
| [`governing_rules`](src/governing_rules.rs) | The rules that govern each live room: the reading of the declarations in [`crate::scoped_rules`], for the room a system or an entity is in. |
| [`navigation`](src/navigation/mod.rs) | Platformer navigation: where a body can stand in a room, and how it gets from one place to the next, measured by the movement kernel it really runs. |
| [`placements`](src/placements.rs) | Authored placement records on the room IR. |
| [`platforms`](src/platforms/mod.rs) | Authored moving platforms: the spec an editor writes, the motion it resolves to, and the runtime state the simulation advances. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declarations for derived world state. |
| [`ron_room`](src/ron_room.rs) | The `ron-room` loader: rooms as serialized world IR. |
| [`rooms`](src/rooms/mod.rs) | Room graph and authored room IR. |
| [`scoped_rules`](src/scoped_rules.rs) | Rules that one game states for the rooms it governs (ADR 0033). |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`world_manifest`](src/world_manifest.rs) | Which authored world documents a game ships, and where play starts. |

_15 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
