# Time domains, proper time, and simulation clocks

**Checked 2026-08-07.** Ambition already has a richer time model than a single
`delta_time` plus global slow-motion scalar. The related-work section should
compare that implemented contract against the clock models developers already
know from Bevy, Unity and Unreal.

## The Ambition capability that already exists

[`ambition_time`](../../crates/ambition_time/src/lib.rs) names separate time
questions instead of asking every system to read a generic delta:

- `WorldTime::sim_dt()` — authoritative gameplay time;
- `WorldTime::wall_dt()` — unscaled duration of the simulation step;
- `WorldTime::player_dt(observer)` — observer/cognitive-clock seam;
- `WorldTime::entity_dt(ProperTimeScale)` — per-entity proper time;
- `PresentationTime::scaled_dt()` — render-frame presentation time affected by
  global world scaling;
- `PresentationTime::wall_dt()` — render-frame wall time for UI/audio/tooling;
- `SimTick` — the canonical integer simulation timeline used by input, hashing
  and rollback.

The source also documents an important scheduling invariant: presentation must
not consume the most recent fixed simulation `WorldTime` as if it were render
frame time, because that would make animation speed depend on display refresh.
The two clocks are produced at different boundaries on purpose.

`ProperTimeScale` is already a component rather than a speculative design note,
and [`ambition_relativity`](../../crates/ambition_relativity/src/lib.rs) /
[`ambition_relativity2d`](../../crates/ambition_relativity2d/src/lib.rs) build
relativistic proper-time and worldline behavior above this general clock seam.

---

## Bevy `Time<Real>`, `Time<Virtual>`, and `Time<Fixed>` — typed clocks in the substrate

Bevy distinguishes real, virtual and fixed time. `Time<Virtual>` can be paused or
scaled, while fixed time advances in fixed increments following virtual time.
Bevy's fixed-timestep examples explicitly separate simulation advancement from
render-frame cadence.

Sources:

- [Bevy 0.12 time redesign](https://bevy.org/news/bevy-0-12/)
  (Bevy, official; this is the release that documents the typed clock split).
- [Run physics in a fixed timestep](https://bevy.org/examples/movement/physics-in-fixed-timestep/)
  (Bevy, official current example).

### Comparison

Ambition is correctly building **on top of**, rather than replacing, this Bevy
mechanism. Its additional value is semantic narrowing: engine systems ask for
“authoritative sim dt”, “presentation wall dt”, or “this entity's proper dt”
instead of selecting a Bevy clock ad hoc.

That is exactly the sort of policy layer an engine should own. Bevy provides the
clock machinery; Ambition defines which clock is legal for which gameplay
meaning.

---

## Unity `Time.timeScale` and `unscaledDeltaTime` — familiar global scaled/unscaled time

Unity exposes a global `Time.timeScale` controlling the rate of in-game time and
an `unscaledDeltaTime` that ignores that scale.

Sources:

- [`Time.timeScale`](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Time-timeScale.html)
  (Unity, official).
- [`Time.unscaledDeltaTime`](https://docs.unity3d.com/6000.0/Documentation/ScriptReference/Time-unscaledDeltaTime.html)
  (Unity, official).

### Comparison

This maps cleanly onto only one slice of Ambition:

```text
Unity timeScale            ~ ClockState::time_scale
Unity deltaTime            ~ scaled world/presentation dt, depending on stage
Unity unscaledDeltaTime    ~ wall dt
```

Ambition should make that mapping easy for incoming engine users, but it should
not collapse back to Unity's vocabulary because the engine already needs more:
rollback's integer simulation tick, render-vs-sim cadence, observer time and
per-entity proper time are distinct contracts.

---

## Unreal `CustomTimeDilation` — per-actor time scaling exists in a mature engine

Unreal Actors expose custom time dilation in addition to world time dilation.
This is important prior art: **per-entity time scaling itself is not a novel
feature**.

Source: [Unreal `AActor`](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/AActor?lang=en-US)
(Epic, official; `CustomTimeDilation` is part of the actor API).

### Comparison

Ambition's differentiating question is not “can one actor run slower?” It is
whether the engine can make the relevant time domain **explicit and composable**
across gameplay, rollback, presentation and special relativity.

`ProperTimeScale` is therefore better understood as a general engine clock
primitive than as an effect field on a particular actor class. It can serve
ordinary haste/slow effects, observer cognitive time and relativistic proper
rate without making those systems share implementation.

---

## The time taxonomy Ambition should publish

The source already implies a taxonomy that deserves a single public table:

| Clock/question | Authority | May scale? | Rewinds? | Typical consumers |
|---|---|---:|---:|---|
| `SimTick` | simulation host | no; integer step | yes under rollback | input frames, hashes, causal facts, snapshots |
| sim dt | authoritative simulation | global world scale | replayed deterministically | movement, combat, AI, gameplay timers |
| entity proper dt | authoritative simulation | global × entity rate | replayed deterministically | per-body cooldowns/animation/relativity |
| participant cognitive dt | simulation policy | potentially participant-specific | must follow host policy | future observer/player-time mechanics |
| presentation scaled dt | presentation | world scale | no simulation rewind | visual interpolation/effects |
| presentation wall dt | host/presentation | no | no | UI, audio, hot reload, diagnostics |

That table is a stronger onboarding tool than telling users “use `Time`”. It also
makes illegal cross-domain reads mechanically reviewable.

## Design work the comparison exposes now

### 1. Decide composition laws for multiple scales

The source currently has global `ClockState::time_scale` and per-entity
`ProperTimeScale`. Specify the complete law when other regimes arrive:

- world scale × participant cognitive rate × entity proper rate?
- which domains affect input windows, cooldowns, animation, AI and audio?
- can a capability request a rate, or only read the granted rate?

The answer should be one policy service, not multiplication scattered across
systems.

### 2. Make timer ownership encode its clock

A timer/cooldown should state its clock domain or be wrapped by a typed timer
whose constructor does. This would let tests and static review detect a UI timer
accidentally running on sim time or a rollback-owned cooldown reading wall time.

### 3. Define proper-time behavior across possession and body changes

Because control authority can move between bodies, body proper time and
participant cognitive time must remain different. Possessing a slowed body
should not silently redefine what the participant clock means. The architecture
already has the vocabulary; this interaction should be pinned with tests.

### 4. Include clock state in causal explanations

When a cooldown, animation phase or movement result surprises a developer, the
explanation should report the relevant domain and effective dt/rate. That turns
“why was this only half finished?” into a semantic query rather than a hunt for
which time scalar happened to be read.

## What this comparison changed

Ambition's time layer is already an engine feature worth documenting as related
work. The comparison says:

- **use Bevy's typed clocks as substrate;**
- **offer Unity-familiar scaled/unscaled mental mappings;**
- **acknowledge Unreal's per-actor dilation as prior art;**
- **differentiate through an explicit clock-domain contract spanning fixed
  simulation, rollback, presentation, participants and proper time.**
