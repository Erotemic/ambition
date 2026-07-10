# `ambition_characters` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_characters** — The actor BEHAVIOR + identity layer — the "minds and cast" of the workspace.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actor`](src/actor/mod.rs) | Reusable, content-free actor vocabulary: identity + the control contract. |
| [`boss_encounter`](src/boss_encounter.rs) | Boss encounter state machine. |
| [`brain`](src/brain/mod.rs) | Universal brain interface. |
| [`perception`](src/perception.rs) | `WorldView` + `WorldMemory` — the **world-out** port (architecture roadmap S4). |

_4 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
