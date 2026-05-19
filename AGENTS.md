# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Cold start

For non-trivial work, read in this order:

1. `README.md`
2. `AGENTS.md`
3. `dev/README.md`
4. `dev/SEARCH.md`
5. `docs/README.md`
6. `docs/current/state.md`
7. One focused concept, system doc, recipe, tool doc, planning doc, or vision doc for the task

Do not read all of `docs/` or `dev/` by default.

## Source-of-truth order

1. Fresh user instructions.
2. ADRs under `docs/adr/`.
3. Current state under `docs/current/`.
4. Concept pages under `docs/concepts/`.
5. Focused system/tool docs and recipes under `docs/systems/`, `docs/tools/`, and `docs/recipes/`.
6. Planning, vision, history, and brainstorms under `docs/planning/`, `docs/vision/`, `docs/history/`, and `docs/brainstorms/`.
7. Engineering memory under `dev/`.
8. Generated navigation indexes under `.agent/`.

Historical notes under `docs/archive/` are evidence, not current authority. Generated indexes aid localization but do not override source files.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.

## Engineering memory and benchmark candidates

Before a non-trivial patch, search prior mistakes:

```bash
rg -n "<subsystem>|<symptom>|<failure class>" dev/journals dev/benchmark-candidates
```

Use `dev/journals/` for symptom postmortems and `dev/benchmark-candidates/` for invariant traps before refactors.

If you notice a reusable failure mode, invariant trap, or repo-specific question that would catch a future agent mistake, opportunistically add or update a benchmark candidate under `dev/benchmark-candidates/` and link it from `dev/benchmark-candidates/index.md`. Do this only for durable lessons, not transient task state.

## Generated indexes

Regenerate indexes after moving docs, tests, or code symbols:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

## Commit messages

- Make detailed commit messages as you might normally do it, but also include a
  summary of the prompt that inspired them. I.e. why the change is being made.

## Patch discipline

- Prefer reviewable changes with targeted validation.
- Do not hand-edit `sandbox.ldtk`; use Ambition LDtk tooling.
- Update concepts, recipes, ADRs, or dev memory when a durable invariant changes.

## Common validation commands

```bash
cargo fmt --check
cargo test -p ambition_engine
cargo test -p ambition_sandbox --lib
cargo run -p ambition_sandbox --bin headless
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Use narrower tests when a focused test already covers the touched concept.
