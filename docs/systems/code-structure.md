# Code structure

Ambition is a multi-crate Rust/Bevy workspace plus author-time Python tools.

## Crates

| Crate | Purpose |
|---|---|
| `ambition_engine` | Reusable Bevy-native mechanics, geometry, movement, combat, projectiles, actor vocabulary, tests. |
| `ambition_asset_manager` | Asset IDs, platform profiles, source resolution, Bevy integration. |
| `ambition_sfx` | Stable SFX IDs / generated sound vocabulary. |
| `ambition_sfx_bank` | Runtime SFX bank parsing/lookup. |
| `ambition_sandbox` | Playable Bevy app, LDtk runtime, input, audio, UI, presentation, dev tools, platform feature sets. |

## Tools

Author-time generators and validators live under `tools/`. See `docs/tools/index.md`.

## Refactor rule

When splitting Rust modules:

1. keep the old public facade until downstream imports are updated;
2. preserve tests and `#[cfg(test)]` helper visibility;
3. search `dev/` for prior module-split traps;
4. run focused tests before broad tests;
5. regenerate `.agent` indexes after moves.

See `docs/concepts/rust-module-boundaries.md`.
