# `ambition_platformer2d_items` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_items** — Item adapters that compose actor simulation with item capabilities.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`conditions`](src/conditions.rs) | Authored inventory conditions. |
| [`match_spawn`](src/match_spawn.rs) | Deterministic match item spawns. |
| [`narrative`](src/narrative.rs) | What the narrative asked for, applied by the simulation. |
| [`persist`](src/persist.rs) | Persist the player's inventory + wallet across save/load. |
| [`pickup`](src/pickup/mod.rs) | Item schedule integration and checkpoint custody restoration. |
| [`starting_bag`](src/starting_bag.rs) | The bag that a session begins with. |
| [`wallet_conditions`](src/wallet_conditions.rs) | Authored WALLET conditions — "can the player pay for this?" |

_7 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
