# Headless verification

How we know a change is right without a human watching pixels. This is a load-bearing
capability, not a nicety: a perfect engine is one you can *drive and inspect headless
from any state*. The stance is in [`AGENTS.md`](../../../AGENTS.md); this is the how.

---

## Drive the real sim

The full game runs headless — the real gameplay app with rendering, audio, and
windowing stripped, the actual systems intact:

- **`Platformer2dSimHarness::new_with_options(opts).step(AgentAction)`** — build the real app,
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
  `Platformer2dSimHarness` and assert on resulting state.

> "Can't test it" is almost never true. If the real sim can't be exercised headless
> from some state, **fixing that is the priority**, never building a proxy. (The
> brain-arena with its own kinematics is exactly the proxy to retire.)

## ⭐ Headless DECODES ART now — since 2026-09-02, and it changes what these runs prove

`124684f56` ("The no-window builder finishes its plugins: images decode headless
for the first time"). ⛔ **Before it, `ImagePlugin` registered the image loader in
`Plugin::finish`, which never ran under the `app.update()` loop `--headless`
uses — so every asset stage after "demanded" was measuring an EMPTY POPULATION.**
A headless assertion about residency, decode, insertion or extraction taken
before that commit was true of nothing.

Two consequences for anything written against the old behaviour:

- **A headless run is now a real asset consumer.** Verified on the calculex VM at
  `3f3b42407`: `AMBITION_HEADLESS_GAMEPLAY_ROOM=hall_of_characters` under
  `scripts/headless_room_frame.sh` decodes a full hall population, and a
  `capture_scene` run of the same room reports 235 images / 29.5 MP / 118.1 MB
  resident on its `[image-census]` line. The equivalent run before the fix
  reported none.
- ⚠ **So headless frame numbers across that commit are not comparable**, and
  `performance-and-iteration.md` says so where the affected table lives: every
  phase moved up, not as a regression but because the binary is now doing asset
  work it previously skipped. ⛔ 156 commits separate those two rows; nothing
  should be credited the delta.

⇒ What this ADDS to headless verification is real: residency, ownership and
draw-stage claims are now assertable without a window. What it REMOVES is the
right to quote any pre-`124684f56` headless asset number.

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

This was written as a horizon; it exists. `game/ambition_app_tools/src/bin/capture_scene.rs`
runs the **real presentation plugins**, forces the main camera through the same
`CameraSnapshot2d` policy for an arbitrary focus point, renders into an offscreen
target, and writes that target to a PNG:

```
capture_scene <ROOM_ID> <X,Y|player> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]
capture_scene --route <ROUTE_ID> [OUT.png] [WIDTHxHEIGHT] [OPTIONS]
  --warmup N  --frames N  --stride K  --character ID  --press SEQ
  --include-ui  --dev-overlays  --combat-overlay  --screen-effect E  --fit-room
```

⭐ **it composes through `build_visible_app_with`, the same builder the desktop
binary uses** (2026-08-08). It used to hand-assemble a second app for rooms, and
that copy silently lost five features including the entire room; `--show-window`
went with the fork, having only ever opened a blank window (every camera is
retargeted to the offscreen image).

So an agent CAN spot-check visuals the same way it spot-checks simulation, and
"always draw blind" work should produce an image rather than assert it cannot.

✔ **ROUTES too, since `--route` landed** (the 2026-07-28 finding that the tool
captured rooms only — so the launcher, the startup cards, the versus stage and
its HUD could not be photographed — is closed). `--route smash_gameplay
--character ID` seats a match; `--press` drives keys/touches through the route's
own lobby before the shutter; `--frames N --stride K` photographs a sequence.
✔ **`--press-during N` LANDED 2026-09-02**, so a frame that happens DURING an
input (a page turn's first frame, a menu's opening frame) is capturable. The flag
opens the shutter `N` press-driving frames in instead of after the sequence
completes: `--press Enter --press-during 1` photographs the frame Enter is still
held on, a tap's press and release being two frames by design. With `--frames N`
the stride is counted in press-driving frames too — the ordinary `stride_left`
countdown returns above the press driver, so a sequence that used it would freeze
the input it is photographing. ⛔ It EXITS 2 rather than falling back to the
ordinary capture when the sequence runs out before frame `N`: a post-press image
written to a path a `--press-during` command line named is a good photograph of
the wrong moment, which is the quiet failure this binary's other guards refuse.
`--press-during` without `--press`, and `--press-during 0`, are refused at parse.
The schedule is pinned by unit tests in the binary (including the arm that the
flag's ABSENCE never opens the shutter mid-sequence, which is the whole
byte-identical promise); the pixels are pinned by `scripts/verify_press_during_capture.sh`,
which is a script and not a `#[test]` because the claim needs two full app boots
and a readback — minutes each on a software rasteriser, a cost the suite should
not carry for a developer tool. MEASURED 2026-09-02 on llvmpipe at 320x180,
`--route ambition_launcher --press Enter`: the two shutters land on opposite
sides of the route change the confirmation starts — `--press-during 1` is the
launcher's "Choose Game" list with Enter still down (26446 bytes, and the tool
prints `NO SUBJECT` because a launcher has no body), the ordinary one is the game
it reached (28776 bytes, a subject and its HUD). Distinct hashes, and distinct
for the right reason rather than by a pixel of noise.
⚠ On a machine with no GPU pair it with `AMBITION_QUALITY_PROFILE=ultra`; the
tool's `--help` says why (Potato scales screen shaders and the parallax to
nothing).

