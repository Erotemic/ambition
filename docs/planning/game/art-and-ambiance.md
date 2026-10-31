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
  cave, cove, water, forest, skybridge, open_sky, boss, eclipse. Two more
  came later in the run: alarm and undertown.
- [x] The old parallax tool in the main repository is retired; the sky of the
  two-state room look moved to the submodule with the scenes.
- [x] Terrain skins for ten biomes (twelve at the end of the run), laid on the blocks of each room that names
  a theme: a fill, and a trim on each open edge.
- [x] A foreground layer for ten biomes: the one panel in front of the play.
- [x] Motes that drift in the air of ten biomes (dust, embers, fireflies,
  bubbles, petals).
- [x] Ground decor for ten biomes: a few small things on the open top edges of
  a room's blocks, clear of each thing of the play.
- [x] Biomes assigned: 28 sandbox rooms and one intro room name a scene of
  their own (skybridge, eclipse, boss, basement, cave, cove, forest). The
  count of rooms for each theme at the end of the run: sandbox, 13 `lab`,
  11 `basement`, 10 `boss`, 8 `skybridge`, 6 `eclipse`, 5 `cave`, 4 `cove`,
  2 `hub`, 2 `open_sky`, 1 `forest`, 1 `water`, 1 `hub_clean`; intro, 4
  `undertown`, 3 `alarm`, 3 `lab`, 1 `skybridge`. The hall of characters is
  `hub` and the rope room is `cave`.
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
- [x] A shadow on the ground under each actor and the player. **Turned off
  2026-10-10** (Jon: it does not suit the style of the game). The system is an
  opt-in for each game (`GroundShadowRooms`), and no game asks.
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
- [x] A ladder wider than its picture is one ladder with long rungs. It was
  two ladders side by side.
- [x] The rebound pad is a plate on two springs with two arrows that point
  up. It had a row of gold triangles, which a player reads as spikes.
- [x] The dressing of a room is in the room's load manifest: the cover stays
  until the skin is there (it came in a few frames late).

## Characters and props, 2026-10-10

Jon: the pirates and the props can be made again completely, the posing too.
A sprite never has a drop shadow in it, and a shadow under each body does not
suit the style of the game.

- [x] The five pirates (admiral, raider, quartermaster, lookout, navigator)
  are five characters: a look for each (`LOOKS` in `_pirate_common.py`), full
  bodies, a head with a face. The head is a bone of the chest, so no pose
  takes it away from the body. The arm with the sword is on the side the
  pirate looks to, and the two arms are in front of the body. Each pose is
  made again, and each but the death has a foot on the ground. The sheet
  frame is 220x164.
- [x] Thirteen entity props are drawn with form and light: the crate (three
  states), the spikes, the terminal, the boss core, the practice bag, the
  moving platform, the pogo orb, the catwalk (one-way platform), the bolt of
  energy, the spike ball and the ability pickup.
- [x] No published sprite that the game loads has a ground shadow in it. It
  is gone from eight entity props, the two chests, the shrine, the gate, the
  Sanic ring and spring, the town tiles, Marie Curry, Admiral Grass Hopper,
  the parrot and the heavy ninja. Two checks found them: a count of flat
  blobs of translucent black in the published sheets, and a search of the SVG
  drawings for a layer with the name shadow (the scan did not see the chests).
  The copies of the entity props at the top of `assets/sprites/` are old
  files that no loader reads (the loader reads `sprites/entities/`): they
  still have the old pictures.
- [x] The shadow of the engine under each body is off (`GroundShadowRooms`,
  an opt-in for each game that no game takes).
- [x] The Kernel Guide is a character on the rig of the pirate crew
  (`targets/characters/kernel_guide.py`): a hooded robe, glasses of light, a
  slate. It was a preset of the toon rig with four short rows. It has eight
  rows now: idle, walk, talk, interact, slash, shoot, hurt and death. It is
  drawn larger than the pirates (catalog `collision_scale` 1.9). A provoked
  guide fights (Jon, 2026-10-10): the action set `kernel_guide_slate` is a
  hit with the slate and a bolt from its screen.
