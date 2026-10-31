# `ambition_sprite_part_format` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_sprite_part_format** — Compile sprite-part metadata before application startup.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`parts`](src/parts.rs) | The published sprite-part wire format. |

_1 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

This crate has no Bevy dependency. Build tools and the live sprite reader use
one set of serde types.
