---
status: current
last_verified: 2026-10-10
related_docs:
  - docs/systems/parallax-backgrounds.md
  - docs/systems/room-looks.md
  - docs/tools/generated-visual-tools.md
---

# Art and ambiance

Jon, 2026-10-10: "Let's build a game with ambiance." The backgrounds get an
overhaul, two or three biomes are developed in depth for sandbox and main-game
rooms, sprites and props get better art, and some areas get a light foreground.

## Rules of this lane

- **Art is authored, and it lives in the authoring submodule**
  (`tools/ambition_sprite2d_renderer`). A shader composes art; it does not
  draw it.
- **A level must look good with shaders off.** The base look of a room is
  published textures. A shader adds motion, light and fog on top of it.
- **Nothing lowers quality for cost.**
- **Presentation never decides simulation.**
- A look is verified with `capture_scene`. A capture is not a window on a GPU.

## What a biome is made of

| Part | Where it is authored | How the game draws it |
| --- | --- | --- |
| Four parallax layers | `backgrounds/scenes.py` in the submodule | `ambition_render` parallax panels |
| Foreground layer | `backgrounds/foregrounds.py` | the last parallax panel, in front of the play |
| Terrain skin (fill, cap, underside, side, one-way) | `terrain/skins.py` | `rendering/terrain_skin.rs` |
| Motes | `terrain/motes.py` | `rendering/ambient_motes.rs` |

The system doc is [`docs/systems/parallax-backgrounds.md`](../../systems/parallax-backgrounds.md).

## Run progress (8 h run, armed 2026-10-10T05:22Z)

- [x] Parallax panels are sized against what the camera shows of the world.
  They were sized in window pixels, so the play camera (640 by 360 units in a
  1600 by 900 window) showed the middle of each panel two and a half times too
  large. This was the main cause of the "muddy" backgrounds.
- [x] Eleven scenes drawn again with a numpy paint box: hub, lab, basement,
  cave, cove, water, forest, skybridge, open_sky, boss, eclipse.
- [x] The old parallax tool in the main repository is retired; the sky of the
  two-state room look moved to the submodule with the scenes.
- [x] Terrain skins for ten biomes, laid on the blocks of each room that names
  a theme: a fill, and a trim on each open edge.
- [x] A foreground layer for ten biomes: the one panel in front of the play.
- [x] Motes that drift in the air of ten biomes (dust, embers, fireflies,
  bubbles, petals).
- [x] Ground decor for ten biomes: a few small things on the open top edges of
  a room's blocks, clear of each thing of the play.
- [x] Biomes assigned: 28 sandbox rooms and one intro room name a scene of
  their own (skybridge, eclipse, boss, basement, cave, cove, forest). 22
  sandbox rooms stay `lab`.
- [x] Portals: the capture of an end draws its own window when its pair looks
  at itself (the row of images between two portals that face). The rule is
  tested; the picture is NOT confirmed: see `docs/systems/portals.md`.
- [ ] `capture_scene --camera-zoom` shows no portal window at a wider zoom.
  Find out if a wide camera zone in the game has the same fault.
- [ ] Props and characters.
- [ ] Gizmos in portal captures (optional).
- [x] The intro rooms show the skin of their biome: the uniform painted tile
  layer that hid it is cleared, and the pattern of a skin is fixed to the room
  (the intro ground is cells of 16).
- [x] A door for each biome.
- [x] A room with a look takes the skin of its theme on a device that does not
  draw the look (`hub_clean` is marble).

## For Jon to decide

- **The lowest tier draws no parallax at all** (`potato.parallax.enabled` is
  false, and a test pins it). So with shaders off a room has its skin, its
  doors and its decor, and a black sky. One sky layer is one sprite of a
  256 px texture. If "levels should look good with shaders off" means that
  tier, it should draw the sky layer, and perhaps the far one.

## Not seen

Each item above was looked at in `capture_scene` captures only. A window on a
GPU was not seen.
