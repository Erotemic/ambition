# `ambition_hit_spec` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_hit_spec** — Contact policy shared by authored moves, projectiles, and body reactions.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`snapshot_impls`](src/snapshot_impls.rs) | Encode hit properties for rollback. |

_1 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._

The shared contact values have no ECS or SDK dependency. See
[hit properties](../../docs/systems/hit-properties.md) for the runtime rules.
