---
status: current
related_docs:
  - docs/systems/room-looks.md
  - docs/systems/camera-and-visual-profiles.md
  - docs/concepts/generated-assets-audio.md
---

# Room scenery: parallax, terrain skins and motes

Room scenery is provider-owned presentation: parallax backdrops, collision-block
skins, decorations, ambient motes and optional atmospheric effects. Headless
simulation does not depend on these assets. Published images provide the art;
shaders may composite or modulate it.

## Select a theme

Author the LDtk level field `parallax_theme`. The conversion stores it in
`RoomMetadata::visual_profile.parallax_theme`, and
`ParallaxTheme::named_by_room_metadata` reads **only that field**.
`biome`, `visual_theme`, `music_track` and `visual_profile` do not silently
select a scenery theme. `ParallaxTheme::from_room_metadata` uses `default` when
none is specified; skin and mote setup can choose not to dress an unnamed room.
This matters for games such as Mary-O that use their own room art.

Parallax assets are registered in the game's asset catalog. The theme key is
a content key, so new themes require corresponding published imagery and
catalog entries. `crates/ambition_sprite_sheet/src/game_assets/mod.rs` owns theme
lookup and the available asset variants.

## Author and publish art

Art sources live under
`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/`:

| Art | Source | Published asset family |
| --- | --- | --- |
| Parallax and foreground layers | `backgrounds/` | `backgrounds/parallax_layers/` |
| Terrain skins, decorations, fixtures and doors | `terrain/` | `room_dressing/` |
| Boss sky/space elements | `backgrounds/sky_parts.py` | `backgrounds/sky_parts/` |

Run `scripts/regen/backgrounds.sh` to publish the normal scenery families.
`scripts/regen/quality_variants.sh --backgrounds-only` produces lower-quality
parallax layers. Published/generated binaries are not checked into Git. The
boss-specific sky parts are positioned by their encounter presentation rather
than the generic parallax controller.

## Parallax panels

`ambition_render::rendering::parallax` presents one world-view-sized panel per
layer and updates its offset from the relevant camera's visible-world region.
The panel moves within a finite overhang; it is not an endlessly tiled texture.
`RUNTIME_PARALLAX_LAYERS` defines depth, motion factors and sizing. Keep those
values consistent with the sprite-renderer's scene preview when changing the
authoring composition.

The foreground is the final layer, in front of gameplay. Lower quality tiers
can omit layers; the lowest tier selects a published flat backdrop instead of
live parallax. `ensure_parallax_layers_for_room` requests the assets when
needed, and the residency policy retires assets when rooms and their neighbors
no longer use them. A room look may composite two authored sky themes; see
[room looks](room-looks.md).

`room_mirrors_its_backdrop` may mirror a room's scene deterministically from
its room name. It never mirrors themes with an alternate/corrupted state,
whose compositing requires fixed panel alignment.

## Terrain and room fixtures

`ambition_render/src/rendering/terrain_skin.rs` derives decoration from
**authored collision blocks** rather than from a separate gameplay mesh.
A theme may provide fills, edges, one-way platforms, blink walls, hazards,
ladders, water and themed doors. Without a published skin, the ordinary
block/fixture artwork remains.

- Skins and trims are anchored in room coordinates so adjacent blocks share
  continuous patterns. `anchored_pieces` owns the slicing rules.
- `TerrainKeepOut` prevents decorative props from occupying authored doors,
  spawns and other reserved locations. `TerrainDecorDensity` controls spacing;
  zero disables decor.
- Decorative glows are authored sprites, not simulation lights. Decorations
  must not be mistaken for interactive world objects.
- Optional water backplates sit behind gameplay and in front of parallax.
  Water tint and surface presentation remain separate.
- Authored LDtk tile paint and `EntityArt` can override the generated
  dressing. The look's own skins take precedence when a room has a `palette`.
- Surface trims are generated at presentation time. If a block is later
  removed, the removed block's trims go with it; the remaining neighbors do
  not automatically acquire newly exposed trim.

## Motes, shadows and view limitations

`rendering/ambient_motes.rs` uses theme-specific `MoteStyle` and a
presentation clock. Motes are visual only. Their current field follows the
first main camera, so it does not yet give each split view its own field.
Lowest-quality parallax settings omit motes.

`rendering/ground_shadows.rs` implements optional ground sprites, gated by
`GroundShadowRooms`. No shipped game currently enables them. They follow
screen-down and static ground rather than arbitrary local gravity; do not
assume them correct for rotated-world rooms.

## Inspecting a theme

The `hall_of_biomes` sandbox area contains comparable theme exhibits, authored
by `tools/ambition_ldtk_tools/specs/hall_of_biomes/make_specs.py`. Change its
`BIOMES` table when introducing a new exhibit. Inspect a targeted room through
`capture_scene` or the running game; see
[headless room verification](../recipes/headless-room-verification.md).

Scene imagery, parallax transforms, terrain dressing and motes are distinct
layers of presentation. A change to one should not require rebuilding its own
shadow collision or alternative room simulation.
