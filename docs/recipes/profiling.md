# Profiling Ambition

## Which instrument answers which question

These tools are not interchangeable, and reaching for the wrong one wastes
hours. Pick by the question you are actually asking:

**Start here.** `./scripts/profile_desktop.sh`, play, quit. It builds the
optimized `profiling` profile with tracing on, captures perf + Tracy + the
in-process workload censuses for the whole session, and writes one bundle whose
`summary.md` answers most of the table below at once. Reach past it only when
that summary points somewhere specific.

| Question | Instrument | Reads as |
|---|---|---|
| Anything, first pass | `scripts/profile_desktop.sh` → `summary.md` | text, agent-readable |
| Which machine layer owns the CPU — game code, GPU driver, or a software rasterizer? | same bundle, "Where the native time went" | text, agent-readable |
| Where did startup time go? | `[startup]` phase logger (always on) | text, agent-readable |
| Which native function is hot, and during which part of my session? | same bundle, `timeline.md` + `perf_windows/` | text, agent-readable |
| Which *Bevy system* is hot? | same bundle, `tracy_summary.md` | text, agent-readable |
| Which *phase of the frame* owns the time — with no profiler, on any platform? | `[census] phases` → `schedule_phases.csv` | CSV, agent-readable |
| Did the profiler cost more than the game? | same bundle, "Observer effect" | text, agent-readable |
| Which *render pass* is hot, on CPU and GPU? | same bundle, `render_diagnostics.csv` | CSV, agent-readable |
| How many cameras/views/portal captures were live? | same bundle, `camera_views.csv` / `portal_activity.csv` | CSV, agent-readable |
| How big did the scene get? | same bundle, `runtime_census.csv` / `draw_census.csv` | CSV, agent-readable |
| Which frames stuttered, and when? | `[frame-spike]` / `[frame-census]` (always on) → `frame_spikes.csv` | text, agent-readable |
| Which textures decoded, how big, and when? | `[image]` / `[image-census]` (always on) → `image_decodes.csv` | text, agent-readable |
| Did a sprite get re-bound at a different size? | `[sprite-bind]` (always on) | text, agent-readable |
| Am I re-reading assets from disk every frame? | `profile_desktop.sh asset-run` | text, agent-readable |
| Is this CPU-bound, memory-bound, or stalled? | `profile_desktop.sh stat-run` | text, agent-readable |
| What can I measure on a VM with no GPU? | `profile_desktop.sh --headless` | text, agent-readable |
| Give me a flame graph picture | `cargo flamegraph` | SVG, browser |

Two properties matter more than they look:

**`perf` sees native symbols, not engine concepts.** It reports
`<ambition_render::foo>` and `libvulkan_lvp.so`. It cannot tell you "the
bloom pass costs 4ms" or "system `sync_camera` costs 0.3ms", because those
are Bevy-level concepts with no one-to-one symbol. Worse, when rendering
falls back to a CPU rasterizer, most cycles land in JIT-compiled shader code
that has no symbols at all and never will — `perf` can prove *that*
rasterization dominates and nothing about *what* is being rasterized.

**Tracy answers the Bevy-level questions.** Its GUI needs a human, but it
also ships two CLI tools that do not: `tracy-capture` records a trace from a
running game headlessly, and `tracy-csvexport` turns that trace into a table
of zone statistics — per-Bevy-system timings an agent can rank.

### Getting the tools

`./run_developer_setup.sh --profile` installs the whole profiling toolchain:
`perf` and `strace` (which `profile_desktop.sh` requires), `vulkaninfo`/`glxinfo`
(which its host-environment report reads), `cargo-flamegraph`, `hotspot`,
`heaptrack`, and Tracy built from source into `~/.local/bin` — plus the cargo
analysis tools (`llvm-cov`, `modules`, `sweep`, `mark-sweep`, `nextest`).

