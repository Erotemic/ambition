# TwinTrack

> **Status (2026-08-05): SR-3 causal-pursuit prototype landed — compiled, five
> acceptance tests green, and PHOTOGRAPHED.** `capture_twintrack` is the
> instrument; it found three presentation defects the headless tests could not
> see (a clear that erased the laboratory, a camera whose near plane culled half
> the panel, a layout that asked the window how big it was). ▢ **still open: the
> four HUD slots overlap into unreadable text in the top-left** — that predates
> this work and is the demo's biggest remaining readability problem.

TwinTrack is the 2D executable acceptance game for Ambition's flat-spacetime
relativity stack. The controlled traveler leaves a laboratory, accelerates on a
one-dimensional rail, and emits a light pulse from an onboard transmitter. A
second observatory viewport renders an observer-local synthetic sky and compact
source images selected from the traveler event's past light cone. A
stationary passband receiver accepts the pulse only when relativistic Doppler
shift moves its source-local frequency into range. The same packet reaches a
radar retroreflector and returns at the invariant speed. Receiving the echo
starts a causal-pursuit challenge: a compact beacon begins a known inertial
worldline, its red retarded image diverges from its current and future position,
and the participant must aim a new null signal along the exact green intercept
direction before returning to reunite.

The prototype deliberately combines coordinate-time and proper-time systems:

- laboratory machinery, signal propagation, and receiver events use the
  Minkowski chart's coordinate time;
- traveler clocks, animation, and transmitter recharge use traveler proper
  time;
- received frequency is measured in the receiver's local inertial frame;
- the HUD reports real arrival events in both laboratory coordinates and the
  traveler's instantaneous inertial coordinates;
- the observatory applies exact point-source aberration and Doppler shift,
  while a bounded spacetime strip plots worldlines, null packets, and the
  traveler's past light cone;
- a causal-targeting view publishes the target's retarded apparent direction,
  coordinate-now direction, exact constant-velocity null intercept, and the
  observer-local firing direction without steering the controlled body.

## Consumes

- the normal provider/session lifecycle;
- ordinary platformer movement and collision as the sole controlled-traveler motion authority;
- `ambition_relativity` Minkowski events, clocks, Lorentz boosts, and Doppler
  measurement;
- `ambition_relativity2d` proper clocks, analytic null signals, local emitters
  and receivers, event history, and bounded worldline telemetry;
- observer past-light-cone solving and photon-local measurement;
- declared HUD, optional visible gizmos, and a private-layer observatory camera.

## Owns

- the laboratory's authored invariant speed (`c = 600` world units/s);
- a traveler tuned to a subluminal `0.9c` maximum rail speed;
- the transmitter's `100 Hz` proper frequency and `0.75 s` proper-time cooldown;
- the stationary Doppler station's accepted frequency band;
- the radar reflector, causal-pursuit phase, turnaround, reunion, and result rules;
- a deterministic inertial chase beacon used as both a retarded-image source
  and a swept light receiver;
- observer-local aim state and lab/dual/optical-focus instrument modes;
- course wording, spectral colors, receiver markers, the synthetic star field,
  and the compact spacetime trace.

## Play loop

1. Hold right to depart and accelerate.
2. Press Interact/F/RB near maximum speed. The forward pulse is blue-shifted
   into the stationary station's passband.
3. Continue toward the radar station while the pulse propagates exactly at
   `c`.
4. The radar station coherently retroreflects the same packet. Catch the echo.
5. The view expands into optical-focus mode. The moving beacon's red marker is
   its retarded image; the green marker is the null-intercept direction. Aim the
   cyan reticle with the observer-local aim controls and fire a pursuit pulse.
6. After the pulse intersects the beacon's future worldline, turn around and
   reunite with the laboratory clock.
7. Compare coordinate arrivals, receiver proper time, pursuit intercept time,
   both twins' elapsed proper times, and the transformed event facts.

## Acceptance

- the provider runs standalone and in `ambition_app`;
- one session-scoped Minkowski provider owns coordinate time;
- the traveler and laboratory clocks are present and rollback-safe;
- a forced `0.9c` worldline accumulates proper time at approximately
  `sqrt(1 - 0.9^2)` times the laboratory rate;
- a scripted participant can accelerate, emit one qualifying packet, observe
  ordered arrivals at the passband station and radar reflector, catch the
  reflected packet, acquire an observer-local lead solution, hit the moving
  beacon with a null signal, reverse, and reunite;
- the passband measurement matches the exact emitter-to-receiver Doppler
  calculation;
- signal crossings are processed in coordinate-arrival order, independent of
  ECS/channel iteration order;
- transmitter cooldown advances in proper time;
- worldline telemetry discards samples from abandoned rollback futures and
  from prior coordinate-time epochs;
- leaving the session removes the spacetime provider and clears derived views;
- the moving chase beacon is published at a past-light-cone event whose
  separation from reception is null within the sampled solver tolerance;
- the constant-velocity intercept solves `|r + vt| = ct`, returns the earliest
  future root, and rejects targets whose worldlines are not timelike;
- converting the exact chart firing direction into observer-local aim and back
  round-trips within numerical tolerance;
- the red apparent marker, yellow coordinate-now marker, green intercept marker,
  and cyan participant aim remain explicitly distinct in the observatory;
- the optical star field and compact-source proxies respond to observer
  aberration, Doppler factor, and a documented point-source beaming proxy;
- no relativity system changes body position or velocity; TwinTrack alone owns the beacon's prescribed inertial content worldline.

TwinTrack is not Slower Light. Its observatory is exact for ideal distant
point sources and for compact emitters represented by one sampled worldline,
but it does not transform arbitrary nearby tile/sprite geometry, perform full
radiative transfer, vary `c`, or provide a 3D observer view. The main viewport
remains the laboratory coordinate chart; the private observatory camera renders
only derived proxies. This grounds the clock, event, null-signal, observer, and
past-light-cone machinery those later systems need.
