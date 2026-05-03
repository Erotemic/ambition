# Entity graphics TODO triage

## Shipped in this package
- Static/state PNGs for the current sandbox feature families: hazards, boss placeholder, sandbag dummy, breakables, chests, pickups, and NPC terminal.
- Static/fixture PNGs for prominent room mechanics: moving platform, rebound pad, pogo orb, soft/hard blink walls, solid block, one-way platform, door zone, edge exit, and a small energy projectile placeholder.
- `assets/entities/entity_manifest.yaml` maps each PNG to the Rust-ish gameplay vocabulary it is intended to support.
- `assets/entities/entity_contact_sheet.png` gives a quick visual review grid.

## P0: wire into Rust fallback pipeline (DONE)
- ✅ `GameAssets` Bevy resource owns optional handles for character sheets,
  the boss spritesheet, and per-entity static sprites.
- ✅ `assets/entities/*.png` load non-fatally; missing files keep colored-
  rectangle fallbacks. The new `--no-assets` CLI flag forces fallback for
  every art layer.
- ✅ `spawn_room_object`, `spawn_block`, `spawn_loading_zone` consume
  entity sprites. `spawn_moving_platform` is the last hold-out — see P1.
- ✅ `sync_visuals` flips `chest_closed`/`chest_open` and
  `breakable_intact`/`cracked`/`broken` from runtime state.
- ✅ Schema drift resolved: `CharacterAnim` now declares 11 rows (Idle/
  Walk/Run/Jump/Fall/Slash/Hit/Death/BlinkOut/BlinkIn/Dash) matching the
  generator's output. `Dash` is wired in `pick_player_anim`; BlinkOut/
  BlinkIn await a `blink_anim_timer` companion to `slash_anim_timer`.
- ✅ Boss has its own `BossAnim`/`BossSheetSpec`/`BossAnimator` pipeline
  since its rows (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/
  death) don't fit the character grid. Live boss feature entities now
  get the animated spritesheet when present, with `boss_core.png` as the
  fallback.

## P1: add compact animation sheets for entities that need motion
- Chest opening: closed -> open -> reward flash.
- Breakable crumble: intact -> cracks -> debris burst.
- Pickup sparkle / bob / collect pop.
- Hazard pulse / spike warning flash.
- NPC idle / talk light state.
- Boss core intro, attack telegraph, hit flash, defeated.
- Moving platform glow / direction marker.

## P2: improve metadata and engine usability
- ✅ Spritesheet manifest now emits `body_metrics` — measured opaque-pixel
  bbox, feet-pixel coordinates, and Bevy-anchor-convention
  `feet_anchor_norm` — from the first emitted frame, so future runtime
  loaders can replace the per-spec `collision_scale`/`feet_anchor_y`
  heuristic constants with values derived from the actual rendered art.
- Emit an atlas option for all entity sprites to reduce Bevy texture handles.
- Add LDtk identifier aliases in the manifest (`ChestSpawn`, `PickupSpawn`, `HazardBlock`, etc.).
- Add themed palette variants per biome/room family.
- Add inventory/menu icons that reuse the same pickup art vocabulary.

## P3: polish and testing
- Golden-image or perceptual smoke tests for all entity sprites.
- Review sheets grouped by category/state.
- A command to copy generated PNGs into `crates/ambition_sandbox/assets/sprites` or `assets/entities` once the Rust loader path is finalized.
- More projectile, VFX, and boss hazard variants.
