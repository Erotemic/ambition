# Ambition documentation map

Use this file to choose what to read. Do not load the whole documentation tree by default.

## Start here

- [`../README.md`](../README.md) — stable project overview.
- [`../AGENTS.md`](../AGENTS.md) — short agent operating instructions.
- [`current/state.md`](current/state.md) — current architecture and implementation state.
- [`current/risks.md`](current/risks.md) — high-risk areas and review rules.
- [`current/next.md`](current/next.md) — current next moves.
- [`adr/README.md`](adr/README.md) — durable architectural decisions.
- [`../dev/README.md`](../dev/README.md) and [`../dev/SEARCH.md`](../dev/SEARCH.md) — engineering memory and lookup protocol.

## Durable memory

- [`concepts/index.md`](concepts/index.md) — concepts, aliases, invariants, edit protocols, validation anchors.
- [`systems/index.md`](systems/index.md) — current subsystem docs.
- [`recipes/index.md`](recipes/index.md) — current procedures.
- [`tools/index.md`](tools/index.md) — author-time tools.
- [`mechanics/index.md`](mechanics/index.md) — mechanics expressibility and current gameplay primitives.
- [`adr/`](adr/) — decisions that supersede older plans.

## Direction, planning, history

- [`brainstorms/`](brainstorms/) — active design incubation. This is alive, not archive.
- [`vision/index.md`](vision/index.md) — distilled product direction.
- [`planning/index.md`](planning/index.md) — active sequencing and debt management.
- [`history/index.md`](history/index.md) — compact chronology.
- [`archive/README.md`](archive/README.md) — superseded notes, old handoffs, retired systems, and historical overlays.

## Generated agent indexes

After moving docs, tests, or code symbols, regenerate and validate:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Generated indexes are navigation aids. They do not override source files, ADRs, current docs, or concept pages.

- [`../.agent/manifest.yaml`](../.agent/manifest.yaml)
- [`../.agent/retrieval_evals.yaml`](../.agent/retrieval_evals.yaml)
- [`../.agent/index/`](../.agent/index/)

Generated indexes are navigation aids. They do not override code, ADRs, current docs, or concept pages.

## Reading rule

Read the smallest packet that answers the task:

1. current state/risks/next,
2. one concept page,
3. one focused system/tool doc or recipe,
4. relevant dev-memory search results,
5. generated indexes when locating files, symbols, or tests.

Avoid broad context dumps. They make agents slower and less precise.
