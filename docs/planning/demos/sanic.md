# Track S — Sanic (momentum acceptance demo)

Inspired by Sonic 2, Emerald Hill Zone act 1. Parody-original everything
(blue meme speedster; original silhouette + layout — homage, never copy).

**Purpose:** prove the surface-momentum movement identity end-to-end — a
SECOND movement physics coexisting with the classic AABB path in one
engine, selected per body by data (`MotionModel`), with level design that
only makes sense at speed.

**Status:** furthest along, but the visible/playable shell is currently an
open architecture proof rather than a completed success. The room and music run;
the selected character still falls back to a rectangle and the real input path is
not yet proven end to end. The ordered recovery is
[`sanic-recovery.md`](sanic-recovery.md). Landed: the momentum kernel (chains,
loops, blocks-as-surfaces), Sanic catalog row, `sanic_sandbox` proving area,
chains channel + LDtk converters + debug overlay, swept portal transit at speed,
the demo shell, and **the ball dash (2026-07-10)**. Remaining: recover the public
playable-character composition seam, S4 proofs, and the demo game itself.

## Consumes (by role) / Owns

**Consumes:** [the sim assembly]+[the windowed host] (app shell, mode
scope) · [the movement kernel] (the surface follower — chains, loops,
blocks-as-surfaces) · [the sim heart] (spawn, wear, MotionModel
insertion) · [the space IR]+[the LDtk backend] (its own zone .ldtk; the
`SurfaceChain`/`SurfaceLoop` converters, plus the planned **`SurfaceRamp`**
quarter-circle marker — Q27 ruling: parameterized generator entities keep
LDtk sufficient; a ramp = quarter-arc chain (params: radius, corner
orientation, segments≈8) for floor↔wall transitions, same converter
pattern as `SurfaceLoop`) · [the combat resolver] (rolling hit volume,
on-hit bit-scatter) · [the observation boundary] (HUD reads) ·
[the authoring spine]/[the sprite-geometry authority] (rows + sheets).

**Owns (`ambition_demo_sanic`):** the zone world (3 acts), the level rules
plugin (bit count, act clear, timer — mode-scoped), the spin-dash
technique registration, the bits pickup + drop-on-hit policy rows, the
booster/spring entity rows, 2–3 patrol enemy rows, the goal-gate +
results sequence, HUD.

**Engine prerequisites:** E5-finish landed. Immediate blocker: the public
windowed input + selected-character presentation path in
[`sanic-recovery.md`](sanic-recovery.md). Nice-to-have: CM5 (per-move sfx/vfx).
Jon's generic encounter orchestrator
([`../engine/encounter-orchestration.md`](../engine/encounter-orchestration.md))
LANDED E0–E7 (2026-07-11), so the act-3 mini-boss and later race/chase set pieces
are now unblocked to be its first non-boss customers — but building them is not a
prerequisite for the immediate visible/playable recovery.
Expected oracle-violations: speed-adaptive camera look-ahead knob; anything the
spring/booster surfaces need beyond rebound blocks.

## Design (v1 scope)

