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

Also landed:

- the ring/bits economy: 35 authored `currency:1` rings on the shared economy,
  animated `sanic_ring_prop` sheet, collect SFX (`a8ab166ee`/`7dc7c1711`);
- one complete enemy/contact loop: the badnik with stomp-with-bounce AND
  roll-through defeat through the shared contact/combat pass (`05ebcef2e`,
  `game/ambition_demo_sanic/src/badnik.rs`).

Remaining acceptance work is product/content work
(**this list is the single source; [`status.md`](../status.md) and [`tracks.md`](../tracks.md) refer here**):

- ✅ **The scatter is a real scatter, and you can SEE it — 2026-07-25.** Three
  gaps closed: runtime-spawned rings had no presentation (`rebuild_dynamic_feature_views`
  covered only four families; a **dropped-pickup family** selected by
  `SpawnOrigin::Dynamic` now binds the same spinning `sanic_ring_prop` sheet and
  is retired with its feature); the burst was an upward fan, not radial (now an
  even full-circle spray in shells of six at 520 px/s, pinned by a test
  requiring a ring in every quadrant); and rings fell forever without expiring
  (they now sweep/rebound off room geometry and vanish after 4.2s, with a 0.6s
  untouchable window).
- ✅ **Ring drop-on-hit — RESOLVED 2026-08-08 (D41).** Rings are a life, not a
  score: `resolve_body_hit`'s precedence ladder (i-frames, shield, armor, HP)
  now includes rings as an armor row (`BodyHitResolution::WalletShielded`), so a
  hit taken holding rings costs the rings — placed as static `currency` pickups
  above the body — instead of health, capped at 12 scattered; a hit taken
  holding none lands normally. Getting here needed two fixes to an earlier
  version that only reacted to a `BodyHealth` drop: a lethal hit was never
  observed because `death_respawn_player` reset health first, and the demo's
  own hazards (`pit_hazard`, `mid_spikes`) were `HazardBlock` tile hits that
  never touched `BodyHealth` at all. `mid_spikes` is now a `DamageVolume` so the
  shield hears it; `pit_hazard` stays a `HazardBlock` — falling out is not a hit
  — and Sanic authors `max_health: 1` so a ringless hit is fatal. Guarded by
  `spikes_spend_rings` (`ambition_demo_sanic_app`). Engine change:
  `ambition_platformer2d_actor_monolith::features::ecs::spawn_static::spawn_pickup`
  is now PUBLIC, so a game can drop a runtime pickup indistinguishable from an
  authored one (needed by scatters, enemy loot and chest rewards alike).
