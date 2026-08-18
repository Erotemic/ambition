# `ambition_platformer2d_ldtk` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_ldtk** — LDtk world-composition adapter and validator for the sandbox.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bevy_runtime`](src/bevy_runtime/mod.rs) | Bevy + bevy_ecs_ldtk plugin glue and runtime-spine indexing for the sandbox's LDtk integration. |
| [`contract`](src/contract.rs) | **The authoring contract, as data both languages read.** |
| [`conversion`](src/conversion/mod.rs) | LDtk → Ambition runtime conversion. |
| [`fields`](src/fields.rs) | LDtk field accessors + value parsers for entity instances. |
| [`intgrid`](src/intgrid.rs) | IntGrid layer decoding: grid-cell values → engine collision/water/climbable. |
| [`loading`](src/loading.rs) | LDtk file-loading policy. |
| [`project`](src/project.rs) | LDtk JSON deserialization types. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_platformer2d_ldtk`. |
| [`surfaces`](src/surfaces.rs) | Typed `Surface` authoring primitive: parse + compile to engine collision. |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
