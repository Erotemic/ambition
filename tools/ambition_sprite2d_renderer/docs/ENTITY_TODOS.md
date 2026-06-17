# Entity graphics TODO triage

## Shipped in this package
- Static/state PNGs for the current sandbox feature families: hazards, boss placeholder, sandbag dummy, breakables, chests, pickups, and NPC terminal.
- Static/fixture PNGs for prominent room mechanics: moving platform, rebound pad, pogo orb, soft/hard blink walls, solid block, one-way platform, door zone, edge exit, and a small energy projectile placeholder.
- Gameplay-surface PNGs (rendered in `entities.py`, **not yet wired to runtime consumers** — drop-in for a future patch):
  - `switch_armed.png` / `switch_disabled.png` — encounter switch
    button states (red armed / green disabled). Replaces the flat
    colored-rectangle fallback in `feature_color`.
  - `morph_ball.png` — player curl-up sprite. Drops into
    `MorphBallSprite::handle` and replaces the Rust-side procedural
    texture in `crate::body_mode::build_morph_ball_image`.
  - `save_point.png` — checkpoint pillar placeholder, ready when
    save-point gameplay lands.
  - `spike_ball.png` — radial-spike iron ball for swinging /
    rolling hazard variants beyond the spike strip.
  - Tilable surfaces (full canvas, `Sprite::image_mode = Tiled`):
    `lock_wall_tile.png` (runtime lock-wall barrier),
    `water_surface_tile.png` (ripple overlay layered above the flat
    water tint),
    `ladder_tile.png` (16×32 climbable column for vertical_shaft and
    similar),
    `acid_tile.png` (32×16 neon-green acid hazard),
    `lava_tile.png` (32×16 red-orange lava hazard),
    `bg_circuit_tile.png` (32×32 hub-style circuit-board parallax).
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

Character pipeline follow-up:
- ✅ Sandbag now has an adapter/YAML path (`configs/sandbag.yaml`) in addition
  to the old sparse tack-on renderer.
- ✅ Robot now has an extended review sheet (`configs/player_extended.yaml`)
  for crouch, wall-slide, wall-jump, ledge-grab, climb, swim, interact, talk,
  and block. Rust integration is intentionally deferred.
- Add visual review / integration tasks for selecting the extended rows from
  gameplay state once the animations are accepted.

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
- A command to copy generated PNGs into `crates/ambition_gameplay_core/assets/sprites` or `assets/entities` once the Rust loader path is finalized.
- More projectile, VFX, and boss hazard variants.

## Character review sheet follow-ups

- Compare `player_traversal_review`, `player_combat_review`, and
  `player_social_review` in-game-scale before committing any of the new rows to
  a runtime atlas.
- Decide which advanced rows should be first-class player states versus flavor
  rows for NPCs and cutscenes.
- If `draw-review` becomes too slow again, keep tests on representative config
  subsets and generate full review sheets manually before art review.
