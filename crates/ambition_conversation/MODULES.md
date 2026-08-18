# `ambition_conversation` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_conversation** — **Conversation continuity: the authority, the hold, and the break rule.**

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authority`](src/authority.rs) | **What the simulation believes about the live conversation.** |
| [`dialog`](src/dialog.rs) | Sim-side dialogue glue. |
| [`hold`](src/hold.rs) | **The hold: a projection of the authority onto the body being talked to.** |
| [`instance`](src/instance.rs) | **Which conversation this is, in a form a corrected timeline agrees with.** |
| [`ledger`](src/ledger.rs) | **What the narrative told the simulation, and the tick each fact applies from.** |
| [`opening`](src/opening.rs) | **Deciding that a conversation happens, and opening it.** |
| [`plugin`](src/plugin.rs) | **What `conversation` registers, owned by `conversation`.** |
| [`rules`](src/rules.rs) | **When a conversation ends, and the bark that says so.** |
| [`ui_bridge`](src/ui_bridge.rs) | **The seam between the authority and the text box.** |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
