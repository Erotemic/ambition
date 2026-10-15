# `ambition_time` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_time** — Reusable time vocabulary + Bevy producer for the named-clock dt model (ADR 0010 / 0011).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`day`](src/day.rs) | The time of day: a world clock that turns through morning, day, evening and night. |
| [`rollback_registration`](src/rollback_registration.rs) | Domain-owned rollback declarations; the host supplies the backend registrar. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`time_control`](src/time_control/mod.rs) | Time-control authority as data. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