⚠ **It is opt-in, and it used to be the default.** A bare setup is the fast path
to a running game and installs none of this: `hotspot` alone pulls the KDE
Frameworks stack (~190 apt packages between them), and every cargo tool above is
a source build. Nothing here is needed to run or test the game — `run_tests.py`
falls back to plain `cargo test` when nextest is absent — so it is not on the
zero-to-runnable path. Pass `--profile` when you intend to profile, or `--full`
to add the sampled instrument libraries as well.

Tracy is built rather than installed because Ubuntu does not package it, and
it is pinned to the version read out of the `tracy-client-sys` crate the game
actually links. That pin is not cosmetic: Tracy speaks a versioned wire
protocol and a mismatched server does not warn — it silently refuses to
connect. The headless `capture`/`csvexport` tools build from
`build-essential` + `cmake` alone (~25s each); the GUI needs a desktop's worth
of libraries and is attempted only when they are present, so its absence never
costs you the CLI tools.

Two layers of in-process instrumentation are wired in:

1. **Lightweight startup phase logger** (always on, zero deps).
2. **Bevy + Tracy per-system profiling** (gated behind `--features profile`).

Use #1 to answer "where did startup go" without any tooling. Use #2
when a regression slips in or a frame spends time in places #1 can't
see.

## 1. Startup phase logger

The `StartupProfiler` resource records `Instant` snapshots at named
phase boundaries during the `Startup` schedule. The
`report_startup_phases` system runs once on the first `PostStartup`
tick and prints the per-phase deltas + total to stderr:

```text
[startup] → after_load_data_handle: +0.4ms
[startup] → after_setup_simulation: +312.7ms
[startup] total before first frame: 412.5ms
```

Phase marks are inserted between Startup-chained systems via
`profiling::phase_mark("name")`. The defaults today bracket
`load_data_asset_handle` and `setup_simulation_system`. Add more by
chaining `phase_mark(...)` between Startup systems in
[game/ambition_app/src/app/plugins.rs](../../game/ambition_app/src/app/plugins.rs):

```rust
.add_systems(Startup, (
    profiling::phase_mark("startup_begin"),
    load_thing,
    profiling::phase_mark("after_load_thing"),
    setup_thing,
    profiling::phase_mark("after_setup_thing"),
).chain())
```

Code lives in
[crates/ambition_dev_tools/src/profiling.rs](../../crates/ambition_dev_tools/src/profiling.rs)
(re-exported on the historical `ambition_platformer2d_actor_monolith::dev::profiling` path).

## 1b. Steady-state censuses

The startup logger above stops at the first frame. These four cover what
happens after it, on stderr, so `profile_desktop.sh` stamps them into the
timeline chunk they occurred in:

```text
[frame-spike]    0.512s   184.3ms
[frame-census]   5.000s-10.000s frames=300 p50=16.7ms p95=18.1ms p99=24.0ms max=41.2ms
[image]          0.412s 4150x4046   16.8MP sprites/gnu_ton_boss/gnu_ton_boss_spritesheet.png
[image-census]   1.000s +38 images (+212.4MP) | total 38 images, 212.4MP, 849.6MB resident
[sprite-bind] worn character 'player' collision=30x48 render=64x64 (seed: default body constant)
```

Why each exists:

- **`[frame-spike]`** gives a stutter a *timestamp*, which is what lets you
  line it up against a perf chunk. Frames slower than 33.4ms, capped at 60
  lines so a bad run cannot make logging the slow thing.
- **`[frame-census]`** answers "is it smooth now" from a log rather than from
  a number on screen, which the FPS overlay cannot do for an agent.
- **`[image]` / `[image-census]`** name the *asset*. A native profile can
  prove you are inside `png::filter::paeth::unfilter` and can never say which
  sheet that was — this is the instrument for "are we loading everything at
  startup instead of scoping it to the room".
- **`[sprite-bind]`** records every character-sprite bind with its collision
  and render size. Two lines with different sizes are a visible mid-launch
  resize, and the `seed:` field says which of the two bind sites produced it.

Code: [`ambition_dev_tools::profiling`](../../crates/ambition_dev_tools/src/profiling.rs)
and [`ambition_render::asset_census`](../../crates/ambition_render/src/asset_census.rs).

