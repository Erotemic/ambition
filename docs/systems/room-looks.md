---
status: current
last_verified: 2026-10-10
related_docs:
  - docs/systems/camera-and-visual-profiles.md
  - docs/systems/parallax-backgrounds.md
  - docs/adr/0015-ldtk-tileset-rendering.md
---

# Room looks

A room look is a named style for the architecture of one room. It is
presentation only. A look reads the blocks and doors of the room and draws over
them. Collision does not change, and a headless simulation does not install it.

The looks are content: `game/ambition_content/src/presentation/room_look.rs`,
its modules in `presentation/room_look/`, and the shaders in
`presentation/shaders/`. The renderer crate names no look.

## How a room asks for a look

Set the `palette` level field of the LDtk level.

```bash
cd tools/ambition_ldtk_tools
python -m ambition_ldtk_tools.edit.level_set_field \
    --level central_hub_main --set palette=clean_corrupted --in-place
```

| `palette` | Look | Room that uses it |
| --- | --- | --- |
| `clean_corrupted` | One architecture in two states. Pale stone and gold on one side of a front, dark blocks with lit lines on the other side. | `central_hub_complex` |
| `debug_beautiful` | The collision truth of the room as a drawing. | `tech_bros_basement` |

A room with no `palette` value looks as it did before.

## How a look is built

Each quad of a look is a window, in world space, into one scene. Thus the art
stays on the architecture when the camera moves, and each camera (a split
view, a portal capture) draws the look correctly. The role of a quad selects
the layer.

