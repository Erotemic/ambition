# Track M — Super Mary-O (classic platformer acceptance demo)

**2026-10-26 synchronization scope:** Multiplayer, if selected for this game, is **one level**
with a required-all-participants finish gate; do not require persistent-world multi-room
replication. Compare level-level lockstep and bounded rollback as separate optional
synchronization choices rather than inheriting the flagship GGRS configuration. See [ADR
0036](../../adr/0036-synchronization-is-an-optional-game-capability.md).
Parody-original classic tile platforming: a teach-by-play opening, pipe secrets,
powerups, enemies and a flag sequence, without copied art or layout.

## Purpose

Prove that the conventional axis-swept AABB platformer is a simple customer of
the same engine that supports momentum and relativistic mechanics. Classic
behavior is authored through engine seams, not through a privileged "normal
game" path.

Engine capabilities it stresses: the `AxisSwept` motion model and its authored
tuning; item and equipment pickups (equip on touch, forms, armor rows); posed
body geometry (`BodySource::SpriteAuthored`, `posed_body_geometry`); durable
block-contact identity (breakable bricks, ?-blocks); room transitions
(`ambition_platformer2d_runtime::room_transition`) and `transit_body` warps
(ADR 0024); forward-only camera policy; the provider-declared HUD seam; the
shared economy; LDtk authoring of levels, loading zones and pipes.

## Status

Built: `game/ambition_demo_mary_o` (content) and `game/ambition_demo_mary_o_app`
(standalone shell).

- World 2 (2-1 to 2-4), each level from a script of
  `game/ambition_demo_mary_o/tools/` (`author_mary_o_2_<n>_ldtk.py`): 2-1 the
  desert (dunes, an oasis, a pyramid, a ferry), 2-2 a tomb under it (lifts,
  bars of fire), 2-3 mesas over a fall, 2-4 a castle whose end is the switch
  and the bridge of 1-4. The circuit is 1-1 to 1-4, 2-1 to 2-4, and 1-1 again.
  The levels are not played by a person yet. A script has `--check`, and
  `--apply` does it first: `way_through` in `mary_o_level.py` looks for a way
  from the start to the goal by jumps of a safe size. It finds a pit that is
  too wide and a wall that is too high, and no more than that.
- The desert has a biome of its own, the authored `"stardune"` theme:
  the scene is `backgrounds/outdoors.py::stardune` of the sprite renderer, with
  a row in each terrain table. Its music is the cue `sparse_theme` ("Sparse
  Theme"), named by the level field `music_track` of 2-1 and 2-3.
- Four levels (1-1, 1-2, 1-3, 1-4) in the Mary-O LDtk world of the
  `ambition_map_assets` submodule, read through `authored_levels`. Each level's fields
  state its game mode; stone colors are level or `Solid` fields. Rust still
  dresses blocks by name in `dress_authored_blocks` (pipes, the pole and hidden
  blocks draw as nothing). `test_course.rs` builds a synthetic room in code on
  purpose. Do not grow the programmatic path. A missing authoring concept goes
  into LDtk and the tooling.
- The warp room (`mary_o_warp`, `tools/author_mary_o_warp_ldtk.py`) is a
  secret: a row of pipes, one to the start of each level. The way in is the
  sky of 1-1, an engine `LoadingZone` with a landing zone in the warp room.
  The file has the zone on the top line of the level, and the game puts it
  five tiles over the level (`lift_the_sky_zones`): she must go up out of the
  view to be in it. The game writes the name of a level over its pipe, dark
  with a white edge (`pipe_signs.rs`). A
  pipe whose `role` is `Warp` has no far pipe: its `link` is the id of the
  level (`leave_through_a_warp_pipe`). A new level gets a pipe in `LEVELS` of
  the script. The level circuit does not go through the room
  (`side_room_ids`). No loading zone of Mary-O has a picture.
- World 1-2 is 240 tiles long (it was 60), built by
  `game/ambition_demo_mary_o/tools/author_mary_o_1_2_ldtk.py`: one function
  for each part of the level, `--preview` prints it as text, `--apply`
  replaces the level in the LDtk file. Its parts are in the header of
  `level_1_2.rs`. The room is as high as the corridor and an attic over the
  roof; a `CameraZone` (`clamp_mode: zone_bounds`) holds the camera on the
  corridor. The way into the attic is the lift that goes up through a hole in
  the roof; its pipe comes out before the ferry.
- A moving platform has art: the clockwork lift (`super_mary_o_lift` in the
  sprite renderer). A `MovingPlatform` names it in its `visual` field, and
  `scenery.rs` registers its `PlatformLook`. A lift in a line is `one_way`, so
  the rock that stops its rider does not crush the rider.
- World 1-3 is a level in the sky (208 tiles; it was 64, with one pit and
  one ferry): pillars, three ferries, a climb of shelves with a low road
  under it, a bridge of bricks, and steps down to the pole. World 1-4 is the
  castle (178 tiles): bars of fire over pits, a low hall, a ride under a bar,
  two plants, and a bridge with the boss on it. The castle has no pole (Jon,
  2026-10-11): a switch is past the bridge (`castle_end.rs`, the block
  `goal_switch`), and a wall with a gate in it (`goal_gate`) is past the
  switch. She takes the switch when she lands on it. Then the end of the
  castle is a sequence of beats (Jon, 2026-10-12; `castle_end::CastleBeat`, a
  phase of the level sequence): the button goes down, charges go off on the
  bridge, the planks break one by one, the lava takes the Void Lizard, the
  gate opens, she walks to the robot (`mary_o_robot`, the sheet of
  `player_robot_v3`) who looks at her, his line is on the screen for four
  seconds (a cutscene of the engine, a timed banner, no button), the level
  waits two seconds, and it goes. The view of the bridge starts on its first
  plank, so the camera stays with her until she is over the first post.
  Each has its script beside 1-2's (`author_mary_o_1_3_ldtk.py`,
  `author_mary_o_1_4_ldtk.py`); the tile language is `tools/mary_o_level.py`.
