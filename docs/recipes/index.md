---
status: current
last_verified: 2026-07-18
---

# Recipes

Recipes are copy-pasteable procedures for the current repository. They should
name a supported front door, state what is mutated, and finish with validation.
When a command changes, update or delete the recipe in the same patch.

## Start here

- [`fresh-agent-navigation.md`](fresh-agent-navigation.md) — localize a task with
  `.agent` without loading the repository into context.
- [`headless-room-verification.md`](headless-room-verification.md) — prove a
  gameplay/world change through the real headless composition.
- [`ldtk-authoring.md`](ldtk-authoring.md) — safe world edits and tool-assisted
  spatial authoring.

## Content authoring

- [`adding-a-character.md`](adding-a-character.md)
- [`dialogue-authoring.md`](dialogue-authoring.md)
- [`extending-brains-and-action-sets.md`](extending-brains-and-action-sets.md)
- [`add-showcase-room.md`](add-showcase-room.md)
- [`goblin-encounter.md`](goblin-encounter.md)
- [`generated-music-workflow.md`](generated-music-workflow.md)

## Platform and diagnostics

- [`android-build.md`](android-build.md)
- [`web-build.md`](web-build.md)
- [`web-audio-manual-test.md`](web-audio-manual-test.md)
- [`profiling.md`](profiling.md)

## Recipe quality rule

Before following an old exact path, localize the current owner:

```bash
python scripts/agent_query.py "<task words>"
python scripts/agent_query.py tests "<expected behavior>"
```

Use `./run_tests.sh` as the test front door. Use source/CLI `--help` as the
command authority when prose and code disagree.
