# Overnight reusable extraction run — compact historical summary

**Status:** completed and superseded by later Stage 20 bisection work.

This file used to contain a long task table for extracting reusable pieces from the sandbox. The durable outcome is the pattern, not the task list.

## Durable pattern

For each candidate seam:

1. identify whether coupling is essential or incidental;
2. invert incidental upward dependencies in place;
3. add a same-crate facade when call-site churn would hide behavior changes;
4. gate with focused tests and replay/trace checks;
5. extract only after the vocabulary is stable.

## Landed examples

- `ambition_time` owns the shared time-domain vocabulary.
- `ambition_platformer_runtime` owns body, gravity, orientation, world-query, projectile, transit, and math primitives.
- Portal input/body/item leaks were inverted through intent and transitable adapters before extraction.

## Current authority

Use `../../current/state.md` and `22_monolith_breaker_survey.md` for live follow-up work.
