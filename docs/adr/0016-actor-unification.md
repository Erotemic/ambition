# ADR 0016: Unify NPCs, enemies, hazards, and interactables as actor-like ECS data

## Status

Accepted direction; implementation remains incremental.

## Context

Earlier notes separated interactions, hazards, enemies, bosses, debug labels, and NPCs as independent skeleton systems. That made sense while proving primitives, but it is not the enduring architecture. The current direction is data-driven ECS: authored entities and generated specs become components, systems interpret components, and presentation consumes resulting messages/state.

## Decision

Use actor-like ECS data for gameplay entities that can be spawned, damaged, interacted with, addressed by content IDs, or participate in encounters.

This does not mean every entity shares one mega-component. It means common identity, faction, health/damage, interaction, movement/path, and encounter hooks should compose through components and reusable engine vocabulary.

## Consequences

Old skeleton docs such as interaction/hazard/actor patch notes are historical. Current work should update components, concepts, systems, and tests rather than adding another one-off taxonomy.
