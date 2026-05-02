# ADR 0011: Per-entity proper time and the Galilean→SR ladder

## Status

Accepted.

## Context

Single-player bullet-time has two operationally distinct expressions that
look identical to the affected player:

1. **Scale the global clock.** Every entity ticks slower; the player's
   "thinking speed" remains constant; from the player's POV, the world is
   slowed.
2. **Boost the player's proper time.** The player's per-tick action quota
   and velocity scale up; the world ticks at normal rate; from the player's
   POV, the world appears slowed because the player processes more actions
   per unit world-time.

Operation 1 is incoherent under multiple observers (one player's slow-mo
can't slow another's world). Operation 2 is coherent for any number of
observers. They are observationally equivalent for a single observer in a
single inertial frame.

Beyond multi-player coherence, expressing time control as per-entity proper
time opens a path to genuinely modeling special relativity as gameplay.
Proper time is exactly the SR concept: a quantity that depends on the
observer's velocity through `γ(v) = 1/√(1−v²/c²)`. The Galilean version
(today's bullet-time) treats time scale as independent of velocity. The SR
version (a future theorem unlock in the math/agency storyline) couples them.

This is genuinely beautiful for the AI-agency storyline: the player
discovering they can bend their own proper time *is* a theorem unlock.
Galilean time scaling is the geometry-tier version (independent clocks,
no coupling). SR is the calculus/harmonic-tier version (Lorentz factor,
asymptotic behavior near c, complex-plane Minkowski geometry). The math
the player "learns" is real and the abilities encode it correctly. A
Gradient Sentinel boss could exploit SR effects the player hasn't unlocked
yet — finite signal speed, false simultaneity — and the player learning the
theorem is the literal mechanic that lets them dodge.

## Decision

Every gameplay entity carries a per-entity proper-time scale. The engine
supports two distinct time-control operations that share vocabulary with
ADR 0010 (`ClockScaleRequest`).

### Per-entity proper time

```text
Component: ProperTime {
    scale: f32,                      // default 1.0
    accumulator: f32,                // fractional action carry
}
```

Per Update tick:

- Action quota for the entity = `floor(accumulator + scale)`.
- Accumulator = `(accumulator + scale) - action_quota`.
- Velocity integration scales by `scale` (an entity at scale 2.0 covers
  twice the sim distance per tick, and processes twice the actions).
- All other state derives normally; entities not affected by time abilities
  keep `scale = 1.0` and tick once per Update with no carry.

### Two time-control operations

Both expressed as `ClockScaleRequest` (ADR 0010), distinguished by domain:

```text
ScaleGlobalClock(scale)
  domain = SimClock
  affects every entity uniformly
  coherent only under 1-observer regimes (Solo)
  the SP-natural implementation of bullet-time

BoostEntityProperTime(entity, factor)
  domain = PlayerClock(entity)
  affects only that entity's proper time
  coherent under any regime
  the MP-correct expression of bullet-time
```

In Solo regime both are legal. In CoopConsensual/Competitive (future) only
`BoostEntityProperTime` is granted to player requesters. In RLDeterministic
both are denied.

### Galilean→SR ladder

The engine starts with the Galilean coupling: proper-time scale is a free
parameter set by abilities and policy.

```
Galilean (today):
  proper_dt = ability_factor * sim_dt
  velocity, time independent
```

A future room/regime configuration adds the SR coupling: proper-time scale
is computed from velocity using the Lorentz factor.

```
SR (theorem unlock):
  γ(v) = 1 / √(1 − v²/c²)
  proper_dt = (ability_factor / γ(v)) * sim_dt
  spatial extent contracts by 1/γ along velocity direction
  signal propagation capped at c
  simultaneity depends on observer
```

Pre-relativistic gameplay has v ≪ c so γ ≈ 1 and SR effects are negligible
— the early game looks Galilean even when running through SR-aware code.
Late-game abilities push v toward c and SR effects emerge as *consequences*
of moving fast enough, not as a mode toggle.

Per-room metric is a first-class option for late-game biomes: most rooms
flat Galilean, some flat Minkowski, much later possibly curved (general
relativity territory). This dovetails with the "rooms are charts" framing
in ADR 0009.

## Consequences

- Every gameplay entity gets a `ProperTime` component (default 1.0).
  Most entities never touch it; the cost is a small per-entity field.
- Movement integration must read proper-time scale instead of using a
  global `dt` directly. This is a one-time change to the integrator and a
  structural improvement regardless of SR.
- Bullet-time-blink in SP can be implemented either way; pick whichever is
  simpler for the call site, since both are legal in Solo regime. The
  MP-correct migration later is mechanical.
- SR coupling adds zero new vocabulary — just a per-room flag selecting
  the metric. Adding the SR theorem to the game is a data change, plus the
  γ(v) formula and length-contraction collision math.
- Bosses that exploit SR (finite signal speed, false simultaneity) are
  expressible as story-coherent challenges the player overcomes by learning
  the theorem.
- Determinism for RL is preserved: proper-time math is pure, seeded by the
  current state.

## Initial implementation target

Conservative:

1. Add `ProperTime` component in `ambition_engine` with default 1.0.
2. Migrate the player movement integrator to read proper-time scale.
   Existing single-tick-per-frame behavior is the case `scale = 1.0`.
3. Add fractional action carry to whichever input/action systems read
   actions per tick.
4. Add `BoostEntityProperTime` as a `ClockScaleRequest` with domain
   `PlayerClock(entity)`. Solo regime grants it; the existing bullet-time
   path can use it as an alternative to `ScaleGlobalClock`.
5. Document the SR coupling but do not implement it. SR rooms, γ(v)
   computation, and length contraction are deferred until the math
   progression reaches the relevant theorem class.

## Non-goals for the first implementation

- General relativity, curved metrics, geodesic integration. This is the
  natural endpoint of the ladder, not the start.
- Multiple "speed of light" values per room as a balance lever. `c` is a
  single constant initially; per-room `c` overrides only if a room
  genuinely needs it for narrative/mechanical reasons.
- Visual SR effects (relativistic Doppler shift, headlight effect,
  apparent contraction in renders). These are presentation; the simulation
  carries the math, presentation can opt-in later.
- Replacing classical kinematics globally. Most of the game runs Galilean;
  SR is a localized theorem unlock.

## Review notes

- Use `AMBITION_REVIEW(spatial)` aggressively in the integrator and any
  collision code that scales by proper time — these are easy to get
  subtly wrong, especially around velocity vs proper-velocity vs lab-frame
  velocity distinctions.
- Property-test the integrator: at scale 1.0 the new code must produce
  identical results to the old single-tick integrator within float
  tolerance. This is the regression check that the proper-time refactor
  is benign in the Galilean default.
- When SR rooms land, the integrator needs `proptest` invariants for
  causality (no signal propagating faster than `c` in lab frame) and for
  energy/momentum conservation in elastic collisions, since these are
  load-bearing physical properties the player will rely on.
- Cross-reference ADR 0010 — `BoostEntityProperTime` is one of the
  `ClockScaleRequest` operations governed by regime policy.
