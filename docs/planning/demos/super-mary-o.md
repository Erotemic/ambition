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

## A PROPER Mary-O (2026-07-15, Jon) — EXECUTING

Jon: "let's do the maryo content." Building the full proper-Mary-O, not a thin
slice. **The movement-seam question is RESOLVED against code** (three scout
passes over the kernel, the enemy/stomp vocab, and the powerup seam):

> **Every "Mario move" is already a kernel primitive gated by an `AbilitySet`
> flag or a tuning knob** — variable jump, double/triple jump (`air_jumps`
> count), wall jump, wall cling, fast fall. So the VERBS live in the kernel
> (reusable) and Mary-O enables them by **appending grants** — zero kernel code.
> The FEEL that legitimately varies per character (the air-jump count, a bespoke
> arc) is **tuning**, composed on the catalog exactly like Sanic's `momentum`.
> The only genuinely new mechanic is the ground pound (which turned out to BE the
> fast-fall grant). The stomp is a demo rule over head-contact (NO pogo, per Jon);
> the ? block powerup needed two small reusable engine primitives — see E below.

That answer decides the whole build: it is mostly CONTENT + composition, plus one
real engine keystone (per-character tuning). Slices, each compiling + tested +
committed:

- **A — moveset grants ✅ LANDED (2026-07-15).** Three single-verb `AbilityGrant`s
  (`AirJump`, `WallMobility`, `FastFall`) — the finer-grained bundles the grant
  vocab always anticipated. Mary-O's list is now
  `[RunJump, AirJump, WallMobility, FastFall]`: run + variable + double jump, wall
  cling/kick, fast-fall dive, still none of the Ambition kit. Zero kernel code —
  the verbs already existed as `AbilitySet`-gated primitives.
- **B — per-character axis tuning (KEYSTONE) ✅ LANDED (2026-07-15).** The tuning
  sibling of the ability-composition keystone. `ae::AuthoredMovementTuning` marks
  a body whose feel is authored, so its live axis params come from the row, not
  the global F3 dev tuning (an axis body now escapes that refresh the way a
  `SurfaceMomentum` body already did). Catalog gains `axis_tuning:
  Option<AxisTuningSpec>` (serde-twin/`to_kernel`, mirroring `momentum`). Mary-O
  authors `axis_tuning: Some((air_jumps: 2))` → her `AirJump` grant is a TRIPLE
  jump, without touching Ambition's protagonist. Poison-tested. This is the
  documented resolution of the seam question: **verbs are kernel grants; feel is
  per-character tuning.** The Mario ARC itself stays at the blessed default
  (Jon: "that is the right sort of jump physics") — no feel-tuning pass, per the
  land-architecture-not-feel directive.
- **C — asset-root parity ✅ LANDED (2026-07-15).** Standalone rendered no sprites
  because its `AssetServer` file root was the cwd `assets/`, not the engine's
  `crates/ambition_actors/assets`. A shared `actors_desktop_asset_root()` umbrella
  helper both apps set on the builder, so standalone and hosted cannot diverge.
- **D — the goomba + stomp ✅ LANDED (2026-07-15).** The `ai_slop` sprite as a
  1-HP `Wanderer` walker (demo roster fragment so it has its sprite standalone AND
  hosted), spawned via `SpawnActorRequest` on room load. Head-stomp is a demo
  RULE, and per Jon **NO pogo**: a descending player bounces (`set_jump_velocity`)
  and squashes the goomba (health zeroed that frame, ordered before the shared
  body-contact-damage pass so a stomp never also hurts the stomper); a side touch
  is the engine's existing contact damage.
- **E — reactive blocks + WorldItem + grow powerup ✅ LANDED (2026-07-15).** Jon
  chose the elegant-engine path over demo glue: TWO reusable engine primitives,
  then the powerup as pure content on them.
  - **Reactive blocks.** `ContactSource::Block` now carries the struck block's
    durable `GeoId` (it lived on `Block`; the contact threw it away). Gameplay can
    answer "*which* authored block did I touch" without point-matching. `GeoId`
    owns a `String`, so `Contact`/`ContactSource`/`SupportFact` are now `Clone`
    not `Copy` — a three-derive + two-clone ripple; whole workspace compiles.
  - **`WorldItem`.** A walk-into collectible granting EQUIPMENT — the sibling of
    `GroundItem` split along the collect trigger the pickup module's own review
    note anticipated (touched, not pressed; equips an A3 row via the ONE
    `equip_equipment_row` contract, its first live caller). Full sim-view→visual
    mirror.
  - **The powerup, pure content.** A head-bonk on a ?-block (matched by its
    `GeoId`) pops a milk `WorldItem`; touching it equips `grow_cap`; the tall
    SHEET (`super_mary_o_tall`, a distinct sheet per Jon — "small and tall have
    different sprites") + taller collider are a pure VIEW of wearing the cap
    (feet-anchored grow/shrink, no pushout); a hit spends the cap via the shared
    armor pass → shrink. The equipment state drives both directions, no manual
    revert.
- **F — ground pound + level ✅ (no new code).** Ground pound in this engine's
  vocabulary IS the `FastFall` grant, which `mary_o` carries (Slice A) and the kit
  test asserts is lit; the descent + the goomba stomp cover the mechanic. The
  level authors the ?-blocks at the teach-platform jump height, goombas on the
  flats, and the pit/stair rhythm.

**Bar:** *"it needs to be a proper Mary-O demo"* — met: Mario jump physics
(variable/double/triple/wall/fast-fall), a stompable goomba, and a
struck-block→milk→small↔tall powerup, all together.

**Follow-ups (noted, not blocking):**
- The milk `WorldItem` draws a tinted quad; a real `super_mary_o_milk_carton`
  sprite is generated in the renderer submodule — wiring it needs the prop-sheet
  render path (item_visuals draws a single image, not a sheet frame).
- **Brick-break** is now a cheap SECOND reactive-block consumer (Head/Support
  contact vs a breakable `GeoId` → remove the block) — the primitive's reuse
  proof, deferred because removing a block mid-run is a `World`-mutation slice.
- `WorldItem` has no locomotion yet (resting collectible); a sliding mushroom
  wants the free-body integrator (add when a second moving pickup lands —
  design-balance).
- The goomba squash zeroes health + despawns (bypasses the engine death/score/
  drop path) — a minor smell, fine for a 1-HP walker.
- Republish the hall of characters with everyone (heavy `regen_sprites.sh` run).
- Exit-to-title-quits-standalone (deferred: risks the launcher-relaunch flow the
  OV1 lifecycle tests pin).
