# `ambition_sprite_sheet` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_sprite_sheet** — Runtime sprite-sheet metadata registry.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actor_sprite_metrics`](src/actor_sprite_metrics.rs) | What a generated sheet says about the BODY inside its frames: frame size, the per-animation pixel rectangles, and the render size those imply. |
| [`baked_body_rigs`](src/baked_body_rigs.rs) | The table of every published semantic body rig, `(target, ron_text)` sorted by target, from the full-resolution `assets/sprites` tree. |
| [`baked_landmarks`](src/baked_landmarks.rs) | The landmark table of every published part flipbook that tracks a landmark, `(target, bincode)` sorted by target, and the one digest that covers them all. |
| [`baked_part_flipbooks`](src/baked_part_flipbooks.rs) | The table of every published part-flipbook draw table, sorted by key: `<target>` for the full-resolution table, `<target>.<tier>` for a quality tier's, from the `assets/sprites*` trees. |
| [`baked_portrait_rons`](src/baked_portrait_rons.rs) | The table of `(asset_relative_manifest_path, ron_text)` pairs for every independently published `*_portraits.ron` under `assets/sprites/`. |
| [`baked_sheet_rons`](src/baked_sheet_rons.rs) | The table of `(key, ron_text)` pairs for every published `*_spritesheet.ron`, sorted by key ([`crate::sprite_tree::sheet_manifests`]: `assets/sprites/`, its quality tiers, and one level of subdirectories). |
| [`binding`](src/binding.rs) | Typed animation-row binding for one sprite sheet. |
| [`boss`](src/boss.rs) | Boss spritesheet animation, parallel to `character_sprites` but with the boss generator's own animation rows (rest / floor_slam / side_sweep / spike_halo / dash_echo / hit / death) instead of the standard 8-row `CharacterAnim` grid. |
| [`character`](src/character/mod.rs) | Character sprite-sheet vocabulary and Bevy-side animation helpers. |
| [`frame_space`](src/frame_space.rs) | Art space to body space: the one legal conversion, and the sheet fact it needs. |
| [`frames`](src/frames.rs) | The single frame-addressing algebra for every sprite sheet. |
| [`fx`](src/fx.rs) | The effect sheets the engine ships, and the mapping from effect name to sheet row. |
| [`game_assets`](src/game_assets/mod.rs) | Game asset wiring for character sheets, entity sprites, and parallax art. |
| [`inspection`](src/inspection.rs) | Read published sheets through the runtime parser and pose resolver. |
| [`pack`](src/pack.rs) | [`SpritePackCatalog`]: the runtime schema for a cross-target *ultrapack*. |
| [`portrait`](src/portrait.rs) | Runtime vocabulary for separately published dialogue portrait sheets. |
| [`prop_sheets`](src/prop_sheets.rs) | The `prop_sheets` schema: the prop sheets a game draws, as data in its pack. |
| [`published_cache`](src/published_cache.rs) | A development build's cache of what it derives from the published sprite tree. |
| [`published_tree`](src/published_tree.rs) | A development build's read of the published sprite tree. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_sprite_sheet`. |
| [`sheet_mechanics`](src/sheet_mechanics.rs) | The mechanical projection of a sheet record: the part of it the simulation reads, for the content identity. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotCursor` for this crate's own types — the rollback checksum wire. |
| [`sprite_packs`](src/sprite_packs.rs) | Quality-tiered shared-page sprite packs (ultrapacks) — the runtime side. |
| [`sprite_tree`](src/sprite_tree.rs) | The published sprite tree: which files the sprite publisher wrote, and the key each one is read by. |
| [`surfaces`](src/surfaces.rs) | The standing surfaces a sheet publishes, as the simulation reads them ([`ambition_characters::actor::SurfaceTable`]). |

_25 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
