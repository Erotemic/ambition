# Architecture

Ambition is a Rust/Bevy workspace with reusable support crates, a playable sandbox crate, and author-time tools. The current architecture is Bevy-native and ECS-first; old backend-neutral constraints are superseded by ADR 0002.

## Crate responsibilities

| Crate | Responsibility |
|---|---|
| `crates/ambition_sandbox/src/engine_core/` | Reusable mechanics vocabulary inside the sandbox crate: movement, collision, body modes, geometry, block/world policy, player clusters, and tests. Bevy-friendly types are allowed when useful. |
| `ambition_asset_manager` | Asset identity, platform profile resolution, embedded/served/loose asset roots, and Bevy integration. |
| `ambition_sfx` | Stable generated SFX identifiers and sound vocabulary. |
| `ambition_sfx_bank` | Runtime SFX-bank parsing and lookup. |
| `ambition_sandbox` | The playable Bevy app: LDtk runtime projection, input adapters, player ECS state, presentation, UI, audio, dev tools, platform composition, and headless entry points. |

## Current sandbox module shape

The sandbox crate is organized around themed modules:

```text
crates/ambition_sandbox/src/app/          app setup, schedules, update phases
crates/ambition_sandbox/src/content/      authored feature/content conversion
crates/ambition_sandbox/src/world/        LDtk world, room building, physics, platforms
crates/ambition_sandbox/src/player/       player ECS components and systems
crates/ambition_sandbox/src/input/        action/control-frame/menu input
crates/ambition_sandbox/src/persistence/  save data and settings persistence
crates/ambition_sandbox/src/presentation/ sprites, camera, parallax, rendering, UI fonts
crates/ambition_sandbox/src/dev/          trace, debug overlays, profiling, mechanics tools
crates/ambition_sandbox/src/runtime/      game mode, reset, setup
crates/ambition_sandbox/src/host/         desktop/web/mobile/platform glue
```

`crates/ambition_sandbox/src/lib.rs` still exposes compatibility re-export shims such as `features`, `ldtk_world`, `rooms`, `game_mode`, and `trace`. Treat those as transitional API paths, not as a reason to add new root-level modules.

## Boundary rules

- Put reusable mechanics/data vocabulary in `engine_core` or another focused sandbox module when the mechanic is still sandbox-owned.
- Put Bevy app composition, presentation, platform packaging, and LDtk runtime adaptation in `ambition_sandbox`.
- Use components/resources/messages at runtime integration seams.
- Do not add abstraction layers only to avoid Bevy.
- Keep colors, sprites, audio playback, HUD layout, inspector UI, and packaging policy out of `engine_core`.

## Refactor rules for agents

1. Search `dev/` for prior module-split or stale-component traps before broad refactors.
2. Preserve public compatibility shims until downstream imports are updated in the same patch.
3. Keep tests and test helper visibility working during moves.
4. Prefer one focused module move at a time.
5. Regenerate `.agent/` indexes after moving docs, tests, or code symbols.

## Validation anchors

```bash
cargo fmt --check
cargo test -p ambition_sandbox --lib engine_core
cargo test -p ambition_sandbox --lib
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
python scripts/check_doc_links.py
```

Related decisions: ADR 0002, ADR 0003, ADR 0012, ADR 0016.
