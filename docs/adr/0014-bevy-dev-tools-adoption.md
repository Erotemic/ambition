# ADR 0014: Adopt Bevy dev tools when they improve iteration

## Status

Accepted policy; exact tool coverage is incremental.

## Decision

Use first-party or ecosystem Bevy dev tooling when it improves iteration, debugging, screenshots, state visibility, or CI smoke tests without bloating default distribution builds.

Gate dev-only tooling behind features. Distribution builds should not pay for inspector overlays, dynamic-linking convenience, or screenshot/debug utilities unless explicitly requested.

## Context

Ambition has hand-rolled HUD/debug plumbing plus Bevy inspector panels. As the project leans harder into Bevy-native ECS, dev tooling should also follow Bevy idioms where possible instead of accumulating custom overlays for every diagnostic.

## Consequences

- Dev overlays, frame timing, state inspection, screenshots, and CI helpers should prefer maintained Bevy tooling where it fits.
- Feature flags must keep platform/distribution build costs visible.
- Dev tooling is allowed to change faster than gameplay architecture; do not treat a specific overlay as a durable game system.

## Current implications for agents

- Before adding a custom debug overlay, check whether a Bevy tool already covers the need.
- Keep dev tooling feature-gated and platform-aware.
- Update `docs/systems/developer-tools.md` and `docs/tools/optimization-and-reporting.md` when a dev workflow changes.
