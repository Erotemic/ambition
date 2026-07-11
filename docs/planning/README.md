# `docs/planning` — current direction and current work

This directory is the authoritative planning surface for HEAD and forward work.
It is deliberately not a changelog. Detailed execution reports and superseded
plans live under [`docs/archive/`](../archive/).

## Read in this order

1. [`vision.md`](vision.md) — the product and executor rules.
2. [`decision-principles.md`](decision-principles.md) — how to make autonomous choices.
3. [`status.md`](status.md) — the canonical HEAD summary and evidence links.
4. [`tracks.md`](tracks.md) — the current queue only.
5. [`roadmap.md`](roadmap.md) — phases, binding decisions, and open calls.
6. Only the engine, demo, or game plan touched by the task.

## Directory map

- `engine/` — normative architecture and active engine plans.
- `demos/` — acceptance-game plans.
- `game/` — Ambition-the-game plans.
- `status.md` — current state; do not duplicate it elsewhere.
- `tracks.md` — current executable queue; do not append a historical diary.
- `roadmap.md` — phase map and decision register.

Historical reviews and execution ledgers are not deleted; they are archived in
`docs/archive/reviews/`. They are evidence about how a decision was reached, not
proof that HEAD still satisfies it.

## Evidence discipline

A completion claim must be backed by at least one of:

- an executable test or policy that would fail if the claim regressed;
- a source constant/type whose value directly establishes the claim;
- a mechanically recomputed inventory;
- an acceptance checklist with every item demonstrated at HEAD.

A previous agent's completion report is not evidence by itself. Terms such as
"landed," "complete," "one authority," "none remain," and exact repository-wide
counts require independent corroboration.

For machine-maintained facts, use the `planning-evidence` comments in
[`status.md`](status.md). `scripts/check_agent_kb.py` compares those markers with
HEAD and fails when they drift.

## Living-plan rules

1. **Same-commit correction.** Code that changes status updates `status.md`, the
   owning plan, and `tracks.md` in the same commit.
2. **One status authority.** High-level current state appears only in
   `status.md`. Other plans link to it instead of restating it.
3. **Prune completed detail.** Keep the durable design and a compact evidence
   note. Move session narratives, timing tables, and superseded sketches to the
   archive.
4. **No stale warning banners.** Correct present-tense text or archive it.
5. **No completion by vocabulary alone.** Adding a type does not prove runtime
   semantics, persistence, cleanup, or consumer convergence.
6. **Green guardrails require poison evidence.** A new lint or invariant must be
   shown to fail on the intended injected defect before it is used as completion
   evidence.
7. **State measurement units.** A line count says whether it is physical lines,
   production lines, or test-inclusive lines. Avoid duplicating brittle counts
   when the number does not guide a decision.

## Planning size policy

Live plans should be concise enough to audit. The documentation checker enforces
limits for the front-end files and the largest execution-oriented plans. When a
file reaches its limit, archive history rather than compressing more status into
one paragraph.

## Binding spine

North star: *every upgrade a theorem, every boss a failed objective function,
every biome a mathematical world model.* Engine-first: the game is the first
content crate. The oracle: *could another platformer be built by adding a content
crate without editing core?* Elegance is the objective function; behavior is not
sacred pre-release; verify against the real headless simulation; feel ships
BLIND; delete rather than bridge.
