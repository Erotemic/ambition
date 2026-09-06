# `ambition_character_sprites` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_character_sprites** — Gameplay-derived facts from character sheets.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`anim`](src/anim/mod.rs) | Animation pickers over gameplay-core actor/player state. |
| [`attack_hitbox`](src/attack_hitbox.rs) | Derive a controllable actor's melee attack hitbox from its sprite-sheet manifest — the same data-driven path bosses use (`boss_encounter::attack_geometry`), so the box you author and see in `debug-hitboxes` IS the gameplay damage box. |
| [`posed_body`](src/posed_body.rs) | Derive body and sprite geometry from per-pose sprite-sheet metrics. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
