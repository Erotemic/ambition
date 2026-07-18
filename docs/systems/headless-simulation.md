---
status: current
last_verified: 2026-07-18
---

# Headless simulation

Headless execution is a product surface for tests, replay, AI, and future
netcode—not a stripped mock of the visible game.

## Composition

- `ambition_runtime::PlatformerEnginePlugins` installs headless-safe engine and
  schedule composition.
- provider lifecycle registers/prepares/activates the same content used by the
  visible host.
- `ambition_sim_harness` provides programmatic reset/step, typed actions,
  observations, reward, and termination adapters with caller-supplied
  composition.
- `ambition_app` exposes the `headless` and `trace_replay` binaries and app-level
  acceptance tests.
- presentation crates are optional consumers above the simulation/read-model
  seam.

A headless run must not require a window, camera, sprite, audio device, or menu
to advance authoritative state.

## Commands

```bash
cargo run -p ambition_app --bin headless -- 120
cargo run -p ambition_app --bin headless -- 600 --start-room goblin_encounter
cargo run -p ambition_app --bin headless -- 600 --dump-trace target/trace
cargo run -p ambition_app --bin trace_replay -- target/trace/trace.json
```

Use generated navigation for current flags and tests:

```bash
python scripts/agent_query.py tests "headless reset step observation replay"
./run_tests.sh -p ambition_sim_harness
./run_tests.sh -p ambition_app -k plugin_minimal_app
```

## Rules

- Step the real provider/runtime composition.
- Use typed actions and stable observations, not direct mutation of private app
  resources.
- Authoritative AI decisions use deterministic work budgets, not wall-clock
  cutoffs.
- Reset/restore/room replacement use canonical construction/lowering paths.
- Visual-only failures may require visible inspection; missing headless access to
  outcome-changing state is an architecture defect to fix.
