# Headless simulation

Ambition's sandbox ships two binaries:

- `cargo run -p ambition_sandbox` — the visible Bevy app (windowed gameplay)
- `cargo run -p ambition_sandbox --bin headless [TICKS]` — a no-display
  simulation runner that drives the **full gameplay loop**

Both depend on the `ambition_sandbox` library crate, which owns the module
graph and the cross-cutting resources (`GameWorld`, `SandboxRuntime`). The
visible binary's `main.rs` is the existing playable shell; the headless
binary is a thin shim around `ambition_sandbox::run_headless`.

> **Phase status (2026-05-07):** Phase 1 (no-display tick) and Phase 2
> (gameplay loop runs headless) are both complete. The events refactor
> (ADR 0012, slices `c49c1e5`–`81900dd`) shipped end-to-end; `sandbox_update`
> emits `SfxMessage`/`VfxMessage`/`DebrisBurstMessage` and presentation
> subscribers consume them, so the simulation has no presentation
> coupling. Phase 3 (RL adapter, fixed-timestep determinism) remains
> future work.

This document describes the sim/presentation contract the headless runner
exposes today.

## What headless does today (Phases 1 + 2)

`run_headless(max_ticks)` constructs a Bevy `App` via
`add_simulation_plugins(app)` — `MinimalPlugins`, `AssetPlugin`,
`StatesPlugin`, the LDtk runtime spine, the state-machine plugin, the
physics plugin, and the simulation systems including `sandbox_update`. It
ticks `Update` `max_ticks` times and returns a `HeadlessReport`
summarizing what ran. It validates that:

- the embedded LDtk world parses and validates,
- the runtime `RoomSet` and `LdtkRuntimeIndex` construct from LDtk,
- the runtime-spine systems (`sync_plugin_spawned_ambition_entities`,
  `rebuild_ldtk_runtime_spine_index`, `rebuild_ldtk_runtime_solid_index`,
  `poll_ldtk_file_changes`) compile and tick on a no-display machine,
- `sandbox_update` and the gameplay loop tick to completion under
  scripted `ControlFrame` input without touching presentation.

It does NOT install `bevy_ecs_ldtk::LdtkPlugin`, because that plugin's tile
spawning depends on Bevy's image/render plugins. Without LDtk-spawned
entities the runtime-spine systems run as no-ops on tile data; entities
still spawn through the `sync_plugin_spawned_ambition_entities` path
when the simulation seeds them.

## What headless deliberately does NOT do

The presentation layer is excluded by construction:
`add_presentation_plugins(app)` (which the visible binary calls and the
headless binary does not) installs `DefaultPlugins`, `EguiPlugin`,
`MaterialUiPlugin`, `InputManagerPlugin`, the dialog plugin, the
inspector, audio mixers, and the HUD/VFX subscriber systems that consume
the simulation's `SfxMessage` / `VfxMessage` / `DebrisBurstMessage`
output. The headless binary skips this layer and so:

- emits but does not realize SFX/VFX events,
- does not spawn Camera2d, player Sprite, or HUD Text,
- does not start the Kira audio engine,
- does not read `ButtonInput<KeyCode>` (input is supplied as
  `ControlFrame` values).

Scripted gameplay tests in `crates/ambition_sandbox/tests/scripted_gameplay.rs`
inject `ControlFrame`s and assert on emitted message counts.

## Phase 3 — RL adapter (further out)

With the events refactor landed, an RL adapter is a thin layer:

- input: replace the Leafwing `ActionState` reader with a function that
  produces a `ControlFrame` from an externally-supplied action vector.
- observation: walk `SandboxRuntime`, `GameWorld`, `LdtkRuntimeSolidIndex`
  to construct a typed observation per tick.
- determinism: switch to a fixed timestep, seed any RNG that gameplay
  uses, freeze wall-clock leaks. Most of this is a Bevy `FixedTime` config
  plus an audit pass.

A Python binding via PyO3 is then a separate, optional layer.

## Architecture notes that informed Phase 1

- The library/binary split is the cheapest way to share the module graph
  between the visible app and a headless driver. The presentation
  binary's `main.rs` is now ~10 lines (it just calls
  `ambition_sandbox::run_app`); systems and resources live in the
  library.
- `SandboxRuntime` and `GameWorld` moved to the library so submodules'
  `crate::` paths continue to resolve. They are still the SP-only shape
  per the architecture targets memory; future patches should migrate
  per-player state onto a Player entity.
- `SandboxRuntime` fields are `pub` (not `pub(crate)`) because the
  separate binary crates (`main`, `headless`, scripted gameplay tests
  in `tests/`) need access for HUD reads, scripted-input writes, and
  assertions. This is a structural consequence of the split, not a
  design preference; once HUD wiring fully moves to event subscribers
  these fields could tighten again.

## Verification

In the dev VM (no display, no GPU):

```bash
cargo test -p ambition_engine             # ~196 tests pass
cargo test -p ambition_sandbox            # ~330 tests pass
cargo run -p ambition_sandbox --bin headless 30
```

The last command prints a `HeadlessReport` summary and exits 0. Test
counts grow as coverage expands — the count above is a snapshot, not a
guarantee. The relevant invariant is "exit code 0, no panic."
