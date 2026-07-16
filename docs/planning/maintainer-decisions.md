# Maintainer decisions

This file records decisions Jon made explicitly. It exists to distinguish
maintainer intent from agent analysis, consensus drafts, and inferred design.
Agent-written records may explain a decision, but they do not become Jon's
decisions unless they are represented here or Jon says so directly.

Confidence is Jon's current confidence in the decision, not a permanence promise:

- **High** — proceed on this basis; do not reopen without new concrete evidence.
- **Medium** — current direction, but implementation may reveal a better shape.
- **Low** — tentative preference or deliberately deferred naming/design choice.

Do not backfill confidence for older decisions by guessing. Add or revise a row
when Jon states a decision or changes his confidence.

| Date | Decision | Confidence | Notes |
|---|---|---:|---|
| 2026-07-16 | Perform the identified content evictions. | High | Each eviction must end in an open provider-owned catalog, registration, or presentation seam rather than moving a closed engine-owned table. |
| 2026-07-16 | Extract the reusable programmatic simulation surface as `ambition_sim_harness`. | High | Reset/step, typed actions and observations, headless testing, RL, replay, and fuzzing belong below `ambition_app`. |
| 2026-07-16 | Extract the platformer-provider lifecycle from the `ambition` facade and consolidate the repeated provider protocol. | High | Exact crate name remains open; `ambition_platformer_provider` is the working name. |
| 2026-07-16 | Keep cutscenes and encounters as separate domain systems. | High | Cutscenes are scripted with limited interaction; encounters are interactive with limited scripting. Shared micro-primitives are allowed only when naturally demonstrated. |
| 2026-07-16 | Keep provider registration explicit in the host composition root. | High | The explicit dependency plus plugin registration is intentional; do not add opaque plugin discovery. |
| 2026-07-16 | Defer any boss crate carve until boss behavior converges onto the canonical moveset/action path. | High | Reassess afterward whether a separate boss crate still exists as a coherent subsystem. |
| 2026-07-16 | Reject the proposed named-content scanner and stop adding poison-test ceremony by default. | High | Prefer Rust types, APIs, crate boundaries, visibility, and behavioral tests. A new policy test must justify why those cannot enforce the invariant. |
| 2026-07-16 | Keep the compiler term **lowering** for authored world IR becoming live ECS state. | High | Deserialization/import produces the IR; lowering materializes its canonical runtime representation. |
| 2026-07-16 | Repository-wide knowledge-base hygiene checks are CI/maintainer tools, not routine local validation. | High | Agent-facing docs should not attach them to ordinary code changes. |
| 2026-07-16 | Preserve historical journals as historical records during documentation cleanup. | High | Do not rewrite old journals merely to modernize present-day guidance. |
| 2026-07-16 | A full rename of `ambition_actors/src/features/` may be worthwhile, but the name `sim` is not settled and the work is low priority. | Low | Do not perform a partial rename or let naming block architectural work. |

The fuller multi-agent recon consensus, including accepted campaigns and explicit
non-goals, is in
[`engine/decisions-2026-07-16.md`](engine/decisions-2026-07-16.md).
