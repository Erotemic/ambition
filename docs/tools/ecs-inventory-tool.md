---
status: current
last_verified: 2026-07-18
---

# ECS inventory tool

`scripts/ecs_inventory.py` uses tree-sitter to generate static, reviewable Bevy
ownership evidence: ECS items, plugins, registrations, systems, resource/message
access, and spawn sites. The generated `.agent/ecs_inventory/` packets are a
localization aid, not runtime truth or a replacement for Rust compilation.

The script declares its Python dependencies through inline script metadata. Run
it with an inline-metadata-aware launcher:

```bash
uv run --script scripts/ecs_inventory.py --help
```

Direct `python scripts/ecs_inventory.py` is not the supported clean-environment
invocation unless its tree-sitter dependencies are already installed.

## Workspace index

```bash
uv run --script scripts/ecs_inventory.py \
  --workspace \
  --out-dir .agent/ecs_inventory
```

This scans workspace members under `crates/`, `game/`, and `tests/`, writing a
compact project summary plus one Markdown/JSON shard per crate.

Use the normal query interface rather than loading those shards wholesale:

```bash
python scripts/agent_query.py ecs "ControlPrompt"
python scripts/agent_query.py ecs "RoomScope"
python scripts/agent_query.py crate ambition_input
```

## Focused crate inventory

```bash
uv run --script scripts/ecs_inventory.py \
  --crate crates/ambition_actors \
  --json target/ambition_actors_ecs.json \
  --markdown target/ambition_actors_ecs.md
```

Add `--include-tests` only when test-only registrations/spawns are relevant.
`--check-json <path>` compares the generated inventory with an existing JSON
artifact and exits nonzero on a difference.

## Interpretation

Static inventory is evidence, not proof:

- A registration identifier may be a system, set, run condition, plugin, or
  helper expression.
- Spawn evidence describes source sites, not the complete runtime entity set.
- Resource/query access cannot prove schedule order or semantic authority.
- Re-exports and compatibility adapters may look like owners.

Confirm a hit in source, module docs, plugin composition, and tests before
editing. Use active planning/ADRs for intended direction.

## Regeneration contract

Agent archives should package an index generated from the same source commit.
Regenerate after structural/documentation changes:

```bash
python scripts/generate_agent_index.py
python scripts/agent_query.py overview
```

The generator invokes the appropriate inventory path as part of building the
commit-matched `.agent` knowledge base.