| Role | Quad | Two-state look | Drawing look |
| --- | --- | --- | --- |
| backdrop | the room | the corrupted sky over the clean one, and fog (authored parallax layers) | grid, far outlines, paths with nodes |
| surface | one `Solid`, `OneWay` or `BlinkWall` block | a blink wall's veil of glass in a gold lattice (the terrain is the room's skins) | outline and hatch; a one-way platform is closed on top only; a blink wall has no closed side |
| underside | below one platform | none (the terrain is the room's skins) | drop lines; marks that rise below a one-way platform |
| portal | around one door | none (the door is its authored sprite) | the trigger box of the door |
| overlay | the room | loose blocks, short tears | register marks, a scan line |

A look also chooses the door sprite of the room (`RoomLook::door_art`), with
the renderer's `EntityArt` component. The sprites are `door_stone`,
`door_voxel` and `door_blueprint` in the sprite renderer
(`targets/props/entities.py`). All doors have one outer shape and one size
(126 x 242 px, `DOOR_SPRITE_ASPECT`). The door PNG files are generated and not
in Git: publish them with
`python -m ambition_sprite2d_renderer publish entities --dest-root <sprites>`
and `scripts/regen/quality_variants.sh --sprites-only --target 'door_*'`.

The block sprites stay below the surface quads. If a look does not draw, the
room looks as it did before.

### What draws each role

The drawing look (`debug_beautiful`) draws each role with one shader, each
frame (`room_blueprint.wgsl`).

**No shader of the two-state look draws art** (Jon, 2026-10-10: "we should
not be generating art in shaders. They should be authored assets"). A shader
of the look composites: it reads authored textures, and the front of the room
says what shows. What it adds is of the front, not art: the heat and the burn
of the stone near it, the veil of a blink wall, loose blocks, fog.

The architecture of the two-state look (`clean_corrupted`) is published art:
two terrain skins, `hub_clean` and `hub_corrupt`, drawn by the sprite renderer
(`terrain/two_state.py`) and published with the other skins
(`scripts/regen/backgrounds.sh`). The room's theme is `hub_clean`; a theme
with a corrupted state (`ParallaxTheme::corrupted`) has its room lay the skins
of both states on the same blocks (`RoomTerrain::state_of`).

| Part | File | What it does |
| --- | --- | --- |
| the art | `terrain/two_state.py` (sprite renderer) | The fill, cap, underside, side and one-way of each state. The corrupted state is derived from the clean construction: the same courses as voxels, the gold fillet as a circuit, one walking line on top. |
| the skins | `RoomSkinMaterial`, `room_skin.wgsl` | Each mesh of a skin shows it where the room is in its state, and adds what the front does to the stone near it (heat, burn, stain, cracks). |
| the sky | `RoomSkyMaterial`, `room_sky.wgsl` | Lays the authored corrupted sky over the authored clean one where the air is corrupted, and the fog. See "The sky is authored". |
| the rest | `RoomStateMaterial`, `room_state.wgsl` | The veil of a blink wall and the overlay: effects of the front. |

The cost of the architecture for each pixel is one texture read and the front.
Far from the front (`look_is_settled`: all of a room that is all clean or all
corrupted) the front is one dot product, and the overlay draws nothing.

With no budget for the look (`show_two_state_skins`), each mesh of a skin is a
plain textured mesh, and each square of the room (`SKIN_CHUNK`) shows the skin
of the side of the front it is on. A room that is all one state looks the same
with each shader off. The skins load with the room's theme, so the room comes
with its look on its first frame.

`--flag NAME@TICK` records the flag on a later tick, so `--frames` films the
front as it moves.

## How to add a look

1. Write a material with the three uniforms (`piece`, `room`, `front`) and a
   shader that draws each role. Import `ambition_content::room_look` for the
   hashes, the noise and the far architecture.
2. Implement `RoomLook` for it: the `palette` value and the door. A look whose
   architecture is authored terrain skins sets `TERRAIN_IS_SKINS`, as the
   two-state look does: its art is published by the sprite renderer, never
   drawn by the game.
3. Call `install_look::<YourMaterial>` in `room_look::install`, and serve the
   shader there with `content_asset!` (not `embedded_asset!`): a development
   build reads it off disk at start, so a shader edit costs a restart and no
   build. `each_shader_is_served_as_its_file_is_saved` names a shader that is
   not served.
4. Set `palette` on a level.

## Limits

- The shape of the front is not authored. It is a line with a fixed lean, and
  the state of the room says how far it has gone.
- A tier with no budget for screen shaders (`screen_shader_scale` of zero:
  Potato) draws no look quad: the room is its skins, drawn plain. When the tier changes in a room that is
  presented, the look takes back what it gave to things that are not its own:
  each inked sign gets its own colours and loses its halo. The doors keep the
  look's art: it is authored, as the skins are. When the budget comes back, the look is presented again
  and dresses them again. The systems that dress are the ones that undress
  (`dress_doors`, `ink_labels_on_the_clean_side`), so they run in each budget.
  Guard: `a_room_look_undresses_when_its_budget_goes` (Full, Potato, Full, with
  no room load).
- The two-state skins repeat each 64 units like each skin, so what the
  painted architecture did once for each piece is not there: the arches and
  banners under a platform, the ivy on a corner, the sigils on a large face,
  the fluted pillar, and the frame around a door.
- The accent colours (magenta, cyan) are written in `two_state.py`,
  `room_skin.wgsl` and `room_state.wgsl`. A change to one is a change to all
  three.
- The drawing look (`debug_beautiful`) is still one shader: it draws the
  collision truth of a room, for a developer.
- A field name of a struct in a shader library must not end in a digit. The
  shader composer writes such a name with a suffix, and a shader that imports
  the struct does not find the field (`LookClaim`).
- The glow is drawn in the shader. There is no bloom pass.
- A look dresses the static blocks of the room. A moving platform is not a block of the room, and it keeps its own art.
- On a build that embeds only the core sprites, a look door is not loaded, and
  the door keeps the standard sprite.

## Validation

```bash
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex player out.png 1280x720 --warmup 60
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex player out.png 1280x1350 --fit-room
cargo run -p ambition_app_tools --bin capture_scene -- tech_bros_basement player out.png 1280x720 --warmup 60
```

To measure a flicker, film a still scene and count the pixels that change:

```bash
capture_scene central_hub_complex player /tmp/t.png 1280x720 --warmup 100 \
    --flag look.central_hub_complex.pure --frames 3 --stride 1
python scripts/measure_capture_frame_delta.py /tmp/t.0001.png /tmp/t.0002.png
```

A capture shows that the shaders compile and what they draw. It does not show
a window camera. Look at a window on a GPU before a release.
