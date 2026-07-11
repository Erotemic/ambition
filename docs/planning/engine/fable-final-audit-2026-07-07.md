# Fable final audit — historical pointer

The 2026-07-07 audit was a useful planning checkpoint, but it is not a current
status authority. Its detailed contents are preserved at
[`docs/archive/reviews/planning-history-2026-07-11/fable-final-audit-2026-07-07.md`](../../archive/reviews/planning-history-2026-07-11/fable-final-audit-2026-07-07.md).

Use the live planning front end instead:

- [`../status.md`](../status.md) for HEAD state;
- [`../tracks.md`](../tracks.md) for current work;
- [`../roadmap.md`](../roadmap.md) for phases and decisions.

The architecture-boundary checks cited by the historical audit no longer live in
`game/ambition_app/tests/architecture_boundaries.rs`. Their current owner is the
`tests/ambition_workspace_policy` package and its migration matrix.
