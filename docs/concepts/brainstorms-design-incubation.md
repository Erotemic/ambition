---
id: brainstorms-design-incubation
aliases:
  - brainstorms
  - storylines
  - design incubation
  - idea backbone
implemented_by:
  - docs/brainstorms/
related_docs:
  - docs/brainstorms/README.md
  - docs/vision/goal-state.md
last_verified: 2026-05-17
---

# Brainstorms and design incubation

## Definition

`docs/brainstorms/` is active design incubation. It preserves the ideas that give Ambition life before they harden into current implementation docs, ADRs, features, or TODOs.

## Core invariants

- Brainstorms are alive, not archive material.
- Brainstorms can be provisional without being stale.
- Current implementation docs and ADRs govern code behavior; brainstorms supply direction, concepts, themes, and possibility space.
- Story and world ideas should not force bespoke engine behavior until promoted into reusable primitives or explicit implementation docs.

## Edit protocol

1. Keep speculative language when an idea is not implemented.
2. When an idea becomes current behavior, promote it into `docs/current/`, `docs/concepts/`, a focused system doc, or an ADR.
3. Link back to the brainstorm when preserving design lineage matters.
4. Do not bulk-move brainstorms into `archive/` as part of documentation cleanup.

## Validation

Documentation-only. Check links and make sure current-behavior claims live outside brainstorms when they become authoritative.
