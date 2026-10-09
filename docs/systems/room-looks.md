---
status: current
last_verified: 2026-10-08
related_docs:
  - docs/systems/camera-and-visual-profiles.md
  - docs/systems/parallax-backgrounds.md
  - docs/adr/0015-ldtk-tileset-rendering.md
---

# Room looks

A room look is a named style for the architecture of one room. It is
presentation only. A look reads the blocks and doors of the room and draws over
them. Collision does not change, and a headless simulation does not install it.

The looks are content: `game/ambition_content/src/presentation/room_look.rs` and
the shaders in `presentation/shaders/`. The renderer crate names no look.

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

One `Material2d` draws a whole look. Each quad is a window, in world space,
into one procedural scene. Thus the art stays on the architecture when the
camera moves, and each camera (a split view, a portal capture) draws the look
correctly. The role of a quad selects the layer.

| Role | Quad | Two-state look | Drawing look |
| --- | --- | --- | --- |
| backdrop | the room | sky, far towers, islands, a viaduct, construction lines | grid, far outlines, paths with nodes |
| surface | one `Solid` or `OneWay` block | masonry, cap, gold trim, ivy on the corners | outline and hatch; a one-way platform is closed on top only |
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

The block sprites stay below the surface quads. If a material does not draw,
the room looks as it did before.

## The two states are one drawing

The corrupted state of `clean_corrupted` is not a second set of art. The shader
computes it from the construction of the clean state: the same silhouette in
blocks, the same gold lines as lit lines, the same streams as light and not
water. A scalar field in world space selects the state of each point. Built
things break at a hard edge made of blocks. Open air changes as haze.

Two rules keep the look honest about collision:

- The top row of a corrupted block is always solid, and it has one bright line.
- Ornament below a platform is behind the surfaces and has less contrast.

## How to add a look

1. Write a material with the three uniforms (`piece`, `room`, `front`) and a
   shader that draws each role. Import `ambition_content::room_look` for the
   hashes, the noise and the far architecture.
2. Implement `RoomLook` for it: the `palette` value and the door.
3. Call `install_look::<YourMaterial>` in `room_look::install`.
4. Set `palette` on a level.

## Limits

- The front of the two-state look is not authored. It goes through the centre
  of the room with a fixed lean.
- A look is all or nothing for the visual quality tier. A tier with no budget
  for screen shaders (`screen_shader_scale` of zero: Potato) draws no look, and
  the room is its block sprites. A sign or a door that the look dressed before
  the tier changed keeps its dress until the room loads again.
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

A capture shows that the shaders compile and what they draw. It does not show
a window camera. Look at a window on a GPU before a release.
