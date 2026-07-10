# `ambition_menu` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_menu** — Engine-side unified menu: the renderer-agnostic content model plus two interchangeable presentations of it.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`backend`](src/backend.rs) | Menu backend selection vocabulary. |
| [`map`](src/map.rs) | Map / minimap state — the renderer-agnostic source of truth the Map tab renders. |
| [`render`](src/render/mod.rs) | Renderers for a [`crate::MenuPageModel`]. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
