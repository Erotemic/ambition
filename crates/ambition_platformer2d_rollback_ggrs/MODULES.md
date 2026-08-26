# `ambition_platformer2d_rollback_ggrs` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_rollback_ggrs** — GGRS rollback host for Ambition's platformer simulation.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`codec`](src/codec.rs) | The GGRS bridge over the floor's snapshot vocabulary. |
| [`lifecycle_commit`](src/lifecycle_commit.rs) | Confirmed-frame lifecycle commit (Track B, Piece 2). |
| [`local_session`](src/local_session.rs) | Engine ownership of the local GGRS session. |
| [`probes`](src/probes.rs) | Per-component checksum localization across rollback save/load. |
| [`reconcile`](src/reconcile.rs) | GGRS post-load repair for authored brain bindings. |
| [`registrar`](src/registrar.rs) | The GGRS side of the domain-owned registration seam. |
| [`registration`](src/registration.rs) | GGRS-backed implementation of Ambition's typed rollback registration vocabulary. |
| [`session`](src/session.rs) | GGRS session/input bridge shared by the harness and future network hosts. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