- **World:** one zone, 3 acts of one .ldtk world (`sanic_zone.ldtk`):
  act 1 = flowing intro (slopes, a valley, one loop, springs); act 2 =
  route choice (high fast route / low safe route — the classic Sonic
  contract); act 3 = short + a mini-boss (a patrol enemy scaled up on the
  boss pipeline's `sweep`+`dash_through` seeds). Springs = the rebound-
  block vocabulary re-authored as chain-attached boost pads (engine has
  rebound; a `SurfaceBooster` entity converter is content).
- **Verbs:** run/jump (kernel), ~~**ball dash (spin dash)**~~ ✅ **LANDED
  2026-07-10** — `ambition_demo_sanic::ball_dash`, content-side, zero engine
  additions. See §Ball dash below. [opus]
- **Rings-analog ("bits"):** the deferred `Item`-enum SET opens here on
  real demand — pickups that scatter on hit (drop-on-damage policy: on
  taking a hit, spill N collectables with outward impulses; invulnerable
  while any are airborne — authored as an on-hit effect through the
  damage seam). Match/level state (bit count, act clear) is mode-scoped.
- **Enemies:** 2–3 patrol archetypes on existing brains; stomp/roll kills
  through the landed pogo/on-hit vocabulary; rolling beats standing
  enemies (the rolling flag grants an attack volume — same vocabulary as
  body_contact damage).
- **Camera:** speed-adaptive look-ahead (a CameraZoneSpec policy knob —
  small engine knob, file as oracle-violation when hit).
- **End-of-act:** goal gate + score tally on the cutscene kit.

## Slices

S4 (proofs, [opus]): scripted loop-at-speed / fail-below-threshold /
slope round-trip; possession e2e (movement identity travels); knight-
coexistence combat stays AABB; overlay screenshot artifact (BLIND).
S5 (the game, [opus]): content crate + thin app per the doctrine; acts
1–3; bits; spin dash; mini-boss; title/results.
S6 (hosting): the Sanic wing in ambition's sandbox behind a LoadingZone,
mode-tagged (Phase D-C seam).

**Exit:** doctrine exits (zero engine edits; headless act-completion
tests; violation log) + the momentum-specific one: an input script
completes act 1 faster via the high route than the low route (speed is
REWARDED, verified headlessly).

---

## Ball dash — landed 2026-07-10 (opus)

`game/ambition_demo_sanic/src/ball_dash.rs`. **Zero engine additions**: the E9
oracle (*"could another platformer be built by ADDING a content crate without
editing core?"*) holds for a brand-new movement verb.

**The input needed no new binding.** The demo contract is *hold down, tap attack
(X in the default arrows + Z/X/C preset) to rev, release down to launch; one tap
produces a small dash and repeated taps build to full charge*. The
content rules capture the ordinary `ActorControlFrame::melee_pressed` edge before
the peaceful-persona gate removes generic combat, so X becomes a Sanic technique
without reopening melee. Crouch remains `locomotion.y ≥ threshold`; because that
axis is in the body's LOCAL frame — `+y` toward the feet, not toward the bottom of
the screen — Sanic revs the same way on the ceiling of a loop as on the floor.
The rule samples ride contact and latches Down's falling edge at that
PlayerInput seam, before generic crouch compaction and movement can transiently
detach the momentum circle from a flat block. A short contact grace keeps an
armed charge across a block/chain handoff, while the explicit one-tick release
edge spends it independently of later body-mode or fixed-tick timing. Sustained
airborne time still clears the charge, so it cannot be banked across a real jump.

**The second form uses the same identity seam.** The semantic Utility action
(`D` in the classic preset) toggles `WornCharacter` between `sanic` and
`super_sanic`. Both catalog rows own their generated sheet and declare the same
peaceful momentum profile; gameplay and presentation therefore re-derive through
the canonical worn-character systems rather than a demo-local sprite swap. The
mode rule consumes Utility so the inherited control box cannot also interpret D
as the generic fly toggle.

**The showcase loop has a real raised on-ramp, without an edge seam.** The
rebound lands on a short cubic ramp whose final tangent exactly matches the
first tangent of a densely sampled 270-degree loop. Ramp and arc are one
`SurfaceChain`, so the follower never launches and reprojects at their join.
The open exit points steeply down/right, reaches the floor before the raised
ramp, and then passes underneath with full body clearance. This preserves the
classic entry/exit layering instead of deleting the ramp to avoid the bug.

**The rev has a visible pose without a demo-local animator.** While Down is held,
the rule arms the shared `BodyAnimFacts::dash_startup_timer`. The canonical body
animation picker requests `CharacterAnim::DashStartup`; a rich sheet can draw a
dedicated row, while a lean sheet follows the standard
`DashStartup -> Dash -> Run -> Walk -> Idle` fallback and still shows an existing
motion frame rather than freezing on idle.

**The launch is one line** because the momentum kernel integrates
`v_t += run * accel * dt` with `run = locomotion.x`, so `v_t` and `facing` share a
sign convention: `v_t = facing × launch_speed × charge`, no tangent lookup.
Airborne, the local side axis comes from gravity.

**The ball is not a costume.** Rolling shrinks `BodyKinematics::size`, and the
kernel derives its circle proxy as `size.min_element() × 0.5` — so a balled-up
Sanic is *physically* smaller. The hurtbox narrows because the body did.
`BodyBaseSize` is untouched: it stays the standing reference `pose_view` divides
by for the stance ratio, exactly the seam crouch established.

**Rules, not content.** The systems live in `SanicRulesPlugin`, so they sleep
outside the Sanic rooms when Ambition hosts the demo (the D-C mode-scope pattern).

### It found two engine bugs, propping each other up

Writing the airborne launch meant reading the momentum kernel's airborne side
axis. It was **negated**: `step_airborne` built it as `tangent_of(gravity)` where a
floor's normal is `-gravity`. **Holding right in mid-air accelerated a momentum
body left.** No test held a direction in the air — every airborne test in the
suite was ballistic or a landing.

Fixing that exposed the second: a body running off the **open end of a flat
chain** never fell. `SurfaceChain::project` clamps arc length into the chain, so
the airborne sweep re-attached the body at the very vertex the ride step had just
launched it from — a two-frame limit cycle with the position frozen at the lip.
The mirrored air control had been shoving the body back *over* the chain instead
of off it, so the one test that walked a flat chain end passed for the wrong
reason. `leaving_an_open_end()` is the guard; it is one-directional, so landing on
a ramp's tip while moving inward still attaches.

Both are fixed with tests, in `ambition_engine_core::surface`. Neither is an
engine addition *for the demo* — they are bugs the demo's first honest reader
found, which is exactly the argument for building demos against the real kernel.
