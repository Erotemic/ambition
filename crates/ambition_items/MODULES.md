# `ambition_items` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_items** — Canonical finite item catalog — the game's complete set of pickup items.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`content_schema`](src/content_schema.rs) | The items capability's authored-content SCHEMA registration. |
| [`equipment`](src/equipment.rs) | **Worn equipment → granted actions**, reconciled continuously. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_items`. |
| [`shop`](src/shop.rs) | Merchant economy primitives: buy/sell transactions over the player's [`BodyWallet`] and the 24-item [`OwnedItems`] catalog. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
