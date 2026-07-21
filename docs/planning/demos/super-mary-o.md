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
- a brainless sliding shell prop;
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
- one deterministic scripted headless run that completes level 1 through real
  controls, enters the secret, collects a powerup, and exercises its effect; and
- additional planned levels after the level-1 acceptance gate closes.

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
