# ADR 0016: Unify NPCs, enemies, hazards, and interactables as actor-like ECS data

## Status

Accepted direction; implementation remains incremental.

## Decision

Represent gameplay entities that can be spawned, addressed, damaged, interacted with, or used in encounters as actor-like ECS data. This is a compositional ECS vocabulary, not one mega-component.

Common reusable pieces include identity/content IDs, faction, health/damage, interaction hooks, movement/path behavior, encounter membership, and presentation messages.

## Context

Earlier patch notes separated interactions, hazards, enemies, bosses, labels, and NPCs into independent skeleton systems. That was useful while proving primitives, but it is not the enduring architecture. The current project direction is data-driven ECS: authored/generated data becomes components; systems interpret components; presentation consumes resulting state/messages.

## Consequences

- Old interaction/hazard/actor skeleton docs are historical.
- New one-off taxonomies should be avoided unless a concept genuinely cannot compose through actor-like ECS data.
- Dialogue/commerce, combat, hazards, bosses, and authored LDtk entities should converge on shared identity and interaction vocabulary where practical.

## Current implications for agents

- Before adding a new gameplay entity category, check whether actor/faction/damage/interactable components already express it.
- Keep component vocabulary small and compositional.
- Update concept/system docs when a reusable actor invariant changes.