- Enemy kinds added 2026-10-11. Each cycle reads the simulation tick, so
  none has rollback state of its own.
  - Snake Plant (`plant.rs`): a snake that wears leaves, in a pipe whose
    `role` is `Sealed` (a pipe that goes nowhere). It comes up and goes down,
    and stays down while a player is on its pipe or beside it. A stomp on it
    is a hit.
  - Laden Swallow (`swallow.rs`): the African swallow's sheet. Its row is the
    flyer the planes are, which stays over the player; the rule lets a
    coconut go straight down on its tick. It is squashed as a walker is.
  - Void Star (`void_star.rs`, data only): a small star of the void that goes
    along the ground on its points. No stomp hurts it.
  - Bar of fire: a row of `DamageVolume`s, each on a ring of straight path
    parts about one pivot (`Level.fire_bar` in the tile language). A damage
    volume of a Mary-O room is drawn as the `fire_orb` entity sprite
    (`scenery::dress_fire`).
  - Void Lizard (`void_lizard.rs`, a working title): the boss of 1-4. An
    enemy body, not a boss of the engine's boss encounter: a large walker
    with the health of five sparks that throws an orb along its bridge and
    hops. Its bridge is `MaryOBlock`s of kind `Plank`, and all the planks of
    a room fall when its goal is taken (`bricks.rs`), so the boss falls too,
    one plank after the other from the switch, each with an explosion. She
    waits on the switch while it falls, the lava takes the boss down with
    splashes (`castle_end::the_lava_takes_the_boss`), and one view that does
    not move holds the bridge, the switch, the gate and the robot
    (`Level.arena`).
- Her fireball is the spark blossom (`super_mary_o_spark`, a sheet). A fire
  has a glow (`fire_shader.rs`): a shader that moves an authored flame
  picture through a soft light behind the spark and behind each orb of a bar
  of fire. A device with no budget for shaders draws no glow.
- A void thing has a field about it (Jon, 2026-10-11): the picture loses its
  colour, and a part of its light, near a Void Star, the Void Lizard and each
  orb of the void. The field is the engine's
  (`crates/ambition_render/src/void_field.rs`: a `VoidAura` on a visual, and
  a whole-screen pass on a camera only while an aura is in its view);
  `void_look.rs` says which visuals of Mary-O have one. A device with no
  budget for shaders draws no field.
