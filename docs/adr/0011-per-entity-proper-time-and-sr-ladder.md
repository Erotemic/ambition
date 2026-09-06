# ADR 0011: Per-entity proper time and the Galilean-to-SR ladder
## Status

Accepted. The TwinTrack slice provides the first concrete SR producer and game acceptance surface.

## Decision

Use per-entity proper-time vocabulary as the long-term way to represent local time manipulation. Global clock scaling remains a useful early/single-player behavior, but it is not the only conceptual model.

The long-term ladder is:

1. Galilean/single-clock gameplay for current simple mechanics.
2. Per-entity local clocks for coherent multi-observer slow/fast effects.
3. Minkowski spacetime as the exact SR model (`ambition_relativity` + the opt-in 2D adapter).
4. Prescribed curved-spacetime providers, geodesics, and observer optics when a concrete game needs them.
5. Research numerical-GR providers only behind the same sampling boundary.

## Context

For one player, slowing the world and boosting the player's proper time can feel equivalent. For multiple observers, replay, AI training, or story mechanics about who controls time, those operations differ. The project wants the vocabulary without overbuilding the full system before gameplay requires it.

## Consequences

- Proper-time language is valid design vocabulary.
- The current implementation may still use simpler clock handling.
- Future local-clock mechanics should integrate with ADR 0010 regime policies.

## Current implications for agents

- Do not implement speculative relativity infrastructure without a concrete gameplay use.
- Do not delete the vocabulary just because the full ladder is not landed.
- When adding time-affecting mechanics, say whether it is global sim time, entity proper time, presentation-only time, or wall-clock behavior.

## 2026-08-04 implementation amendment

TwinTrack supplies the concrete mechanic that this ADR previously required before infrastructure work: a player chooses a high-speed worldline and reunites with a stationary clock. The SR implementation is intentionally the flat provider behind a spacetime interface, so future GR work can retain the clocks, observer utilities, rollback state, and Minkowski regression tests without making a global inertial frame an engine assumption.

## 2026-08-05 signal-system amendment

TwinTrack's second slice makes the SR foundation operational beyond clock
comparison. Canonical emission events produce analytic null signals; receivers
measure frequency in their own local inertial frame; coherent retroreflection
preserves packet identity; transmitter cooldown advances in proper time; and
bounded worldline/event telemetry supports both rollback diagnostics and a
spacetime instrument. The movement kernel remains authoritative for bodies.
These event, observer, and photon-measurement concepts are intended to remain
valid when Minkowski is replaced by a curved spacetime provider.

## 2026-08-05 observer-optics amendment

TwinTrack's SR-2 slice adds a derived observer pipeline without changing the
authoritative Minkowski chart. Opt-in compact sources record bounded
worldlines; a fixed-iteration solver intersects them with the controlled
observer's past light cone; the local SR kernel then measures photon direction,
Doppler factor, and a documented point-source beaming proxy. The laboratory
viewport remains coordinate truth while a private-layer camera renders only
observer-derived stars, source proxies, signals, and spacetime traces. This is
the intended growth shape for GR: a future metric/geodesic provider may replace
worldline and photon propagation underneath while preserving observer-local
consumers.

## 2026-08-05 causal-pursuit amendment

TwinTrack's SR-3 causal-pursuit slice makes observer relativity mechanically
consequential: the light-delayed apparent source direction is not the direction that
a newly emitted signal must follow. A separate derived targeting view solves the
future null intercept and exposes observer-local aim, while ordinary control,
body movement, and signal propagation retain their existing authorities.


## 2026-08-06 Relativity Plaza amendment

TwinTrack's SR-4 slice establishes that relativity gameplay need not be a rail
experiment or an instrument panel. A permanent `FreeFlight` grant separates
flight capability from permission to expose a runtime flight toggle. The shared
axis-swept movement authority optionally integrates 2D acceleration in spatial
proper velocity, converts back to guaranteed-subluminal coordinate velocity,
and enforces one radial terminal speed. Relativity remains a consumer of that
canonical worldline rather than a second pose writer.

Light signals now carry stable emitter identity and an opaque game payload,
which makes finite-speed dialogue and social mechanics first-class without
baking TwinTrack rules into the signal engine. Clock reports, Doppler music, and
light tag are content interpretations of the same emission/arrival vocabulary.
The demo's player-facing language uses “light-delayed” and “when this message
left”; standard specialist terminology remains available in research code and
advanced documentation.

The optional 2+1D spacetime sculpture is a derived teaching projection of the
same worldlines, signals, and observer event. It never becomes an authoritative
3D simulation or a steering requirement.

## SR-5 festival polish (2026-08-06)

TwinTrack now exposes existing SR state through visible clocks, labeled light
packets, a continuous exact Doppler-note instrument, progressively reduced
light-tag assistance, and a post-reunion replay. The continuous pitch preview is
derived, not added to canonical rollback state. The guided phase, attempts/hits,
and replay cursor are canonical because they alter course progression or
participant-controlled presentation across rewind; rollback schema v15 records
the changed `TwinTrackExperiment` encoding.
