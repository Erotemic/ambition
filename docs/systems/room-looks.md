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
| surface | one `Solid`, `OneWay` or `BlinkWall` block | masonry, cap, gold trim, ivy on the corners; a blink wall is a veil of glass in a gold lattice | outline and hatch; a one-way platform is closed on top only; a blink wall has no closed side |
| underside | below one platform | brackets, arches, a banner, ivy, water or light that falls | drop lines; marks that rise below a one-way platform |
| portal | around one door | an arch on pilasters | the trigger box of the door |
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

The two-state look (`clean_corrupted`) does not draw its architecture with a
shader each frame. The surfaces of terrain blocks, the undersides and the door
frames are drawn one time for each room into textures, and shown from them:

| Part | File | What it does |
| --- | --- | --- |
| the art | `room_look/architecture.rs` | The colour of one world point of one piece, in each state. Rust, no time, no front. The one authority on the architecture. |
| the plates | `room_look/plates.rs` | Puts each piece, in its two states, on a page of an atlas (a plate), and draws the pages on the compute pool. A texel is one world px. |
| the quads | `RoomPlateMaterial`, `room_plate.wgsl` | Reads the two states from the plate. The front of the room says which one a point shows. Adds what the front does to the stone near it, and what falls from a platform. |
| the sky | `RoomSkyMaterial`, `room_sky.wgsl` | Lays the authored corrupted sky over the authored clean one where the air is corrupted, and the fog. See "The sky is authored". |
| the rest | `RoomStateMaterial`, `room_state.wgsl` | The veil of a blink wall and the overlay: effects of the front. |

The cost of the architecture for each pixel is two texture reads and the
front. Far from the front (`look_is_settled`: all of a room that is all clean
or all corrupted) the front is one dot product, and the overlay draws nothing.

The plate is read with a filter that keeps a texel sharp and blends only at
its edge. The art has lines one px wide, and with no filter each of them
changed pixels when the camera moved by a part of a pixel.

### When the plates are drawn

`prepare_room_plates` keeps the plates of each two-state room that is live and
of each two-state room next to a live room (`RoomPlates`). It draws them in a
task off the main thread, so a room's plates are ready before the room comes
and no frame pays for the drawing. The hub's 84 pieces are 10.5 Mpx on three
pages and take about 60 ms of the compute pool (measured in `capture_scene`,
2026-10-09).

A page is at most 2048 x 2048 texels. That is the unit of the upload budget
for a frame of visible gameplay (`render_asset_budget.rs`, 16 MiB), so the
pages go to the GPU one for each frame.

A room that comes with no plates (the first room of a session, a room a warp
reaches) shows its block sprites until its plates are drawn. An entry whose
room has other pieces now (a level loaded again) is drawn again.

The unit quad of the looks is made at install and not with the first room. A
mesh that goes to the GPU in a frame whose upload budget is spent arrives
late, and Bevy does not try again to draw a quad whose mesh was late (it does
for a late material). With the quad made in the frame of a large upload, no
quad of the look was drawn until its material changed.

## The two states are one drawing

The corrupted state of `clean_corrupted` is not a second set of art.
`architecture.rs` computes it from the construction of the clean state: the
same silhouette in blocks, the same gold lines as lit lines, the same streams
as light and not water. A scalar field in world space selects the state of
each point (`look_field` and `look_claim` in `room_look_common.wgsl`). Built
things break at a hard edge made of blocks. Open air changes as haze.

Rules that keep the corrupted state readable as a place to play:

- The top of a corrupted block has one bright line, and each other side of
  its collision box has a rim. The box is opaque: a block that the mass lost
  is a dark hollow, not the sky.
- The darkest values and the hard bright lines are for what a body touches.
  The sky has mid values, low contrast and soft light; a tower of the sky has
  no lit edge.
- What is not a place to stand lets the sky through: the ornament below a
  platform, a block that grew past its box, a dead leaf, a loose block of the
  front (which is hollow).

### The sky is authored

The sky of the two-state look is two parallax themes, of four layers each, as
each parallax theme is (`docs/systems/parallax-backgrounds.md`):

| Theme | What it is |
| --- | --- |
| `hub_clean` | A pale city of towers, floating islands and a viaduct, on drawing paper, with gold construction lines. |
| `hub_corrupt` | The same city rebuilt in blocks, in violet air, with soft beams of light. |

The parallax renderer draws them
(`tools/ambition_parallax_renderer/ambition_parallax_renderer/room_look_sky.py`),
from one layout, so a tower of one state is the same tower in the other.
Publish them with `scripts/regen/backgrounds.sh`; the PNG files are generated
and not in Git, as each parallax layer is. To look at the two states with no
game: `python -m ambition_parallax_renderer.room_look_sky out.png`.

How the game shows them:

- A room with the look names `hub_clean` as its `parallax_theme` level field,
  and the parallax system draws it as it draws each sky. On a tier with no
  budget for the look, that is the room's sky.
- `RoomSkyMaterial` is one quad over it. It lays the four layers of
  `hub_corrupt` where the air of the room is corrupted, each at the place the
  parallax system puts the same layer of the clean theme (`panel_uv` in
  `room_sky.wgsl` is the rule of `sync_parallax_transform_to_camera`), so the
  two states register. The change is through a lilac mist.

