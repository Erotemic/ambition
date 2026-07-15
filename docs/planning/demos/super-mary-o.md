# Track M — Super Mary-O (classic tile-platformer acceptance demo)

Inspired by SMB1 world 1-1. Parody-original (Mary-O, a plumber of
gradient descent; original tiles/sprites/layout in the 1-1 grammar:
teach-by-play opening, pipe secrets, a flag).

**Purpose:** prove the CLASSIC baseline — the axis-swept AABB path, tile
worlds, powerups-as-equipment, scroll policy, and level-end sequencing —
is pure data + content on the finished engine face. This is the "any
1980s platformer is a weekend of authoring" proof.

## Consumes (by role) / Owns

**Consumes:** [the sim assembly]+[the windowed host] · [the movement
kernel] (axis-swept AABB ONLY — the demo must not touch the momentum
path) · [the sim heart] (spawn, body size seam for the mushroom) ·
[the stuff kit] (equipment rows + the armor-instead-of-HP policy) ·
[the combat resolver] (stomp via pogo/on-hit vocabulary) · [the space
IR]+[the LDtk backend] (its own 3-level world; pipes = LoadingZone Door)
· `ambition_cutscene` (flag sequence) · [the observation boundary] (HUD).

**Owns (`ambition_demo_smb1`):** the 3-level world, the level-rules plugin
(lives/score/coins/timer, level-restart death policy — mode-scoped),
powerup equipment rows (mushroom-analog: size + one-hit armor;
flower-analog: `simple_ranged` grant), goomba/koopa-analog rows + the
shell PROP (a brainless sliding hazard both sides can trigger —
actors-vs-props exercised), the flagpole sequence, HUD, title/results.

**Engine prerequisites:** E5-finish ✅ (landed 2026-07-06 night); A3
(equipment→params) ✅ **LANDED 2026-07-11** as `ambition_characters::equipment`
(see [combat-model.md](../engine/combat-model.md) §8). Expected
oracle-violations: the one-way forward scroll clamp (a `CameraZoneSpec` policy
knob).

## Design (v1 scope)

- **World:** 3 levels of one .ldtk world: 1-1 grammar (open teach,
  pit rhythm, secret pipe room, flag), 1-2 underground variant (ceiling
  crawl, different palette), 1-3 platforming (moving platforms — engine
  has them). Tile visuals via the existing tileset pipeline.
- **M1 powerups as equipment:** mushroom-analog = an equipment row that
  raises `BodyBaseSize` + grants one hit of armor (damage → lose the
  equipment instead of HP — an on-hit equipment policy through the ONE
  damage resolver); flower-analog = equipment granting a ranged prefab
  row (`simple_ranged` + params). Both are GroundItem pickups. THIS is
  A3's adjudicated consumer (numeric modifiers merge into params at
  trigger-resolve; behavioral overrides are components).
- ~~**M2 camera scroll policy**~~ ✅ **LANDED 2026-07-10.**
  `CameraZoneSpec.scroll_policy: CameraScrollPolicy` (`#[serde(default)]`, so every
  zone authored before M2 is byte-identical), applied in
  `resolve_follow_camera_snapshot` AFTER the bounds clamp — the watermark records
  where the camera SETTLED, not where it wanted to be. The watermark lives on
  `CameraEaseState` and is cleared on leaving the zone, so the clamp is
  **per-visit**: re-entering from the other side scrolls again rather than meeting a
  camera pinned to where it stopped an hour ago.

  Two rulings. `ForwardOnlyX` is the ONLY non-default variant — one shipped need,
  one variant; a `ForwardOnly { axis, direction }` generalization waits for a second
  consumer. And the axis is SCREEN `+x`, not gravity-relative, because a
  side-scroller's no-backtrack rule is a statement about the level's authored
  direction of travel, and rotating gravity does not rotate the level.

  The clamp **never eases backward to meet the watermark**: a camera that crept
  toward a high-water mark while the player stood still would be a bug that looked
  like a feature for exactly one playtest.

  The oracle-violation this slice was told to file is filed and closed in the same
  breath: the knob is a `CameraZoneSpec` field, which is authored data, so no engine
  code names a demo.