## 1c. Profiling-only workload censuses

The four above are always on and cheap. A second set answers the question a
native profile structurally cannot — *what did Ambition ask Bevy to do* — and
runs only when `AMBITION_PROFILE_CENSUS` is set, which
`scripts/profile_desktop.sh` does for you:

```text
[census] frame          t=12.000 frames=60 mean=16.71 p50=16.70 p95=18.20 p99=24.00 min=15.90 max=41.20
[census] ecs            t=12.000 entities=8123 archetypes=412 components=1904 bodies=7 players=1
[census] schedules      t=12.000 schedules=14 systems=1183
[census] views          t=12.000 cameras=5 active=5 world_rendering=3 offscreen=2 local_views=1
[census] camera         t=12.000 entity=64v1 role=main_gameplay active=true target=primary_window size=1920x1080 viewport=full order=0 layers=0 presents_view= name="Main Camera"
[census] draws          t=12.000 sprites=2140 sprites_visible=311 text2d=18 per_view_projections=0
[census] render_targets t=12.000 image_targets=2 cpu_bytes=0 largest_dim=512 images_resident=214
[census] portal         t=12.000 rigs=2 active=2 max_resolution=1024 recursion_depth=1 max_active_captures=2 max_updates_per_frame=2 min_refresh_interval_s=0.000 include_parallax=true
[census] render_pass    t=12.000 path=render/main_opaque_pass_2d/elapsed_cpu value=1.204000 avg=1.180000 suffix=ms
[census] assets         t=12.000 decoded_images=214 decoded_megapixels=612.4 decoded_bytes=2449600000 images_resident=214
[census] phases         t=12.000 frames=60 outside=6.50 first=0.04 pre=0.31 state=0.09 fixed=0.88 update=8.10 post=1.20 last=0.40
```

**`[census] phases` is the one frame breakdown that needs no profiler.** Tracy
answers it too, but Tracy is a desktop build whose symbol worker can cost more
than the game (see "Observer effect" in any bundle), and it does not exist on
web, Android, or a Steam Deck in someone else's hands. Six `Instant::now()`
calls per frame put "which phase owns the frame" in reach of any build that can
write to stderr, and `profile_desktop.sh` turns the rows into
`schedule_phases.csv`.

Values are **milliseconds per frame**, averaged over the window, not totals —
a total moves with the frame rate it is trying to explain. `outside` is
everything between the end of `Last` and the next `First`: present/vsync wait
in a windowed run, the runner loop when headless. Attribution is by
TRANSITION — one system at the head of each phase closes the previous one — so
a schedule with no mark of its own is charged to the phase before it. That
makes the row a breakdown of the FRAME, and the parts sum to the frame time.

The marks are not registered at all when `AMBITION_PROFILE_CENSUS` is unset:
eight systems in seven schedules would otherwise join the population they
exist to measure.

Every row in a frame carries the same `t=` because one clock decides which
frame is a sample frame — that is what makes a camera count and a render-pass
time from the same instant joinable. Cadence is 1 Hz
(`AMBITION_PROFILE_CENSUS_HZ`); no census iterates a per-entity population on a
frame that is not a sample frame, and with the variable unset each is a single
bool test.

Measured cost, headless sandbox, 6000 ticks, three interleaved runs each
(2026-08-28): 121.60e9 retired instructions with the census off against
121.51e9 with it on — the enabled run is *lower* than the disabled one, so the
difference is under the ~0.06% run-to-run spread and no overhead is
attributable. Wall-clock on this VM was useless for the comparison: run-to-run
variance from other load was several times any plausible signal, which is why
the number above is instructions retired over a fixed tick count and not
seconds. The one term that grows with the scene is the sprite pass in
`report_draw_census`: one iteration over the sprite population per SAMPLE, so
1 Hz, not per frame.

Camera roles come from the markers the spawner already set (`MainCamera`,
`FrontHudCamera`, `PortalViewRig`, `PresentsView`, an image render target), not
from inference — a camera nobody marked reports `role=other` and its `Name`,
which is the honest answer.

