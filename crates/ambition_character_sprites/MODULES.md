# `ambition_character_sprites` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_character_sprites** — **Derivations FROM a character sheet.** The sheet vocabulary itself — `CharacterAnim`, `SheetRecord`, `SpritePosedBody`, the baked registry — belongs to `ambition_sprite_sheet` and is named from there.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`anim`](src/anim/mod.rs) | Animation pickers over gameplay-core actor/player state. |
| [`attack_hitbox`](src/attack_hitbox.rs) | Derive a controllable actor's melee attack hitbox in world space from its sprite-sheet manifest — the same data-driven path bosses use (`boss_encounter::attack_geometry`), so the box you author and see in `debug-hitboxes` IS the gameplay damage box. |
| [`posed_body`](src/posed_body.rs) | **The sprite is the authority for an actor's body geometry.** |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
