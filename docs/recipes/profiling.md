# Profiling Ambition

Two layers of instrumentation are wired in:

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
[crates/ambition_app/src/app/plugins.rs](../../crates/ambition_app/src/app/plugins.rs):

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
[crates/ambition_gameplay_core/src/dev/profiling.rs](../../crates/ambition_gameplay_core/src/dev/profiling.rs).

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

Add this to `crates/ambition_gameplay_core/Cargo.toml` for symbol-rich
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
# package's `crates/ambition_gameplay_core/assets/`. Without this var, you
# get `Path not found: target/release/assets/...` for every asset
# and bevy_yarnspinner panics on the missing dialogue/ folder.
BEVY_ASSET_ROOT=$PWD/crates/ambition_gameplay_core \
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
readelf -n target/android/ambition_gameplay_core_android/app/src/main/jniLibs/arm64-v8a/libambition_app.so | grep -A1 "Build ID"
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
