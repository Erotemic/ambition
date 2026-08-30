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
* [`docs/planning/tracks.md`](docs/planning/tracks.md) — standing backlog and
  reservoir that feeds the live execution queue.
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
* [`.agent/README.md`](.agent/README.md) — generated, commit-matched navigation and query protocol.

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

> Could another platformer be built by adding a provider/content crate without
> editing core engine crates?

It judges the end state, not each commit: reusable engine capability grows
constantly, and named game content and policy enter through provider crates, Bevy
plugins, and supported Ambition seams. What the oracle forbids is a core that
cannot be extended additively — a game-named branch, a closed roster, or a private
replacement for an ordinary engine responsibility.

The demo suite runs the oracle continuously.
[`fixtures/external_consumer/`](fixtures/external_consumer/) (Outlander) runs it
adversarially: a game authored from outside the workspace, with its own lockfile,
against the `ambition_platformer2d` umbrella alone. `external consumer: outlander` gates it in
`scripts/run_tests.py`, and every engine-internal assumption it is forced to lean
on is recorded as a named API leak.

## Agent-native authoring toolchain

Ambition is intentionally designed so that **LLM agents can author substantial
game content without operating a graphical editor**. The project already has
structured, reproducible authoring systems for worlds, sprites, music, SFX, and
measurements. Human-facing editors remain useful frontends, but they are not the
architectural center of the content pipeline.

Several large authoring/content repositories are git submodules. A source archive,
sparse checkout, or uninitialized clone may therefore show an empty directory.
**Do not infer that the capability does not exist merely because the submodule is
not checked out.** Consult `.gitmodules` and the canonical repository links below.

