# `ambition_extension_rhai` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_extension_rhai** — The Rhai backend of the extension host.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`declare`](src/declare.rs) | A module a script declares itself (scripting audit P3, step 3): a module the game does not link, with its whole declaration in the script. |
| [`source`](src/source.rs) | Prepared script libraries and invocation-local module environments. |

_2 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
