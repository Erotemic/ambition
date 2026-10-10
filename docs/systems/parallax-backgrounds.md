---
status: current
last_verified: 2026-10-10
related_docs:
  - docs/planning/game/art-and-ambiance.md
  - docs/systems/room-looks.md
  - docs/systems/camera-and-visual-profiles.md
  - docs/concepts/generated-assets-audio.md
---

# Room scenery: parallax, terrain skins, motes

The scenery of a room is the art of its theme: the parallax scene behind the
play, the skin on its collision blocks, a light foreground and the motes in its
air. It is provider-owned visual content rendered from camera and room read
models. It must not leak into simulation, collision, or room-loading authority.

All of it is published textures drawn as sprites. A room has its scenery with
each shader off.

## Where the art is

The art is authored in the art submodule and published by
`./scripts/regen/backgrounds.sh` (outputs are not in git):

| Art | Authored in `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/` | Published under `crates/ambition_platformer2d_actor_monolith/assets/` |
| --- | --- | --- |
| Parallax layers and the foreground | `backgrounds/` | `backgrounds/parallax_layers/<theme>_<layer>.png` |
| Terrain skins, motes, decor and doors | `terrain/` | `room_dressing/<theme>_<part>.png` |

`quality_variants.sh --backgrounds-only` makes the smaller tiers of the parallax
layers. The room dressing has no tiers.

## The theme of a room

`ParallaxTheme::from_room_metadata` gives each room a theme: the
`parallax_theme` field of its LDtk level, then its visual profile, then its
biome, then `hub`. `named_by_room_metadata` gives `None` for a room that names
no theme. The sky uses the first: a room must have a sky. The terrain skin and
the motes use the second: a room of another game that names no theme keeps its
own block art.

The art of a theme is loaded by `ensure_parallax_layers_for_room` when a room
of the theme is prepared, and it is retired by the residency rule of the app
(`parallax_residency.rs`) when no live room and no neighbour has the theme.

## Parallax panels

```text
room metadata -> theme -> GameAssets.parallax_layers
    -> present_live_room_parallax (one sprite for each layer)
    -> sync_parallax_layers (size and offset, each frame, for each view)
```

Each layer is one square panel, larger than the view, that shifts inside its
overhang as the camera moves across the room. No tile repeats.

- **A panel is sized against what its camera shows of the world**
  (`visible_world`), not against the pixels of its viewport. The play camera
  shows 640 by 360 units in a window of 1600 by 900. A panel sized in pixels
  was drawn two and a half times as large as the view (fixed 2026-10-10).
- `RUNTIME_PARALLAX_LAYERS` has the factor, the z and the panel scale of each
  layer. `backgrounds/scenes.py` has the same numbers (`RUNTIME`) so that
  `python -m ambition_sprite2d_renderer.backgrounds preview` draws what a view
  shows without the game. Change the two together.
- A 16:9 view shows only the middle band of a square panel. `scenes.py` says
  which band for each layer, and the scenes are composed for it.
- The **foreground** is the last layer and the one layer in front of the play
  (`FOREGROUND_PARALLAX_Z`). It follows the height of the camera only a little
  (`factor_y`), so its hanging things are at the top of each view and its
  standing things at the bottom. A theme with no foreground publishes an empty
  picture. A tier that draws fewer layers (`ParallaxBudget::max_layers`) drops
  it first.

## Terrain skins

`rendering/terrain_skin.rs`. `spawn_block` puts a `TerrainSurface` on the
sprite of each solid block, one-way platform and blink wall of a room that
names a theme. `skin_terrain_surfaces` then gives a solid block the fill of the
skin and one child sprite for each open part of each edge: a cap on a top edge,
an underside on a bottom edge, a shade on a left or right edge. An edge is open
where no solid block and no blink wall is in contact with it. A one-way
platform draws the platform of the skin.

- **The pattern is fixed to the room.** Each part is drawn in pieces that are
  cut where the pattern repeats (`anchored_pieces`), and each piece shows the
  part of the picture at its place in the room. So the pattern goes on with no
  break from a block to the block next to it. A room whose ground is many
  small blocks (each intro room is cells of 16) would show the same corner of
  the picture on each one. A block that would need more than 400 pieces of
  fill repeats the fill from its own corner.
- **Decor.** A few things of the theme's decor picture (a barrel, a crystal, a
  stone lantern) stand on each open top edge. Where they stand is a pure
  function of the span (`decor_on_span`), so a room has the same decor each
  time. `TerrainKeepOut` holds the footprint of each thing of the play in the
  room (doors, placements, spawns, props, hazard blocks), and no decor stands
  on one. `TerrainDecorDensity` (a reflected resource) has the mean distance
  between two things; 0 puts none. A thing of the decor is scenery: nothing
  reads it, and its art must not look like a thing a player can use.
- A theme with no skin on disk changes nothing: its blocks keep the tile of
  their kind.
