# `ambition_platformer2d_provider` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_provider** — Experience-provider boundary between shell loading and platformer runtime state.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authoring`](src/authoring.rs) | Authored provider identity: what an experience declares before any session exists, and the one registration call that installs the shared lifecycle. |
| [`catalogs`](src/catalogs.rs) | Provider catalog declarations, registration conflicts and readiness facts. |
| [`composition`](src/composition.rs) | Shared shell-host composition for one platformer experience. |
| [`lifecycle`](src/lifecycle.rs) | The shared provider lifecycle: preparation, prepared-session ownership, and activation into the live session world. |
| [`presentation`](src/presentation.rs) | Project route declarations into active camera, defense and HUD presentation. |
| [`session_contents`](src/session_contents.rs) | What an experience puts into its own session while the engine builds it. |
| [`world_source`](src/world_source.rs) | A provider's world source and its refusal: what the shared lifecycle runs when the shell asks an experience to prepare, and how a world that cannot be built fails that preparation. |

_7 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
