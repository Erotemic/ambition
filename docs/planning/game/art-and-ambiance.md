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

Hylbert's Hotel has a haunted wing through the corridor's second door.
Its seven rooms use the published hotel props. Eigenghosts, Decomposing
Specters, Spectral Perturbations, and Shift Operators occupy the route.
The service lift opens from the far end. The Gram Reaper guards the treasury;
its seal reads the boss placement's saved defeat state. Quantum Dice are
authored separately and are not placed in this route yet.

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

## Art system state

The October biome/art run completed the parallax sizing, biome terrain
skins, foreground layers, motes, light props, portal-capture look and
room-cover integration. The dated room counts and checked-off build log
are in Git history. Keep the live art rules above. Ground shadows remain
opt-in (`GroundShadowRooms`); no game selects them because they conflict
with the desired look.

- [ ] **Character art beyond the completed pirate crew:** review the current
  cast against the authoring inventory before starting a new redesign.
  See the remaining character work under **Not done** below.

## Character and prop art: standing rules and open work

- Do not bake ground shadows into sprites. The pirate crew and many entity
  props were redrawn during the October run. The old checked receipts,
  before/after descriptions and per-room census are in Git history.
- Keep character props integral to the rig only when they are part of the
  character; coconuts are held world items, not part of a swallow sprite.
- New bird rows reuse the established anatomy/publishing pipeline. A bird
  with visibly different sexes has distinct authored variants.
- Mary-O rooms use `starlight`, `starless` and `void` biomes with legible
  foreground/play separation. The room has one biome; an embedded vault
  does not choose another biome on its own.

- [ ] **Remaining entity prop redesigns:** heart, coin, switches, morph ball,
  save point, doors and tiles. Check the latest assets before selecting an
  art pass; this is the old October inventory, not a fresh defect report.
- [ ] **Coconut lifecycle:** a palm must grow/replenish coconuts so a taken
  ground item can return. The existing swallow carries and drops a coconut;
  taking one directly from a palm or from the air and equipping the European
  swallows remain optional follow-up behavior, not completed work.
- [ ] **Launcher presentation:** the earlier backdrop/menu work landed, but
  a game-specific character, wordmark and a layout beyond the flat menu were
  explicitly left unfinished. Revalidate against the current launcher before
  promoting a presentation redesign.

For completed implementation details use the renderer, biome sources,
`docs/systems/parallax-backgrounds.md` and Git history.

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
