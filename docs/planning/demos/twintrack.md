# TwinTrack

> **Status (2026-08-05): SR signal-course prototype implemented; local Rust compile and visible-feel validation remain.**

TwinTrack is the 2D executable acceptance game for Ambition's flat-spacetime
relativity stack. The controlled traveler leaves a laboratory, accelerates on a
one-dimensional rail, and emits a light pulse from an onboard transmitter. A
stationary passband receiver accepts the pulse only when relativistic Doppler
shift moves its source-local frequency into range. The same packet reaches a
radar retroreflector, returns at the invariant speed, and must be received by
the moving traveler before the twins reunite.

The prototype deliberately combines coordinate-time and proper-time systems:

- laboratory machinery, signal propagation, and receiver events use the
  Minkowski chart's coordinate time;
- traveler clocks, animation, and transmitter recharge use traveler proper
  time;
- received frequency is measured in the receiver's local inertial frame;
- the HUD reports real arrival events in both laboratory coordinates and the
  traveler's instantaneous inertial coordinates.

## Consumes

- the normal provider/session lifecycle;
- ordinary platformer movement and collision as the sole body-motion authority;
- `ambition_relativity` Minkowski events, clocks, Lorentz boosts, and Doppler
  measurement;
- `ambition_relativity2d` proper clocks, analytic null signals, local emitters
  and receivers, event history, and bounded worldline telemetry;
- declared HUD and optional visible gizmo presentation.

## Owns

- the laboratory's authored invariant speed (`c = 600` world units/s);
- a traveler tuned to a subluminal `0.9c` maximum rail speed;
- the transmitter's `100 Hz` proper frequency and `0.75 s` proper-time cooldown;
- the stationary Doppler station's accepted frequency band;
- the radar reflector, turnaround, reunion, and result rules;
- course wording, signal colors, receiver markers, and the compact spacetime
  trace.

## Play loop

1. Hold right to depart and accelerate.
2. Press Interact/F/RB near maximum speed. The forward pulse is blue-shifted
   into the stationary station's passband.
3. Continue toward the radar station while the pulse propagates exactly at
   `c`.
4. The radar station coherently retroreflects the same packet. Catch the echo.
5. Turn around and reunite with the laboratory clock.
6. Compare coordinate arrival times, receiver proper time, the twins' elapsed
   proper times, and the transformed event offset in the traveler's current
   inertial frame.

## Acceptance

- the provider runs standalone and in `ambition_app`;
- one session-scoped Minkowski provider owns coordinate time;
- the traveler and laboratory clocks are present and rollback-safe;
- a forced `0.9c` worldline accumulates proper time at approximately
  `sqrt(1 - 0.9^2)` times the laboratory rate;
- a scripted participant can accelerate, emit one qualifying packet, observe
  ordered arrivals at the passband station and radar reflector, catch the
  reflected packet, reverse, and reunite;
- the passband measurement matches the exact emitter-to-receiver Doppler
  calculation;
- signal crossings are processed in coordinate-arrival order, independent of
  ECS/channel iteration order;
- transmitter cooldown advances in proper time;
- worldline telemetry discards samples from abandoned rollback futures and
  from prior coordinate-time epochs;
- leaving the session removes the spacetime provider and clears derived views;
- no relativity system changes body position or velocity.

TwinTrack is not Slower Light. It does not transform nearby world rendering,
compute retarded images, vary `c`, or provide a 3D observer view. It grounds the
clock, event, null-signal, local-frequency, and observer-coordinate machinery
those later systems need.
