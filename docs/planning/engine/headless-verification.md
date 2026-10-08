# Headless verification

How we know a change is right without a human watching pixels. A good engine
can be driven and inspected headless from any state. The stance is in
[`AGENTS.md`](../../../AGENTS.md). This page is the how.

## Drive the real sim

The full game runs headless: the real gameplay app with rendering, audio and
windowing removed, and the actual systems intact.

- **`Platformer2dSimHarness::new_with_options(opts).step(AgentAction)`** builds
  the real app, steps one frame with an input, and returns an
  `AgentObservation`. Set state (teleport, grant ability, spawn, inject
  geometry), step N frames, and assert on the result.
- **Binaries** are in `game/ambition_app_tools/src/bin/` (`-p
  ambition_app_tools`, which `./run_headless.sh` invokes): `headless`
  (fixed-tick run and trace dump), `trace_replay` (replay a trace and detect
  divergence), `rl_random_walker` and `rl_smoke` (policy fuzzing),
  `capture_scene` (state to PNG), `room_census` and focused probes.
  `game/ambition_app/src/bin/` holds only the game binary.
- **Integration tests** are one aggregated target, `app_it`
  (`game/ambition_app/tests/app_it.rs`, `autotests = false`). The sibling
  `.rs` files are its modules. Run one module with
  `cargo test -p ambition_app --test app_it -- <module_name>`.
- **An unhealthy rollback session is refused, not stepped** (Q138).
  `step` and `step_frame` panic with the session's error; `try_step` returns
  it. An arm whose subject is behaviour over a diverged session says so with
  `step_over_an_unhealthy_session`
  (`the_harness_refuses_to_step_an_unhealthy_session`).

"Can't test it" is almost never true. If the real sim cannot be exercised
headless from some state, fix that first. Do not build a proxy.

Headless runs decode art. A headless run is a real asset consumer, so
residency, ownership and draw-stage claims can be asserted without a window.

## Test invariants, not tuned values

The strongest tests are symmetry and covariance under the relativity
principle: an action behaves identically under C4 gravity rotation and through
portals. These tests stay valid across feel tweaks. Also test: no
out-of-bounds, wedge or NaN; determinism (same inputs, same trace); feature
composition (two systems compose without a special case).

Do not write new regression tests that pin unpolished behavior or magic
numbers.

## Canaries, not cages

Bit-identical and replay tests flag a change you expected to be
behavior-neutral that was not. Expect them to fail over time as elegance
changes behavior. When the diff is not egregious, re-baseline the target. A
failing canary is information, not a wall.

## The differential net for feel-touching refactors

For a structural cut that may shift movement or combat feel, use the trace
tooling:

- `ambition_gameplay_trace`: the per-frame feel-trace ring buffer and its
  markdown/JSON dump.
- `actor_trace`: the out-of-bounds flight recorder over every body's
  kinematics.

Capture a trace before the cut and diff after. Commit each slice as a
checkpoint. Jon verifies subjective feel in-game. Ship a feel-sensitive change
in its own marked commit and ask.

## Render to disk

`game/ambition_app_tools/src/bin/capture_scene.rs` runs the real presentation
plugins, forces the main camera through the `CameraSnapshot2d` policy for a
focus point, renders offscreen, and writes a PNG. It composes through
`build_visible_app_with`, the same builder the desktop binary uses.

```text
capture_scene <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]
capture_scene --route <ROUTE_ID> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]
  --warmup N  --frames N  --stride K  --character ID  --press SEQ
  --press-during N  --include-ui  --dev-overlays  --combat-overlay
  --screen-effect E  --fit-room
```

- `--route` photographs routes (launcher, startup cards, versus stage).
  `--press` drives keys and touches through the route's own lobby.
- `--press-during N` opens the shutter N press-driving frames into the
  sequence. It exits 2 when the sequence ends before frame N, and it is
  refused without `--press` or with 0. `scripts/verify_press_during_capture.sh`
  checks the pixels.
- On a machine with no GPU, set `AMBITION_QUALITY_PROFILE=ultra`. The Potato
  profile scales screen shaders and parallax to nothing.

