# Track M — Super Mary-O (classic platformer acceptance demo)

Parody-original classic tile-platforming: teach-by-play opening, pipe secrets,
powerups, enemies, and a flag sequence without copied art or layout.

**Purpose:** prove that the conventional axis-swept AABB platformer is a simple
customer of the same engine that supports momentum and relativistic mechanics.
Classic behavior must be authored through engine seams rather than a privileged
"normal game" path.

## Current state

Landed:

- provider/demo shells, the authored level-1 room grammar, fixed-tick simulation,
  and the mode-scoped level clock;
- ?-block bonks that spawn real world-item pickups, with equip-on-touch through
  the shared item/equipment path;
- the grow-cap armor row, distinct tall worn identity, collider/body-size update,
  feet-planted grow/shrink behavior, and spark-blossom ranged move;
- breakable bricks through durable block-contact identity;
- crony enemies and shared stomp behavior;
- forward-only authored camera policy; and
- flag contact-height scoring, slide, walk-off, tally, dwell, and cyclic level
  restart.

Remaining acceptance work
(**this list is the single source; status.md and tracks.md refer here**):

- ✅ **The secret pipe and underground room — LANDED 2026-07-21.** A warp pipe on
  the safe run between pits A and B; stand on its mouth, press Interact, drop
  into a sealed coin vault dug under the ground slab, and Interact at its far
  end to surface. The vault is part of the SAME `RoomSpec` rather than a second
  room on purpose: cross-room transition lives in `ambition_app`'s `world_flow`,
  so a room-graph secret would have worked only when Ambition hosted the demo
  and been dead in the demo's own app. The world grew downward
  (`SURFACE_HEIGHT` + `VAULT_DEPTH_TILES`) so the authored surface layout is
  byte-identical. The warp is a real `transit_body` (ADR 0024), not a position
  poke, so entering while wall-clinging reconciles instead of arriving still
  clung to a wall that is not there. Its 8 coins are ordinary `currency`
  placements the shared economy collects — they land in the HUD's COINS readout
  with no demo collection code. NOTE: authoring placements made this demo
  require the pickup lowering interpreter, so its two bare-`App` unit harnesses
  now add `WorldPrepSchedulePlugin`; the real app already had it.
- ✅ **The brainless sliding shell — LANDED 2026-07-21.** A stomped crony now
  leaves a shell instead of nothing. Walk into a resting shell and it launches
  away from the side you touched, so you aim it; walk into a sliding one and it
  stops dead, so it stays a tool rather than something you set loose and lose. A
  sliding shell runs cronies down and reverses at walls, which turns one stomp
  into the demo's one emergent combo. BRAINLESS is literal: the archetype's
  brain is `StandStill`, so nothing ever decides anything for a shell — its
  whole behaviour is three demo rules, and gravity, ground contact, and walls
  are the ordinary body physics every actor already gets. FIXED 2026-07-21 after
  playtest: it shipped inert. The demo matched actors on `Name`, but the spawner
  writes `"Feature actor enemy: {name}"` there and the bare name onto
  `FeatureName` — so shells were never tagged AND sliding shells matched no
  crony. Both now match `FeatureName`. The unit test was green throughout
  because its fixture hand-built `Name`; the regression test drives the real
  spawn path instead.
- ✅ **Title / results presentation — LANDED 2026-07-21.** A centred transient
  card: `WORLD 1-1  MARY-O x3` on entry and after every death, `COURSE CLEAR
  {score}` on the flag. Expressed as ONE declared HUD slot rather than a new
  surface — the engine's `GameplayBanner` renders only in `ambition_app`, so a
  demo could not use it. `HudSlotSpec::centered()` was the whole engine-side
  addition, and the card retires itself: a game publishes text into the slot only
  while it should be up, and an unpublished slot draws nothing, so there is no
  hide path and no despawn.
