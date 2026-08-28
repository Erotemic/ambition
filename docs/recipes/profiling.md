# Profiling Ambition

## Which instrument answers which question

These tools are not interchangeable, and reaching for the wrong one wastes
hours. Pick by the question you are actually asking:

| Question | Instrument | Reads as |
|---|---|---|
| Which machine layer owns the CPU — game code, GPU driver, or a software rasterizer? | `scripts/profile_desktop.sh` (default) | text, agent-readable |
| Where did startup time go? | `[startup]` phase logger (always on) | text, agent-readable |
| Which native function is hot, and during which part of my session? | `scripts/profile_desktop.sh` timeline chunks | text, agent-readable |
| Which *Bevy system* is hot? | `--features profile` → Tracy | GUI, or `tracy-capture` + `tracy-csvexport` as text |
| Which frames stuttered, and when? | `[frame-spike]` / `[frame-census]` (always on) | text, agent-readable |
| Which textures decoded, how big, and when? | `[image]` / `[image-census]` (always on) | text, agent-readable |
| Did a sprite get re-bound at a different size? | `[sprite-bind]` (always on) | text, agent-readable |
| Which *render pass* is hot? | not yet wired (`RenderDiagnosticsPlugin`) | — |
| Am I re-reading assets from disk every frame? | `profile_desktop.sh asset-run` | text, agent-readable |
| Is this CPU-bound, memory-bound, or stalled? | `profile_desktop.sh stat-run` | text, agent-readable |
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

1. Install the Tracy GUI (`tracy-profiler`) matching the Bevy version's
   tracy-client. Bevy 0.18 expects Tracy 0.12.x. Check the bevy/Cargo
   metadata if you upgrade.
2. Launch the GUI **before** the game so the live capture starts at
   T=0.
3. `cargo run -p ambition_app --bin ambition_game_bin --features profile`.
4. Click "Connect" in Tracy. Watch the flamegraph populate live.
5. To save: Tracy menu → "Save trace". `.tracy` files compress well
   and are reproducible.

### What's captured

- Every Bevy system's per-tick CPU time, automatically.
- Custom `info_span!("...")` blocks added in code.
- GPU timing if Tracy's GPU module is wired (off by default).

### What's NOT captured

- Anything in non-Bevy threads unless instrumented manually with
  `tracing` macros.
- Allocation profiling (use `dhat` / `heaptrack` separately).

### Cost

Tracy adds ~5-10% CPU overhead and grows the binary by ~3 MB. Both
are negligible during dev. Default builds drop the dep entirely
since `profile` is opt-in.

### Desktop perf/stat/strace captures

For desktop captures without Tracy, use
[`scripts/profile_desktop.sh`](../../scripts/profile_desktop.sh):

```bash
scripts/profile_desktop.sh
scripts/profile_desktop.sh -- release
scripts/profile_desktop.sh perf-run --duration 30
scripts/profile_desktop.sh perf-attach --duration 30
scripts/profile_desktop.sh stat-run --duration 30
scripts/profile_desktop.sh asset-run --duration 30
```

With no arguments the script does a `timeline-run`: it launches the game the
way `./run_game.sh` would, records until you quit the game (Ctrl-C in the
terminal works too), then slices the capture into 12 time chunks and labels
each with the room/boss/title/session log lines seen in that window. Read
`target/profiles/desktop-timeline-run-*/timeline.md`; each chunk lists its
own top symbols, so "what got slow when I entered the room" is a diff between
two chunks rather than a whole-run average.

Every capture also writes `host-environment.txt` (GPU, DRM render nodes,
installed Vulkan ICDs, `VK_*`/`WGPU_*`/`MESA_*` overrides, session type) and
puts the adapter the game actually selected at the top of
`desktop-profile-summary.md`. Read that section first. Captures are often
analyzed on a different machine than they were taken on, and if the run fell
back to a CPU rasterizer then ~90% of the samples are the rasterizer's
unsymbolized JIT'd shader code — the symbol rankings below it are then
describing the few percent left over, and adapter selection is the bug.

Because that capture is open-ended, `timeline-run` records without call
graphs (~15 KB/s of `perf.data`, so a long session stays manageable) and the
per-chunk reports are flat self-time symbol lists. When you need caller
attribution for a specific hotspot, follow up with a bounded capture:
`scripts/profile_desktop.sh perf-run --duration 30 --report-preset full`,
which keeps the DWARF stacks.

The cargo profile is whatever `run_game.sh` defaults to; pass `-- release` to
profile an optimized build, and a second `--` to reach game arguments
(`-- release -- --start-room mary_o_level_1`).

Attach modes look for the current desktop game process, `ambition_game_bin`,
and also accept the historical `ambition_platformer2d_actor_monolith` name for older local
builds.

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

**"Why did frame time get worse?"** `--features profile`, capture
30 seconds in Tracy, sort the system list by CPU time. Compare a
known-good run side-by-side.

**"Asset loading hitches the first time I enter a room"**
`--features profile` + filter Tracy on the room-load frame. Asset
load spans show up under `bevy_asset` system names.

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
