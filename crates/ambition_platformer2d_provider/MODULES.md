# `ambition_platformer2d_provider` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_provider** — Experience-provider boundary between shell loading and platformer runtime state.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authoring`](src/authoring.rs) | Authored provider identity: what an experience declares before any session exists, and the one registration call that installs the shared lifecycle. |
| [`composition`](src/composition.rs) | Shared shell-host composition for one platformer experience. |
| [`lifecycle`](src/lifecycle.rs) | The shared provider lifecycle: preparation, prepared-session ownership, and activation into the live session world. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
