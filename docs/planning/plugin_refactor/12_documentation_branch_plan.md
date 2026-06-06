# Documentation Branch Plan

The docs branch should update the project's architecture story before the code branch makes large changes.

## Branch

```bash
git switch -c docs/pluginized-platformer-runtime-roadmap
```

## Add generated inventory

Run and check in inventory snapshots:

```bash
python tools/ecs_inventory.py \
  --json docs/generated/ambition_ecs_inventory.baseline.json \
  --markdown docs/generated/ambition_ecs_inventory.baseline.md

python tools/ecs_inventory.py --include-tests \
  --json docs/generated/ambition_ecs_inventory.with_tests.baseline.json \
  --markdown docs/generated/ambition_ecs_inventory.with_tests.baseline.md
```

If the tool's CLI differs, use the closest equivalent and document the command that generated the files.

## Docs to add or update

Suggested docs:

```text
docs/planning/plugin_refactor/
docs/systems/plugin-boundaries.md
docs/systems/entity-lifecycle.md
docs/systems/content-pack-boundary.md
docs/systems/build-personas.md
docs/systems/portal-plugin.md
docs/current/risks.md
docs/current/state.md
```

## Stale-doc search topics

Search existing docs for statements that imply:

```text
- engine_core is the final reusable engine boundary
- ambition_sandbox is the final home for reusable mechanics
- app/plugins.rs is the acceptable subsystem registration hub
- portal/item/boss systems are sandbox-local by default
- headless is only a test harness rather than a supported build persona
- pluginization is optional cleanup rather than the next architecture layer
- LDtk conversion should know every optional mechanic directly forever
```

## Documentation decisions to record

Record explicit decisions for:

```text
- proto-crate module before real crate extraction
- plugin ownership before file splitting for portal
- build personas over arbitrary feature combinations
- data owns nouns/tuning, Rust plugins own behavior
- portal gun portals and authored gate portals are separate concepts
- mechanics plugins should be reusable without Ambition content
```

## Output expectation

The docs branch should make it possible for an implementation agent to start without rereading the entire conversation. It should answer:

```text
What is the target architecture?
What are the next stages?
What is allowed to depend on what?
What does the portal API look like?
What are the validation commands?
What risks are known?
```
