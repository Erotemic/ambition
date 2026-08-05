# Relativity capability

> **Status (2026-08-05): SR clocks, Lorentz events, analytic null signals, Doppler receivers, and bounded worldline telemetry are implemented for the TwinTrack prototype; local Rust compile validation remains.**

Ambition treats special relativity as the first exact spacetime model, not as a
bag of visual effects. The reusable boundary is:

```text
relativistic consumers
  clocks, local mechanisms, signals, observers, presentation
                         ↑
2D spacetime-provider and null-signal adapter
                         ↑
Minkowski now; analytic/sampled/evolved GR later
```

## Ownership

- `ambition_relativity` owns dimension-independent Minkowski interval,
  proper-time, rapidity, velocity-composition, event-boost, and photon-frequency
  mathematics. It has no Bevy dependency.
- `ambition_relativity2d` reads canonical 2D body kinematics, samples a
  session-owned spacetime provider, writes the existing `ProperTimeScale`,
  accumulates f64 proper time, propagates analytic null signals, measures local
  receiver frequency, records bounded arrival/worldline telemetry, and
  publishes presentation read models.
- A provider owns the selected spacetime model and coordinate-time epoch.
  TwinTrack selects Minkowski with an authored invariant speed.
- Relativity observes the movement kernel. It does not become a second pose or
  velocity authority.

## Current SR systems

### Clocks and intervals

Marked entities receive a spacetime-derived proper-time rate. Null and
spacelike worldlines are classified without generating NaNs. Engine mechanisms
that already consume `ProperTimeScale` require no relativity-specific branch.

### Events and observer coordinates

`MinkowskiEvent` and exact Lorentz boosts transform event-coordinate
differences between inertial frames. These are coordinate transforms, not
claims about optical appearance. TwinTrack uses this surface to report a real
arrival event relative to the traveler's instantaneous inertial frame.

### Analytic null signals

A light signal is canonical state defined by an emission event, normalized
chart direction, chart frequency, packet identity, and invariant speed. Its
position is evaluated analytically rather than integrated as an ordinary
projectile. Swept signal/receiver intersections prevent tunneling and are sorted
by coordinate-arrival fraction before effects are applied.

### Local emission and reception

An emitter declares a source-local proper frequency. The SR kernel converts it
to chart frequency from the emitter four-velocity. A receiver then measures
that photon frequency against its own local four-velocity. Passbands are
content policy; the measurement is reusable engine work.

Reflect mode is a coherent retroreflector: the incoming frequency measured in
the receiver frame becomes the outgoing source-local frequency for a reversed
chart direction. This is exact for the one-dimensional TwinTrack use and leaves
room for later tetrad/specular policies in more dimensions.

### Coordinate and proper timers

Session coordinate time advances once per simulation tick. `ProperTimeCooldown2d`
advances through the marked entity's proper-time rate. Coordinate-time resets
increment an epoch so derived histories cannot mix separate experiments.

### Telemetry

Worldline telemetry is opt-in, bounded, and derived. It is keyed by simulation
tick, truncates abandoned rollback futures, and clears when coordinate epochs
change. Signal-arrival history is bounded canonical state because game rules
may depend on which packet reached which receiver and when. Receiver passbands
and deterministic signal-pool slots are canonical too, while transient emission
and arrival message buffers are cleared on rollback before resimulation.

## Cost contract

Games that do not enable the facade's `relativity` feature do not link either
crate. Linking the crates does no work. Installing `Relativity2dPlugin` adds only
spacetime-presence checks until a session owns `ActiveSpacetime2d`.

Costs are proportional only to opted-in data:

- marked clocks: one model sample and small clock calculation per tick;
- active signals: analytic position plus swept tests against registered
  receivers;
- tracked worldlines: one bounded sample per marked track per tick;
- presentation: rebuild only when a live spacetime exists.

There is no universal shader, global history buffer, velocity clamp, or
relativity scan over ordinary bodies.

## GR growth path

A future GR provider can preserve the consumer-facing concepts while adding
metric samples, tetrads, derivatives, and geodesic integration through additive
traits. Likely providers are:

1. analytic stationary metrics;
2. sampled numerical-relativity output;
3. reduced-order or AI surrogate fields;
4. a live Rust field solver.

Minkowski remains the exact flat limit and regression oracle. Proper clocks,
emission/reception events, photon wavevectors/frequencies, local observer
measurements, and worldline diagnostics remain useful around curved or evolved
metrics. `ambition_relativity2d` means a two-coordinate game adapter, not a
commitment to physical 2+1-dimensional gravity.

## Explicitly absent from this slice

- curved metrics and timelike/null geodesic solvers;
- dynamic spacetime or backreaction;
- general relativistic collision response;
- retarded rendering of nearby geometry;
- Doppler/aberration full-screen shaders;
- a relativistic movement authority or global speed clamp;
- the future 3D Slower Light game.