The sky is behind the play, and it must read so. The play is sharp, has the
darkest values and the bright lines. The sky is the opposite:

- It is out of focus. The blur is in the art: `BLUR` in `room_look_sky.py`, a
  radius for each layer, more for a farther one. Change it there and publish.
- It has mid values and low contrast, and a block of the corrupted sky has no
  lit edge.
- There is fog between it and the play: slow, wide patches, pale over the
  clean sky and lilac over the corrupted one. `RoomLookDepth` holds its
  numbers, to tune by eye:

| Field | Default | What it is |
| --- | --- | --- |
| `fog` | 0.14 | How much fog is in front of the sky, 0 to 1. |
| `fog_patches` | 0.5 | How much the fog is in patches, 0 (even) to 1. |

To tune the fog in a window, open the developer inspector: `RoomLookDepth` has
a window of its own, and each change is drawn in the next frame. To photograph
a set of numbers:

```bash
capture_scene central_hub_complex player out.png 1280x720 --warmup 100 \
    --flag look.central_hub_complex.corrupt --look-depth 0.14,0.5
```

The numbers are a session resource and are not saved: when they are right,
write them as the defaults in `room_look.rs`.

Rules that keep the clean state still:

- Nothing of the architecture moves. The ivy does not sway: a strand one px
  wide that moved changed pixels each frame, which reads as a flicker.
- The cracks in clean stone near the front are where the front is, and not
  where its edge is this frame.
- The tears of the front are on its corrupted side only.

## The state of a two-state room

A room with the `clean_corrupted` look has a state: pure, balanced or corrupt.
The state is a fact in the save, as two world flags
(`look.<room>.pure`, `look.<room>.corrupt`; both off is balanced), so it is
saved and rolled back as each flag is. `game/ambition_content/src/room_look_state.rs`
owns it.

The authored verb `look.cycle <room>` moves a room to its next state: balanced,
corrupt, pure. A `Switch` authors it in `on_activate`. The hub has one
(`hub_state_switch`, next to the spawn).

The look reads the state and moves its front toward it at a constant speed, so
a change of state spreads across the room. The doors and the ink of the signs
follow the front. A room starts in the state the save records.

To photograph a state, record its flag in a capture:

```bash
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex player out.png 1280x720 \
    --warmup 320 --flag look.central_hub_complex.corrupt
```

`--flag NAME@TICK` records the flag on a later tick, so `--frames` films the
front as it moves.

## How to add a look

1. Write a material with the three uniforms (`piece`, `room`, `front`) and a
   shader that draws each role. Import `ambition_content::room_look` for the
   hashes, the noise and the far architecture.
2. Implement `RoomLook` for it: the `palette` value and the door. A look whose
   architecture is static sets `ARCHITECTURE_IS_PLATES` and gives its art as
   pixels, as the two-state look does.
3. Call `install_look::<YourMaterial>` in `room_look::install`.
4. Set `palette` on a level.

## Limits

- The shape of the front is not authored. It is a line with a fixed lean, and
  the state of the room says how far it has gone.
- A look is all or nothing for the visual quality tier. A tier with no budget
  for screen shaders (`screen_shader_scale` of zero: Potato) draws no look, and
  the room is its block sprites. When the tier changes in a room that is
  presented, the look takes back what it gave to things that are not its own:
  each door gets the art of its kind, and each inked sign gets its own colours
  and loses its halo. When the budget comes back, the look is presented again
  and dresses them again. The systems that dress are the ones that undress
  (`dress_doors`, `ink_labels_on_the_clean_side`), so they run in each budget.
  Guard: `a_room_look_undresses_when_its_budget_goes` (Full, Potato, Full, with
  no room load).
- The architecture of the two-state look is drawn by code when a room comes
  (`architecture.rs`), into textures. It is not a shader, and it is not an
  authored tileset.
- The drawing look (`debug_beautiful`) is still one shader: it draws the
  collision truth of a room, for a developer.
- On a build with one thread (web), the task that draws the plates runs on the
  main thread.
- A field name of a struct in a shader library must not end in a digit. The
  shader composer writes such a name with a suffix, and a shader that imports
  the struct does not find the field (`LookClaim`).
- The glow is drawn in the shader. There is no bloom pass.
- A look dresses the static blocks of the room. A moving platform is not a block of the room, and it keeps its own art.
- A door that a portal hides keeps its frame.
- On a build that embeds only the core sprites, a look door is not loaded, and
  the door keeps the standard sprite.

## Validation

```bash
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex player out.png 1280x720 --warmup 60
cargo run -p ambition_app_tools --bin capture_scene -- central_hub_complex player out.png 1280x1350 --fit-room
cargo run -p ambition_app_tools --bin capture_scene -- tech_bros_basement player out.png 1280x720 --warmup 60
```

`capture_scene` prints one line when a room's plates are drawn: the pieces,
the pages and the size.

To measure a flicker, film a still scene and count the pixels that change:

```bash
capture_scene central_hub_complex player /tmp/t.png 1280x720 --warmup 100 \
    --flag look.central_hub_complex.pure --frames 3 --stride 1
python scripts/measure_capture_frame_delta.py /tmp/t.0001.png /tmp/t.0002.png
```

A capture shows that the shaders compile and what they draw. It does not show
a window camera. Look at a window on a GPU before a release.
