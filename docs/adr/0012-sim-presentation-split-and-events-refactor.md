# ADR 0012: Simulation emits gameplay messages; presentation consumes them

## Status

Accepted. The original event-refactor roadmap has landed and is archived.

## Context

The sandbox originally mixed simulation, presentation, audio, VFX, debris, setup, and app-builder concerns in broad update paths. That made headless tests weak and broad overlays risky.

## Decision

Simulation systems should produce typed gameplay messages/events or component state. Presentation systems consume that state to play audio, spawn VFX, draw sprites, update HUD/UI, and drive debug overlays.

The Bevy app should be composed through named plugin/setup phases rather than one monolithic update function. Minimal/headless test paths should exercise gameplay without requiring rendering/audio/window plugins.

## Current implications

- Keep app-builder helpers small and named by responsibility.
- Do not make simulation code directly depend on presentation effects.
- Avoid widening internal fields just to let tests poke through; prefer messages/components/resources with focused test setup.
- When replacing broad sandbox files, check platform feature gates and presentation entrypoints.

## Consequences

The archived event-refactor plan is historical evidence. Current agents should read this ADR, `docs/concepts/sim-presentation-seam.md`, and focused source/tests instead of following the old migration checklist.

## Current implications for agents

- Keep gameplay semantics in simulation data/messages.
- Keep audio, VFX, sprites, UI, and debug overlays in presentation.
- Use tests/headless paths for simulation behavior where possible.