Code: [`ambition_dev_tools::runtime_census`](../../crates/ambition_dev_tools/src/runtime_census.rs)
(sim side, runs headless) and
[`ambition_render::runtime_census`](../../crates/ambition_render/src/runtime_census.rs)
(cameras, targets, portals, render passes).

## 2a. cargo flamegraph (no-GUI flame graph SVG)

For a "give me a flame graph as a file" workflow that doesn't need
the Tracy GUI installed, use [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph).
It wraps Linux `perf` and writes an interactive SVG you open in a
browser.

### One-time setup

```bash
cargo install flamegraph
# Linux: perf needs kernel.perf_event_paranoid <= 2 (or sudo)
sudo sysctl kernel.perf_event_paranoid=1
# Optional, restart-persistent: echo "kernel.perf_event_paranoid=1" | sudo tee /etc/sysctl.d/local-perf.conf
```

Add this to `crates/ambition_platformer2d_actor_monolith/Cargo.toml` for symbol-rich
release builds (already there if you've enabled it elsewhere; safe
to keep on for normal `cargo run --release`):

```toml
[profile.release]
debug = true  # keep DWARF for unmangled flamegraph frames
```

### Capture a startup flame graph

```bash
# Build first so capture only times the run, not compilation.
cargo build --release -p ambition_app --bin ambition_game_bin

# BEVY_ASSET_ROOT is required: cargo-flamegraph runs the binary
# directly (not via `cargo run`), so Bevy looks for assets relative
# to the binary path (`target/release/assets/`) instead of the
# package's `crates/ambition_platformer2d_actor_monolith/assets/`. Without this var, you
# get `Path not found: target/release/assets/...` for every asset
# and bevy_yarnspinner panics on the missing dialogue/ folder.
BEVY_ASSET_ROOT=$PWD/crates/ambition_platformer2d_actor_monolith \
cargo flamegraph -p ambition_app --bin ambition_game_bin \
    --release \
    --output flamegraph_startup.svg \
    -- --start-room=central_hub_complex
# Close the game window after a few seconds to stop sampling.
```

Open `flamegraph_startup.svg` in a browser. Width = CPU time spent
in that frame; click to zoom. Search box (top right) jumps to a
function name.

### Capture a single problem area

If you already know roughly where the time goes (per phase logger),
add a sleep at the end of the suspect block, capture, then remove:

```bash
# Useful for "I want a flamegraph that's only the post-Startup
# room-load tick" — make Startup short, hit a known idle frame.
```

For per-frame regressions during play, just run the game normally
under `cargo flamegraph` and keep playing for ~30 seconds.

## 2b. Bevy + Tracy per-system profiling

