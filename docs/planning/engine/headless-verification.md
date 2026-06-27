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
- **Binaries** — `headless` (fixed-tick run + trace dump), `trace_replay` (replay a
  recorded trace, detect determinism divergence), `rl_*` (policy-driven fuzzing).
- **Integration tests** — `ambition_app/tests/*` (`dash_stability`,
  `blink_run_reachability`, `scripted_gameplay`, `collision_invariant_oracle`) drive
  `SandboxSim` and assert on resulting state. ~1 min to build, sub-second to run.

> "Can't test it" is almost never true. The only thing you may be unsure of is
> subjective **visual feel** — and even that is headed for headless render-to-disk
> (state → image, for spot-checks). If the real sim can't be exercised headless from
> some state, **fixing that is the priority**, never building a proxy. (The
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

## The horizon

Headless **render-to-disk**: given a game state, render what it looks like to an
image file, so an agent can spot-check visuals the same way it spot-checks
simulation. When that lands, the last "I can't verify" — visuals — closes too.

## Pointers

- `ambition_app/src/rl_sim/runtime.rs` (`SandboxSim`, `AgentAction`,
  `AgentObservation`), `src/bin/{headless,trace_replay,rl_*}.rs`.
- `ambition_app/tests/*` for the build → step → assert pattern.
- `ambition_gameplay_trace/` (trace buffer + dump), the `actor_trace` OOB recorder.
