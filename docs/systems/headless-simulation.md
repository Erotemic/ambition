# Headless simulation

Ambition's runnable binaries live in `ambition_app`:

- `cargo run -p ambition_app --bin ambition_game_bin` — visible Bevy app.
- `cargo run -p ambition_app --bin ambition_game_bin -- --headless --headless-ticks 120` — visible binary's no-display fallback path.
- `cargo run -p ambition_app --bin headless -- 120` — dedicated no-display simulation runner.

`ambition_gameplay_core` is the gameplay core library used by those binaries. It
is not the playable or headless package.

## Dedicated headless binary

The current dedicated entry point is:

```text
crates/ambition_app/src/bin/headless.rs
  -> ambition_app::run_headless(max_ticks)
  -> crates/ambition_app/src/headless.rs
  -> builds a Bevy App with MinimalPlugins + minimal asset/image/transform/state plugins
  -> installs SandboxSimulationPlugin
```

Usage:

```bash
cargo run -p ambition_app --bin headless                 # 120 ticks (default)
cargo run -p ambition_app --bin headless -- 600          # 600 ticks
cargo run -p ambition_app --bin headless -- 600 --dump-trace path/
cargo run -p ambition_app --bin headless -- 600 --start-room goblin_encounter
```

`--dump-trace DIR` writes a `GameplayTraceBuffer` JSON + Markdown dump after the
final tick so `trace_replay` can re-drive the same input sequence later. The
first positional non-flag argument is the tick count; the dedicated `headless`
binary does not currently accept `--ticks`.

## What headless does today

`run_headless(max_ticks)` constructs a Bevy `App`, installs
`SandboxSimulationPlugin`, ticks `Update` `max_ticks` times, and returns a
`HeadlessReport` summarizing what ran. It validates that:

- the embedded LDtk world parses and validates,
- `RoomSet` and the LDtk runtime resources initialize,
- room/entity runtime-spine systems compile and tick on a no-display machine,
- gameplay systems can run to completion under scripted `ControlFrame` input,
- named content needed by the sim is installed through `AmbitionContentPlugin`.

The sim-only path skips the full visible app stack: no window, no camera, no HUD,
no audio mixer, no inspector, and no visible sprite presentation.

## Simulation composition

`SandboxSimulationPlugin` is the app-level composition point for a sim-only Bevy
App. It lives in `ambition_app` because it names the gameplay core and named
content together. It installs:

- simulation resources and schedules,
- `ambition_content::AmbitionContentPlugin`,
- gameplay core plugins/systems for player, gravity, portal mechanics, items,
  combat, LDtk runtime, encounters, effects, reset, traces, and affordances,
- a small amount of neutral schedule glue needed by the sim.

Presentation belongs to `SandboxPresentationPlugin` and `ambition_render`.
Visible builds add that layer in addition to `SandboxSimulationPlugin`.

## Current boundary note

`SandboxSimulationPlugin` currently installs
`ambition_render::cutscene::CutsceneSchedulePlugin`. That plugin uses neutral
cutscene state/schedule vocabulary, but it lives in the render crate today. Treat
this as known boundary debt: it is allowed by the current source tree, but future
cleanup should move neutral cutscene scheduling into `ambition_cutscene` or
`ambition_gameplay_core` and leave only visual cutscene UI in `ambition_render`.

## RL stepping API

`crates/ambition_app/src/rl_sim/` contains the Rust stepping API used by the
headless binary's trace-dump path and experimental RL drivers:

- `AgentAction` — sparse per-tick intent struct converted to `ControlFrame`.
- `AgentObservation` — owned snapshot of player, room, health, ability, and
  episode state.
- `SandboxSim::new()` / `SandboxSim::new_with_options(...)` — build the same
  sim app shape used by headless and run an initial tick.
- `sim.step(action)` / `step_n` / `step_with_reward` — tick the sim with scripted
  input.
- `sim.reset_episode()` — drives the existing reset machinery.
- `sim.world()` / `sim.world_mut()` — escape hatches for focused inspection or
  scripted setup.

The `rl_sim` feature-gated binaries currently rely on the default app feature set.
Do not advertise `--no-default-features --features rl_sim` as a supported command
until that feature combination is fixed and validated.

## Trace replay

`crates/ambition_app/src/bin/trace_replay.rs` reads a `GameplayTraceBuffer` JSON
dump and drives a fresh `SandboxSim` with the recorded `ControlFrame` sequence at
fixed-60Hz timestep. Use it for:

- reproducing a player-submitted trace,
- checking deterministic replay after a refactor,
- pinning broad gameplay invariants with an in-tree fixture trace.

Current command shape:

```bash
cargo run -p ambition_app --bin trace_replay -- path/to/trace.json
cargo run -p ambition_app --bin trace_replay -- path/to/trace.json --tolerance 0.5
```

## Visible-binary headless fallback

`run_visible` (`cargo run -p ambition_app --bin ambition_game_bin`) detects a
missing display before installing `DefaultPlugins` and falls back to
`run_headless`:

- Linux: if neither `DISPLAY` nor `WAYLAND_DISPLAY` nor `WAYLAND_SOCKET` is set.
- Any platform: if the user passed `--headless` on the CLI.
- Override the tick count with `--headless-ticks N` (default 120).

The dedicated `--bin headless` runner remains the recommended entry point for CI
or scripted runs that do not need the visible-binary fallback behavior.

## Verification

In a no-display development VM:

```bash
cargo test -p ambition_gameplay_core --lib
cargo test -p ambition_app --test scripted_gameplay
cargo run -p ambition_app --bin headless -- 30
```

The last command should print a `HeadlessReport` summary and exit 0. Test counts
change over time; the invariant is exit code 0 with no panic.
