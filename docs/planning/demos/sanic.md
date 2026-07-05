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
transit at speed. Remaining: S4 proofs, the ball-dash special, the demo
game itself (S5, gated on E5-finish).

**Depends on:** E5-finish (runtime+host groups); CM5 (per-move sfx/vfx)
nice-to-have; no other engine tracks.

## Design (v1 scope)

- **World:** one zone, 3 acts of one .ldtk world (`sanic_zone.ldtk`):
  act 1 = flowing intro (slopes, a valley, one loop, springs); act 2 =
  route choice (high fast route / low safe route — the classic Sonic
  contract); act 3 = short + a mini-boss (a patrol enemy scaled up on the
  boss pipeline's `sweep`+`dash_through` seeds). Springs = the rebound-
  block vocabulary re-authored as chain-attached boost pads (engine has
  rebound; a `SurfaceBooster` entity converter is content).
- **Verbs:** run/jump (kernel), **ball dash (spin dash)** — Sanic's
  special: a charge technique (`simple_charge` shell) that on release sets
  `v_t` (grounded) or velocity (airborne) along facing to
  `dash_speed × charge`, with a rolling state flag that narrows the
  hurtbox (BodyBaseSize seam). This is the S-track's one new technique;
  registered content-side. [opus]
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
