# `ambition_platformer2d_assets` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_assets** — Asset catalog assembly, character materialization, and startup asset loading.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`character_sprites`](src/character_sprites/mod.rs) | Character sprite loading, atlas residency, and catalog-to-sheet resolution. |
| [`gameplay_defaults`](src/gameplay_defaults.rs) | The gameplay defaults asset: what a fresh platformer body starts with. |
| [`materialization`](src/materialization/mod.rs) | Engine-owned character loading and materialization. |
| [`platformer_assets`](src/platformer_assets/mod.rs) | Build asset catalog inputs from the selected content registries. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
