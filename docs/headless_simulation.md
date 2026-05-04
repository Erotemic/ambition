# Headless simulation

Ambition's sandbox now ships two binaries:

- `cargo run -p ambition_sandbox` — the visible Bevy app (windowed gameplay)
- `cargo run -p ambition_sandbox --bin headless [TICKS]` — a no-display
  simulation runner

Both depend on the `ambition_sandbox` library crate, which owns the module
graph and the cross-cutting resources (`GameWorld`, `SandboxRuntime`). The
visible binary's `main.rs` is the existing playable shell; the headless
binary is a thin shim around `ambition_sandbox::run_headless`.

This document describes the sim/presentation contract the headless runner
exposes today, what it deliberately does NOT do yet, and the path to
"gameplay loop runs headless" via the events refactor.

## What headless does today (Phase 1)

`run_headless(max_ticks)` constructs a Bevy `App` from `MinimalPlugins` plus
`AssetPlugin` and `StatesPlugin`, registers the simulation-side LDtk
runtime-spine systems, ticks `Update` `max_ticks` times, and returns a
`HeadlessReport` summarizing what ran. It validates that:

- the embedded LDtk world parses and validates,
- the runtime `RoomSet` and `LdtkRuntimeIndex` construct from LDtk,
- the runtime-spine systems (`sync_plugin_spawned_ambition_entities`,
  `rebuild_ldtk_runtime_spine_index`, `rebuild_ldtk_runtime_solid_index`,
  `poll_ldtk_file_changes`) compile and tick on a no-display machine.

It does NOT install `bevy_ecs_ldtk::LdtkPlugin`, because that plugin's tile
spawning depends on Bevy's image/render plugins. Without LDtk-spawned
entities the runtime-spine systems run as no-ops; the report reflects zero
spawned entities. That is the correct Phase 1 outcome — the goal is "no
panic on a display-less VM," not "RL-ready simulation."

## What headless deliberately does NOT do (yet)

The headless runner does not call `sandbox_update`, `setup`, `update_hud`,
or any of the audio/VFX/HUD/dialogue systems. These are still presentation-
coupled in non-trivial ways:

- `sandbox_update` directly emits `play_sound` calls, `spawn_burst`,
  `spawn_dust`, `spawn_impact`, `spawn_blink_effects`, `spawn_slash_preview`,
  and physics debris bursts inside its event-handling helpers.
- presentation setup spawns rendering entities (Camera2d, player Sprite, room
  visuals, HUD Text) and creates the generated Kira audio library, which is
  only registered by the visible app path.
- The hot-reload entry path reads `ButtonInput<KeyCode>`, which requires
  the input plugin.

Inverting any one of these inside `run_headless` would require a
conditional/feature-flag layer, which the project has explicitly chosen not
to ship. The right move is the events refactor (Phase 2 below).

## Phase 2 — events refactor (planned, not yet implemented)

To run the gameplay loop headless, the simulation needs to emit typed events
instead of directly calling presentation APIs. Concretely:

- `play_sound(commands, bank, SoundCue::Jump)` becomes
  `event_writer.send(SfxEvent::Jump { pos })`. An audio system in
  presentation subscribes to `SfxEvent` and plays the actual Kira SFX channel.
- `spawn_burst(...)` becomes `event_writer.send(VfxEvent::Burst { ... })`,
  consumed by an fx system in presentation.
- `physics::spawn_debris_burst(...)` is already partly engine-side
  (Avian2D bodies); the trigger should still flow through an event so the
  headless build doesn't need Avian's debug visuals.
- HUD reads (`update_hud`) stay in presentation; they're already pure
  consumers of resources.

After the events refactor, `add_simulation_plugins(app)` and
`add_presentation_plugins(app)` become honest split points. The visible
binary adds both; the headless binary adds only the simulation side and
gets a working gameplay loop. RL drivers and replay tooling become
adapters that produce `ControlFrame` per tick and consume the resulting
events.

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
  between the visible app and a headless driver. No deep refactor of
  `main.rs` was needed; the existing 1300-line file simply imports its
  module graph through the wildcard `use ambition_sandbox::*;`.
- `SandboxRuntime` and `GameWorld` moved to the library so submodules'
  `crate::` paths continue to resolve. They are still the SP-only shape
  per the architecture targets memory; future patches should migrate
  per-player state onto a Player entity.
- All `pub(crate)` fields on `SandboxRuntime` were widened to `pub` because
  the binary `main.rs` is now a separate crate from the library and needs
  access for HUD reads and `sandbox_update` writes. This is a structural
  consequence of the split, not a design preference; once `sandbox_update`
  moves into the library proper (or splits across systems that live there),
  these fields can tighten again.

## Verification

In the dev VM (no display, no GPU):

```bash
cargo test -p ambition_engine             # 3 tests pass
cargo test -p ambition_sandbox            # 6 tests pass (incl 2 headless)
cargo run -p ambition_sandbox --bin headless 30
```

The last command prints a `HeadlessReport` summary and exits 0.
