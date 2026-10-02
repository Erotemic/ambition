# Track M — Super Mary-O (classic platformer acceptance demo)

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

- Three levels (1-1, 1-2, 1-3) in the Mary-O LDtk world of the
  `ambition_map_assets` submodule, read through `authored_levels`. Each level's fields
  state its game mode; stone colors are level or `Solid` fields. Rust still
  dresses blocks by name in `dress_authored_blocks` (pipes, the pole and hidden
  blocks draw as nothing). `test_course.rs` builds a synthetic room in code on
  purpose. Do not grow the programmatic path. A missing authoring concept goes
  into LDtk and the tooling.
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

- **Beacon discoverability (Q46 in
  [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md)).**
  A grown Mary-O can bonk a ?-block and get the beacon
  (`a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`), but the
  level does not show the player how. The first ?-block always pays the wand to
  a small Mary-O. The beacon does not walk to the player
  (`ItemMotionPlan::still()`). 1-1's third ladder block, at x=1920, stands over a
  pit, so no body can bonk it from the ground. This is a content-layout choice
  for Jon.
- **Further authored levels** beyond 1-3.

## How to run

```bash
./run_game.sh mary-o                         # windowed standalone shell
./run_game.sh mary-o --headless -- --ticks 600
cargo test -p ambition_demo_mary_o -p ambition_demo_mary_o_app
cargo run -p ambition_demo_mary_o_app --features capture --bin capture_mary_o \
    -- OUT.png [WIDTHxHEIGHT] [--room ID] [--warmup N] [--walk N] [--at X,Y] [--no-ui]
```