- The castle has a biome of its own, `void_castle` (Jon, 2026-10-11: red
  lava pits and red bars of fire, so the field of a void thing has colour to
  take). The story: the void took the castle, and Mary-O fights it with the
  powers of a magical girl. Its parts:
  - The scene, the stone and the dressing are rows of the art tables of the
    sprite renderer (`backgrounds/outdoors.py::void_castle`, `terrain/`): a
    hall of plum stone with coloured glass and red banners, and the void
    (black, with a white edge) in its vault. The void is not drawn in the
    air where the play is: a black thing with a white edge there is an enemy.
  - Lava: `Level.lava` of the tile language makes a damage volume whose name
    starts with `lava`. The game draws it with two sheets
    (`super_mary_o_lava_top`, `super_mary_o_lava_body`; `scenery.rs`), in
    front of the cast, and hides the engine's picture of the hazard. Lava is
    a hit, and the fall under it is the death, as a pit was.
  - The quasar is a ward (`VoidWard`, `void_look.rs`): near Mary-O with the
    quasar the field of the void takes nothing. A quasar is in a brick at
    the start of the garden of 1-4.
- Eternal Cosmic Mary-O (Jon, 2026-10-11): her ultimate form, and today the
  tool a tester moves through a level with. The Utility control (the `d` key)
  puts her in the form and takes her out of it (`cosmic.rs`).
  - The form is the catalog row `mary_o_cosmic`: she is not hurt and she
    harms what she touches (`empowered`), each press of jump in the air is a
    jump (`AirJump`, and 255 air jumps in the `cosmic` tuning), and she is
    faster. The form is off her ladder of forms: she is in it while she wears
    its row, and she comes back to the rung her equipment gives her.
  - Her sheet is `mary_o_v2_cosmic` of the sprite renderer, drawn on her v2
    body. A jump is one beat of her wings. The row `transform` is the
    transformation from her fire form, 36 frames; the game holds her in it
    for as long as it takes to draw, and a second press ends it.
  - Light is not in her sheet: the glow with stars behind her is a shader
    that reads the starlight shimmer picture (`cosmic_aura.rs`). The engine's
    spark particles are squares and were taken out; a star particle is not
    made. She has the ward against the void at all times in the form
    (`void_look.rs`).
  - Three layers of light (Jon, 2026-10-11): the glow under her, a barrier on
    the border of her body, and the overlay of the quasar over her while she
    holds a quasar (not from the form alone). The barrier is about her hair
    and her figure and not about her wings: she is drawn from parts in the
    form (each row but `transform`), and the part renderer says where the
    parts of her body are (`cosmic_barrier.rs`, `PartSetRequest`).
  - A hit stops the world for 0.02 s at most in the form
    (`cosmic::shorten_her_hit_stop`).
  - Her theme is the cue `eternal_cosmic` of the music renderer: an intro as
    long as the transformation, then a loop of forty bars
    (`cosmic::COSMIC_MUSIC_TRACK`). The castle plays `castle_crash_override`
    (its score is in `scores/active/` of the music renderer, as all scores
    are), by the level field
    `music_track` of 1-4.
  - Not decided: what gives her the form in the game (today each player has
    it at all times), a transformation from each of her other forms (the one
    row starts from fire), and a clip for her way out of the form.
- Physics: one `classic` entry in `axis_tuning_presets`
  (`data/character_catalog.ron`), which every form names. Launch bands are
  offsets on `jump_speed`, so one ground-jump height authority exists, and a
  running jump is her highest.
- Forms: small, wand (grow-cap armor row, tall worn identity, feet-planted
  resize) and beacon (spark-blossom ranged move). A taller form refuses a weaker
  pickup.
- Enemies: crony walkers with shared stomp; a brainless sliding shell
  (`StandStill` brain; a stomp or a touch launches a resting shell and stops a
  sliding one); the Solid Snake, whose collision box comes from the sheet per
  pose.
- Flag: contact-height scoring, slide, walk-off, tally, dwell, course-clear
  sting, and level cycle. Multi-coin blocks pay through the shared economy and
  emit `VfxMessage::CoinPop`.
- HUD for score, coins, time and lives; a centred title/results card in one
  declared HUD slot.
- Camera: one-way forward scroll + no-backtrack clamp, as an authored
  `CameraScrollPolicy`.
