# `ambition_platformer2d_publication` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_publication** — Room publication: prepare a hidden candidate room, check its construction receipt, and publish it into the live world or refuse it, as one operation.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`custody_handoffs`](src/custody_handoffs.rs) | What a publication owes the custodian of a hand it emptied. |
| [`first_room`](src/first_room.rs) | A session's first room: published into a session root that may still be a hidden candidate, before any body of the session is built. |
| [`stage`](src/stage.rs) | Canonical prepared room construction. |
| [`transaction`](src/transaction.rs) | Room construction transaction boundary. |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
