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