| Capability | Local path | Canonical repository | Agent-facing source/contract |
|---|---|---|---|
| Procedural sprites and portraits | `tools/ambition_sprite2d_renderer/` | [ambition_sprite2d_renderer](https://github.com/Erotemic/ambition_sprite2d_renderer) | Python/YAML targets -> deterministic sheets, metadata, portraits, review renders |
| Music | `tools/ambition_music_renderer/` | [ambition_music_renderer](https://github.com/Erotemic/ambition_music_renderer) | MusicIR YAML -> render/audit/bundle/publish |
| Sound effects | `tools/ambition_sfx_renderer/` | [ambition_sfx_renderer](https://github.com/Erotemic/ambition_sfx_renderer) | SFXIR YAML -> deterministic render manifests and audio |
| Engineering measurements | `dev/ambition_dev_measurements/` | [ambition_dev_measurements](https://github.com/Erotemic/ambition_dev_measurements) | retained measurement data kept out of the main repository |
| LDtk world assets | `game/ambition_map_assets/` | [ambition_map_assets](https://github.com/Erotemic/ambition_map_assets) | canonical LDtk worlds mounted into consuming game crates |

The semantic LDtk mutation/query toolkit itself lives in this repository at
`tools/ambition_ldtk_tools/`. It supports structured inspection, validation,
transactional edits, semantic diffs, room summaries/renders, spatial queries,
and other operations intended specifically to let agents reason about worlds
without hand-editing LDtk JSON.

For the durable authoring doctrine, read
[`docs/concepts/agent-native-authoring.md`](docs/concepts/agent-native-authoring.md).
For tool entry points, read [`docs/tools/index.md`](docs/tools/index.md).

A normal full clone can initialize the submodules with:

```bash
git submodule update --init --recursive
```

`./run_developer_setup.sh` also initializes the active authoring submodules and
creates their tool-local Python environments. If an agent is operating from a
source export where submodules cannot be fetched, it should explicitly report
that its audit of those capabilities is partial rather than planning replacements
for unseen tooling.

## Durable engine shape

The package list will keep changing. The responsibilities and dependency
direction should not. Read
[`docs/concepts/engine-mental-model.md`](docs/concepts/engine-mental-model.md)
for the full model.

```text
foundations and stable data contracts
    -> shared platformer vocabulary
    -> focused domain services
    -> unified simulation heart
    -> observation/read models
    -> presentation

runtime/provider/host compose those layers
providers own named game content
thin apps choose a provider and platform persona
```

The most important consequences are:

* **One body, one path.** Player, enemy, boss, NPC, possessed body, and RL body
  differ by data, capabilities, and controller—not by parallel movement/combat
  engines.
* **Providers own names.** Worlds, characters, dialogue, art, audio, encounters,
  quests, and game rules live above reusable engine crates.
* **Simulation owns outcomes.** Presentation consumes read models and semantic
  effects and can disappear in a headless composition.
* **Construction is transactional.** Provider/world content is prepared and
  validated before one lifecycle-scoped session or room commit.
* **Stable identity is authored.** Bevy `Entity` values are allocator handles,
  not persisted/provider identity.
* **The engine is executable without Ambition.** Demo providers are acceptance
  tests for reusable capability and clean dependency direction.

Use the generated index for the current package map instead of this README:

```bash
python scripts/agent_query.py overview
python scripts/agent_query.py crate <likely-owner>
python scripts/agent_query.py "<task words>"
```

Active architecture direction lives in
[`docs/architecture/engine-architecture.md`](docs/architecture/engine-architecture.md).


## Developer setup

From a fresh clone, the supported zero-to-runnable path is:

```bash
./run_developer_setup.sh
./run_game.sh
```

The setup script installs host and Rust dependencies, initializes submodules,
creates an isolated `.venv` inside each active Python authoring tool, regenerates
all runtime assets, and checks the desktop game target. It is a fresh-clone or
environment-repair command, not a prerequisite for ordinary asset regeneration.
Once the tool-local environments exist, run the relevant renderer or regeneration
script directly. Re-run setup only after dependency, Python-version, submodule,
or host-tooling changes. Use `./run_developer_setup.sh --help` for phase-specific
skip flags.

## Run the game

The normal desktop entry point is:

```bash
cargo run -p ambition_app --bin ambition_game_bin --release
```

The helper script wraps common desktop modes:

```bash
./run_game.sh
./run_game.sh release
./run_game.sh --ship
./run_game.sh hot release
./run_game.sh validate
./run_game.sh hot release -- --start-room goblin_encounter
```

Use `./run_game.sh --help` for the full list. `--ship` selects the portable
native distribution profile used for the packaged Steam build: fat LTO, one
codegen unit, aborting panics, no debug information, and stripped symbols. It
deliberately does not use host-specific CPU tuning.

Each demo also ships its OWN standalone shell, and the script launches them by
name — windowed by default:

```bash
./run_game.sh sanic
./run_game.sh mary-o
./run_game.sh smash
./run_game.sh twintrack
```

`--headless` opts a demo into its sim-only shell instead, which prints a summary
rather than opening a window:

```bash
./run_game.sh twintrack --headless -- --ticks 600
```

The first Bevy build can take a while.

## Headless simulation

The dedicated headless runner is:

```bash
cargo run -p ambition_app_tools --bin headless -- 120
```

Useful variants:

```bash
cargo run -p ambition_app_tools --bin headless -- 600
cargo run -p ambition_app_tools --bin headless -- 600 --dump-trace path/to/trace_dir
cargo run -p ambition_app_tools --bin headless -- 600 --start-room goblin_encounter
```

The visible binary also has a no-display fallback:

```bash
cargo run -p ambition_app --bin ambition_game_bin -- --headless --headless-ticks 120
```

Trace replay:

```bash
cargo run -p ambition_app_tools --bin trace_replay -- path/to/trace.json
cargo run -p ambition_app_tools --bin trace_replay -- path/to/trace.json --tolerance 0.5
```

See [`docs/systems/headless-simulation.md`](docs/systems/headless-simulation.md)
for current details.

## Common validation

Use the repository test runner as the canonical headless front door:

```bash
./run_tests.sh --list                                   # what would run
./run_tests.sh -p <owning-package> -k <test-substring>  # focused
./run_tests.sh                                          # backbone: python + cargo test --workspace
./run_tests.sh --run-everything-you-probably-dont-need-this   # exhaustive, ~1 hour
```

The default is the backbone, not the exhaustive plan. Each run prints what it
did not cover; the exhaustive plan is worth its hour before a release or after
touching features, an SDK surface, or the web path — rarely mid-edit.

Localize the narrowest tests before running broad suites:

```bash
python scripts/agent_query.py tests "<invariant>"
python scripts/agent_query.py ecs "<resource, message, or system>"
```

Useful non-Rust checks and runtime probes:

```bash
python scripts/check_doc_links.py
python scripts/generate_agent_index.py
cargo run -p ambition_app_tools --bin headless -- 30
```

Formatting is advisory rather than an acceptance gate. Test authoritative
invariants and the real headless composition; reserve visible smoke tests for
presentation feel.


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
