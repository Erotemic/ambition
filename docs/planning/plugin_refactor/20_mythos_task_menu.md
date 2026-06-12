# Stage 20 task menu — compact historical summary

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


**Status:** executed. The detailed menu was consumed by `21_stage20_attack_plan.md` and later superseded by the current bisection state.

## What mattered

The useful ordering was:

1. make machinery content-free enough to promote `ambition_content`;
2. separate generic combat-kit pieces from named encounters;
3. promote the app shell and content layer instead of leaving everything in `ambition_sandbox`;
4. extract low-risk foundation crates such as audio/time/runtime support;
5. measure compile-time and dependency impact before promising deeper splits.

## Current authority

Use `../../current/state.md` for the landed crate graph and `22_monolith_breaker_survey.md` for the remaining measured candidates.
