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
[crates/ambition_sandbox/src/app.rs](../crates/ambition_sandbox/src/app.rs):

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
[crates/ambition_sandbox/src/profiling.rs](../crates/ambition_sandbox/src/profiling.rs).

## 2. Bevy + Tracy per-system profiling

Bevy ships with built-in tracing instrumentation. Enabling
`--features profile` flips on `bevy/trace` and `bevy/trace_tracy`,
which streams per-system spans to a [Tracy](https://github.com/wolfpld/tracy)
GUI listener.

### Build + run

```bash
cargo run -p ambition_sandbox --features profile
```

The binary will block on startup until Tracy connects (or proceed
without if the GUI isn't running — your build, your call).

### Collect a profile

1. Install the Tracy GUI (`tracy-profiler`) matching the Bevy version's
   tracy-client. Bevy 0.18 expects Tracy 0.12.x. Check the bevy/Cargo
   metadata if you upgrade.
2. Launch the GUI **before** the game so the live capture starts at
   T=0.
3. `cargo run -p ambition_sandbox --features profile`.
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
