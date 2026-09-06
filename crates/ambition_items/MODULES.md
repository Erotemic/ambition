# `ambition_items` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_items** — Canonical 24-slot item catalog and owned-item state.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`content_schema`](src/content_schema.rs) | The items capability's authored-content SCHEMA registration. |
| [`equipment`](src/equipment.rs) | Derive granted actions and moves from identity plus worn equipment. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_items`. |
| [`shop`](src/shop.rs) | Merchant economy primitives: buy/sell transactions over the player's [`BodyWallet`] and the 24-item [`OwnedItems`] catalog. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
