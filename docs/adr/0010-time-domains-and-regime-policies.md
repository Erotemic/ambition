# ADR 0010: Time domains, time-control authority, and regime policies

## Status

Accepted.

## Context

Ambition supports time-bending gameplay verbs (bullet-time blink, future boss
attacks that freeze the world, scripted cutscene pauses). The first
implementations of bullet-time scaled Bevy's `Time` resource directly. That
worked for single-player but was incoherent under multiple observers (a
hypothetical multiplayer mode), produced an opaque "feature flag" rather than
a designable verb, and gave the engine no way to enforce who was allowed to
do what to time.

A series of design conversations clarified that:

- Single-player remains the first-class default. The engine should not
  simplify expressiveness to make SP easier; SP is what falls out of a
  permissive regime configured over the full vocabulary.
- Multi-player (future) and RL-deterministic (future) regimes can coexist
  with single-player by selecting a different policy over the same engine
  vocabulary, not by branching to a different code path.
- Time-control authority is sometimes a narrative property: a boss that
  freezes sim time is "the boss got root on the simulator" — a deliberate
  story claim, not a hardcoded mechanic. Authority should be expressible as
  data, including room/encounter-scoped overrides.
- Bullet-time-blink, boss freeze, scripted cutscene pause, and future
  consensual co-op shared bullet-time are all the *same operation* with
  different requesters under different policies.

The companion architecture-targets memory and the Galilean→SR ladder ADR
(0011) build on this vocabulary.

## Decision

The engine provides a uniform vocabulary for time control. Regimes are
permission tables over that vocabulary, expressed as data.

### Vocabulary

```text
ClockDomain         = SimClock | PlayerClock(p) | WallClock
ClockScaleRequest   = { domain, scale, requester, reason }
RegimePolicy        = (requester, domain) -> Permission
Permission          = Grant | Deny | Rebind(domain) | Broadcast
```

`SimClock` is the global game tick. `PlayerClock(p)` is per-player proper
time (see ADR 0011). `WallClock` is real time, never scaled — used by audio
buses, network code, and presentation effects that must track the host
machine.

`ClockScaleRequest` is the only way for gameplay code to mutate any clock.
Direct `Time::set_relative_speed` calls from gameplay systems are prohibited.

### Regimes

Regimes are configurations of the policy table. Initially defined:

- **Solo** — permissive. Player requesters granted `SimClock` authority.
  Bosses gain authority via room-scoped policy overrides (narrative grants).
  This is the current single-player default.
- **RLDeterministic** — deny-all on time-scale requests. Fixed timestep,
  seeded RNG, no clock mutations from any requester. RL training and CI use
  this.
- **CoopConsensual** *(future)* — a player's grant is broadcast to all
  PlayerClocks; SimClock authority shared among consenting players.
- **Competitive** *(future)* — self-only PlayerClock requests; SimClock
  locked.
- **Cinematic** — narrative scripts hold time authority during scripted
  sequences; player requests deferred until cinematic completes.

Adding a regime is a data change, not a code change.

### Narrative authority

Room/encounter scope can grant a requester authority temporarily. A boss
"getting root on the simulator" is a policy delta scoped to the boss arena,
not a hardcoded mechanic. This dovetails with the AI-agency storyline:
*who has authority over the timeline* is a narrative axis the player
explores.

## Consequences

- Bullet-time-blink, boss sim-freeze, scripted cutscene pause, future co-op
  shared bullet-time are uniform `ClockScaleRequest` dispatches that differ
  only in requester identity and active policy.
- Existing bullet-time code that touches `Time` directly must migrate to
  emit `ClockScaleRequest` instead. The Solo regime grants the request as
  before; the call site discipline becomes uniform.
- Adding a new regime, a new ability that affects time, or a narrative
  policy override is a small, type-safe change against the vocabulary.
- The engine encodes which time-control operations are coherent in which
  regime. SP can't accidentally ship an ability that breaks MP, because
  that ability's request would be denied or rebound under MP regime.
- RL training is just "run the engine in `RLDeterministic` regime." No
  separate "RL build" needed.
- This composes with the per-entity proper-time model in ADR 0011: a
  `BoostEntityProperTime` request and a `ScaleGlobalClock` request are
  observationally equivalent for one observer, but only the former is
  coherent under N-observer regimes.

## Initial implementation target

Conservative migration:

1. Add `ClockDomain`, `ClockScaleRequest`, `RegimePolicy`, `Permission`
   types in `ambition_engine`. No behavior change to existing systems.
2. Add a `RegimeConfig` resource. Single-player default = Solo regime.
3. Migrate the existing bullet-time path to emit `ClockScaleRequest` and
   apply via the regime policy. Solo regime grants identically; behavior
   unchanged.
4. Add an opt-in `RLDeterministic` regime config that any future RL adapter
   selects at app build time.
5. Document narrative authority overrides as room-scoped policy deltas in
   the LDtk authoring docs (deferred until first boss uses it).

## Non-goals for the first implementation

- Multi-player drivers, network sync, lockstep determinism, or
  command-gather phases. CoopConsensual and Competitive regimes are
  defined for future use; not implemented yet.
- Per-domain reflection of all current `Time` reads. Existing engine code
  that reads `Time` continues to do so; the discipline is on *time
  mutations* first, then *reads* migrate as systems evolve.
- A regime DSL covering save state, replay, or non-time concerns. Time is
  the first dimension; other dimensions added when their use case lands.

## Review notes

- Use `AMBITION_REVIEW(spatial)` near anywhere a request's effect on a
  clock interacts with movement integration; per-frame velocity scaling is
  particularly easy to get subtly wrong.
- Add `proptest`-style invariants: under `RLDeterministic`, the same input
  + same seed must produce the same observation; this is the canonical
  regime correctness check.
- Cross-reference ADR 0011 (per-entity proper time) — the two ADRs share
  the `ClockScaleRequest` vocabulary and must stay aligned.
