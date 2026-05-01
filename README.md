# Ambition

**Ambition** is a code-first Rust/Bevy 2D metroidvania/platformer project and reusable mechanics engine. The game is meant to feel excellent as raw collision boxes first, then layer procedural visuals, generated audio, story, and mathematical world rules on top.

The stable design law is:

> Make the movement toy excellent before making the world huge.

## Current source of truth

The README is intentionally high-level. It should not try to track every transient patch or experimental room. Use these documents for current details:

- [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) — current architecture, active systems, experimental areas, and known limitations.
- [`docs/GOAL_STATE.md`](docs/GOAL_STATE.md) — long-term product/engine vision.
- [`docs/AGENT_HANDOFF.md`](docs/AGENT_HANDOFF.md) — instructions for future agents and contributors.
- [`docs/PROGRESSION_LOG.md`](docs/PROGRESSION_LOG.md) — compact chronology of major patch waves.
- [`docs/adr/README.md`](docs/adr/README.md) — architectural decision records. ADRs are the durable source of truth for decisions that supersede older notes.

Historical notes under `docs/` are preserved because they explain why the project moved in certain directions, but some are intentionally older than the current design. When a decision conflicts with an ADR or `CURRENT_STATE.md`, prefer the ADR/current-state document and update the stale note or add a supersession pointer.

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
- Put volatile state in `CURRENT_STATE.md`.
- Put long-term vision in `GOAL_STATE.md`.
- Put decisions in ADRs.
- Put implementation notes near the system they describe.
- Preserve history, but clearly mark superseded guidance.


