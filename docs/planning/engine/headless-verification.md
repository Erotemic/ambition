# Headless verification

How we know a change is right without a human watching pixels. This is a load-bearing
capability, not a nicety: a perfect engine is one you can *drive and inspect headless
from any state*. The stance is in [`AGENTS.md`](../../../AGENTS.md); this is the how.

---

## Drive the real sim

The full game runs headless — the real gameplay app with rendering, audio, and
windowing stripped, the actual systems intact:

- **`SandboxSim::new_with_options(opts).step(AgentAction)`** — build the real app,
  step it one frame with an input, read an `AgentObservation` back. Set state
  (teleport, grant ability, spawn, inject geometry), step N frames, assert on the
  result. This is the substrate.
- **Binaries** (`game/ambition_app/src/bin/`) — `headless` (fixed-tick run + trace
  dump), `trace_replay` (replay a recorded trace, detect determinism divergence),
  `rl_random_walker` / `rl_smoke` (policy-driven fuzzing), `capture_scene`
  (state → PNG; see "Render-to-disk" below).
- **Integration tests** — ONE aggregated target, `app_it`
  (`game/ambition_app/tests/app_it.rs`, with `autotests = false`); the ~50 sibling
  `.rs` files are its MODULES, not separate targets. Run a single module with
  `cargo test -p ambition_app --test app_it -- <module_name>`. They drive
  `SandboxSim` and assert on resulting state.

> "Can't test it" is almost never true. If the real sim can't be exercised headless
> from some state, **fixing that is the priority**, never building a proxy. (The
> brain-arena with its own kinematics is exactly the proxy to retire.)

## Test invariants, not tuned values

The strongest tests are **symmetry / covariance under the relativity principle** — an
action behaving identically under C4 gravity rotation and through portals — because
they stay valid across feel tweaks. They are covariant with the design, not pinned to
a number. Also test: no out-of-bounds / wedge / NaN; determinism (same inputs → same
trace); feature composition (two systems compose without a special case).

Do **not** write new regression tests to pin unpolished behavior or magic numbers.
That is the over-preservation tax we're paying down, not adding to.

## Canaries, not cages

Bit-identical / replay tests have one job: flag when a change you *expected* to be
behavior-neutral actually wasn't — a smell worth a look. **Expect them to fail over
time** as elegance changes behavior; when the diff isn't egregious, re-baseline the
target (script the update if it's tedious). A failing canary is information, not a
wall.

## The differential net for feel-touching refactors

For a structural cut that may shift movement/combat feel (the keystone collapse, the
player-pipeline route), the net is the trace tooling:
- `ambition_gameplay_trace` — the per-frame feel-trace ring buffer + markdown/JSON
  dump.
- the out-of-bounds flight recorder (`actor_trace`) — one query over every body's
  kinematics, non-player-centric.

Capture a trace before the cut, diff after. Replay/feel may change — only *it
compiles* + the feel diff gate it. Commit each slice as a checkpoint, keep moving.
Jon verifies subjective feel in-game; ship a feel-sensitive change blind in its own
marked commit and ask — round-trips are expensive, reverts are cheap.

## Render-to-disk — LANDED (corrected 2026-07-19)

This was written as a horizon; it exists. `game/ambition_app/src/bin/capture_scene.rs`
runs the **real presentation plugins**, forces the main camera through the same
`CameraSnapshot2d` policy for an arbitrary focus point, renders into an offscreen
target, and writes that target to a PNG:

```
cargo run -p ambition_app --bin capture_scene -- <ROOM_ID> <X,Y|player> [OUT.png] \
    [WIDTHxHEIGHT] [--warmup N] [--character ID] [--include-ui] [--show-window]
```

So an agent CAN spot-check visuals the same way it spot-checks simulation, and
"always draw blind" work should produce an image rather than assert it cannot.
The sibling capture is `ambition_actors/examples/render_room_geometry.rs capture`
(geometry only, no render stack).

## Pointers

- **`crates/ambition_sim_harness/`** owns the reusable headless surface:
  `runtime.rs` (`SandboxSim`), `action.rs`, `observation.rs`, `options.rs`,
  `reward.rs`, `random_policy.rs`. The old `ambition_app/src/rl_sim/runtime.rs`
  is gone; `game/ambition_app/src/rl_sim/mod.rs` survives as the thin Ambition
  BINDING — it re-exports the harness and supplies the one product-specific
  piece, the composition that installs Ambition content +
  `SandboxSimulationPlugin` onto the harness App. A demo or test with different
  content calls `ambition_sim_harness::SandboxSim::build` with its own
  composition and never links the app crate.
- `game/ambition_app/src/bin/` for the driver binaries.
- `game/ambition_app/tests/app_it.rs` for the build → step → assert pattern.
- `ambition_gameplay_trace/` (trace buffer + dump), the `actor_trace` OOB recorder.
