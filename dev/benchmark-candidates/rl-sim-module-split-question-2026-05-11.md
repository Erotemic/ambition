# Benchmark candidate: split and rename an RL simulation facade without leaving parse or public-interface regressions

## Context

A Rust module named `rl.rs` was renamed to `rl_sim` and split into child modules
(`action`, `observation`, `options`, `runtime`, and `tests`). The intent was to
remove `rl` as a standalone public name while keeping `rl_sim` acceptable in
module names, feature names, and binary prefixes.

The first generated overlay had two easy-to-miss extraction defects:

1. A dangling `#[cfg(test)]` attribute was left at the end of
   `rl_sim/runtime.rs` after moving the test module to `rl_sim/tests.rs`.
2. Imports for Bevy plugins and timestep helpers stayed in the `rl_sim.rs`
   facade even though the code that used them had moved to `rl_sim/runtime.rs`.

## Prompt

Given a Rust crate with a public module `rl.rs`, rename and split it to
`rl_sim` so that no standalone public `rl` interface remains. Preserve behavior,
update Cargo features / required-features / cfg gates, and split the module into
focused child files.

## Expected answer properties

- Delete or explicitly remove the old `src/rl.rs` file when using an overlay
  workflow that cannot represent deletions.
- Replace public imports such as `crate::rl::...` and
  `ambition_gameplay_core::rl::...` with `rl_sim` equivalents.
- Rename the Cargo feature from `rl` to `rl_sim`, including `required-features`
  and `#[cfg(feature = ...)]` gates.
- Ensure each extracted child module owns the imports it needs.
- Ensure the facade contains only module declarations, re-exports, and docs; it
  should not retain imports used only by moved implementation code.
- Ensure no dangling attributes or doc comments remain after moving tests or
  items to child modules.

## Validation commands

```bash
cargo fmt --all
cargo test -p ambition_gameplay_core --lib rl_sim
cargo test -p ambition_gameplay_core --test fuzz_random_walker
cargo test -p ambition_gameplay_core --test replay_fixture_regression
cargo test -p ambition_gameplay_core --test crouch_stability
cargo test -p ambition_gameplay_core --test dash_stability
cargo test -p ambition_gameplay_core --test repro_walls
```

## Failure signatures to catch

```text
error: expected item after attributes
   --> crates/ambition_app/src/rl_sim/runtime.rs:276:1
    |
276 | #[cfg(test)]
```

```text
warning: unused import: `bevy::asset::AssetPlugin`
```
