---
status: current
related_docs:
  - docs/systems/camera-and-visual-profiles.md
  - docs/systems/parallax-backgrounds.md
---

# Room looks

A *room look* is an authored presentation style for a room's architecture.
Set its LDtk level field `palette` to select it. Looks draw from the room's
blocks, loading-zone/door visuals, and authored art; they do not change
collision or headless simulation.

| `palette` | Appearance | Example |
| --- | --- | --- |
| `clean_corrupted` | An authored stone-and-gold skin and its corrupted counterpart, divided by a moving front | `central_hub_complex` |
| `debug_beautiful` | Blueprint-like view of collision blocks and room structure | `tech_bros_basement` |

Unset `palette` uses ordinary room presentation. The implementation is in
`game/ambition_content/src/presentation/room_look.rs` and its shader modules,
not in an engine-wide look registry.

## Representation

A look renders world-space quads for five roles:

| Role | Scope | Purpose |
| --- | --- | --- |
| Backdrop | Room | Background or authored sky compositing |
| Surface | Blocks | Block/one-way/blink-wall treatment |
| Underside | Platforms | Hanging architecture |
| Portal | Doors | Optional door framing |
| Overlay | Room | Front-related details and atmospheric treatment |

The quads remain attached to room geometry as the camera moves. `RoomLook`
provides the palette key, a material constructor and `door_art` for selecting
an authored door sprite.

The `clean_corrupted` look uses **published terrain skins**, `hub_clean` and
`hub_corrupt`, for its masonry. `RoomSkinMaterial` chooses which skin is visible
at each point of the front; `RoomSkyMaterial` composites published parallax
sky art; `RoomStateMaterial` provides the front's overlays and blink-wall
veil. Shaders combine existing art and state—they are not terrain-art
generators. Source art is authored in the sprite renderer under
`terrain/two_state.py` and `terrain/ornaments.py`, then published by
`scripts/regen/backgrounds.sh`.

`debug_beautiful` uses `RoomBlueprintMaterial` and
`room_blueprint.wgsl` to render a developer-oriented drawing of the
architecture.

## Add or change a look

1. Author and publish its imagery when it needs texture assets; generated
   images remain outside Git.
2. Implement `RoomLook` in the content presentation layer. A look using
   terrain skins sets `TERRAIN_IS_SKINS`; a look with authored sky layers sets
   `SKY_IS_AUTHORED_LAYERS`.
3. Provide the material shader and register it with `install_look` in
   `room_look::install`. The existing content-shader registration uses
   `content_asset!` so local shader edits can be read from disk at startup.
4. Set the level's `palette` and, if the look needs a theme,
   `parallax_theme` explicitly. Use LDtk tools, not hand-edited JSON.
5. Inspect the room in a visible camera. For a first rendering check, use
   `capture_scene` as described in
   [headless room verification](../recipes/headless-room-verification.md).

Example field edit (from the repository root):

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools.edit.level_set_field \
  --level central_hub_main --set palette=clean_corrupted --in-place
```

The target field must exist in the LDtk project's `defs.levelFields`.

## Budget and lifecycle behavior

If quality settings disable screen shaders, the two-state room retains its
published skins; `show_two_state_skins` presents them as ordinary textured
meshes, selected by room-state chunks. The effect overlay is omitted and the
room uses the corresponding flat backdrop. When quality settings change,
`dress_doors` and `ink_labels_on_the_clean_side` update their presentation
without recreating the simulation room.

The architectural front currently has a fixed orientation, with progression
controlled by the room's clean/corrupt state. Ornaments are placed by terrain
rules rather than individually authored on each block; moving platforms keep
their own art. The renderer's `terrain_skin.rs` owns placement details and
thresholds, so change those there rather than copying measurements into a
second document.
