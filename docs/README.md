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
- [`adr/`](adr/) — decisions that supersede older plans.
- Focused system docs such as `ldtk_world_composition.md`, `movement`/collision docs, audio docs, input docs, and testing docs remain useful when the task matches them.

## Active design incubation

- [`brainstorms/`](brainstorms/) is alive. It is where project ideas are worked out before they become current implementation docs, ADRs, features, or TODOs.
- Do not move brainstorms to an archive just because they are exploratory.
- When implementing code, prefer current docs and ADRs for present behavior; use brainstorms for intent, direction, and possibility space.

## Engineering memory lives outside docs

`dev/` intentionally stays outside `docs/`:

- `dev/journals/` records hard-won debugging postmortems by symptom.
- `dev/benchmark-candidates/` records invariant traps and benchmark questions from real mistakes.

When a lesson becomes a durable rule for the codebase, promote the rule into a concept page, recipe, or ADR and link back to the dev-memory evidence.

## Reading rule

Read the smallest packet that answers the task:

1. current state/risks/next,
2. one concept page,
3. one focused implementation doc or recipe,
4. relevant dev-memory search results.

Avoid broad context dumps. They make agents slower and less precise.
