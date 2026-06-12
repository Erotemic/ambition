# Stage 20 attack plan — compact historical summary

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


**Status:** executed 2026-06-10. The bisection landed and the repo now has the four-layer crate graph described in `../../current/state.md`.

## Landed result

```text
foundations -> ambition_sandbox machinery -> ambition_content -> ambition_app
```

Key outcomes:

- `ambition_content` became the home for named game content.
- `ambition_app` became the composition layer and owns binaries/full-stack tests.
- `ambition_sandbox` became machinery rather than the playable shell.
- Additional foundation crates split reusable runtime/audio/menu/portal/time/input/assets/SFX concerns out of the monolith.
- Boundary tests became the ratchet for dependency direction.

## Durable lessons

- Content-free is not the same as extractable; measure outward dependencies.
- Move modules up to `ambition_app` only when their library consumers are gone or can use a small vocabulary slice.
- Extract modules down to reusable crates only after named-content and upward-machinery dependencies are inverted.
- Preserve replay/trace checks around behavior-sensitive moves.

## Current follow-up

Use `22_monolith_breaker_survey.md` for remaining sandbox-bulk candidates. This file is historical record, not a live action queue.