- ✅ **Provider-owned goal, results, and end-of-act — LANDED 2026-07-21.**
  Crossing `GOAL_X` (matched to the authored `FINISH` label so sign and trigger
  cannot drift) clears the act: the clock stops, and the time and rings are
  CAPTURED at that instant rather than re-derived, so a ring picked up during
  the outro cannot rewrite a result already on screen. A centred results card
  (`ACT CLEAR  TIME  RINGS  SCORE`) holds for a dwell through the declared-HUD
  seam, then the act restarts on the engine's ordinary `RoomReplayRequested` —
  the same cycle Mary-O's level uses, no demo-specific restart.

  Three bugs found and fixed while proving this headlessly: the standalone app
  had no consumer for `RoomReplayRequested` at all (only `ambition_app`
  registered one, and the demo app doesn't depend on it), so "restart" reset
  `SanicActState` and nothing else — the consumer now lives in
  `ambition_platformer2d_runtime::sandbox_reset` and rides `PlatformerEnginePlugins`
  into all three hosts, proved per host in `tests/room_replay.rs` (`cf5095576`);
  a death on a hazard strip 144px past the goal went undetected because death
  leaves `SanicActPhase::Cleared` untouched (`914a7ee3d`); and the goal-clear
  brake cleared `locomotion`/jump but not a charged `BallDash`, so crouch-release
  semantics fired a spin dash on the line — the goal now takes the whole control
  frame through the shared `ScriptedControl` seam and disarms stored charge
  (GPT-5.6 review, 2026-07-25).

  ⚠ **Still live:** `GOAL_X` sits 400px from the level's right edge, and
  clearing doesn't brake or close the course, so Sanic can coast off the end and
  die inside his own results dwell (`code_smells.md`, 2026-07-21) — the
  act-clear replay proof stamps the cleared phase under controlled conditions
  rather than extending `act_completion.rs` for exactly this reason.

  `act_score` = a time bonus against par plus a per-ring bonus, pinned as a pure
  function by `the_act_score_pays_for_speed_and_for_rings_kept`; whether the
  fast and safe routes actually compete is a claim about the authored level that
  no test drives. Rebalanced 2026-07-21 (par 60s→30s, ring bonus 10→100) once a
  clean run measured ~6s — at the original numbers time swamped rings by an
  order of magnitude.
- additional authored act content beyond the single speedway room; and
- ✅ **A deterministic headless completion proof — LANDED 2026-07-21**
  (`ambition_demo_sanic_app/tests/act_completion.rs`). Plays the app to the goal
  and asserts the act clears, time is captured, and the clock stops; caught the
  goal being unreachable (`LEVEL_WIDTH - 130` vs a runnable extent topping out
  near `LEVEL_WIDTH - 270`) and an authored pit that traps a hold-right script.
  Sanic cannot be teleported to the goal to check this — he rides the momentum
  kernel, so `pos` is derived from his surface parameter and a poked position is
  overwritten next tick; running there is the only proof.
  ⚠ **STILL OPEN:** the high-route-beats-safe-route comparison — completion is
  proven, the two-route contest is not.

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

✔ **Re-measured 2026-09-03 — all three hold**, which is worth recording because a
boundary nobody checks is the one everyone assumes has drifted:

* **No `ambition_app` dependency.** `game/ambition_demo_sanic/Cargo.toml` never
  names it; the only occurrence anywhere in the pair is a COMMENT in
  `game/ambition_demo_sanic_app/Cargo.toml:34` stating the rule itself —
  *"depends on `ambition_platformer2d`, never on `ambition_app`. That is the demo
  gate."* The gate is written where a violation would have to be typed.
* **No app-local input system.** Across 42 `add_systems` registrations in the
  demo lib, none reads a device or writes a `ControlFrame`. The single `KeyCode`
  reference (`game/ambition_demo_sanic/src/lib.rs:280`) is a HUD LABEL that asks
  the engine's own bindings what key an action is bound to — presentation of the
  input vocabulary, not a second input path.
* **No direct sprite binding.** The one place the demo touches an atlas is
  `register_sanic_ring_prop_sheet` (`game/ambition_demo_sanic/src/lib.rs:830`),
  which registers demo-owned PROP art (the ring) through the engine's own
  `load_prop_sheet_for_target` into `GameAssets::characters::props`. Demo art
  entering by the engine's seam is the arrangement this clause asks for, not an
  exception to it.

## Proposed — polish backlog (2026-07-16)

Landed this pass (commit `f558d124e` + generator bump `5e1ee9b` in `tools/ambition_sprite2d_renderer`): the rev-dash
**ball is now a real looping curl** (not a squished run) and momentum riders show
a **skid** pose — both engine-reusable, through the ONE `pick_body_anim` ladder,
with the stance-squash hack retired per pose whenever a sheet owns the row.

**SFX suite landed (commit `94e66909c`).** The whole Sanic sound palette was
rebuilt: an **ascending three-tier spin-dash rev** picked by charge bucket
(`rev_tier_id`) plus a distinct launch whoosh; **Pogo** spring, **Reset**
pit-death, and a reusable engine **Land** cue emitted once per touchdown edge in
`emit_movement_fx`; distinct **monitor**, **badnik**, and **skid** voices; and a
**transform** sound derived from the worn-identity edge in
`sync_super_form_traits` so it fires once regardless of cause. ⛔ **2026-08-16 —
the monitor cause is gone and must not come back:** Jon, *"the sanic level should
not offer super form. at all. There is a key for it."* `monitor_super` is deleted
from the course, from `author_speedway_ldtk.py` and from `monitors.rs`; the
Utility action is the only way into the form, and the worn-identity edge means
the cue still fires for whatever wears it next.

**Rings landed as a collection loop (commit `a8ab166ee`).**
`author_speedway_ldtk.py` places **35 rings** as `currency:1` pickups, so the
shared economy does the work with no demo collection code: `magnetize_pickups` +
`collect_ecs_pickups` credit the player's `BodyWallet` (the ring counter), spark,
and ding (the demo voices `world.coin.pickup`, the id that loop emits). Rings
render as the shared coin sprite.

Deferred, in priority order:

- ~~**Persistent ring HUD counter.**~~ **LANDED 2026-07-21.** `RINGS n` draws
  from a single declared slot. It needed no new simulation at all: rings are
  authored `currency:1` pickups, the shared economy credits `BodyWallet`, and
  `PlayerHudFacts` already republished that balance every tick — so the whole
  feature is `readouts.set_labelled(RINGS_HUD_SLOT, "RINGS", facts.balance)`.
  The predicted OV1 relaxation happened as described, and both directions are
  now pinned: engine-owned UI must be exactly 0 (filtering by the demo marker
  alone would let an engine node hide by wearing it), and the demo's own HUD
  must draw exactly as many nodes as it declared. A separate test reads the
  text back, because a HUD that spawns the right number of EMPTY nodes is
  indistinguishable from a working one in a node count — poison-tested by
  removing the publisher, which leaves every count green and fails only that.
- ~~**Dedicated ring sprite.**~~ LANDED (commit `7dc7c1711`): rings draw the
  animated `sanic_ring_prop` sheet via the new engine capability *animated feature
  sprites* (`animate_feature_sprites` + `PickupSpec.sprite`) — a pickup carries an
  optional prop-kind sheet and idle-spins, no PropVisual conflation. Remaining
  polish: the sheet's `collect` row (a pop/sparkle) isn't played on pickup — the
  ring idle-spins and the spark VFX covers collection; playing the collect row on
  a brief render-held despawn is the follow-up. And the app loads the ring sheet
  by bypassing the asset catalog (smell #19: a per-game prop-catalog contribution
  seam is the elegant fix).
- **Super-form ring drain.** A future super-form ring drain wears the form off
  the same worn-identity seam the toggle uses (`sanic.ring_loss` cue is authored
  ahead). Drop-on-hit scatter itself has since landed — see above.
- **50/100-ring milestones** (extra life / jingle), and a **swept high-speed
  collection** test — `collect_ecs_pickups` uses a per-frame overlap, so at Sonic
  velocities a ring can tunnel; the magnet's 130px range masks it for now, but the
  `cast::aabb_path_contacts` swept route is correct.
  ⚠ **Both halves of that sentence moved on 2026-09-03 and the citation still
  resolved, which is the hazard rather than an aside.** The callout is now in
  `crates/ambition_held_items/src/lib.rs` — *"two endpoints are not a trajectory
  … `aabb_path_contacts` is the repo's own answer to this"* — carried there by
  the pickup carve; and `collect_ecs_pickups` is in
  `crates/ambition_platformer2d_actor_monolith/src/features/ecs/pickups.rs`,
  which is a different file from the `pickup/mod.rs` this row used to name.
  ⛔ That old path STILL EXISTS as the kernel's schedule residue, so a
  bare-filename citation to it passes every checker while pointing at the wrong
  code. Cite the crate-qualified path and the NAME; the name is what fails
  loudly when a carve moves it.
- **Optional engine enhancement:** a per-play pitch/gain on `SfxMessage` would let
  ONE rev cue pitch-climb continuously instead of bucketed tiers — a reusable win
  for any charge-up sound.
- **Action-sprite survey:** the Sanic sheet is already rich (34 rows incl. the
  ball+skid). Small future adds only if a verb needs them — a ledge/edge teeter,
  a goal/victory pose beyond `taunt`, a spring-launch upward stretch. Low priority;
  no current verb is undrawn.