- [x] The hall of biomes: a hall in the sandbox with a door for each biome
  (15), from a door in the hub. Each door goes to the same small room in the
  art of that biome. The rooms are made by
  `tools/ambition_ldtk_tools/specs/hall_of_biomes/make_specs.py`: a new biome
  is one row there. Two of the rooms are room looks (`palette`): the
  clean / corrupted look with its state switch, and the blueprint look.
- [x] The cart of the intro is drawn again, and it is a platform (Jon,
  2026-10-10). It is a wooden wagon in a lab of steel: that is the joke, and
  it is not made to fit the lab. Its bed is a `MovingPlatform` that stands
  still (`speed: 0`, `one_way`, `visual: intro_cart`), as a shark of the
  Mockingbird is. Its sheet has four rows of one frame: `idle` is drawn behind
  each body and `front` (the near rail and the wheels) in front of each body
  (`PlatformLook::front_row`), so a body on the bed is IN the cart. The back
  end of the cart is let down as a ramp (the row `ramp`; the row `gate` is the
  back end shut, for a cart that rolls: not in the game yet). The mover of the
  player has no slopes, so the collision of the ramp is 14 one-way steps, each
  7 long and 4 under the one before: a body on the ground lands on a one-way
  face less than 8 over its feet, so it walks the steps in the two directions
  (`the_cart_of_the_wake_room.rs`). The first step draws the ramp
  (`visual: intro_cart_ramp`, a look that names the sheet of the cart) and the
  others have no picture (`visual: none`). The wake room has no door back to
  the hub: the door of the hub is one way, and its zone in the wake room is a
  landing pad on the bed with no picture (`hide_loading_zone`). A look
  states the frame its numbers were measured in (`frame_px`), so a sheet of a
  lower tier or of a pack is drawn at the same size. After a new sheet of a
  packed target (`data/pack_plan.yaml`), build the packs again (the
  `ultrapack` stage of `scripts/regen/sprites.sh`): the game draws a packed
  target from its pack, and a stale pack draws the old picture.
- [x] The boss vault has no dark sun (that is the mark of the eclipse): its
  scene is the shut gate of a vault with fire behind it. Ground that is not
  plain brick: `basalt` (boss), `monolith` (eclipse, slabs with signs),
  `ruin` (deep water), `rock` (skybridge, and `open_sky`, which had no
  ground dressing and has each part now). `basement`, `undertown` and
  `hub_clean` are laid courses still.
- [x] Seen in the game with `capture_scene` (not in a window on a GPU): the
  five pirates in the hall of characters and in the cove, the crate, the
  spikes, the pogo orb, and no shadow under a body.
- [ ] The other entity props (the heart, the coin, the switches, the morph
  ball, the save point, the doors, the tiles) are as they were.
- [x] Two new birds for the T-rex's call (Jon, 2026-10-10): the `chicken`
  (a white cockerel with a dark tail, a ground bird: idle, walk, run, jump,
  fall, slash, taunt, hurt, death) and the `owl` (a horned owl, its body
  from the side and its face to the viewer, a flyer: idle, walk, fly, slash,
  taunt, hurt, death). Each is one SVG posed by a rig on the `bird` anatomy
  (`scripts/svg_art/{chicken,owl}_art.py` with `birdkit.py`,
  `scripts/build_{chicken,owl}_rig.py`, `targets/characters/{chicken,owl}.py`
  with `_bird_fx.py` for the effects). Side view only: the game reads no
  turnaround row. A further bird is the same four files, one row of the
  catalog and one row of `FLOCK` in the T-rex's conductor.
- [x] Ten more birds the same day: `ostrich`, `emu`, `cassowary`, `kiwi` and
  `shoebill` (one long-legged build: `rigbuild/ratite.py`,
  `scripts/svg_art/ratites_art.py`, `scripts/build_ratite_rigs.py`),
  `european_swallow`, `african_swallow` and `vulture` (flyers:
  `scripts/build_flyer_rigs.py`), and the hens `chicken_hen` and
  `ostrich_hen`. Rule (Jon): a bird whose two sexes do not look alike has
  both, the cock by the bird's id and the hen by `<id>_hen`. Seen in the
  Hall of Characters with `capture_scene`.
- [x] The coconut is a thing of the world (Jon): NOT in the African
  swallow's drawing. `coconut` is a held item (a pure throwable) that the
  player picks up and throws; a thrown item with a row in
  `ambition_content::items::thrown_impact` hurts the body it strikes and
  bounces off (the coconut: one blow of 2). A boss is not hurt by one yet.
