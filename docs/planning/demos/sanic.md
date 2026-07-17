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

**SFX suite landed (commit `94e66909c`).** The whole Sanic sound palette was
rebuilt: an **ascending three-tier spin-dash rev** picked by charge bucket
(`rev_tier_id`) plus a distinct launch whoosh; the previously-dropped engine cues
now authored+voiced (**Pogo** spring, **Reset** pit-death, and a new reusable
engine **Land** cue emitted once per touchdown edge in `emit_movement_fx`);
distinct **monitor**, **badnik**, and **skid** voices; and the **transform** sound
derived from the worn-identity edge in `sync_super_form_traits` so it fires once
regardless of cause (D-toggle / monitor / future ring drain).

**Rings landed as a collection loop (commit `a8ab166ee`).** Correction to the note
below: the "291 ring refs" were `ring` inside `String` — there were **zero** rings.
`author_speedway_ldtk.py` now places **35 rings** as `currency:1` pickups, so the
shared economy does the work with no demo collection code: `magnetize_pickups` +
`collect_ecs_pickups` credit the player's `BodyWallet` (the ring counter), spark,
and ding (the demo voices `world.coin.pickup`, the id that loop emits). Rings
render as the shared coin sprite.

Deferred, in priority order:

- **Persistent ring HUD counter.** The count lives in `BodyWallet` and is
  proven by tests, but the standalone Sanic app renders no HUD/banner (that is
  Ambition-app only; OV1 asserts the presentation face adds zero UI nodes). On
  collect the player gets the spark + ding but no on-screen tally. Building a
  demo-owned app-side HUD (`RINGS: N`) means relaxing the OV1 node-count assertion
  to distinguish presentation-plugin nodes from demo-owned HUD nodes.
- ~~**Dedicated ring sprite.**~~ LANDED (commit `7dc7c1711`): rings draw the
  animated `sanic_ring_prop` sheet via the new engine capability *animated feature
  sprites* (`animate_feature_sprites` + `PickupSpec.sprite`) — a pickup carries an
  optional prop-kind sheet and idle-spins, no PropVisual conflation. Remaining
  polish: the sheet's `collect` row (a pop/sparkle) isn't played on pickup — the
  ring idle-spins and the spark VFX covers collection; playing the collect row on
  a brief render-held despawn is the follow-up. And the app loads the ring sheet
  by bypassing the asset catalog (smell #19: a per-game prop-catalog contribution
  seam is the elegant fix).
- **Drop-on-hit scatter + super drain.** The `sanic.ring_loss` cue is authored
  ahead. A badnik/spike hit should scatter rings (a natural home for the "lose
  your rings" reaction) and a future super-form ring drain wears the form off the
  same worn-identity seam the toggle uses.
- **50/100-ring milestones** (extra life / jingle), and a **swept high-speed
  collection** test — `collect_ecs_pickups` uses a per-frame overlap, so at Sonic
  velocities a ring can tunnel; the magnet's 130px range masks it for now, but the
  `cast::aabb_path_contacts` swept route (called out in `pickup/mod.rs`) is correct.
- **Optional engine enhancement:** a per-play pitch/gain on `SfxMessage` would let
  ONE rev cue pitch-climb continuously instead of bucketed tiers — a reusable win
  for any charge-up sound.
- **Action-sprite survey:** the Sanic sheet is already rich (34 rows incl. the
  ball+skid). Small future adds only if a verb needs them — a ledge/edge teeter,
  a goal/victory pose beyond `taunt`, a spring-launch upward stretch. Low priority;
  no current verb is undrawn.
