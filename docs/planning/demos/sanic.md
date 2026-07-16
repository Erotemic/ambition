# Track S — Sanic (momentum acceptance demo)

Inspired by the momentum-platformer contract, with parody-original art and level
design rather than copied content.

**Purpose:** prove that a second movement identity can coexist with classic AABB
platforming in one engine, selected per body by authored data, and that a new
provider obtains input, simulation, presentation, camera, audio, and hosted
lifecycle without editing engine internals.

## Current state

Landed foundations include the surface-momentum kernel, chains/loops and route
junctions, Sanic character data, the standalone/hosted demo shells, swept portal
transit at speed, and the provider-owned ball-dash rule.

The remaining acceptance work is product-facing rather than a new movement
kernel:

- deterministic provisioning and binding of the native Sanic art/profile;
- an end-to-end standard input/slot/control proof in the runnable shell;
- bits, enemies, HUD/results, and complete acts;
- a headless act-completion run demonstrating that speed and route choice matter.

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

A visible run must provide standard keyboard/gamepad input, native selected
character art and animation, camera/audio, and the authored momentum route. A
headless run must use the same selected profile and control path and complete act
1 faster through the rewarded high route.

The demo app remains small and contains no app-local input system, direct sprite
binding, or dependency on `ambition_app`.
