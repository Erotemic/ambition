# Relativity capability

> **Status (2026-08-04): SR foundation implemented by the TwinTrack candidate overlay; Rust compile validation remains required after application.**

Ambition treats special relativity as the first exact spacetime model, not as a
bag of effects. The reusable boundary is:

```text
relativistic consumers
  clocks, observers, signals, presentation
              ↑
2D spacetime-provider adapter
              ↑
Minkowski now; analytic/sampled/evolved GR later
```

## Ownership

- `ambition_relativity` owns dimension-independent Minkowski interval, clock-rate,
  rapidity, and velocity-composition mathematics. It has no Bevy dependency.
- `ambition_relativity2d` reads canonical 2D body kinematics, samples a session-owned
  spacetime provider, writes the existing `ProperTimeScale`, accumulates f64 proper
  time, and publishes a read model.
- A provider owns the selected spacetime model. TwinTrack selects Minkowski with
  an authored invariant speed.
- Relativity observes the movement kernel. It does not become a second pose or
  velocity authority.

## Cost contract

Games that do not enable the facade's `relativity` feature do not link either
crate. Linking the crates does no work. Installing `Relativity2dPlugin` adds only
spacetime-presence checks until a session owns `ActiveSpacetime2d`; only entities
marked `RelativisticClock2d` are sampled.

There is no universal history buffer, shader, speed clamp, or world scan.

## GR growth path

A future GR provider can preserve the consumer API while adding metric samples,
tetrads, derivatives, and geodesic integration through additive traits. Likely
providers are:

1. analytic stationary metrics;
2. sampled numerical-relativity output;
3. reduced-order or AI surrogate fields;
4. a live Rust field solver.

Minkowski remains the exact flat limit and the first regression oracle for every
curved provider. `ambition_relativity2d` means a two-coordinate game adapter, not
a commitment to physical 2+1-dimensional gravity.

## Explicitly absent from the first slice

- curved metrics and geodesics;
- dynamic spacetime or backreaction;
- retarded rendering and perception;
- Doppler/aberration shaders;
- a global relativistic movement cap;
- the future 3D Slower Light game.
