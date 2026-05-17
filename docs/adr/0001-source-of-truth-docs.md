# ADR 0001: Use a layered repository knowledge base

## Status

Accepted and updated 2026-05-17.

## Context

Ambition has code, ADRs, concept docs, recipes, active brainstorms, historical notes, dev journals, benchmark questions, and generated indexes. Older migration notes were competing with current guidance, and redirect stubs made `docs/` noisy for agents.

## Decision

Use this hierarchy:

1. `README.md` and `AGENTS.md` route the reader.
2. ADRs record durable architectural decisions and must stay modern.
3. `docs/current/` records active state, risks, and next moves.
4. `docs/concepts/` records durable terms, invariants, edit protocols, and validation anchors.
5. `docs/systems/`, `docs/recipes/`, `docs/tools/`, and `docs/mechanics/` contain current focused docs.
6. `docs/brainstorms/` stays active as idea incubation.
7. `dev/` stores engineering memory: postmortems and benchmark traps.
8. `.agent/` stores generated indexes.
9. `docs/archive/` stores historical evidence only.

Delete redirect-only stubs instead of keeping compatibility clutter. If an old note still matters, either rewrite it as a current doc or archive it with an explicit supersession.

## Consequences

Agents should read less by default and trust current docs more. Maintainers should actively prune stale docs instead of preserving every old plan at a live path.
