# Track S — Sanic (momentum acceptance demo)

Inspired by Sonic 2, Emerald Hill Zone act 1. Parody-original everything
(blue meme speedster; original silhouette + layout — homage, never copy).

**Purpose:** prove the surface-momentum movement identity end-to-end — a
SECOND movement physics coexisting with the classic AABB path in one
engine, selected per body by data (`MotionModel`), with level design that
only makes sense at speed.

**Status:** furthest along. Landed: the momentum kernel (chains, loops,
blocks-as-surfaces), Sanic catalog row + sheet, `sanic_sandbox` proving
area, chains channel + LDtk converters + debug overlay, swept portal
transit at speed, the demo shell + presentation (playbook exit 3 + OV1),
**the ball dash (2026-07-10)**. Remaining: S4 proofs, the demo game itself
(S5 — UNBLOCKED 2026-07-06 night: E5-finish landed; copy the demo-shell
fixture in `crates/ambition_host/tests/demo_shell_smoke.rs`).

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

**Owns (`sanic_content`):** the zone world (3 acts), the level rules
plugin (bit count, act clear, timer — mode-scoped), the spin-dash
technique registration, the bits pickup + drop-on-hit policy rows, the
booster/spring entity rows, 2–3 patrol enemy rows, the goal-gate +
results sequence, HUD.

**Engine prerequisites:** E5-finish ✅ (landed 2026-07-06 night). Nice-to-have: CM5 (per-move
sfx/vfx). Expected oracle-violations: speed-adaptive camera look-ahead
knob; anything the spring/booster surfaces need beyond rebound blocks.

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

**The input needed no new binding.** Sonic 2's spin dash is *hold down, tap jump
to rev, release down to launch*, and every one of those is already on
`ActorControlFrame`: crouch is `locomotion.y ≥ threshold`, rev is `jump_pressed`,
launch is the crouch's release edge. Because `locomotion` is in the body's LOCAL
frame — `+y` toward the feet, not toward the bottom of the screen — Sanic revs the
same way on the ceiling of a loop as on the floor. That is what the relativity
principle buys, cashed.

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
