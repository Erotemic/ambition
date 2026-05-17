# Agent guide for Ambition

This file is a short operating guide. It routes agents to the right memory layer; it is not the knowledge base.

## Cold start

For non-trivial work, read in this order:

1. `README.md`
2. `AGENTS.md`
3. `dev/README.md`
4. `docs/README.md`
5. `docs/current/state.md`
6. One focused concept/system/recipe doc for the task

Do not read all of `docs/` or `dev/` by default.

## Source-of-truth order

1. Fresh user instructions.
2. ADRs under `docs/adr/`.
3. Current state under `docs/current/`.
4. Concept pages under `docs/concepts/`.
5. Focused system/recipe docs under `docs/`.
6. Engineering memory under `dev/`.

Historical notes are useful evidence, but they do not override current docs or ADRs.

## Engineering memory check

`dev/` is active long-running engineering memory, not trash and not an archive.

Before a non-trivial patch, search prior mistakes:

```bash
rg -n "<subsystem>|<symptom>|<failure class>" dev/journals dev/benchmark-candidates
```

Use:

- `dev/journals/` for symptom-driven postmortems.
- `dev/benchmark-candidates/` for invariant traps before refactors.
- `dev/SEARCH.md` for suggested searches.

Benchmark questions are distilled from real Ambition mistakes. Treat them as pre-flight checks.

## Brainstorms are alive

`docs/brainstorms/` is active design incubation. Do not archive, delete, or treat it as stale just because it is exploratory. Current implementation docs and ADRs still govern code changes, but brainstorms preserve the ideas that give the project direction.

## Patch discipline

- Prefer small, reviewable changes with targeted validation.
- When producing overlay packages, include complete replacement files and do not remove the user's working tree.
- Do not hand-edit `sandbox.ldtk`; use Ambition LDtk tooling.
- Preserve Android/web/platform entrypoints when replacing shared files.
- Update docs/concepts, recipes, ADRs, or dev memory when a durable invariant changes.

## Common validation commands

```bash
cargo fmt --check
cargo test -p ambition_engine
cargo test -p ambition_sandbox --lib
cargo run -p ambition_sandbox --bin headless
```

Use narrower tests when a focused test already covers the touched concept.
