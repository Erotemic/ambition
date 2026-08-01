# `ambition_platformer2d_ldtk` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_ldtk** — LDtk world-composition adapter and validator for the sandbox.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bevy_runtime`](src/bevy_runtime/mod.rs) | Bevy + bevy_ecs_ldtk plugin glue and runtime-spine indexing for the sandbox's LDtk integration. |
| [`conversion`](src/conversion/mod.rs) | LDtk → Ambition runtime conversion. |
| [`fields`](src/fields.rs) | LDtk field accessors + value parsers for entity instances. |
| [`hot_reload`](src/hot_reload.rs) | LDtk file-watch + transactional hot-reload state. |
| [`intgrid`](src/intgrid.rs) | IntGrid layer decoding: grid-cell values → engine collision/water/climbable. |
| [`loading`](src/loading.rs) | LDtk file-loading policy. |
| [`manifest`](src/manifest.rs) | The `WorldManifest` VALUE (JD4 / AJ2, K2a): a GAME declares its LDtk worlds and entry room; the engine keeps the room kit (`RoomSpec`/`RoomSet`, projection, validators) and ships ZERO worlds — the R3.2 asset move relocated the payload to `ambition_content::worlds`, which builds one. |
| [`project`](src/project.rs) | LDtk JSON deserialization types. |
| [`surfaces`](src/surfaces.rs) | Typed `Surface` authoring primitive: parse + compile to engine collision. |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
