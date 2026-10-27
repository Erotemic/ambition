# ADR 0001: Use a layered repository knowledge base

## Status

Accepted and updated 2026-10-26; decision records are revisitable.

## Context

Ambition has code, ADRs, concept docs, recipes, active brainstorms, historical notes, dev journals, benchmark questions, and generated indexes. Older migration notes were competing with current guidance, and redirect stubs made `docs/` noisy for agents.

## Decision

Use this hierarchy:

1. `README.md` and `AGENTS.md` route the reader.
2. ADRs record durable architectural decisions and must stay modern.
3. `docs/current/` records active state and risks; `docs/planning/` records proposals, experiments, execution selection, and open questions.
4. `docs/concepts/` records durable terms, invariants, edit protocols, and validation anchors.
5. `docs/systems/`, `docs/recipes/`, `docs/tools/`, and `docs/mechanics/` contain current focused docs.
6. `docs/brainstorms/` stays active as idea incubation.
7. `dev/` stores engineering memory: postmortems and benchmark traps.
8. `.agent/` stores generated indexes.
9. `docs/archive/` stores historical evidence only.

Delete redirect-only stubs instead of keeping compatibility clutter. If an old note still matters, either rewrite it as a current doc or archive it with an explicit supersession.

## 2026-10-26 amendment: evidence and decision precedence

An ADR records the rationale and status of a decision; an **accepted ADR is not irrevocable**. A maintainer may reopen even a previously explicit ruling. Treat an agent-authored plan, roadmap priority, or ADR assertion as a hypothesis until its claimed authority and source evidence are established. A previously accepted design is evidence of why an implementation exists, not an independent user constraint.

Separate in every architectural review:

- **Explicit maintainer requirement:** a statement the maintainer actually made; record its scope and distinguish firm constraints from tentative preferences.
- **Observed implementation:** a claim tied to current source and tests, with a date or revision; it may become stale.
- **Selected decision:** an accepted design, including alternatives and the conditions for revisiting it.
- **Proposal or experiment:** an open technical choice; documentation does not authorize code changes or imply feasibility.

When new evidence or maintainer direction contradicts an old ADR, amend or supersede it *in place* and update linked planning rather than layering an unacknowledged contradiction. Correct implementation-status claims before using them as a planning premise. `docs/planning/queue.md` remains the execution selector; a proposed ADR or research plan does not silently enqueue a refactor.

New proposals [ADR 0035](0035-distributed-multiplayer-without-required-services.md) through [ADR 0039](0039-evidence-based-crate-boundaries.md) record the 2026-10-26 architectural reassessment. Their proposed mechanisms do not change the shipped system by virtue of being written.

## Consequences

Agents should read less by default and trust current docs more. Maintainers should actively prune stale docs instead of preserving every old plan at a live path.

## Current implications for agents

- Keep live docs routed and token-efficient.
- Delete redirect-only stubs instead of preserving flat-doc clutter.
- Move stale plans to `docs/archive/` or delete them when git history is enough.
