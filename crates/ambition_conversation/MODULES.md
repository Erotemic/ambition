# `ambition_conversation` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_conversation** — Conversation continuity and rollback authority.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authority`](src/authority.rs) | Rollback-owned authority for the live conversation. |
| [`banter`](src/banter.rs) | Combat-banter registry (generic half). |
| [`dialog`](src/dialog.rs) | Sim-side dialogue glue. |
| [`hold`](src/hold.rs) | Project conversation authority onto participant control. |
| [`instance`](src/instance.rs) | Deterministic identity for one logical conversation. |
| [`ledger`](src/ledger.rs) | Bridges narrative inputs from non-rollback dialogue into deterministic simulation ticks. |
| [`music`](src/music.rs) | Conversation-selected music is presentation state scoped to the current room. |
| [`opening`](src/opening.rs) | Deciding that a conversation happens, and opening it. |
| [`plugin`](src/plugin.rs) | Conversation-domain plugin registration and schedule ownership. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_conversation`. |
| [`rules`](src/rules.rs) | Conversation continuity rules and cut notifications. |
| [`ui_bridge`](src/ui_bridge.rs) | Bridge conversation simulation authority and the presentation text runner. |

_12 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
