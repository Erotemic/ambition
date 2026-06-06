# ECS Inventory Notes

The ECS inventory is a planning input, not a hard pass/fail metric.

## Current high-level counts

From the current reported run:

```json
{
  "architecture_items": 15,
  "bundles": 8,
  "components": 251,
  "events": 0,
  "message_channels": 27,
  "messages": 19,
  "migration_candidates_high": 135,
  "migration_candidates_medium": 151,
  "module_summaries": 81,
  "non_ecs_items": 506,
  "plugins": 39,
  "registered_systems": 368,
  "registrations": 348,
  "resource_access_entries": 156,
  "resources": 149,
  "spawn_sites": 209,
  "system_like_functions": 565,
  "unique_registration_identifiers": 638
}
```

The `non_ecs_items` count has known false positives, so use it cautiously.

## How to use the inventory

Use the JSON to answer:

```text
Which modules own the most components/resources?
Which systems are registered from app-level plugins?
Which modules have many spawn sites?
Which systems access cross-cutting resources?
Which migration candidates cluster around portal, item, gravity, room, or app code?
```

Use the Markdown to support human review and planning.

## Suggested checked-in paths

```text
docs/generated/ambition_ecs_inventory.baseline.json
docs/generated/ambition_ecs_inventory.baseline.md
docs/generated/ambition_ecs_inventory.with_tests.baseline.json
docs/generated/ambition_ecs_inventory.with_tests.baseline.md
```

## Useful trends after each refactor branch

Track these rough trends:

```text
app/plugins.rs registrations should decrease.
Portal-owned systems should move into portal plugin registration.
Raw room-local spawn sites should decrease or become lifecycle-helper calls.
Generic helpers imported from portal should go to zero outside portal.
Ambition-specific item/quest/boss/world references should migrate into ambition_content.
Rendering/audio/devtool imports should leave headless/runtime layers.
```

Do not optimize for the counts themselves. Optimize for clearer ownership and supported build personas.

## Inventory-driven questions for the code branch

When deeper code work starts, ask:

```text
1. Which resources are accessed by both portal and non-portal systems?
2. Which portal systems must run before or after player/item/actor simulation?
3. Which spawn sites are room-local but lack explicit lifecycle helpers?
4. Which components are presentation-only but live in simulation modules?
5. Which modules import `crate::portal` for non-portal utilities?
6. Which systems in app/plugins.rs can move behind subsystem plugins first?
7. Which tests assume portal is always compiled?
```
