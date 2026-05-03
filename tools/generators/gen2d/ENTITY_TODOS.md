# Entity graphics TODO triage

## Shipped in this package
- Static/state PNGs for the current sandbox feature families: hazards, boss placeholder, sandbag dummy, breakables, chests, pickups, and NPC terminal.
- Static/fixture PNGs for prominent room mechanics: moving platform, rebound pad, pogo orb, soft/hard blink walls, solid block, one-way platform, door zone, edge exit, and a small energy projectile placeholder.
- `assets/entities/entity_manifest.yaml` maps each PNG to the Rust-ish gameplay vocabulary it is intended to support.
- `assets/entities/entity_contact_sheet.png` gives a quick visual review grid.

## P0: wire into Rust fallback pipeline
- Add an optional `EntitySpriteAssets` resource analogous to `CharacterSpriteAssets`.
- Load `assets/entities/*.png` without making missing files fatal.
- Use entity sprites in `spawn_room_object`, `spawn_block`, `spawn_loading_zone`, and `spawn_moving_platform`, while retaining colored rectangle fallbacks.
- Use runtime state to pick `chest_closed` vs `chest_open`, and `breakable_intact` vs `breakable_cracked` vs hidden/broken.
- Resolve the character-sheet schema mismatch: Rust currently declares eight character rows, while the generator now emits `blink_out`, `blink_in`, and `dash` rows too.

## P1: add compact animation sheets for entities that need motion
- Chest opening: closed -> open -> reward flash.
- Breakable crumble: intact -> cracks -> debris burst.
- Pickup sparkle / bob / collect pop.
- Hazard pulse / spike warning flash.
- NPC idle / talk light state.
- Boss core intro, attack telegraph, hit flash, defeated.
- Moving platform glow / direction marker.

## P2: improve metadata and engine usability
- Emit per-sprite anchor, intended collision size, z-order class, and default custom-size hints.
- Emit an atlas option for all entity sprites to reduce Bevy texture handles.
- Add LDtk identifier aliases in the manifest (`ChestSpawn`, `PickupSpawn`, `HazardBlock`, etc.).
- Add themed palette variants per biome/room family.
- Add inventory/menu icons that reuse the same pickup art vocabulary.

## P3: polish and testing
- Golden-image or perceptual smoke tests for all entity sprites.
- Review sheets grouped by category/state.
- A command to copy generated PNGs into `crates/ambition_sandbox/assets/sprites` or `assets/entities` once the Rust loader path is finalized.
- More projectile, VFX, and boss hazard variants.