The sibling capture is `ambition_platformer2d_actor_monolith/examples/render_room_geometry.rs capture`
(geometry only, no render stack).

## ⭐ A MACHINE WITH NO GPU CAN RUN THIS — measured 2026-09-03

`capture_scene` runs on the calculex host, which has no `/dev/dri`, no display
and no `nvidia-smi`. Mesa's **lavapipe** presents a working Vulkan device
(`llvmpipe (LLVM 20.1.2, 256 bits)`, `PHYSICAL_DEVICE_TYPE_CPU`), and the tool
renders offscreen, so no window and no `Xvfb` is needed.

The default plan's own render acceptance —
`capture_scene central_hub_complex player … 320x180 --warmup 20` — produced a
correct 320×180 frame: room geometry, doors and their labels, the parallax
background, the driven character, and the HUD line *"Drop through the floor
opening to reach the stitched basement"*.

⭐ **AND THE GPU STAGES OF THE IMAGE LEDGER ARE GENUINELY EXERCISED**, which the
headless room runs cannot do because they compose no render app:

```text
gpu +117 (+21.4MP)  insert→gpu p50 151ms max 220ms | awaiting gpu 0
never drawn 106 (20.8MP) | re-decodes 0 | dropped before gpu 0
```

⇒ So *"an agent CAN spot-check visuals"* is true on a GPU-less host too, and
"always draw blind" work has no excuse here either. It also means the ledger's
stage 3 (GPU) and stage 4 (first draw) — invisible to
`scripts/headless_room_frame.sh`, whose `[census] render_pass_summary` reports
`cpu_spans=0 gpu_spans=0` — have a road on this machine.

⛔⛔ **NEVER QUOTE ITS TIMINGS AS HARDWARE NUMBERS.** `insert→gpu p50 151ms` is a
SOFTWARE rasterizer moving bytes with the CPU; a real adapter is orders of
magnitude away. This arm answers *"does it render, and is the picture right"*,
never *"how fast does it render"*. A row that mixes an llvmpipe millisecond into
a hardware budget is worse than no row.
⚠ And the picture being right is a claim about THIS composition at THIS size —
320×180 with 20 warmup frames. It is not a substitute for looking at the game.

## When a headless app dies naming nothing

⛔⛔ **`Parameter <Enable the debug feature to see the name> failed validation:
Resource does not exist` NAMES NEITHER THE SYSTEM NOR THE PARAMETER**, and a
headless composition that pulls in presentation or debug-viz systems is exactly
where it fires — their parameters are render-stack resources that
`add_headless_foundation` does not supply.

⇒ Re-run under `RUST_BACKTRACE=1` and read the `run_unsafe<fn(..)>` frame. The
whole parameter list is spelled out in that type, and it is the only place it
appears. Without it the message is unactionable, which is how such a failure sits
open: 2026-09-02 it hid three different systems in succession (a missing
`Assets<TextureAtlasLayout>`, then `GizmoConfigStore`, then `Assets<Mesh>`), each
looking identical to the last.

⚠ And the fix is usually NOT to register the resource. A gizmo or mesh system
with no render stack should be `run_if(resource_exists::<..>)`-guarded so it
skips — `avatar::trail.rs` is the pattern — because registering render assets
one at a time into a headless app is fitting the app to the test. See B3 in
[`project-build-and-distribution.md`](project-build-and-distribution.md).

## Pointers

- **`crates/ambition_sim_harness/`** owns the reusable headless surface:
  `runtime.rs` (`Platformer2dSimHarness`), `action.rs`, `observation.rs`, `options.rs`,
  `reward.rs`, `random_policy.rs`. The old `ambition_app/src/rl_sim/runtime.rs`
  is gone; `game/ambition_app/src/rl_sim/mod.rs` survives as the thin Ambition
  BINDING — it re-exports the harness and supplies the one product-specific
  piece, the composition that installs Ambition content +
  `AmbitionGameSimulationPlugin` onto the harness App. A demo or test with different
  content calls `ambition_sim_harness::Platformer2dSimHarness::build` with its own
  composition and never links the app crate.
- `game/ambition_app/src/bin/` for the driver binaries.
- `game/ambition_app/tests/app_it.rs` for the build → step → assert pattern.
- `ambition_gameplay_trace/` (trace buffer + dump), the `actor_trace` OOB recorder.
