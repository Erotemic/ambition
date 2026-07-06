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

**Owns (`mary_o_content`):** the 3-level world, the level-rules plugin
(lives/score/coins/timer, level-restart death policy — mode-scoped),
powerup equipment rows (mushroom-analog: size + one-hit armor;
flower-analog: `simple_ranged` grant), goomba/koopa-analog rows + the
shell PROP (a brainless sliding hazard both sides can trigger —
actors-vs-props exercised), the flagpole sequence, HUD, title/results.

**Engine prerequisites:** E5-finish ✅ (landed 2026-07-06 night); A3 (equipment→params — design PINNED in
[combat-model.md](../engine/combat-model.md) §8 "A3 design"; M1 is its
adjudicated consumer). Expected oracle-violations: the one-way forward
scroll clamp (a `CameraZoneSpec` policy knob).

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
- **M2 camera scroll policy:** one-way forward scroll + no-backtrack
  clamp as a `CameraZoneSpec` extension (authored data; the knob is the
  oracle-violation to file).
- **M3 level-end sequencing:** flagpole grab → slide → walk-off → score
  tally on the cutscene kit + `RoomLoaded`/gate vocabulary; pipes =
  LoadingZone Door activation (exists).
- **Enemies:** goomba-analog (walker, stomp-kill via pogo/on-hit
  vocabulary), koopa-analog (stomp → shell prop that becomes a sliding
  hazard-projectile both sides can trigger — the actors-vs-props
  taxonomy exercised: shell is a PROP with velocity + hurt-on-contact,
  not a brained actor).
- **Mode state:** lives/score/coins/timer — mode-scoped resources per
  the doctrine; death → restart level (the respawn-policy work makes
  "level restart" an authored policy, not the engine default).

## Slices

M1 equipment chain [opus] (with A3); M2 scroll knob [opus]; M3 flag
sequence [opus]; M4 the game (3 levels + enemies + HUD + title/results)
[opus]; M5 hosting wing in ambition (Phase D-C) [opus].

**Exit:** doctrine exits + the classic-specific one: an input script
speedruns 1-1 headlessly; warp-pipe secret reachable; and the whole demo
authored withOUT touching the momentum kernel (proves the two movement
identities stay independent).
