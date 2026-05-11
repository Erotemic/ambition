# Headless simulation

Ambition's sandbox ships two binaries:

- `cargo run -p ambition_sandbox` — the visible Bevy app (windowed gameplay)
- `cargo run -p ambition_sandbox --bin headless [TICKS]` — a no-display
  simulation runner that drives the **full gameplay loop**

Both depend on the `ambition_sandbox` library crate, which owns the module
graph and the cross-cutting resources (`GameWorld`, `SandboxRuntime`). The
visible binary's `main.rs` is the existing playable shell; the headless
binary is a thin shim around `ambition_sandbox::run_headless`.

> **Phase status (2026-05-07):** Phase 1 (no-display tick), Phase 2
> (gameplay loop runs headless), and the **first half of Phase 3** (RL
> adapter API) are all complete. The events refactor (ADR 0012, slices
> `c49c1e5`–`81900dd`) shipped end-to-end; `sandbox_update` emits
> `SfxMessage`/`VfxMessage`/`DebrisBurstMessage` and presentation
> subscribers consume them, so the simulation has no presentation
> coupling. Remaining Phase 3 work: fixed-timestep determinism + RNG
> seeding + a Python binding via PyO3.

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

## Phase 3 — RL adapter (in progress)

The first half of the RL adapter has landed in
`crates/ambition_sandbox/src/rl_sim.rs`. It exposes:

- **`AgentAction`** — sparse per-tick intent struct (move x/y, jump,
  jump_held, dash, attack, blink, interact, projectile, fly_toggle,
  reset, aim x/y, …). All fields default to zero / false; agents can
  set just the knobs they care about. `From<AgentAction> for ControlFrame`
  does the conversion at the seam.
- **`AgentObservation`** — owned `String` / primitive-tuple snapshot of
  player pos/vel/size, on_ground/on_wall/clinging/climbing flags,
  facing, fast_falling/fly/glide flags, dash_charges/air_jumps/blink
  state, hp / hp_max, mana / mana_max, time_alive, resets, body_mode
  label, active_room id, world_size, world_spawn, last_safe_pos, plus
  per-tick flags (`recently_damaged`, `in_hitstun`, `invincible`).
- **`SandboxSim::new()`** — builds the same App `run_headless` does
  (MinimalPlugins + AssetPlugin + ImagePlugin + TransformPlugin +
  StatesPlugin + `init_sandbox_resources` + `add_simulation_plugins`).
  Runs the first tick so the player and `SandboxRuntime` are spawned
  before the caller sees an observation. Returns `Err` on LDtk
  validation failure.
- **`sim.step(action)`** — writes the converted `ControlFrame` into the
  resource and calls `app.update()` once. Returns `AgentObservation`.
- **`sim.step_n(action, n)`** — convenience for "hold this action for
  n frames" without writing the loop.
- **`sim.reset_episode()`** — presses Reset for one frame, idles for
  one, returns the post-reset observation. Goes through the existing
  reset machinery rather than rebuilding the App.
- **`sim.world()` / `sim.world_mut()`** — escape hatches for advanced
  consumers (custom observation extractors, scripted teleports, etc.)
  that want to inspect / mutate ECS state directly.

The whole module is `Send` + thread-local; multi-threaded RL training
should keep one `SandboxSim` per worker.

Remaining Phase 3 work:

- **PyO3 binding**: a thin Python module exposing `SandboxSim` /
  `AgentAction` / `AgentObservation` so research code in Python can
  step the simulation without writing Rust glue. Not required for
  fuzz / scripted-replay use cases (which are happy in pure Rust).
- **`bevy_rl` evaluation**: we may converge `SandboxSim` toward the
  `bevy_rl` adapter shape if it buys us tooling we'd otherwise build
  from scratch.

### Determinism (landed)

`TimestepMode::Fixed { dt }` makes step-level trajectories bit-exact
reproducible. Internally this installs Bevy's
`TimeUpdateStrategy::ManualDuration(dt)` resource before the Startup
tick runs so tick 0 lands at the same state regardless of how long
LDtk validation took on the host. Two sims with the same fixed timestep
+ same action sequence end up with identical (player_pos, player_vel,
hp) at every step — the property the determinism unit test pins.

Gameplay code does not currently consume any RNG (no procedural
generation in the gameplay loop), so RNG seeding is not required for
determinism today. If a future system adds RNG, route it through a
seeded resource that `SandboxSim::new_with_timestep` initializes from a
configurable seed parameter.

### Bug record / replay

`crates/ambition_sandbox/src/bin/trace_replay.rs` reads a
`GameplayTraceBuffer` JSON dump and drives a fresh `SandboxSim` with
the recorded `ControlFrame` sequence at fixed-60Hz timestep. It prints
max-divergence + first-divergence frame between the live sim and the
recorded `player.pos`. Use cases:

- **Bug repro from production**: drop an `ambition_trace_*.json` from
  a player's machine into the repo, run the binary, watch where the
  live sim diverges from the recorded state.
- **Determinism validation**: replay an old trace after a refactor; if
  every frame matches within tolerance, the change preserved
  semantics.
- **CI guardrail (future)**: an in-tree fixture trace can become a
  regression test that pins many gameplay invariants in one shot.

## Visible-binary headless fallback

`run_visible` (the `cargo run -p ambition_sandbox --bin ambition_sandbox`
entry point) detects missing display before installing `DefaultPlugins`
and falls back to `run_headless`:

- Linux: if neither `DISPLAY` nor `WAYLAND_DISPLAY` nor `WAYLAND_SOCKET`
  is set, fall back.
- Any platform: if the user passed `--headless` on the CLI, fall back.
- Override the tick count with `--headless-ticks N` (default 120).

The fallback prints a one-line diagnostic to stderr so users on a
display-less VM see why their `cargo run` didn't open a window. The
dedicated `--bin headless` runner is still the recommended entry point
for CI / RL drivers that want to skip the visible-binary plugin
foundation entirely.

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
