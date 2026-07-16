# Ambition

**Ambition** is an experimental Rust/Bevy 2D platformer engine and game
project. The engine goal is ambitious on purpose: build a composable,
ECS-native 2D platformer/action-platformer/platform-fighter stack in the Bevy
style, with Ambition as the first flagship game and a suite of demo games as
capability proofs.

We are interested in answering the question:

> How ambitious of a game engine and game can we make with 2026+ level LLMs?

The current north star is:

> A reusable, composable, ECS-native 2D platformer engine on Bevy and Rust,
> where elegance, architectural beauty, headless testability, and agent
> navigability are first-class constraints.

Ambition is pre-release. Behavior and feel are not sacred yet. The project
optimizes for the elegant long-term design over compatibility shims, duplicated
paths, and local hacks.

## Source of truth

Start here, then route to the smallest relevant doc packet:

* [`AGENTS.md`](AGENTS.md) — short operating guide for coding agents.
* [`docs/README.md`](docs/README.md) — documentation map and reading router.
* [`docs/planning/README.md`](docs/planning/README.md) — **the master plan**:
  vision, roadmap, live work queue, and design docs for planned systems.
* [`docs/planning/tracks.md`](docs/planning/tracks.md) — live execution queue
  and status ledger.
* [`docs/planning/vision.md`](docs/planning/vision.md) — the project vision and
  executor model.
* [`docs/planning/decision-principles.md`](docs/planning/decision-principles.md)
  — Lead designer criteria for autonomous architecture choices.
* [`docs/adr/README.md`](docs/adr/README.md) — durable architectural decisions.
* [`docs/concepts/index.md`](docs/concepts/index.md) — reusable concepts,
  invariants, and edit protocols.
* [`docs/systems/index.md`](docs/systems/index.md) — current subsystem docs.
* [`docs/recipes/index.md`](docs/recipes/index.md) — build, authoring,
  profiling, and maintenance workflows.
* [`docs/tools/index.md`](docs/tools/index.md) — author-time tools.
* [`dev/README.md`](dev/README.md) and [`dev/SEARCH.md`](dev/SEARCH.md) —
  engineering memory from real mistakes.
* [`.agent/manifest.yaml`](.agent/manifest.yaml) — generated navigation index.

`docs/planning/` is the source of truth for direction and tasking. ADRs,
concept docs, system docs, recipes, and source code describe current facts and
may lag. If planning and an older doc disagree about direction, planning wins.
If planning and code disagree about current reality, the code wins and the plan
should be updated in the same commit that discovers the drift.

`docs/current/` is retired. Historical notes live under `docs/archive/`.
Brainstorms under `docs/brainstorms/` are design incubation space; agents
do not write there.

## Project stance

Ambition is:

* **Bevy-native.** Use Bevy and ECS directly where that improves correctness,
  integration, or expressibility.
* **Engine-first.** Ambition-the-game is the first content crate, not the
  engine's hardcoded reason for existing.
* **Data-driven.** Authored/generated data should feed entities, components,
  resources, systems, messages, and validated runtime seams.
* **LDtk-authored today.** LDtk owns world and level authoring for now. Other
  backends such as Tiled or Godot-scene importers are legitimate future
  siblings, not reasons to keep today's code vague.
* **Headless-testable.** Anything that affects simulation outcomes should be
  runnable and verifiable without rendering.
* **Actor-unified.** Player, enemy, boss, NPC, possessed body, and controlled
  participant are data/configuration distinctions over one actor/body model,
  not separate code paths.
* **Frame-aware.** Mechanics should ask “relative to what?” and avoid
  player-centric assumptions.
* **Pre-polish.** Tuning is deferred when the right values can be knobs.
  Architecture is not deferred when the wrong shape would create technical
  debt.

The design oracle is:

> Could another platformer be built by adding a content crate without editing
> core engine crates?

The demo suite exists to make that oracle executable.

## Current project shape

The workspace is mid-decomposition. Some target crates already exist; some
older machinery still lives in `ambition_actors` while the planning
tracks carve it apart.

Current high-level layers:

```text
foundations / vocabulary
  ambition_engine_core
  ambition_platformer_primitives
  ambition_entity_catalog
  ambition_characters
  ambition_combat
  ambition_input
  ambition_interaction
  ambition_menu
  ambition_time
  ambition_audio
  ambition_sfx
  ambition_sfx_bank
  ambition_asset_manager
  ambition_gameplay_trace
  ambition_cutscene
  ambition_sprite_sheet
  ambition_ui_nav
  ambition_vfx
  ambition_portal

simulation machinery
  ambition_actors
  ambition_runtime

observation and presentation
  ambition_sim_view
  ambition_render
  ambition_portal_presentation

content
  ambition_content

app / host / binaries
  ambition_host
  ambition_app

platform and support
  ambition_touch_input
```

Important current boundary notes:

* `ambition_actors` is the unified simulation heart. It is not awaiting a
  size-driven crate carve; remaining work removes content/lifecycle residue and
  converges action paths.
* `ambition_sim_view` is the observation boundary. Presentation and headless
  consumers read stable facts rather than mutating simulation internals.
* `ambition_runtime` owns headless-safe composition and global ordering;
  `ambition_host` owns window/device/presentation composition. Provider lifecycle
  and the programmatic simulation harness are accepted extraction seams.
* Game providers own named content through catalogs, registrations, and
  presentation plugins. Reusable engine crates do not own a flagship roster.
* `ambition_portal` and `ambition_portal_presentation` remain the exemplar split
  between simulation semantics and presentation.

The target crate stack, including crates that may not be fully carved out yet,
is documented in [`docs/planning/engine/architecture.md`](docs/planning/engine/architecture.md).


## Run the game

The normal desktop entry point is:

```bash
cargo run -p ambition_app --bin ambition_game_bin --release
```

The helper script wraps common desktop modes:

```bash
./run_game.sh
./run_game.sh release
./run_game.sh hot release
./run_game.sh validate
./run_game.sh hot release -- --start-room goblin_encounter
```

Use `./run_game.sh --help` for the full list.

The first Bevy build can take a while.

## Headless simulation

The dedicated headless runner is:

```bash
cargo run -p ambition_app --bin headless -- 120
```

Useful variants:

```bash
cargo run -p ambition_app --bin headless -- 600
cargo run -p ambition_app --bin headless -- 600 --dump-trace path/to/trace_dir
cargo run -p ambition_app --bin headless -- 600 --start-room goblin_encounter
```

The visible binary also has a no-display fallback:

```bash
cargo run -p ambition_app --bin ambition_game_bin -- --headless --headless-ticks 120
```

Trace replay:

```bash
cargo run -p ambition_app --bin trace_replay -- path/to/trace.json
cargo run -p ambition_app --bin trace_replay -- path/to/trace.json --tolerance 0.5
```

See [`docs/systems/headless-simulation.md`](docs/systems/headless-simulation.md)
for current details.

## Common validation

Prefer the narrowest validation that covers the touched subsystem. Common
anchors:

```bash
cargo fmt --check
cargo test -p ambition_engine_core
cargo test -p ambition_actors --lib
cargo test -p ambition_content --all-features
cargo test -p ambition_app
cargo run -p ambition_app --bin headless -- 30
python scripts/check_doc_links.py
```

The standing gate changes as the plan advances. Check
[`docs/planning/tracks.md`](docs/planning/tracks.md) before broad work.

When moving docs, tests, or code symbols, regenerate navigation indexes:

```bash
python scripts/generate_agent_index.py
python scripts/check_doc_links.py
```

## Documentation discipline

The planning docs are living tasking documents, not aspirational notes. Work
that completes, invalidates, or changes a planned slice should update the
relevant planning doc in the same commit.

Documentation routing:

* durable decisions → `docs/adr/`
* current concepts and invariants → `docs/concepts/`
* current subsystem facts → `docs/systems/`
* procedures/workflows → `docs/recipes/`
* author-time tools → `docs/tools/`
* active plan and tasking → `docs/planning/`
* Human-only incubation → `docs/brainstorms/`
* historical/superseded notes → `docs/archive/`
* engineering memory from bugs and traps → `dev/`

Keep `AGENTS.md` short. Keep `docs/README.md` as the router. Do not turn either
one into a context dump.
