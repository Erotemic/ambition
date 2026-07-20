# `ambition_sprite_sheet` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_sprite_sheet** — Runtime sprite-sheet metadata registry.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`baked_portrait_rons`](src/baked_portrait_rons.rs) | Compile-time table of `(asset_relative_manifest_path, ron_text)` pairs for every independently published `*_portraits.ron` under `assets/sprites/`. |
| [`baked_sheet_rons`](src/baked_sheet_rons.rs) | Compile-time table of `(filename_root, ron_text)` pairs for every `*_spritesheet.ron` under `assets/sprites/`. |
| [`boss`](src/boss.rs) | Boss spritesheet animation, parallel to `character_sprites` but with the boss generator's own animation rows (rest / floor_slam / side_sweep / spike_halo / dash_echo / hit / death) instead of the standard 8-row `CharacterAnim` grid. |
| [`character`](src/character/mod.rs) | Character sprite-sheet vocabulary and Bevy-side animation helpers. |
| [`frames`](src/frames.rs) | The single frame-addressing algebra for every sprite sheet. |
| [`game_assets`](src/game_assets/mod.rs) | Game asset wiring with fallback-friendly loading. |
| [`pack`](src/pack.rs) | [`SpritePackCatalog`]: the runtime schema for a cross-target *ultrapack*. |
| [`portrait`](src/portrait.rs) | Runtime vocabulary for separately published dialogue portrait sheets. |
| [`sprite_packs`](src/sprite_packs.rs) | Quality-tiered shared-page sprite packs (ultrapacks) — the runtime side. |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
