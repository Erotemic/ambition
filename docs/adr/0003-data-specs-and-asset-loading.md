# ADR 0003: Data specs feed Bevy ECS; LDtk owns world authoring

## Status

Accepted and updated 2026-05-17.

## Decision

Use authored/generated data as inputs to Bevy ECS runtime state. LDtk is the current source of truth for world and level authoring. RON room manifests are historical, not the world-authoring direction.

Current ownership:

- LDtk owns areas, levels, collision layers, loading zones, authored spatial entities, and world composition.
- Bevy ECS owns runtime entities, components, resources, schedules, and messages.
- RON remains acceptable for tuning, save/settings, generated-audio specs, and small structured non-world data.
- `ambition_asset_manager` owns asset identity/source resolution across platform profiles.
- Generated tools may create source artifacts, but runtime use requires an explicit publish/install/catalog step.

## Context

Early Ambition used RON room/world manifests to bootstrap reliable iteration. That was useful, but the project has shifted toward LDtk-authored world data and Bevy-native ECS integration. Keeping live docs that imply RON room manifests are the future confuses agents and encourages parallel world state.

## Consequences

World/level changes should flow through LDtk tooling and runtime ECS projection. Code should not add a second durable room format unless a new ADR intentionally reopens that decision.

Platform-specific asset loading is still allowed: web, Android, desktop, Steam Deck, and headless profiles may resolve the same logical asset differently.

## Current implications for agents

- Do not hand-edit `sandbox.ldtk`; use `tools/ambition_ldtk_tools/`.
- Do not describe RON room manifests as current world truth.
- When adding runtime content, identify whether it is authored LDtk data, generated asset data, tuning/config data, or save/settings data.
- Check `docs/systems/ldtk-world-composition.md`, `docs/tools/ldtk-tools.md`, and `docs/systems/asset-manager.md` before changing world or asset loading paths.
