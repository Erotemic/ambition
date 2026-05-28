# ADR 0002: The engine must be Bevy-native

## Status

Accepted. Supersedes the older “engine may be Bevy-native” phrasing. Largely subsumed 2026-05-28 by the deletion of the `ambition_engine` crate — what remains of "the engine" is now a sandbox module at `crates/ambition_sandbox/src/engine_core/`. The decision below still applies to that module: Bevy-native semantics, no presentation/app-shell concerns.

## Context

Earlier notes tried to keep `ambition_engine` backend-neutral. That direction is now counterproductive. Ambition is built around Bevy 0.18, Bevy math, Bevy ECS integration, Leafwing input, Bevy states/schedules, inspector/dev tooling, LDtk runtime integration, and platform feature composition.

The important boundary is not "no Bevy." The important boundary is whether a type or system is reusable mechanics vocabulary or sandbox presentation/app-shell policy.

## Decision

The engine_core module is Bevy-native. It may use Bevy math/types and Bevy-friendly vocabulary when that improves correctness, testability, or integration.

The project should lean into Bevy idioms:

- ECS components/resources/events/messages as integration seams,
- feature-gated app composition,
- Bevy math and geometry compatibility,
- Bevy-friendly asset and platform integration,
- tests that can run with minimal Bevy apps where useful.

The engine_core module still should not own sandbox-only presentation concerns: sprite selection, HUD layout, inspector windows, debug overlay styling, temporary feature-room visuals, app shell wiring, or platform packaging scripts.

## Consequences

Do not add abstractions merely to preserve backend neutrality. Add seams when they improve testability, reuse, or platform composition. Prefer data-driven ECS flow over parallel bespoke state whenever the system is moving toward runtime gameplay.

## Current implications for agents

- Prefer Bevy-native ECS/data flow over backend-neutral wrappers.
- Keep sandbox presentation/app-shell policy out of reusable engine semantics.
- Do not revive the old "may be Bevy-native" ambiguity.
