# Plugin refactor action plan — compact historical summary

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


**Status:** superseded by the landed Stage 20 crate graph and the remaining-work survey in `22_monolith_breaker_survey.md`.

This file used to contain the full 1,000-line execution checklist for the original plugin-refactor run. The useful outcome is now small enough to keep as a summary.

## Durable rules that survived

- Move one architectural seam per branch.
- Prefer architecture-revealing compile failures over long compatibility layers.
- Keep compatibility re-exports only when they are the intended public API.
- Add guardrails before or during the refactor that needs them.
- Every stage should end with focused validation commands and a note about what became easier to express.
- When a stage reveals unexpected coupling, record the coupling instead of hiding it behind a bridge.

## What landed

The plan eventually produced the current four-layer graph:

```text
foundations -> ambition_sandbox machinery -> ambition_content -> ambition_app
```

Important extracted or promoted pieces include:

- `ambition_engine_core`
- `ambition_platformer_runtime`
- `ambition_portal`
- `ambition_time`
- `ambition_input`
- `ambition_menu`
- `ambition_audio`
- `ambition_sfx` / `ambition_sfx_bank`
- `ambition_asset_manager`
- `ambition_content`

## Current follow-up

Use `22_monolith_breaker_survey.md` for remaining `ambition_sandbox` breakup work. Use `../../architecture/architecture-boundaries.md` for the guardrails that replaced the old per-stage checklist.
