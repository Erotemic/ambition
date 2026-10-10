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
| Ground shadow | `terrain/motes.py` | `rendering/ground_shadows.rs` |
| Door, ladder, water | `terrain/doors.py`, `terrain/fixtures.py` | `rendering/terrain_skin.rs` |

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
  at itself (the row of images between two portals that face). Seen in a
  capture with a pair shot onto two faces: five images of the player in the
  view. The command is in `docs/systems/portals.md`.
- [x] `capture_scene --camera-zoom`: a wider zoom shows the portal windows
  (seven images of the player at `arena`). The note that it showed none was
  wrong: it was the pair of that capture, not the zoom.
- [x] Props that are part of a room: doors, ladders, water, blink walls,
  hazards, decor and its light, ground shadows (see the items below).
- [ ] Characters. Not started: see "Not done".
- [x] Gizmos in portal captures: they are drawn. Seen in a capture of two
  portals that face with `--combat-overlay`: each image of the player in a
  window has its boxes, its arrows and its bars. No code changed for it.
- [x] The intro rooms show the skin of their biome: the uniform painted tile
  layer that hid it is cleared, and the pattern of a skin is fixed to the room
  (the intro ground is cells of 16).
- [x] A door for each biome.
- [x] A room with a look takes the skin of its theme on a device that does not
  draw the look (`hub_clean` is marble).
- [x] A shadow on the ground under each actor and the player, in the colour
  of the biome, smaller and fainter when the body is in the air.
- [x] Ladders and water take the art of the biome (steel rungs in the lab, a
  rope ladder in the cove and the forest).
- [x] About half of the rooms that share a scene show it mirrored, so two
  rooms of one biome are less alike.
- [x] The marble hub has motes of its own.
- [x] Blink walls are a field of violet light with a line of light on each
  open edge (the soft kind) and plates of violet armour (the hard kind), in
  place of a flat tile with a hatch.
- [x] A biome for the raid in the main game: `alarm`, the lab with its power
  out, dark, in the red of its beacons, with smoke under the roof, a beam of
  the gantry down and sparks at the ends of the cut cables. The raid
  corridor, the escape shaft and the lower gate stack name it. The wake room
  and the two labs after stay `lab`, so the raid is a change the player sees.
- [x] A biome for the rooms under the town in the main game: `undertown`,
  brick drains with the mouths of other drains in the far wall, the mains of
  the town, lamps in cages, day through the grates of the street, and water
  that gives a green light. The two relay rooms, the drain alley and the
  under-town pipes name it. They had the crystal cave, which stays the biome
  of the cave rooms of the sandbox.
- [x] Hazard blocks are a danger fill with a row of spikes on each open edge
  that point out of it, in place of a tile of spikes that pointed up on each
  side of a block.
- [x] The foreground of the lab, the basement and the boss room has no long
  dark thing along the bottom: it was a band on the floor of each room whose
  camera is low.
- [x] A pool of light round each thing of the decor that gives light
  (lanterns, braziers, floor lights, crystals): a published picture, no
  shader.
- [x] The rebound pad is a plate on two springs with two arrows that point
  up. It had a row of gold triangles, which a player reads as spikes.
- [x] The dressing of a room is in the room's load manifest: the cover stays
  until the skin is there (it came in a few frames late).

## Not done

- **Characters.** No character art changed. The sprite renderer has 165
  character targets on a rig pipeline, a redesign is a new `_v2` target with
  the old one kept, and a full sheet regen is longer than this run had left
  after the scenery. It needs its own lane, and a choice of which characters.
- **A ladder wider than 16 units** draws two ladders side by side.
- **Ambience sound.** The game has no ambience channel. Eleven beds to
  listen to are in `untracked/sfx-candidates/room_ambience/` (not in git),
  with a reel and a script that makes them again. They wait for Jon's ear:
  nothing plays them.

## For Jon to decide

- **The lowest tier draws no parallax at all** (`potato.parallax.enabled` is
  false, and a test pins it). So with shaders off a room has its skin, its
  doors and its decor, and a black sky. One sky layer is one sprite of a
  256 px texture. If "levels should look good with shaders off" means that
  tier, it should draw the sky layer, and perhaps the far one.

## Not seen

Each item above was looked at in `capture_scene` captures only. A window on a
GPU was not seen.

The rooms of the other games were looked at through their routes
(`capture_scene --route sanic_gameplay`, `mary_o_gameplay`, `smash_gameplay`,
`versus_gameplay`), one view of each: they keep their own ground and take the
new scenes behind it. The Sanic rooms name a theme, so a one-way platform
there takes the skin of that theme.
