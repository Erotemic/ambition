# Stage 17 content / ability boundary run — compact historical summary

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


**Status:** done. This was the first content-boundary cleanup inside `ambition_sandbox` before the later `ambition_content` crate promotion.

## What landed

- Loose player ability / weapon mechanics were grouped under a clearer ability home.
- Named intro/story content moved toward the content nucleus.
- Guardrails were tightened so the sandbox root could not silently regrow loose content modules.

## Durable lesson

Not every content-looking module is movable as-is. Bosses, enemies, and feature ECS code often mix named roster data with reusable mechanics. Split the named nouns from the generic verbs before moving crates.

## Current authority

Use `../../current/state.md` for the current crate graph and `22_monolith_breaker_survey.md` for remaining sandbox breakup candidates.
