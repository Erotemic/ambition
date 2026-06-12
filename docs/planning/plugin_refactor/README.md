# Plugin refactor notes

This folder is now mostly historical execution record for the 2026 platformer-runtime / pluginization push.

## Current authority

Start here instead of loading the whole folder:

- `../../current/state.md` — current crate graph and ownership model.
- `../../current/next.md` — active next moves.
- `22_monolith_breaker_survey.md` — current remaining-bulk survey for `ambition_sandbox`.
- `../../systems/architecture.md` and `../../architecture/architecture-boundaries.md` — durable boundary rules.
- `../../adr/0019-pluginized-platformer-runtime.md` — accepted decision that set the direction.

## What landed

The staged refactor produced the current layer stack:

```text
foundations: ambition_engine_core, ambition_platformer_runtime,
             ambition_portal, ambition_time, ambition_input,
             ambition_menu, ambition_audio, ambition_sfx[_bank],
             ambition_asset_manager
machinery:   ambition_sandbox
content:     ambition_content
app:         ambition_app
```

The durable lessons are:

- grow same-crate seams before extracting crates;
- prefer self-owning plugins to app-level hand wiring;
- measure outward dependencies before promising a module extraction;
- keep content names above reusable machinery;
- use architecture-boundary tests to ratchet dependency direction.

## Historical files

Older numbered files are retained only for provenance and commit archaeology. They should not be used as live task queues unless their header says so. In particular, the detailed action plans for Stages 14, 17, 19, 20, and 21 have landed or been superseded by the current crate graph.

Use this folder sparingly. If a rule is still current, promote it into `docs/current/`, `docs/systems/`, `docs/concepts/`, or an ADR.


## Active files in this folder

- `22_monolith_breaker_survey.md` — current survey of remaining monolith pressure.
- `runtime_extraction_backlog.md` — current extraction backlog.
- `stale_docs_index.md` — current doc cleanup index when maintained.

The numbered files `01_` through `21_` are historical execution records. Do not treat them as current instructions unless an active file explicitly points back to one for context.