Bevy ships with built-in tracing instrumentation. Enabling
`--features profile` flips on `bevy/trace` and `bevy/trace_tracy`,
which streams per-system spans to a [Tracy](https://github.com/wolfpld/tracy)
GUI listener.

### Build + run

```bash
cargo run -p ambition_app --bin ambition_game_bin --features profile
```

The binary will block on startup until Tracy connects (or proceed
without if the GUI isn't running — your build, your call).

### Collect a profile

Use `scripts/profile_desktop.sh`. It starts `tracy-capture` before the game and
finalizes the trace when the game exits, then exports
`tracy_zones.csv` + `tracy_summary.md` — a ranked table of zones by total time,
count, mean, and max. No GUI, no connect button, no timing on your part.

The GUI is still there if you want it: open `tracy.trace` from the bundle in
`tracy-profiler`. Note that a live GUI capture must be connected before the
game starts to see T=0, which is exactly the coordination the script removes.

### What's captured

- Every Bevy system's per-tick CPU time, automatically.
- Custom `info_span!("...")` blocks added in code.
- Per-render-pass CPU time, and GPU time where the adapter supports timestamp
  queries: `bevy/trace_tracy` turns on `bevy_render/tracing-tracy`, which makes
  `RenderPlugin` install `RenderDiagnosticsPlugin` itself. Without that feature
  the presentation census installs the same plugin when
  `AMBITION_PROFILE_CENSUS` is set, so `--no-tracy` runs still get per-pass
  rows in `render_diagnostics.csv`.

### What's NOT captured

- Anything in non-Bevy threads unless instrumented manually with
  `tracing` macros.
- Allocation profiling (use `dhat` / `heaptrack` separately).

### Cost

Tracy adds ~5-10% CPU overhead and grows the binary by ~3 MB. Both
are negligible during dev. Default builds drop the dep entirely
since `profile` is opt-in.

### The one-command workflow

```bash
./scripts/profile_desktop.sh
```

Play normally, reproduce the slowdown, quit the game normally. There is no
Tracy window to open, no capture button, and no moment you have to hit. The
script prints one output directory; read the `summary.md` in it.

What it does, in order:

1. asks `run_game.sh --print-plan` which executable this invocation resolves
   to — it never guesses a path;
2. builds it with the optimized `profiling` cargo profile and
   `--features profile`;
3. starts `tracy-capture` (headless, no GUI) if the tools are installed and
   that binary carries the Tracy client;
4. launches the game under `perf record`, with `AMBITION_PROFILE_CENSUS=1` so
   the in-process workload censuses run at 1 Hz;
5. stamps every line the game logs with seconds since launch;
6. when the game exits — normally, by Ctrl-C, or by crashing — finalizes the
   Tracy trace, slices the perf capture into time windows, turns the census log
   into CSVs, writes `summary.md`, and tars the directory.

Every optional profiler is optional. A missing `perf`, a missing Tracy, an
adapter with no timestamp queries, a missing `strace` — each costs its own
artifact, records why, and the rest of the run is still collected.

### The no-GPU / VM workflow

```bash
./scripts/profile_desktop.sh --headless
```

This runs the game's own supported headless path (`--headless
--headless-ticks N`, default 1800) in the **sandbox** scenario, which composes
no renderer. You still get Tracy system timings, `perf`, the simulation
systems, schedule and entity counts, body counts, asset CPU work, and the
frame-interval census. The report marks every GPU and render-pass measurement
**not applicable** rather than absent, because a headless run is not evidence
that rendering is cheap.

The scenario is chosen because a bare headless host sits on the startup/launcher
route and simulates **zero bodies** — it succeeds, and it profiles nothing worth
profiling. `sandbox` is `run_game.sh`'s ordinary direct-entry alias, not a
profiling-only setup; the profiler adds the word and changes nothing else. Name
your own and it is used instead:

```bash
./scripts/profile_desktop.sh --headless -- smash
./scripts/profile_desktop.sh --headless -- -- --start-room goblin_encounter
```

`summary.md` records which scenario ran, in the `scenario` row of its first
table.

Do not use a software rasterizer as a stand-in for a GPU. If a windowed run on
this machine falls back to llvmpipe/lavapipe, `summary.md` says
**SOFTWARE RENDERING** at the top and the symbol rankings below it are mostly
the rasterizer's unsymbolized JIT'd shader code — that is a measurement of a
CPU emulating a GPU, and adapter selection is the bug to fix first.

#### Exercising the render path with no GPU

`lvp_icd.json` (lavapipe, Mesa's software Vulkan) plus `Xvfb` is enough to make
the whole render composition run on this VM:

```bash
xvfb-run -a -s "-screen 0 1280x720x24" ./scripts/profile_desktop.sh --duration 60
```

The bundle then carries real camera resolutions and a populated
`render_diagnostics.csv` — five passes with `elapsed_cpu`, `elapsed_gpu`,
vertex/fragment shader invocations and clipper counts. Use it to check that a
diagnostic is WIRED, and to see the shape of the pass set.

⛔ **The timings are a CPU emulating a GPU and are not a GPU measurement.**
`summary.md` says SOFTWARE RENDERING at the top for exactly this reason. Never
report a lavapipe `elapsed_gpu` as a rendering cost.

#### Tracy on a VM with no invariant TSC

The Tracy client aborts at startup — taking the game with it — on a CPU that
does not advertise `constant_tsc` **and** `nonstop_tsc`, which is every VM whose
hypervisor hides the second flag. The script detects that, sets
`TRACY_NO_INVARIANT_CHECK=1`, and writes `tracy.caveat` into the bundle. Zone
RATIOS stay sound; treat the absolute microseconds as approximate.

### Dev build versus optimized runtime

These are two different questions and the bundle names which one it answered:

```bash
./scripts/profile_desktop.sh              # why is the optimized runtime slow?
./scripts/profile_desktop.sh --dev-build  # why is my edit/play build slow?
```

`[profile.dev]` deliberately builds `ambition_app`, `ambition_render`, and
`ambition_platformer2d_runtime` at `opt-level = 0` (see the measured table in
`Cargo.toml`), so dev-build numbers are not release numbers and must never be
reported as an architecture finding. `[profile.profiling]` is release
optimization with `debug = 1` and `strip = "none"`, which is what lets `perf`
and Tracy attribute a frame; `ship` is the wrong instrument here because it
strips symbols and fat-LTOs everything into one unattributable blob.

### Other modes

```bash
scripts/profile_desktop.sh perf-run --duration 30 --report-preset full
scripts/profile_desktop.sh perf-attach --duration 30
scripts/profile_desktop.sh stat-run --duration 30
scripts/profile_desktop.sh asset-run --duration 30
scripts/profile_desktop.sh --no-tracy --build-profile release -- sandbox
```

The default open-ended capture records without call graphs (~15 KB/s of
`perf.data`, so a long session stays manageable) and the per-window reports are
flat self-time symbol lists. When you need caller attribution for a specific
hotspot, follow up with `perf-run --duration 30 --report-preset full`, which
keeps the DWARF stacks.

Arguments after `--` go to `run_game.sh`; a second `--` reaches the game
(`-- sandbox -- --start-room mary_o_level_1`).

Attach modes look for the launched binary by name, then `ambition_game_bin`,
and also accept the historical `ambition_platformer2d_actor_monolith` name for
older local builds. They cannot see the game's stdio, so an attach bundle has
no census CSVs; it says so in `census.missing`.

### What is in the bundle

`summary.md` is the front page and carries a table of every file with a
present/absent column. The rest:

| file | contents |
|---|---|
| `metadata.txt`, `metadata.json` | commit, branch, dirty files, cargo profile, features, executable, rustc, target, host, capture settings |
| `host-environment.txt` | CPU, memory, session type, DRM render nodes, Vulkan ICDs, `VK_*`/`WGPU_*`/`MESA_*` overrides, adapters |
| `timeline.md` | per-window perf symbols labelled with the game's own log markers |
| `frame_times.csv` | per-census-window frame percentiles (mean/p50/p95/p99/min/max) |
| `frame_spikes.csv`, `frame_windows.csv` | every frame over 33.4ms; the always-on 5s census |
| `camera_views.csv` | ONE ROW PER CAMERA PER SAMPLE: role, active, target kind, resolution, viewport, order, render layers, presented view, name |
| `view_totals.csv` | cameras / active / world-rendering / offscreen / local-view counts |
| `portal_activity.csv` | capture rigs, active rigs, and the effective capture budget bounding them |
| `render_target_census.csv` | offscreen image targets, their bytes, largest dimension |
| `render_diagnostics.csv` | Bevy per-pass `elapsed_cpu`, `elapsed_gpu`, and pipeline statistics |
| `runtime_census.csv` | entities, archetypes, components, bodies, players |
| `draw_census.csv` | sprites, visible sprites, `Text2d`, per-view projections |
| `schedule_census.csv` | registered systems per sample |
| `asset_activity.csv`, `image_decodes.csv` | cumulative decode work; every notable texture with its path |
| `tracy_summary.md`, `tracy_zones.csv` | per-Bevy-system and per-render-pass zones, ranked |
| `tracy_zone_windows.csv` | the same zones bucketed into time windows (needs a `tracy-csvexport` with `--unwrap`) |
| `tracy.trace` | the raw trace, if you do want the GUI |
| `perf_windows/`, `perf_report.txt`, `perf-report-by-dso.txt` | the native profile |
| `game-stderr-stamped.txt` | the game's whole log, stamped with seconds since launch |

Everything shares one clock: the `[   12.345s]` stamp is seconds since the
profiler launched the process and appears on every CSV row as `wall_s`, and
each census row also carries `t=`, seconds since the census clock started
inside the game. That is what makes "frame time rose at 74s, and so did the
world-rendering camera count" a join rather than a guess.

### Which measurements need real hardware

| measurement | needs |
|---|---|
| per-pass `elapsed_cpu` | nothing; always recorded when the render app exists |
| per-pass `elapsed_gpu` | a Vulkan or DX12 adapter with `TIMESTAMP_QUERY` |
| primitive / shader-invocation counts | an adapter with `PIPELINE_STATISTICS_QUERY` |
| anything about GPU rendering at all | a real GPU — not llvmpipe, not headless |
| kernel-side perf symbols | `kernel.perf_event_paranoid <= 1` and `kptr_restrict = 0` (the script requests both) |

`summary.md` distinguishes **measured**, **supported but unavailable on this
machine/backend**, and **not applicable (headless run)** for each of these. A
diagnostic is never silently omitted.

## 2c. Android native allocation profiling

For Android allocation callstacks, use Perfetto/heapprofd through the
Android profile script:

```bash
# Build/install a symbol-friendly APK if needed.
scripts/profile_android.sh prepare --profile-build

# Open the game on the phone, navigate to the slow state, then attach.
scripts/profile_android.sh heap --no-launch --duration 30
```

The output directory contains `heap.perfetto-trace`. Open that file in
<https://ui.perfetto.dev>, click the `Native heap profile` track, and
switch between:

- `Total Malloc Size` for allocation bytes/churn.
- `Total Malloc Count` for allocation frequency/churn.
- `Unreleased Malloc Size` / `Unreleased Malloc Count` for retained
  allocations.

`--profile-build` keeps debug info and forces an ELF Build ID on the
Android app library. That Build ID is important for matching Perfetto
heap-profile mappings back to the local `libambition_app.so` symbols.
To verify the latest profile APK:

```bash
readelf -n target/android/ambition_platformer2d_actor_monolith_android/app/src/main/jniLibs/arm64-v8a/libambition_app.so | grep -A1 "Build ID"
```

If the capture reports heapprofd buffer overruns, rerun with a coarser
sample interval:

```bash
scripts/profile_android.sh heap --no-launch --duration 30 --heap-sampling-interval 16384
```

## Quick recipes

**"Why is startup slow?"** Default build → check the `[startup]`
lines in stderr. If a single phase dominates, add finer phase marks
inside it.

**"Why did frame time get worse?"** `./scripts/profile_desktop.sh`, reproduce
it, quit. Read `summary.md`: worst frames first, then the camera/view counts
and render-pass times at the same second, then the Tracy zone table.

**"Is a feature costing me while it is visually inactive?"** Same bundle.
`camera_views.csv` shows an `active=true` camera with no visible output;
`portal_activity.csv` shows rigs alive with nothing on screen;
`draw_census.csv` shows the gap between `sprites` and `sprites_visible`.

**"Asset loading hitches the first time I enter a room"** Same bundle.
`image_decodes.csv` names the sheet and the second, `frame_spikes.csv` says
whether that second stuttered, and `timeline.md` says which room you were
entering.

**"I only have the VM."** `./scripts/profile_desktop.sh --headless`. You lose
GPU and render-pass numbers (the report says so) and keep everything about the
simulation, the schedule, the entity population, and asset CPU work.

## Adding manual spans

Inside any system (or helper) where you want fine-grained timing
without enabling Tracy globally, wrap the work in a `tracing` span:

```rust
use bevy::log::tracing::info_span;

let _span = info_span!("expensive_room_init").entered();
build_room(...);
```

These spans show up automatically in Tracy under `--features profile`
and are no-ops in default builds.
