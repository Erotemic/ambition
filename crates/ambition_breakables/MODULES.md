# `ambition_breakables` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_breakables** — Destructible state, accepted damage, respawn, and collision publication.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`construction`](src/construction.rs) | Lower authored placements into live domain state. |
| [`returns`](src/returns.rs) | Mirror live breakable state into the world ledgers: the due time of a respawning breakable, and the `Spent` row of one broken for good. |
| [`rollback_registration`](src/rollback_registration.rs) | Register domain state for rollback. |
| [`simulation`](src/simulation.rs) | Per-frame tick for breakable feature entities: respawn countdown and the stand-to-break collapse trigger. |
| [`volumes`](src/volumes.rs) | Publish the damage volume of each live breakable. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
