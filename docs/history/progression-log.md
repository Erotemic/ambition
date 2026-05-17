# Progression log

Compact chronology of major direction changes.

## Early direction

- The engine began as a mostly code-driven mechanics sandbox.
- RON room/world manifests were used to get reliable startup and quick iteration.
- Backend-neutrality was considered useful while primitives were young.

## Current direction

- Backend-neutrality is superseded: Ambition is Bevy-native.
- LDtk is the world/level authoring source.
- Bevy ECS is the runtime integration language.
- RON remains useful for tuning, save/settings, and other compact data, but not as the primary room/world authoring path.
- The sandbox now cares about desktop, web, Android/mobile touch, controller, and Steam Deck from the start.

## Documentation direction

- `docs/brainstorms/` remains active idea incubation.
- ADRs must stay modern.
- Stale migrations and one-off patch notes move to `docs/archive/` or are deleted.
- Concept pages and generated `.agent/` indexes make the repo easier for agents to navigate.
