# Crate split plan — compact archive

**Status:** superseded by the landed Stage 20 crate graph.

Current crate layering is documented in `docs/current/state.md`. The durable lesson from this older plan is still valid: split only around real ownership boundaries, then enforce dependency direction with tests.

Use `docs/planning/plugin_refactor/22_monolith_breaker_survey.md` for remaining breakup candidates.
