# `ambition_damage` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_damage** — ⛔ AND IT RE-EXPORTS NOTHING. Three `pub use` lines forwarded `ambition_combat::util` and `_core::hit_response` items from here, which is the same facade shape that hid this module's real coupling for months.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`extension`](src/extension.rs) | Script delivery of native reception decisions after damage resolution. |
| [`healing`](src/healing.rs) | Health restoration requests and their application to bodies. |
| [`ray_jump`](src/ray_jump.rs) | A ray can start a second strike after a body takes damage. |
| [`reception`](src/reception.rs) | Native hit decisions with identity captured at the reception cut. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declarations for health effects. |
| [`wallet_shield`](src/wallet_shield.rs) | Where a wallet absorbs a hit. |

_6 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
