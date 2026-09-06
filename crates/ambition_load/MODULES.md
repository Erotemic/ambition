# `ambition_load` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_load** — Headless, contributor-neutral loading coordination.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`coordinator`](src/coordinator.rs) | Deterministic in-memory load coordination and barrier derivation. |
| [`id`](src/id.rs) | Stable load, barrier, and work identifiers. |
| [`model`](src/model.rs) | Contributor-neutral load plans, work states, forecasts, and snapshots. |
| [`plugin`](src/plugin.rs) | Bevy message adapter for the load coordinator. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