- Acceptance run: `level_1_acceptance.rs` plays spawn to pole with one
  state-aware controller and no positional set-up: ?-block powerup, secret pipe
  and vault, all three pits, pole, tally, replay, with all three lives kept.
  `two_rooms.rs` crosses into 1-2 by the real DOWN verb, rides the ferry, and
  keeps the earned form across the transition.

Standing rules:

- ⛔ Zero `coyote_time` and zero `jump_buffer` are deliberate (Jon). The classic
  games grant no ledge forgiveness and no pre-landing buffer. Do not "fix" them.
- ⛔ Lives go negative and play continues forever (Jon: *"for now, let's allow
  lives to go negative ... so no game over screen yet"*). There is no run
  restart.
- Charge a life from `ActorDiedMessage`, the engine's attempt-lost fact. Do not
  use `BodyLifetime.resets`: a room replay also bumps it.
- A reset restores engine-owned state only. Body size and facing come from their
  authority (`ResetFacing::{Keep, Toward}`), not from defaults.
- A `WorldItemArt` id that names a missing texture fails silently: the item is
  collectible and never drawn. Every item art id needs a generator target.

## Ownership

`ambition_demo_mary_o` owns its levels, rules, lives, score, coins, timer,
equipment rows, enemy and content rows, shell prop, flag sequence, HUD, title
and results. A need found while authoring becomes engine work only when it is a
reusable missing seam.

## Open acceptance

This list is the single source. [`status.md`](../status.md) and
[`tracks.md`](../tracks.md) refer here.

- **Beacon discoverability (Q46, ruled 2026-10-04 in
  [`../maintainer-decisions.md`](../maintainer-decisions.md)).**
  A grown Mary-O can bonk a ?-block and get the beacon
  (`a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`), but the
  level does not show the player how. The first ?-block always pays the wand to
  a small Mary-O. The beacon does not walk to the player
  (`ItemMotionPlan::still()`). 1-1's third ladder block, at x=1920, stands over a
  pit, so no body can bonk it from the ground. Ruling: add a fourth reachable
  `Toward(Lantern)` ?-block over solid ground in 1-1 (keep the pit block). This
  is a level-content fix; the engine does not compensate (no item that walks
  to the player only for this, no bonk-from-above exception). 1-1 is
  hand-edited in `game/ambition_map_assets/ambition_demo_mary_o/worlds/mary_o.ldtk`;
  check the edit with the semantic LDtk comparison (queue LDTK-SEMANTIC-DIFF)
  so that the diff shows one added entity and no other change. Acceptance: the
  fire arm of `weaker_form_refusal.rs` (absent today for this reason) runs in
  1-1 from the ground.
- **The world, made larger (Jon, 2026-10-11).** 1-1 stays as it is. Done:
  1-2, 1-3, 1-4, and the enemy kinds above. Not done:
  - Jon has not played 1-3 and 1-4, and no test plays a level to its end.
  - The boss is a candidate. Jon on the first two: no fire lizard (it looked
    like another game's), not a name near to that game's boss; something that
    evokes the void, white highlights, very desaturated. "Void lizard is a
    working title." The Void Star was the second candidate for the boss, and
    Jon kept its name for a small enemy.
  - No enemy has a sound of its own, and the coconut and the orb have none.
  - A level has no checkpoint: a death at the end of a level is a start from
    its first column. Not asked for; levels of this length show the need.
- **1-2 is hard for a first world (Jon, 2026-10-11, after he played it).**
  "Pretty good. Some of the jumps are a bit technical for an early world, but
  I think that's fine for now. We can always make that a world 2-2 or
  something later and swap in an easier 1-2." No test plays it to its end:
  its tests are about its shape (the ferry reaches the two lips of its pit,
  no block is in rock, the roof is closed, a lift wraps out of the room).

## How to run

```bash
./run_game.sh mary-o                         # windowed standalone shell
./run_game.sh mary-o --headless -- --ticks 600
cargo test -p ambition_demo_mary_o -p ambition_demo_mary_o_app
cargo run -p ambition_demo_mary_o_app --features capture --bin capture_mary_o \
    -- OUT.png [WIDTHxHEIGHT] [--room ID] [--warmup N] [--walk N] [--at X,Y] [--no-ui]
```
