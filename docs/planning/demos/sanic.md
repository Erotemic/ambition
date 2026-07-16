# Track S — Sanic (momentum acceptance demo)

Inspired by the momentum-platformer contract, with parody-original art and level
design rather than copied content.

**Purpose:** prove that a second movement identity can coexist with classic AABB
platforming in one engine, selected per body by authored data, and that a new
provider obtains input, simulation, presentation, camera, audio, and hosted
lifecycle without editing engine internals.

## Current state

Landed:

- the surface-momentum kernel, chains/loops, route junctions, and the speedway
  room with deterministic route/loop/orbit/stranding oracles;
- provider-owned Sanic and Super Sanic character profiles, native sprite binding,
  transformation, and hosted/standalone shells;
- the production keyboard/gamepad input path proven end to end through
  device input → `ControlFrame` → fixed-tick latch → player slot → brain → body;
- the provider-owned ball dash on the standard action/control seam;
- leak-free launch, quit, and relaunch through the shared provider/session
  lifecycle; and
- basic act state, clock, and standard SFX publication.

Remaining acceptance work is product/content work:

- bits and drop-on-hit behavior;
- at least one complete enemy/contact loop using shared rolling/stomp/combat
  vocabulary;
- a provider-owned goal, HUD, results, and end-of-act sequence;
- additional authored act content beyond the single speedway room; and
- a deterministic headless completion proof in which the rewarded high route
  beats the lower safe route under the same control contract.

The detailed 2026-07-11 recovery investigation is archived at
[`docs/archive/reviews/sanic-visible-playable-recovery-2026-07-11.md`](../../archive/reviews/sanic-visible-playable-recovery-2026-07-11.md).

## Consumes

- runtime + windowed host composition;
- the shared body/control path and `MotionModel` selection;
- surface-momentum movement and frame-aware input;
- world IR + LDtk conversion for chains, loops, ramps, boosters, and routes;
- combat/effect vocabulary for rolling contact and bit scatter;
- `SimView` for HUD/agent observation;
- provider-owned character, action, sprite, audio, and world catalogs.

## Owns

`ambition_demo_sanic` owns its worlds, mode-scoped rules, ball-dash technique,
bits/drop policy, boosters/springs, enemy rows, act completion, result sequence,
and HUD. These remain content even when they expose a reusable engine gap.

## V1 design

- **World:** three acts in one authored zone. Act 1 teaches flow; act 2 rewards a high fast route versus a low safe route; act 3 adds a short encounter customer.
- **Verbs:** run, jump, and the landed provider-side ball dash; no Sanic-specific engine action enum.
- **Bits:** provider-owned collectible/economy state with drop-on-hit behavior through the shared damage/effect seams.
- **Enemies:** ordinary actor rows and brains; rolling/stomp outcomes use shared contact/combat vocabulary.
- **Camera:** speed-aware look-ahead expressed as reusable camera policy only if the existing seam cannot author it.
- **End of act:** provider-owned goal/results sequence using the cutscene domain, not encounter timeline unification.

## Acceptance

A visible run provides standard keyboard/gamepad input, native selected-character
art and animation, camera/audio, and the authored momentum route. The remaining
headless gate uses that same selected profile and control path to complete act 1
faster through the rewarded high route.

The demo app remains small and contains no app-local input system, direct sprite
binding, or dependency on `ambition_app`.

## Proposed — polish backlog (2026-07-16)

Landed this pass (commit `f558d124e` + generator bump `5e1ee9b`): the rev-dash
**ball is now a real looping curl** (not a squished run) and momentum riders show
a **skid** pose — both engine-reusable, through the ONE `pick_body_anim` ladder,
with the stance-squash hack retired per pose whenever a sheet owns the row.

Deferred, in priority order:

- **Rev SFX suite (Jon: "the current rev sfx isn't very good").** The rev reuses
  the generic Jump chirp per tap and the launch reuses Dash. Author dedicated
  procedural cues in Sanic's `SfxRegistry` (content-side, no bank): an **ascending
  spin-dash rev** picked by charge bucket (`BallDashStep::Charging(charge)` already
  carries the float — 3 tiers reproduces the classic "reh-reh-REH" without engine
  pitch support), and a distinct **launch/release whoosh** on `BallDashStep::Launch`.
- **Wire the already-emitted-but-dropped engine cues.** The shared `emit_movement_fx`
  writes `Pogo` for rebound pads (`MovementOp::Rebound`) and `Reset` on pit death,
  but Sanic authorizes only Jump+Dash so both are silently dropped. Add `SfxSpec`s
  with `cue: Pogo` (a bright spring "boing") and `cue: Reset` (a descending fail
  tone) — authoring both authorizes AND voices them.
- **Distinguish same-cue events.** Super transform ON vs OFF both play Dash;
  monitor break, badnik pop, and the transform share Jump/Dash. Give the
  transform a rising power chord and the monitor a glass-pop.
- **Skid SFX** (a tire-scrape) gated on `BodyMotionFacts::skidding` rising — the
  fact now exists; nothing voices it yet.
- **Land SFX** is engine-wide missing (dust VFX only, `movement_fx.rs:259`) — a
  shared touchdown thud would benefit every game, not just Sanic.
- **Rings are decoration.** `sanic_speedway.ldtk` has 291 `ring` refs but there is
  NO collection system, `RingCount`, ring-collect SFX, or ring-loss-on-hit — they
  are pure scenery today. Either build the pickup+counter loop (a natural home for
  a "lose your rings" hurt reaction that ties into the super-form drain the toggle
  doc already anticipates) or accept them as visual dressing and thin them out.
- **Optional engine enhancement:** a per-play pitch/gain on `SfxMessage` would let
  ONE rev cue pitch-climb continuously instead of bucketed tiers — a reusable win
  for any charge-up sound.
- **Action-sprite survey:** the Sanic sheet is already rich (34 rows incl. the new
  ball+skid). Small future adds only if a verb needs them — a ledge/edge teeter,
  a goal/victory pose beyond `taunt`, a spring-launch upward stretch. Low priority;
  no current verb is undrawn.
