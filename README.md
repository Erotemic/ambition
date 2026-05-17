# Ambition

**Ambition** is a code-first Rust/Bevy 2D metroidvania/platformer project and reusable mechanics engine. The game is meant to feel excellent as raw collision boxes first, then layer procedural visuals, generated audio, story, and mathematical world rules on top.

The stable design law is:

> Make the movement toy excellent before making the world huge.

## Current source of truth

The README is intentionally high-level. It should not try to track every transient patch or experimental room. Use these documents for current details:

- [`AGENTS.md`](AGENTS.md) — short operating guide for coding agents.
- [`docs/README.md`](docs/README.md) — documentation map and reading router.
- [`docs/current/state.md`](docs/current/state.md) — current architecture and active implementation state.
- [`docs/current/risks.md`](docs/current/risks.md) — high-risk systems and review rules.
- [`docs/current/next.md`](docs/current/next.md) — current next good moves.
- [`docs/GOAL_STATE.md`](docs/GOAL_STATE.md) — long-term product/engine vision.
- [`docs/concepts/index.md`](docs/concepts/index.md) — durable concept pages: invariants, aliases, edit protocols, validation paths.
- [`docs/systems/index.md`](docs/systems/index.md) — focused subsystem documentation.
- [`docs/recipes/index.md, docs/vision/index.md, docs/planning/index.md`](docs/recipes/index.md, docs/vision/index.md, docs/planning/index.md) — procedural workflows for builds, tests, refactors, profiling, packaging, and content authoring.
- [`dev/README.md`](dev/README.md) — engineering memory: journals and benchmark candidates from real mistakes.
- [`.agent/manifest.yaml`](.agent/manifest.yaml) — generated indexes for file/symbol/test localization.
- [`docs/adr/README.md`](docs/adr/README.md) — architectural decision records. ADRs are the durable source of truth for decisions that supersede older notes.

Historical notes under `docs/` are preserved because they explain why the project moved in certain directions, but some are intentionally older than the current design. When a decision conflicts with an ADR or `docs/current/`, prefer the ADR/current-state document and update the stale note or add a supersession pointer.

## Project shape

```text
ambition_engine
  Reusable Bevy-native mechanics crate:
  movement, abilities, collision, geometry queries, rooms, transitions,
  combat, actors, hazards, interactables, state-machine vocabulary,
  generated-audio specs, and testable data structures.

ambition_sandbox
  Playable Bevy development shell:
  RON data, all-abilities sandbox rooms, feature basement labs, input presets,
  generated audio playback, visuals, debug overlays, inspector tooling,
  and fast iteration experiments.

future game/story crates
  Thin content crates:
  campaign progression, dialogue, biomes, story flags, world graphs,
  and authored/generated room data.
```

The engine is allowed to depend on Bevy-adjacent crates when that gives Ambition robust primitives. The earlier idea that the engine must be Bevy-independent is superseded.

## Run

From the repository root:

```bash
cargo run -p ambition_sandbox --release
```

The first Bevy build can take a while. The sandbox currently embeds its RON manifest for reliable startup while also moving toward Bevy asset loading.

## Controls

The sandbox maps physical inputs to semantic actions through Leafwing Input Manager. Key presets are deliberately sandbox-facing and may change while the feel is tuned.

Universal dev controls:

| Input | Action |
|---|---|
| `Escape` | Pause / resume |
| `Delete` or `Backspace` | Reset sandbox |
| `F1` | Toggle debug overlay |
| `F2` | Toggle slow motion |
| `F3` | Toggle reflected resource inspector windows |
| `F4` | Toggle full Bevy world inspector |
| `F9` / `F10` | Previous / next control preset |

See `crates/ambition_sandbox/src/input.rs` for the current preset definitions.

## Design target

The first playable experience should prove that Ambition can support a real-feeling miniature metroidvania: a hub, a small progression loop, enemies, hazards, one boss, one ability unlock, dialogue hooks, and a shortcut back to safety.

The longer-term ambition is larger: movement/combat mastery, mathematical spaces and abilities, story about AI agency and ethical incentives, and procedural/code-owned art/audio that remains inspectable and reproducible.

## Documentation rules

- Keep the README stable and short.
- Put volatile state in `docs/current/`.
- Put long-term vision in `GOAL_STATE.md`.
- Put decisions in ADRs.
- Put current subsystem notes in `docs/systems/`.
- Put procedural workflows in `docs/recipes/`.
- Promote durable cross-cutting rules into `docs/concepts/`.
- Keep hard-won debugging memories and benchmark traps in `dev/`.
- Keep generated navigation aids in `.agent/` and verify them with `python scripts/check_agent_kb.py`.
- Preserve history, but clearly mark superseded guidance.