- [ ] Still to do for the coconut: a palm that GROWS them (nothing puts a
  taken ground item back). The African swallow picks one up and lets it go
  over the player now (the entry on the six rooms, below).
- [x] A biome of pasta for the Noodly Temple (Jon, 2026-10-12: "a much more
  noodly biome"): `noodly` (`ParallaxTheme::Noodly`). A colander over the
  world with the light through its holes, columns of rigatoni, curtains of
  noodles; ground of noodles under marinara, with noodles under each block
  and ledges of lasagne. See [`noodly-temple.md`](noodly-temple.md). Seen
  with `capture_scene` at the ultra profile, not in a window on a GPU.
- [x] The primeval trail, a linear dinosaur level (Jon): the hub ->
  `primeval_trailhead` -> `primeval_run` -> `primeval_canopy` ->
  `primeval_tyrant_hall` (the T-rex, with his own encounter id
  `primeval.trex`). Its enemies are the flock and the raptors, placed as
  `EnemySpawn` with a catalog `character_id`. Its rooms are written by
  `tools/ambition_ldtk_tools/specs/primeval/make_specs.py`. A new biome,
  `primeval` (a row in each art table, `ParallaxTheme::Primeval`, a mote
  row), and a scenery prop, `coconut_palm`. First pass: the rooms are
  simple, the biome has no foreground layer and no room in the hall of
  biomes, and reaching the tyrant's hall asks nothing of the player but
  walking there.
- [x] The trail is six rooms, and two more kinds hunt on it (Jon,
  2026-10-11): `primeval_rookery` (penguins on rocks) and `primeval_ridge`
  (a long flat road) are between the run and the canopy, and the run is a
  third longer. Willson, the pelican on a bicycle, is an enemy row now
  (`willson_peck`; he stands in the hall because its placements say so): one
  rides the end of the run and two the ridge road. The penguin is placed in
  the rookery. An African swallow picks up a coconut at rest in its room,
  carries it over what it hunts and lets it go
  (`ambition_content::primeval::swallows_carry_coconuts`; the take and the
  release are the held-item crate's one take and one release, and the
  release is shared with the throw). The system keeps no state. Not done: a
  swallow does not take a coconut out of the air or off a palm, and the
  European swallows of the run carry nothing.
- [x] Four dinosaurs that are not birds, for the trail (Jon): the
  `compsognathus` (tiny, placed in threes), the `dilophosaurus` (it spits
  from a distance), the `pachycephalosaurus` (it rams) and the `triceratops`
  (the heavy). The three on two legs are the `ratite` anatomy with arms on
  the wing bones (`scripts/build_ratite_rigs.py`); the triceratops is the
  `quadruped` anatomy and a tail bone (`scripts/build_triceratops_rig.py`).
  The T-rex does not call them: his flock is birds. Its spit flies as the
  thrown `Rock` and looks like a glob of venom (2026-10-13).
- [x] Two biomes for Mary-O (Jon, 2026-10-11), in the colours of a magical
  girl and with the shapes of an old platform game. `starlight` is the land
  above: a pink sky, a crescent moon, a rainbow, round hills with marks on
  them, a far castle, round trees and lamps with a star; ground blocks of
  pink with mint icing. Rooms `mary_o_1_1` and `mary_o_1_3` name it.
  `starless` is the dark one, for the room under the ground (`mary_o_1_2`):
  a moon eaten to a ring in a rift, pointed arches, the wheels of a clock,
  a window of coloured glass, candles; violet brick with a line of magenta
  light. Each is a row in each art table and a `ParallaxTheme`. Jon on the
  first look: "the hearts are a bit much", so the trees have round crowns
  and no decor is a heart.
- [x] After Jon played it (2026-10-11): the two Mary-O scenes have no
  foreground layer. Each had one (strings of stars; chains with moons), and
  it went over the blocks and the coins of rooms that have play at each
  height of the view. And the blocks under the ground have the skin of
  their biome: 1-2 stated a `block_color` and the two vaults a `color` on
  each block, and a block with a colour gets no skin, so they were flat
  boxes. No level of Mary-O states a colour now. A vault is in the room of
  its level, so it has the skin of that room (`starlight`): a part of a
  room can not name a biome of its own.
- [x] The title screen has a backdrop (Jon, 2026-10-11: "very boring right
  now"). Behind the launcher is the parallax scene of the game the cursor is
  on: the hub for Ambition, the open sky for Sanic, the starlight kingdom
  for Mary-O, the eclipse for Smash Siblings, the cove at night for Exit.
  Its four layers drift at four speeds and a move of the cursor fades one
  scene into the next (`game/ambition_app/src/app/title_screen.rs`, host
  presentation; the art is the published layers). A row of the launcher is
  the name of its game, with no description, and the Smash row is "Smash
  Siblings". The text of the row the cursor is on is dark on its gold bar:
  it was light, because the menu renderer gave a focus that came after the
  spawn its background and not its text colour. Not done: a character of
  the game on the screen, a wordmark, a layout that is not the flat menu.
- [x] The launcher's menu has the shape of a title screen (Jon, 2026-10-11:
  "style the menu a bit"). The heading is "Choose Game" (it was "Ambition",
  the name of the first row also) and the tab is "Games". The window is
  narrow, with round corners and a thin edge; a gold rule is under the
  heading; each row has round corners and an edge, and the edge of the
  cursor's row breathes. The host gives the shape
  (`style_the_launcher_menu`, before the layout of the frame, so a change
  of tab does not show the renderer's shape for one frame). The fill of a
  row and its text colour stay the flat renderer's.
- [x] A ride through a Mary-O pipe makes no landing sound and no dust (Jon,
  2026-10-11). The transit puts her on its path each tick, and the movement
  step took her as a body that fell. The pipe now says her pose is not her
  own (`PoseOwnedExternally`), and the step of a player body reads that
  fact: it passed a literal `false`.
- [x] A shader for the two Mary-O biomes: the shimmer
  (`game/ambition_demo_mary_o/src/shimmer_shader.rs`, `shaders/shimmer.wgsl`).
  It draws no shape. Each biome has a shimmer picture
  (`terrain/shimmer.py`, published as `room_dressing/<biome>_shimmer.png`):
  red is the shapes (sparkles, or embers), green is when each one is bright,
  blue is ribbons. The shader moves the picture behind the blocks, makes
  each shape come and go, and gives it its colour. A device with no budget
  for a shader that fills the screen draws none, and the room is then its
  scene and its skin.
- [x] A biome for the void in Mary-O's land, `void` (Jon, 2026-10-30: "a
  really cool void zone / dungeon ... with the black / white theming of the
  void characters"). No colour: black things with a white or grey edge. The
  scene is black space with white specks, broken rings about the heart of
  the void, stars that went out, and the castle of the land broken into
  pieces that drift apart (`backgrounds/outdoors.py::void`). The skin is
  black with one crisp white line on each edge; the door, the ladder, the
  decor (cracked obelisks, splinters, a fallen star) and the motes (white
  specks) are the same. Only the play has a pure white edge: the scene and
  the decor have grey edges, and nothing in the near layer is in the band
  of the play, so the void enemies stay clear. No ground shadow, no
  foreground.
- [x] The snake with the box (`solid_snake`) is drawn again in place (Jon):
  a body with bands that tapers to its tail, a head with a gold eye under a
  brow, a bandana with two tails, a box of corrugated board in three faces
  with tape, a hand hole and a "this side up" mark. Boxed, its eyes show in
  the hand hole. To peek, the box lifts on one side. Its rows, its frame and
  the place of its body and its box did not change (the sheet's body
  metrics moved by 5 px or less).
- [x] The bonus block has an interrobang again (Jon). The drawing had not
  changed, but its stroke was as thin as its hook and it read as a `?` in
  play. The stroke is upright, thicker than the hook, and starts four rows
  above the hook (`bonus_block_tile` in `targets/props/entities.py`, and the
  16 px copy in the Mary-O tileset).

## Plain shots, 2026-10-13 (Jon)

Jon: "We need to generally improve the art for these generic fireballs /
projectiles. The robot fireballs need new art. And I think there is a spitting
dinosaur that has generic square projectiles."

One sheet, the renderer's `shot_kit`, has a row for each: `fireball`,
`coldfire`, `hadouken`, `hadouken_super`, `spit`, `rock`, `bolt`. The head of
each shot is at the middle of its frame, so the tail is behind the body.
`game/ambition_content/src/projectiles.rs` registers the looks.

No shot of Ambition is the engine's orange box now:

- A ranged style has a default look (`RangedStyle::default_visual`): a thrown
  rock, an arrow (the `bow_arrow` sheet, which no shot used), the pistol round.
- A shot that names no look (the empty id) is the magic bolt. This is every
  `Bolt` action and eight boss moves that name none.
- The dilophosaurus names `spit`.
- The Officer's `pistol_round` sheet was not in the publish roster, so his
  shot was the magenta box of a missing sheet. It is in the roster.

To see a look: `capture_scene ROOM X,Y out.png --shot TICK:X,Y:DX,DY:LOOK`.

A shot that is turned to its flight and flies left is mirrored top to bottom,
so its top is up (it was turned half round, upside down).

Open: Jon has not judged this art. The eight boss moves share one bolt; each
could have a look of its own.

## The bubble shield, 2026-10-13 (Jon)

Jon: "I would also like to improve the art for the bubble shield and
potentially make it reactive and more interesting looking with a shader."

The art is authored (the renderer's `bubble_shield` target), three white
pictures: the field, a honeycomb that tiles, and cracks. The field was a
gradient that Rust made at start. The game gives the hue, which is the state
of the guard, as before.

- The sprite draws the field. That is the whole look with no shader budget
  (the `potato` tier, Android).
- A shader layer over it (`ambition_render`'s `bubble_shield_fx`) draws the
  honeycomb, wrapped on the bubble as on a ball: it shows toward the rim and
  drifts, a ring goes round the ball from the point where a blow landed, and
  all of it is lit in a parry. The cracks grow from the rim as the guard is spent. The
  shader draws no art of its own.

To see it: `capture_scene ROOM X,Y out.png --guard X,Y:INTEGRITY[:parry|:hit|:blow@PHASE]`
(`blow@0.3` holds one moment of the beat of a blow).

Jon, 2026-10-13: "New art looks good."

Each guard that blocks a blow records where it landed and for how long that
reads (`BodyShieldState::note_blocked_blow`, rollback state; schema 349). A
shield that is not a resource records it too, so a guard outside a match
flares when it blocks. Before, only shieldstun showed a blow, and such a
shield owes none.

## Jon's three dogs: Honey, Roadie, Bezoar (2026-10-13)

Jon, with photographs: "I want 3 new dog characters based on these 3 dogs. I
want them to be roaming in different rooms in the sandbox. Not the central
hub, but in the test areas connected to it."

Each is an SVG rig with the companion dog's parts about its joints, so the
companion dog's clips play on each (the renderer's `_house_dog.py`,
`scripts/svg_art/house_dogs_art.py`). What Jon said of each:

- **Honey** (35 lb): "the smallest and shortest". The tip of her right ear
  has "a very minor flap", so she has a drawing of each side and `~mirrored`
  rows: a flip of the sheet would move the flap to her other ear. The tip flops
  outward. Side on, that is toward the viewer (from her right: the tip hangs
  down over the face of the ear) or away (from her left: the ear ends in a
  round fold).
- **Roadie** (50 lb): "bigger, but still only a medium sized dog".
- **Bezoar**, "Bea" (40 lb): "taller but lanky". Her coat is "very close to
  Honey's color, but just slightly tanner". Her two ears have "slight flops".
  "Her ears only have the dark tips, and her muzzle isn't that dark. It is
  towards the end, but it sort of blends in." "She should be shorter than
  roadie but taller than honey."
- "The eyebrows for honey and beazor are important": Honey has a pale pip
  over the eye, Bezoar a dark one.
- The tongue "shouldn't be hanging off the top of their mouth": it lies on
  the lower jaw, and the muzzle is drawn over its root. The bark mouth is that
  mouth open a little more (of a wide one: "too wide too big").

Where they roam is in `docs/systems/npc-navigation.md`.

Open:

- Jon wrote "Beazoar should be physically smaller than honey" and then sent
  a photograph in which she sits taller than Honey. As drawn she is taller,
  on long thin legs, with a slimmer and shorter body. Jon has not said this
  reading is right.
- The spelling "Bezoar" is mine (Jon wrote "beazor" and "beazoar").
- Roadie has no drawing of its other side, and no second round of polish.
- The place a hand pets (`contact_offset`) is the companion dog's on each;
  it was not fitted to the three sizes.
- No dog has its own bark sound: each uses `npc.dog.woof`.

## The General, redrawn (2026-10-13)

Jon, with a picture of a cartoon general: "Please redo the art for the absurd
general. I'm going for a general specific sort of vibe (although again, we
are not copying the character, this is just inspiration)".

He is an SVG rig of his own (the renderer's `absurd_general` target; he was a
preset of the toon template): a coat like a box on two legs like sticks, a
nose like a bulb, teeth shut tight, arms akimbo, a swagger stick under his
arm. The six rows he had are the same six.

- Of the tall leaning hat of the first draft: "No, redo the general hat, Just
  a normal officer hat". It is a peaked service cap.
- "He needs more badges and metals on his coat." He has sixteen ribbons,
  seven medals, two orders, pins along his sash and stars on his collar.

His nose is a part of its own, in front of his far eye; the top of that eye
shows over it ("a little bit of the eye peaking out so you can see it").

Open: Jon has seen the first draft only, not the cap and the medals. His
`standing_height` is as it was (69.7, the cap included). He has no row that
faces the viewer, and no strike rows for when he is provoked (he had none).

## The burning flying shark: a great white, more on fire (2026-10-13)

Jon, with a picture of a pale shark under a sheet of fire: "we have to make
the burning flying shark look less like an orca ... and we need to make it
look even more on fire with good fire animation."

- It is pale grey over white with a black eye. It was dark over pale.
- The fire is one body of flame. The first try put one small tongue at many
  places; Jon: "too same-y ... We shouldn't just slapdash the same small fire
  piece around. It looks tiled." It is four manes now: each is a whole
  stretch of fire drawn as one picture, with tongues of different heights and
  leans on one mass of root, each tongue on a beat of its own. The fins and
  the tail keep single tongues, in three sizes, and small flames come away.
- Its body box is authored in its target (`BODY_BOX`), not measured from the
  picture: the game sizes the shark and seats its rider from that box, and
  the fire moved it. Its catalog row states `standing_height` (56, the size
  it had as a mount), so it has that size in the Hall too ("should also have
  a bigger size in the hall"); a stand's small box had made it tiny there.

Open: Jon has not judged the manes. A row has 6 or 8 frames, so the fire has
that many pictures in a loop. The saddle and the tack are in the fire and do
not burn. In the Hall the shark is as wide as its stand, and its fire goes
over the next stand a little.

## Not done

- **The giant gnu, redrawn: in the game, with the fight of today** (Jon,
  2026-10-11). A giant that is like a gnu and is not one, almost a centaur
  (a beast on four long legs, a chest that stands up at its front, two
  shoulders with an epaulet on each); the horns of a gnu; its back is a
  slope from a low rump to the left shoulder. Its face has no brows and
  shows no teeth, and its parts make its expression. The drawing, its poses
  and its rows are the target's own module in the renderer
  (`targets/characters/gnu_ton_boss/giant_art.py`); `scripts/svg_art/
  giant_gnu_art.py` draws one pose of it to look at. It is larger than the
  first giant: its body is 630 x 464 and the tips of its horns are 760
  above the floor (the first one was 415 tall). The arena's stones moved
  for it, and the hall's row of giants is taller.
  - Moss is where an actor can stand, and the game agrees: the slope is ten
    steps of one-way ground and each horn has a ledge
    (`mount.ledges` of `npc_giant_gnu`). NOT YET TRUE TO THE PICTURE: the
    slope is steps, so a body goes up it in hops and does not walk it. A
    mount's ground is flat blocks; a slope that moves with a mount is work
    for the fight's rework.
  - The sheets have NO ARMS: the fists are bodies of their own, as in the
    fight of today. The arms, the stance `reared` as a stance of the fight,
    and the portals are the fight's rework (`bosses.md`, "GNU-ton — a giant
    with arms, and portals").
  - Its faces: `ordered` at rest (what the sheets draw: it is ridden), and
    `free`, `smile`, `serious`, `angry`, `dazed`, `sleepy` for when it
    reacts. The rows use `angry`, `serious`, `dazed` and `sleepy`. `free`
    (the giant on its own) is not drawn by a row yet.
  - The candidates and the concept animations of the fight are in
    `untracked/giant-gnu-candidate/` (not in git).
  - What was done the cheap way to get it in, and the design that should
    replace it, is in `../engine/the-art-says-where-a-body-stands.md`.
- **A bird for the giant gnu.** Wanted later (Jon, 2026-10-11): a small
  bird that rides the giant. It is NOT a part of the giant's sprite: it is
  to be an actor of its own, with its own simulation (it sits, it moves, it
  flies off). Not started, and not to be started with the redraw.
- **Characters.** Started 2026-10-10 with the pirate crew: see "Characters
  and props, 2026-10-10" below. The other targets (about 160 of them, on a
  rig pipeline) are as they were. A redesign is made in place on its target,
  and the two looks are one entry in the design history of the renderer
  (`tools/ambition_sprite2d_renderer/design_history/lineages.yaml`).
  To see the whole cast as the game draws it, photograph the hall of
  characters (138 of them, in rows; 36 captures, about 4 minutes):

  ```bash
  for y in 230 614 998 1382 1790 2174 2558 2942 3470; do
    for x in 284 852 1420 1764; do
      AMBITION_QUALITY_PROFILE=ultra xvfb-run -a target/debug/capture_scene \
        hall_of_characters $x,$y hall_${y}_${x}.png 1280x720 --warmup 120
    done
  done
  ```
- **Ambience sound.** The game has no ambience channel. Twelve beds to
  listen to are in `untracked/sfx-candidates/room_ambience/` (not in git),
  with a reel and a script that makes them again. They wait for Jon's ear:
  nothing plays them.

## For Jon to decide

- **The lowest tier draws no parallax at all** (`potato.parallax.enabled` is
  false, and a test pins it). So with shaders off a room has its skin, its
  doors and its decor, and a black sky. One sky layer is one sprite of a
  256 px texture. If "levels should look good with shaders off" means that
  tier, it should draw the sky layer, and perhaps the far one.

## To look at first in a window on a GPU

None of this was seen in a window. A capture is one still frame from a
software rasteriser, so it says nothing about motion, bloom or a seam that
comes and goes.

- **Seams.** The fill of a block, the water and the trims are many sprites
  that meet edge to edge (`anchored_pieces`). Look for a thin line between
  two pieces when the camera moves slowly, most of all in the part-clear
  ones: water, a soft blink wall, the danger fill of a hazard.
- **The light pools and bloom.** A pool is a plain sprite at an alpha of 0.4
  or less. If bloom makes the lanterns of the forest or the braziers of the
  boss rooms too bright, the strength of each is in `LIGHTS` in
  `terrain/decor.py`.
- **Motion.** The motes (count, speed and size are `AmbientMoteStyles`), the
  foreground panel when the camera goes along a room.
- **The first frame of a room.** The dressing is in the load manifest of the
  room now. In a capture the parallax and the dressing are still counted as
  drawn after the cover, because a capture has no cover: look at a real room
  transition.
- **The alarm rooms after the wake room.** The change from the calm lab to
  the dark red one is the point of that biome. If the story wants the raid
  to start later, the three rooms name the theme in `intro.ldtk`
  (`parallax_theme`).
- **The lower gate stack is the choice I am least sure of.** It has a label
  "lab ruins - collapsed", and I gave it `alarm` for that. Its other labels
  are those of a gate station that works (a delayed gate, tolls), and it is
  the room that joins the under-town to the hub and to the sky arena. If it
  is a station and not a part of the raided lab, `lab` or `hub` is nearer.
  The combat calibration lab and the first system boss are `lab`, as they
  were.

## Not seen

Each item above was looked at in `capture_scene` captures only. A window on a
GPU was not seen.

The rooms of the other games were looked at through their routes
(`capture_scene --route sanic_gameplay`, `mary_o_gameplay`, `smash_gameplay`,
`versus_gameplay`), one view of each: they keep their own ground and take the
new scenes behind it. The Sanic rooms name a theme, so a one-way platform
there takes the skin of that theme.
