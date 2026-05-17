# ADR 0003: Data specs feed Bevy ECS; LDtk owns world authoring

## Status

Accepted and updated 2026-05-17.

## Context

Early Ambition used RON room/world manifests to get reliable startup and fast iteration. That was useful, but the project has shifted to LDtk for world authoring and to Bevy ECS as the runtime integration language. The old phrase "rooms are moving toward RON" is now stale.

## Decision

Use data specs, asset manifests, generated outputs, and authored files as inputs to Bevy ECS runtime state.

Current ownership:

- LDtk owns world/level authoring, collision layers, loading zones, and authored spatial entities.
- RON remains acceptable for movement tuning, ability/tuning config, save/settings data, generated-audio specs, and other compact structured data.
- `ambition_asset_manager` owns asset identity/source resolution across platform profiles.
- Bevy asset loading and embedded/static fallbacks may coexist when required by web/Android/startup constraints.
- Runtime gameplay should move toward components/entities/systems rather than parallel manifest mirrors.

## Consequences

Docs should not describe RON room manifests as the current world source of truth. Agents changing world/level authoring should read the LDtk docs and tools docs first. Agents changing tuning/audio/settings should check whether RON is still the intended data format for that specific subsystem.
