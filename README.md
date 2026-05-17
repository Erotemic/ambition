# Ambition

**Ambition** is a Rust/Bevy 2D metroidvania/platformer sandbox plus reusable mechanics engine. The current direction is **Bevy-native, data-driven, and ECS-first**: LDtk and other authored/generated data produce Bevy entities and components, while reusable mechanics live in engine crates that remain easy to test.

The stable design law is:

> Make the movement toy excellent before making the world huge.

## Current source of truth

Start here, then route to the smallest relevant doc packet:

- [`AGENTS.md`](AGENTS.md) — short operating guide for coding agents.
- [`docs/README.md`](docs/README.md) — documentation map and reading router.
- [`docs/current/state.md`](docs/current/state.md) — current architecture and implementation state.
- [`docs/adr/README.md`](docs/adr/README.md) — durable architectural decisions.
- [`docs/concepts/index.md`](docs/concepts/index.md) — cross-cutting concepts, invariants, edit protocols, and validation anchors.
- [`docs/systems/index.md`](docs/systems/index.md) — current subsystem docs.
- [`docs/recipes/index.md`](docs/recipes/index.md) — current build, authoring, profiling, and maintenance workflows.
- [`docs/tools/index.md`](docs/tools/index.md) — author-time tools and when to use each.
- [`dev/README.md`](dev/README.md) and [`dev/SEARCH.md`](dev/SEARCH.md) — engineering memory from real mistakes.
- [`.agent/manifest.yaml`](.agent/manifest.yaml) — generated indexes for file/symbol/test localization.

Historical notes are preserved under `docs/archive/`. They explain the path here, but they do not override ADRs, `docs/current/`, or current source code.

## Project shape

```text
ambition_engine
  Bevy-native reusable mechanics vocabulary:
  movement, collision, body modes, combat intents, projectiles,
  actors, interactions, geometry, state-machine vocabulary, and tests.

ambition_asset_manager
  Asset identity/resolution policy across desktop, web, Android, Steam Deck,
  embedded assets, served assets, and loose development files.

ambition_sfx / ambition_sfx_bank
  Generated SFX IDs and packed runtime banks.

ambition_sandbox
  Playable Bevy shell:
  LDtk-authored world, ECS runtime, input/touch/controller adapters,
  presentation, audio, debug tools, and platform-specific app composition.

tools/
  Author-time generators and validators for LDtk, music, SFX, sprites,
  backgrounds, parallax assets, reports, and experiments.
```

## Current direction

- **Lean into Bevy.** The engine may use Bevy math/types and Bevy-friendly components when that improves correctness or integration.
- **Prefer data-driven ECS flow.** LDtk, asset manifests, generated specs, and authored configs should feed Bevy entities/components/systems instead of parallel code-owned world descriptions.
- **LDtk owns level/world authoring.** The old RON room-manifest direction is historical. RON remains useful for tuning, save/settings, and audio/data configs where it is still the right fit.
- **Support many platforms.** Desktop, web, Android, mobile/touch, controller, and Steam Deck should remain first-class. iOS is deferred until there is macOS test hardware, not rejected.
- **Keep validation close to the edit.** Concept pages and recipes should tell agents which tests or tools to run.

## Run

```bash
cargo run -p ambition_sandbox --release
```

The first Bevy build can take a while. For targeted validation, prefer the focused command listed in the relevant concept page or recipe.

## Documentation rules

- Keep `AGENTS.md` short. Route to docs; do not dump knowledge into it.
- Keep `docs/README.md` as the docs router.
- Put durable decisions in ADRs and keep ADRs modern.
- Put active implementation state in `docs/current/`.
- Put current subsystem facts in `docs/systems/`.
- Put procedures in `docs/recipes/`.
- Put reusable concepts/invariants in `docs/concepts/`.
- Put active idea incubation in `docs/brainstorms/`.
- Put historical/superseded material in `docs/archive/` or delete it.
- Put hard-won postmortems and benchmark traps in `dev/`.
- Regenerate `.agent/` indexes after doc/code/test moves.
