---
status: current
last_verified: 2026-11-10
---

# Headless simulation

Headless Ambition runs authoritative simulation without a window. It is useful
for gameplay investigation, deterministic replay, and machine-driven agents.
It is not a separately maintained mock of the game.

## Owners

- `ambition_sim_harness` supplies reset, step, actions, observations, rewards,
  and termination adapters over caller-selected composition.
- `ambition_platformer2d_runtime` provides engine and scheduling composition.
- `ambition_app_tools/src/bin/headless.rs` runs the Ambition sandbox in the
  same provider/runtime model as the visible host.
- `ambition_app_tools/src/bin/trace_replay.rs` reads a recorded trace and
  replays its input frames.
- `ambition_gameplay_trace` owns the dump format; the simulation recorder
  emits the frames and events.

A headless check should exercise the same action and body authority as the
visible game. Presentation may consume the resulting state, but cannot be
required for simulation to advance.

## Run a room

```bash
cargo run -p ambition_app_tools --bin headless -- 120
cargo run -p ambition_app_tools --bin headless -- 600 --start-room goblin_encounter
```

For a trace and replay:

```bash
cargo run -p ambition_app_tools --bin headless -- 600 --dump-trace target/trace
TRACE=$(find target/trace -maxdepth 1 -name 'ambition_gameplay_trace_*.json' -type f | sort | tail -n 1)
cargo run -p ambition_app_tools --bin trace_replay -- "$TRACE"
```

The dump command writes a timestamped `ambition_gameplay_trace_*.json`, **not**
a fixed `trace.json`. It drives *idle input* during the recorded run. To
exercise jumps, attacks, or controlled trajectories, drive the harness with
explicit actions instead. `--bench` is available for measuring the selected
headless run; use it when performance is the question.

## Useful boundaries

- The provider prepares the same authored content used in visible gameplay.
- Deterministic decisions use simulation inputs and work budgets, rather than
  wall-clock timeouts.
- Restores and room replacement use the owning construction and restore APIs.
- For collision geometry, prefer the registered read-only
  `ambition_geometry_observation` providers over a separately approximated map.
- Check a visible rendering problem with a rendering capture rather than
  trying to infer pixels from headless state.

Run the smallest relevant scenario for the change; a broad integration suite
is not a prerequisite for using the headless driver.