- ✅ **HUD for score/coins/time/lives — LANDED 2026-07-21** through the new
  provider-declared HUD seam (`with_hud`), four readouts in the reserved top
  surround the 4:3 profile already owed. `MaryOLevelState` grew `score` (banked
  from `FlagPhase::Tallied` when the level cycles, so it is a running total
  rather than the last banner) and `lives`; coins read the shared economy's
  wallet through `PlayerHudFacts`, the same fact Sanic's rings use.
- ✅ **Lives are spent — LANDED 2026-07-21.** A death costs one and zero lives
  restarts the run (lives, score, and clock all return to start). Mary-O authors
  no death test: she watches `BodyLifetime.resets`, the counter the ENGINE bumps
  in `reset_body_clusters` on every respawn, so any future hazard that respawns
  her already costs a life with no new demo wiring. Running out of time is the
  demo's own rule and converges on the same path by asking the engine for a
  respawn instead of teleporting her. Poison-tested both ways: spending on the
  counter's VALUE instead of its EDGE drains a life per frame, and failing to
  refill the clock lets one timeout spend every remaining life on consecutive
  frames. STILL OPEN from this line item: **title/results presentation**.
- ✅ **The deterministic scripted run — LANDED 2026-07-21**
  (`ambition_demo_mary_o_app/tests/scripted_level_run.rs`). Boots the real demo
  app, walks her through the real `ControlFrame` seam, takes the secret pipe,
  banks the vault's coins through the shared economy, surfaces, and finishes on
  the flag into a settled tally and a level cycle. Two things it had to learn:
  the clock is pinned with `TimeUpdateStrategy::ManualDuration`, because a
  fixed-tick host without one runs a machine-speed-dependent number of ticks per
  update and the same script then walks a different distance every run; and it
  is gated `#![cfg(not(feature = "input"))]`, because under `input` the
  participant pipeline legitimately OWNS `ControlFrame` and repopulates it from
  device state each frame — scripting the device layer is a different claim that
  `app_it::participant_input` already owns. Traversal between beats is set up
  rather than played: crossing the pits under scripted input would make this a
  platforming-precision test fragile to any jump tuning change, when what it
  exists to prove is that the SEAMS connect.
- additional planned levels — the level-1 acceptance gate is now CLOSED, so this
  is the next thing this demo wants.

## Consumes

- runtime, provider lifecycle, and windowed host composition;
- the shared body/control path using the axis-swept motion model;
- item/equipment and canonical action/moveset execution;
- combat/contact vocabulary for stomps and sliding hazards;
- world IR + LDtk rooms/loading zones;
- the cutscene domain for presentation sequencing where appropriate;
- `SimView` for HUD and programmatic observation.

## Owns

`ambition_demo_mary_o` owns its levels, rules, lives/score/coins/timer, equipment
rows, enemy/content rows, shell prop, flag sequence, HUD, title, and results. A
need discovered while authoring becomes engine work only when it is a reusable
missing seam.

## V1 design

- **World:** three levels sharing one authored world: an opening grammar, an underground variant, and a moving-platform level.
- **Powerups:** a grow/armor equipment row and a ranged-action grant. Numeric effects fold through equipment parameters; behavior grants compose through action data.
- **Enemies:** ordinary actor rows for walkers plus a brainless sliding shell prop; this exercises the actors-versus-props distinction.
- **Camera:** forward-only scroll is an authored camera policy, not Mary-O-specific engine code.
- **Flag:** provider-owned gameplay state captures contact height and drives the body deterministically; presentation/results may use the cutscene domain without turning cutscenes into encounter logic.
- **Death:** level restart is authored session/game policy rather than a universal engine default.

## Acceptance

A scripted headless run completes the first level, reaches the pipe secret, uses
a powerup through the real pickup/equipment path, and never touches the
surface-momentum implementation. The visible app uses the same provider and body
state, including size and equipment presentation.

The demo app remains an explicit composition root, not a second implementation of
input, session lifecycle, sprite binding, or platformer simulation.
