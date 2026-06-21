# Agent guide for Ambition

This is the repository operating guide for coding agents. Keep it short, session-agnostic, and focused on routing. Put durable project knowledge in `docs/`, engineering memory in `dev/`, and generated navigation aids in `.agent/`.

## Core Values

* Avoid player-centrism. Value the principle of relativity.
* Find the elegant solution. Jon will push back on hacks.
* Correctness is emergent from elegance.

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
6. Planning, vision, and brainstorms under `docs/planning/`, `docs/vision/`, and `docs/brainstorms/`.
7. Engineering memory under `dev/`.
8. Generated navigation indexes under `.agent/`.

Historical notes under `docs/archive/` are evidence, not current authority. Generated indexes aid localization but do not override source files.

## Current architectural stance

- Ambition is Bevy-native. Do not resurrect backend-neutral constraints unless a new ADR says so.
- Prefer data-driven ECS flow: authored/generated data -> Bevy components/entities -> systems -> messages/effects.
- LDtk owns world/level authoring. RON room manifests are historical; RON may still be used for tuning, save/settings, and other data where appropriate.
- Preserve desktop, web, Android/mobile/touch, controller, and Steam Deck paths. iOS is deferred for hardware, not excluded.
- **Layered crate split (Stage 20, 2026-06-10):** `ambition_gameplay_core` is the
  gameplay core library: content-free simulation systems, runtime state, world/LDtk
  integration, player/session systems, combat/items/encounter machinery, persistence,
  schedules, and historical facade re-exports. `ambition_render` is the Bevy
  presentation layer (sprites, camera, parallax, HUD, dialog/cutscene UI, fonts,
  and render-only visual systems). `ambition_content` is the named game content
  (quests, bosses, rosters, dialogue, intro, banter, portal adapters) and depends
  on the machinery. `ambition_app` is the assembly + every binary
  (`ambition_game_bin`, `headless`, `trace_replay`, `rl_*`) + the full-stack
  integration tests, and is the only crate allowed to name both machinery and
  content. Machinery must not import content — `architecture_boundaries` enforces
  it. Schedule vocabulary (`SandboxSet` etc.) stays in
  `ambition_gameplay_core::schedule`.

## Autonomous decision-making

When operating autonomously and you hit an architecture or design fork, **make the
choice Jon would most likely make and act** — read
`docs/concepts/autonomous-decision-making.md`. The short version: most
architecture/implementation forks are yours to decide (reserve questions for
product/scope, irreversible/outward-facing acts, or true intent ambiguity); score
candidates by elegance (obvious single source of truth, follows seams, no hidden
ordering), the layer boundaries (Rust=behavior, RON=content, LDtk=space, machinery
imports no named content), runtime efficiency, maintainability, and conciseness;
refactor toward the better-scoring option rather than taking the easy path; prefer
single-commit replacement over compatibility shims (pre-release); and on a timed
or autonomous run, **infer and keep going — do not stall to ask.** A behavior-
neutral change must keep replay bit-identical; a behavior fix ships with a focused
test.

## Spatial authoring discipline (LDtk, gates, hitboxes)

If you are placing entities, gates, walls, hitboxes, or other map
geometry, read `docs/concepts/llm-spatial-authoring-discipline.md`
before asking the user "where exactly?". The short version: read the
map, infer the *purpose* of the component (block exit / block entry
/ gate progression), place it along the seam that fulfils that
purpose, and state the reasoning in the commit message. Asking
"where?" is the wrong default.

## Engineering memory and benchmark candidates

Before a non-trivial patch, search prior mistakes:

```bash
rg -n "<subsystem>|<symptom>|<failure class>" dev/journals dev/benchmark-candidates
```

Use `dev/journals/` for symptom postmortems and `dev/benchmark-candidates/` for invariant traps before refactors.

If you notice a reusable failure mode, invariant trap, or repo-specific question that would catch a future agent mistake, opportunistically add or update a benchmark candidate under `dev/benchmark-candidates/` and link it from `dev/benchmark-candidates/index.md`. Do this only for durable lessons, not transient task state.

## Generated indexes

`.agent/index/` is generated, intentionally ignored by Git, and should not be committed.

If `.agent/index/` is missing, stale, or needed for file/symbol/test lookup,
regenerate it before using it:

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

## Style

To keep merge conflicts simple to resolve use a style formatter.

- Use `cargo fmt` on any modified Rust files.
- Use `ruff format` on any modified Python files.

## Common validation commands

```bash
cargo fmt --check
cargo test -p ambition_gameplay_core --lib
cargo test -p ambition_content --all-features
cargo run -p ambition_app --bin headless
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Use narrower tests when a focused test already covers the touched concept.
