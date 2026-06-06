# 0001 — docs inventory baseline

Added generated ECS inventory baselines under `docs/generated/` and updated the
plugin-refactor planning notes with how to use those outputs.

Why this matters:

- Gives reviewers a concrete before-state for component/resource/system counts.
- Records where generated inventory artifacts live.
- Adds a stale-doc index so architecture work can avoid trusting outdated docs.

Main files:

- `docs/generated/ambition_ecs_inventory.baseline.{json,md}`
- `docs/generated/ambition_ecs_inventory.with_tests.baseline.{json,md}`
- `docs/planning/plugin_refactor/13_inventory_notes.md`
- `docs/planning/plugin_refactor/stale_docs_index.md`