- ~~**M3 level-end sequencing**~~ ✅ **LANDED 2026-07-10.** Flagpole grab → slide
  → walk-off → score tally, in `game/ambition_demo_smb1/src/flag.rs`. **Zero engine
  code.** `step_flag_sequence(&mut FlagSequence, &FlagPole, body, dt) -> Option<Vec2>`
  is the whole thing: a pure function of state, geometry, and a clock, wrapped by
  one system that writes the result onto `BodyKinematics`.

  Three rulings.

  **The score is decided at the moment of contact, not read off the slide.** The
  points depend on how high you *caught* the pole; that is a fact about one instant.
  Deriving it from the live body position would let a player who grabbed high and
  slid fast score differently from one who grabbed high and slid slow — a bug that
  reads as physics.

  **`FlagSequence::driven` holds the position once the sequence takes over.** If each
  tick re-read the body, a gravity step landing between this system and the next
  would accumulate into the slide, and the slice's correctness would depend on
  system ordering. Pinned by
  `a_grabbed_sequence_ignores_whatever_physics_does_to_the_body`, which shoves the
  body a full tile every frame and gets the same score and the same landing.

  **DEVIATION, stated out loud: this does NOT ride the cutscene kit,** which this
  doc's original M3 line asked for. `CutsceneBeat` is `{Wait, Dialogue, CameraPan,
  Fade, SetFlag, Banner}` — a presentation script. It has no beat that moves a body,
  and adding one would be engine code written to serve a demo, in a crate whose
  timing then decides a gameplay score. The flag is a rules state machine, so it
  lives with the rules. The kit is still the right home for M4's *results screen*,
  which is presentation and nothing else.

  `goal_pole()` is the single place the flag's geometry lives; `level_1_1()` builds
  the block from the same constants. The `flag_geometry_oracle` that asserts they
  agree caught a real bug on its first run (a hardcoded tile size), which is the
  whole argument for writing it.

  Pipes remain LoadingZone Door activation (exists).
- **Enemies:** goomba-analog (walker, stomp-kill via pogo/on-hit
  vocabulary), koopa-analog (stomp → shell prop that becomes a sliding
  hazard-projectile both sides can trigger — the actors-vs-props
  taxonomy exercised: shell is a PROP with velocity + hurt-on-contact,
  not a brained actor).
- **Mode state:** lives/score/coins/timer — mode-scoped resources per
  the doctrine; death → restart level (the respawn-policy work makes
  "level restart" an authored policy, not the engine default).

## Slices

M1 equipment chain [opus] (with A3) — 🟡 **DATA + MECHANISM LANDED 2026-07-11**:
A3 shipped (`ambition_characters::equipment`) and `ambition_demo_smb1::powerups`
authors the two rows (`grow_cap` = size+armor, `spark_blossom` = ranged grant +
fire-time damage buff) entirely through the umbrella — zero engine edits, E9
oracle held a third time. The equip CONTRACT also landed
(`ambition::combat::moveset::equip_equipment_row`). REMAINING for M1: the powerup
PICKUP ENTITY + equip-on-touch system + level spawns + art, and the live
BODY-scale collision/render read-fold — the visible/feel half. ~~M2 scroll knob~~ ✅ **DONE
2026-07-10**; ~~M3 flag sequence~~ ✅ **DONE 2026-07-10**; M4 the game (3 levels +
enemies + HUD + title/results) [opus]; M5 hosting wing in ambition (Phase D-C)
[opus].

**Exit:** doctrine exits + the classic-specific one: an input script
speedruns 1-1 headlessly; warp-pipe secret reachable; and the whole demo
authored withOUT touching the momentum kernel (proves the two movement
identities stay independent).

## Proposed — "a PROPER Mary-O" (2026-07-15, Jon) — NEEDS A PLANNING PASS

Landed 2026-07-15 (session): real `super_mary_o` sprite, jump SFX, run+jump-only
kit, tiled surfaces (`Block::solid_tiled`), cyclic level (flag →
`RoomReplayRequested`). Kit authoring landed the **ability-composition** keystone
(commit 6875aeaea): Mary-O composes `abilities: Some([RunJump])`, a grant list
that unions into her `AbilityBase`; the F3 dev editable is now a MASK over the
base, not a wholesale replace — which also fixed a still-live bug where Mary-O's
kit was clobbered back to the full Ambition set every frame. Adding a Mario verb
below is now appending a grant, not forking a preset. That makes it a real slice;
Jon wants it to become a PROPER Mary-O. Requirements, captured to plan — the
movement split question below is the one that genuinely needs thought (candidate
for a Fable planning pass, like the shell-UX cluster):

- **Mario jump physics = the target feel.** "That is the right sort of jump
  physics." Mary-O is on the AxisSwept kernel path (NOT the momentum kernel Sanic
  uses). Variable-jump already exists (`AbilitySet::variable_jump`). Task: tune the
  AxisSwept params + gravity/air-control to the classic arc. **Open architecture
  question:** how much of "Mario movement" is the movement KERNEL vs a layer a
  SHELL higher? Jon: *"maybe some of it is in the movement kernel … wall jumps but
  not sure if that is movement kernel, might be a shell higher. Need to think about
  it."* Resolve this seam before building — it decides whether these verbs are
  engine primitives (reusable) or demo-local rules.
- **Wall jumps.** Kernel already has `wall_jump`/`wall_cling` abilities; decide if
  Mary-O's wall jump is that primitive or a shell rule (see the seam question).
- **Double / triple jump.** Kernel has `double_jump` (one extra air jump) +
  `air_jumps` tuning. Triple needs the air-jump count to go past 1 — check whether
  `air_jumps` already generalizes or needs a count knob.
- **Ground pound.** A down slam (down-air / fast-fall into a stomp). Likely a
  moveset move (`attack_air_down`-shaped) or a movement verb — depends on the seam
  question. Classic pairs with breaking blocks / stomping enemies.
- **Mushroom (the "milk" powerup) spawns from a block, small ↔ tall switch.** M1
  already landed the equipment DATA + mechanism (`grow_cap` = size+armor) through
  the umbrella. REMAINING (already noted in M1): the pickup ENTITY that emerges
  from a `?`-block, equip-on-touch, and the live BODY-scale collision/render fold
  (small↔tall Mary-O). Needs an elegant "spawn item from a struck block" seam.
- **An enemy.** Reuse the `ai_slop` enemy archetype as the goomba/koopa (stompable;
  classic head-stomp bounce + side-contact damage). This is M4's enemy leg.
- **Bar:** *"it needs to be a proper Mary-O demo"* — the above together, not a
  thin slice. Sequence after the movement-seam question is answered.
