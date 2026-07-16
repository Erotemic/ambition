# `ambition_platformer_provider` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer_provider** — The platformer experience-provider layer.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authoring`](src/authoring.rs) | Authored provider identity: what an experience declares before any session exists, and the one registration call that installs the shared lifecycle. |
| [`lifecycle`](src/lifecycle.rs) | The shared provider lifecycle: preparation, prepared-session ownership, and activation into the live session world. |

_2 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