The geometry-only sibling is
`ambition_platformer2d_actor_monolith/examples/render_room_geometry.rs capture`.

So an agent can spot-check visuals. Work that "always draws blind" should
produce an image.

### A machine with no GPU

`capture_scene` runs on a host with no `/dev/dri` and no display. Mesa's
lavapipe presents a CPU Vulkan device (`llvmpipe`), and the tool renders
offscreen, so no window or `Xvfb` is needed. This run exercises the GPU and
first-draw stages of the image ledger, which `scripts/headless_room_frame.sh`
cannot reach.

⛔ Never quote llvmpipe timings as hardware numbers. A software rasterizer
answers "does it render, and is the picture right", never "how fast". A correct
picture at one size and warmup is not a substitute for looking at the game.

## When a headless app dies naming nothing

`Parameter <Enable the debug feature to see the name> failed validation:
Resource does not exist` names neither the system nor the parameter. It fires
when a headless composition pulls in presentation or debug systems whose
parameters are render-stack resources that `add_headless_foundation` does not
supply.

- Re-run under `RUST_BACKTRACE=1` and read the `run_unsafe<fn(..)>` frame. The
  parameter list is spelled out in that type.
- Usually, do not register the resource. Guard the system with
  `run_if(resource_exists::<..>)` so it skips (`avatar::trail.rs` is the
  pattern). See B3 in
  [`project-build-and-distribution.md`](project-build-and-distribution.md).
- A prerequisite check names the resource the system demands, never a proxy
  for its subsystem. A headless demo has an `AssetPlugin` and no render stack,
  so an asset-registry guard is true in exactly the composition that fails.
- Read the whole signature once and guard every parameter that can fail
  validation. This class arrives with each new render-adjacent crate.

## Pointers

- `crates/ambition_sim_harness/` owns the headless surface: `runtime.rs`
  (`Platformer2dSimHarness`), `action.rs`, `observation.rs`, `options.rs`,
  `reward.rs`, `random_policy.rs`, `capture.rs`.
- `game/ambition_app/src/rl_sim/mod.rs` is the thin Ambition binding. It
  re-exports the harness and installs Ambition content and
  `AmbitionGameSimulationPlugin`. A demo with other content calls
  `Platformer2dSimHarness::build` with its own composition and never links the
  app crate.
- `game/ambition_app/tests/app_it.rs` shows the build, step, assert pattern.
- `ambition_gameplay_trace/` and the `actor_trace` recorder.

## Architecture acceptance must exercise the advertised profile

[Packet A9](actor-monolith-work-frontier.md) adds independent profile evidence.
A headless fixture that inherits the whole flagship dependency closure does not
prove render-optional composition. A manifest that excludes a crate does not
prove the remaining capabilities construct and step a real subject. Record both
the Cargo closure and a behavior witness for each supported profile.

Use a small external consumer plus the full Ambition and Smash fixtures.
Required negative fixtures: absent render/audio, absent items with checkpoint
restoration, invalid technique key or parameters, out-of-range flow, failed
publication, and duplicate registration. Feature absence removes the
requirement. It does not supply a dummy resource that makes the test pass.

Targeted runtime tests, source boundary guards and public API compile tests
answer different questions. A zero-test filter is not a behavioral pass.
Missing Cargo, targets, assets or GPU prerequisites produce an incomplete
receipt with the exact command and reason. They do not become a successful
lane.

## Independent iteration evidence

Use [FI1-FI10](fast-iteration-acceptance.md) for the selected extension
packet. A behavior witness steps the actual reducer, materializer or GGRS
restoration it claims to test. A metadata declaration, source-string match,
empty App or codec roundtrip cannot replace that evidence. First prove the
fixture reaches the intended branch. Then mutate one obligation and require
the correct assertion to fail.

Keep lightweight schema and compiler tests separate from an assembled host
witness. Add cases to existing shared integration targets. A content-only edit
runs its admission and trace lane. A host or registry change also exercises
that assembly boundary. This does not require the full suite after each edit.
