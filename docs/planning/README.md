# `docs/planning` — current direction and current work

This directory is the authoritative planning surface for HEAD and forward work.
It is deliberately not a changelog. Completed execution ledgers, superseded
plans, and review evidence belong under [`docs/archive/`](../archive/).

## Read in this order

1. [`vision.md`](vision.md) — product and engine north star.
2. [`maintainer-decisions.md`](maintainer-decisions.md) — decisions Jon made explicitly, with confidence.
3. [`decision-principles.md`](decision-principles.md) — how to choose when Jon has not decided.
4. [`status.md`](status.md) — current source-backed state.
5. [`tracks.md`](tracks.md) — executable queue only.
6. [`roadmap.md`](roadmap.md) — phases and durable architecture decisions.
7. Only the focused engine, demo, or game plan needed for the task.

## What belongs here

- `engine/` — normative architecture and active engine designs.
  The longer-term character-authoring direction is
  [`engine/svg-component-character-migration.md`](engine/svg-component-character-migration.md):
  editable SVG component scenes, freeform Python animation, and a gradual
  legacy-to-shadow-to-SVG migration with raster-equivalence checks.
  The next major architecture campaign is
  [`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md).
  Its immediate room-lifecycle customer is
  [`engine/room-transition-loading.md`](engine/room-transition-loading.md), which
  completes adaptive readiness-gated room transitions without flashing loading
  UI for fast loads or exposing partial rooms for slow loads. The external-game
  input endpoint and its participant/action/context migration live in
  [`engine/participant-action-system.md`](engine/participant-action-system.md);
  [`engine/participant-input.md`](engine/participant-input.md) records the landed
  startup/launcher slice. Cross-cutting ownership, shipping/bootstrap, measured
  scale, and deferred provider-boundary follow-ups from the July 19–20 review live
  in [`engine/closeout-review-followups-2026-07-20.md`](engine/closeout-review-followups-2026-07-20.md).
- `demos/` — acceptance-game specifications.
- `game/` — Ambition-the-game direction.
- `maintainer-decisions.md` — direct maintainer rulings; agent consensus is not a substitute.
- `status.md` — current state; do not duplicate it elsewhere.
- `tracks.md` — current execution order; do not append a historical diary.
- `roadmap.md` — phase map and durable decision register.

A dated audit or implementation plan stays live only while it directs current
work. Archive recent material when its evidence is still useful; otherwise rely
on git history rather than leaving a retrieval trap at the top level.

## Evidence discipline

A completion claim needs at least one of:

- an executable behavioral or architectural test;
- a source owner whose type or constant directly establishes the fact;
- a mechanically recomputed inventory;
- an acceptance checklist demonstrated against HEAD.

A prior agent report is evidence about what was attempted, not proof that HEAD
still has the claimed property. Avoid fragile exact counts unless the count is
decision-relevant.

## Living-plan rules

1. Keep only material current status and next work.
2. Remove completed task-card narration; durable design remains, execution history does not.
3. Archive a recent superseded document only when it remains useful context for present work.
4. Do not preserve a stale document merely because it took effort to write.
5. Do not create scanners, poison fixtures, or policy ceremony for a rule better expressed by architecture or behavior.
6. A migration-only checklist or matrix is deleted when the migration closes.
7. Use **DONE**, **OPEN**, and **BLOCKED** for executable slices. A parent may be **PARTIAL** only while a named child remains OPEN or BLOCKED.

## Binding spine

North star: *every upgrade a theorem, every boss a failed objective function,
every biome a mathematical world model.* The game is the first engine customer.
The oracle is: *could another platformer be built by adding a provider/content
crate without editing core?* Elegance is the objective function; behavior is not
sacred pre-release; verify against the real headless simulation; delete duplicate
paths rather than bridging them.