- A room with a look of its own (a `palette`,
  [`room-looks.md`](room-looks.md)) takes no skin: the look draws its blocks.
  On a device that draws no screen shader (`ShaderBudget::draws_screen_shaders`)
  the look is not drawn, and the room takes the skin of its theme (`hub_clean`
  is marble). The choice is made when the room is presented.
- **Doors.** A door of a room that names a theme takes the door of the theme
  (`dress_themed_doors`, `RoomDressingPart::Door`). Each door has the shape of
  the door of the entity sheet, so it keeps its size and its place.
- **Blink walls.** A blink wall takes the field of its kind
  (`RoomDressingPart::BlinkSoft`: light, part clear; `BlinkHard`: armour, not
  clear) and a line of light on each open edge (`BlinkEdge`). The art is the
  same in each biome: violet is the colour of a blink in each room, and a
  player must know the wall at a look. A blink wall covers the edges of the
  blocks it is in contact with, as a solid block does.
- **Ladders and water.** A ladder and a body of water of a room that names a
  theme take the art of the theme (`dress_themed_fixtures`,
  `RoomDressingPart::Ladder`, `WaterClear`, `WaterMurky`, `WaterSurface`). The
  art is laid in pieces fixed to the room, as the fill is, and the flat
  placeholder goes when the picture is there. A theme with no such picture
  keeps the placeholder.
- A painted LDtk tile layer draws over the skin (it is at `WORLD_Z_BLOCK + 0.5`).
  The intro rooms had the fill of `tileset paint` there, one tile over the whole
  collision, and it hid the skin: it was cleared 2026-10-10.
- A block with an authored placeholder colour, a lock wall, and a block with
  `EntityArt` keep their own art.
- The sizes of the parts are constants of `terrain_skin.rs` and of
  `terrain/skins.py`. Change the two together.
- The trims are found one time. A block that is removed later takes its own
  trims with it; the blocks next to it do not get the trim of the edge that is
  now open.
- LDtk can pick tiles with rules (an auto-layer). It is not used here: the
  trims come from the collision blocks, so each room has them with no
  authoring. A room can still have an authored tile layer over its blocks (the
  intro rooms do).

## Motes

`rendering/ambient_motes.rs`. A theme with a `MoteStyle`
(`AmbientMoteStyles`, a resource a game can change) and a mote picture gives
each of its rooms that many small sprites. Each mote has a home in a field a
little larger than the view. The field goes with the camera and wraps, so a
mote is in the world and there are always the same number in view. Some motes
are in front of the play and the others are behind the blocks.

- The motion is a pure function of the time of the presentation clock
  (`mote_at`). Nothing in the simulation reads a mote.
- A tier with parallax off has no motes.
- The field goes with the first main camera: in a split view the motes of a
  room are around that camera only.

## Ground shadows

`rendering/ground_shadows.rs`. Each actor and the player has one soft dark
ellipse on the ground under it (`RoomDressingPart::Shadow`, a published
picture in the colour of the theme: no shader). The shadow is smaller and
fainter the higher the body is over the ground, and gone at `REACH` (150
units). It is behind each actor and in front of the terrain.

- The ground is the nearest top of a solid block, a blink wall or a one-way
  platform under the middle of the body (`ground_below`). The shadow is not
  less than 30 units wide: the box of a body is narrower than its picture.
- A shadow shows on a light floor (grass, sand, marble). On a dark floor (the
  steel of the lab) it is hard to see: it is a dark picture, and it adds no
  light. A body on a moving
  platform or on another body has its shadow on the ground under that.
- Down is down the screen. A room with another gravity has shadows that are
  not under the feet.
- A room that names no theme has none, and a tier with parallax off has none.
- It reads the read models of the renderer only (`BodyPoseView`,
  `FeatureViewIndex`, the presented poses). Nothing in the simulation reads a
  shadow.

## A backdrop can be mirrored

`room_mirrors_its_backdrop(name, theme)` says if a room draws its panels
mirrored left to right. It is a pure function of the room's name, so about
half of the rooms that share a theme show the scene the other way round and
two rooms next to each other are less alike. A scene with writing or a thing
that has a hand (a clock) must not be mirrored: there is none now.
A theme with a corrupted state (`hub_clean`) is never mirrored: the look of
its room lays the corrupted layers over the clean ones in a shader that has
the placement of the panel and not its mirror.

## Invariants

- Headless simulation does not create or require scenery entities or assets.
- Room/session scope cleanup removes scenery on transition and reset.
- Camera-relative offsets are derived; they are not persisted authority.
- Asset transport differences (desktop/web/Android) do not create gameplay
  forks.
- Generated visual assets have a reproducible source and an explicit publish
  step.

## Validation

```bash
cargo test -p ambition_render --features portal_render -- parallax terrain_skin ambient_motes
cargo test -p ambition_sprite_sheet
(cd tools/ambition_sprite2d_renderer && python -m pytest tests/test_backgrounds.py tests/test_terrain.py)
```

A look is checked with `capture_scene <room> player out.png 1280x720` under
`AMBITION_QUALITY_PROFILE=ultra`. A capture is not a window on a GPU.
