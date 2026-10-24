# `ambition_body_seed` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_body_seed** — The actor BODY SEED: the prepared body facts a construction path spawns from.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`clusters`](src/clusters.rs) | The per-tick view of an actor: [`ActorMut`](crate::clusters::ActorMut), the borrow the integration mutates in place, and the query that assembles it from live components. |
| [`physical_baseline`](src/physical_baseline.rs) | Prepared-character physical facts shared by every body-construction path. |
| [`snapshot_impls`](src/snapshot_impls.rs) | Rollback wire encoders for the seed's own components. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
