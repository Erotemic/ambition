# Ambition documentation map

Use this file to choose what to read. Do not load the entire documentation tree by default.

## Start here

- [`../README.md`](../README.md) — stable project overview.
- [`../AGENTS.md`](../AGENTS.md) — short agent operating instructions.
- [`current/state.md`](current/state.md) — active implementation state.
- [`current/risks.md`](current/risks.md) — high-risk areas and review rules.
- [`current/next.md`](current/next.md) — current next good moves.
- [`adr/README.md`](adr/README.md) — durable architectural decisions.
- [`../dev/README.md`](../dev/README.md) — engineering memory purpose.
- [`../dev/SEARCH.md`](../dev/SEARCH.md) — how to search prior lessons and benchmark traps.

## Durable memory

- [`concepts/index.md`](concepts/index.md) — first-class concepts, aliases, invariants, edit protocols, and validation links.
- [`systems/index.md`](systems/index.md) — focused subsystem docs.
- [`recipes/index.md`](recipes/index.md) — procedural docs for builds, tests, refactors, content authoring, profiling, and packaging.
- [`adr/`](adr/) — decisions that supersede older plans.

## Generated agent indexes

- [`../.agent/manifest.yaml`](../.agent/manifest.yaml) — generated-index manifest and provenance.
- [`../.agent/retrieval_evals.yaml`](../.agent/retrieval_evals.yaml) — repository-specific retrieval/localization evals.
- [`../.agent/index/`](../.agent/index/) — generated file, symbol, and test maps.

Generated indexes are navigation aids. They do not override code, ADRs, current docs, or concept pages.

## Active design incubation

- [`brainstorms/`](brainstorms/) is alive. It is where project ideas are worked out before they become current implementation docs, ADRs, features, or TODOs.
- Do not move brainstorms to an archive just because they are exploratory.
- When implementing code, prefer current docs and ADRs for present behavior; use brainstorms for intent, direction, and possibility space.

## Engineering memory lives outside docs

`dev/` intentionally stays outside `docs/`:

- `dev/journals/` records hard-won debugging postmortems by symptom.
- `dev/benchmark-candidates/` records invariant traps and benchmark questions from real mistakes.

When a lesson becomes a durable rule for the codebase, promote the rule into a concept page, recipe, or ADR and link back to the dev-memory evidence.

## Archive

- [`archive/README.md`](archive/README.md) preserves retired notes, stale handoffs, old agent prompts, port notes, and overlay records.
- Archive files are useful context, but they are not current authority.
- `docs/brainstorms/` is explicitly excluded from archive status.

## Reading rule

Read the smallest packet that answers the task:

1. current state/risks/next,
2. one concept page,
3. one focused system doc or recipe,
4. relevant dev-memory search results,
5. generated indexes when locating files, symbols, or tests.

Avoid broad context dumps. They make agents slower and less precise.
