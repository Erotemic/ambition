# Extension iteration: evidence, measurements and decisions

**Scope:** the measurement program M0-M3 for fast iteration, the results it
has, and the choices that measurements still decide. **Owner:**
[extension model](extension-model.md). [Packets](fast-iteration-implementation.md)
consume the results. This page is not a work queue.

The DO decisions are exact identity, separation of dependency and lifetime, one
domain owner and explicit visibility. M0-M3 choose encoding, caching, storage,
batching and backend details. Safe reload and instance isolation need
behavioral evidence; a faster benchmark does not waive them.

If the toolchain or a representative machine is not available, record that,
continue DO work and leave the performance claim open.

## M0 - edit-to-observable-result baseline and comparison

Owner: B7 in [build and distribution](project-build-and-distribution.md). Use
the normal developer machine and the real host and profile. Record the commit
and dirty diff; toolchain, target, CPU, memory, filesystem and free disk;
features, profile, jobs, linker, flags, cache state and asset prerequisites. Do
not clean the shared target or change profiles between a before and after
pair.

Run these edit classes separately:

| Loop | Specimen | Required observation |
| --- | --- | --- |
| Move timing or damage | One scalar in a real moveset | Artifact validator and load only; the changed value affects a controlled exchange |
| Character configuration | One authored character field | Only the responsible schema or source compiler runs; the new definition appears at a supported activation boundary |
| Procedural mechanic | One boss special algorithm, then one new state field | Small module build and activation; no host linker invocation |
| Presentation | A loose asset edit with no mechanical effect | Existing asset path; simulation digest and replay unchanged |
| Engine | A scoped runtime edit | Normal heavyweight lane, measured separately |
| Validation | Content tests, module tests, host tests | Correct selected lane; no implicit workspace suite |

For each specimen: warm the exact command, record an unchanged no-op, make a
small visible edit, build, prepare and admit, and wait for a host
acknowledgment tied to the new generation and the observed behavior. Undo the
edit and repeat paired runs. Report all samples, median and range, and record
competing machine load. Choose the sample count from observed variation. Do not
certify a p95 because a fixed sample count was met.

Split elapsed time into compile or check, builder or module link, preparation,
transfer or load, host admission, reconstruction, first simulation result and
visual readiness. Observe linker process invocations with a matched trace. An
unchanged executable hash does not prove that the linker did not run. A
`cargo check` alone omits codegen and linking.

The output contract of an iteration recorder (I0):

```json
{
  "source_commit": "exact commit",
  "edit_class": "move-data",
  "manifest_and_feature_args": ["exact arguments"],
  "toolchain_target_profile": "recorded separately or embedded",
  "cache_state": "warm",
  "sample_index": 1,
  "changed_input_digest": "actual edited input",
  "artifact_digest": "actual emitted artifact",
  "admitted_generation_digest": "host acknowledgment",
  "compile_seconds": null,
  "module_or_builder_link_seconds": null,
  "host_link_invocations": null,
  "prepare_seconds": null,
  "activate_seconds": null,
  "first_observed_result_seconds": null,
  "peak_rss_bytes": null,
  "status": "unmeasured"
}
```

Null means unmeasured, not zero. A faster failure, or a load of the previous
generation, is not a faster iteration. Extend the record with changed and
reused section counts and reasons, candidate peak bytes, scenario-restore time,
stale or cancelled attempts and the active generation.

### M0 results

| Measurement | Result | Limits |
| --- | --- | --- |
| Graph position (`scripts/measure/compile_cost.py --scenario check` against `--scenario check-leaf`: one file edited in `actor_monolith` or in `platformer2d_core`, then `cargo check -p ambition_app`) | about 2x: 2.14x on a 30-core host capped at `-j2`; 1.91x on a 6-core VM | Two hosts that differ in cap and core count; one edit class (`append-private-fn`); consistent with a property of the dependency graph, not established |
| Module road (an edit to a loadable `.wasm` with `scripts/regen/build_extension_modules.sh`) | 1.36 s for the first edit after a cold module build; 0.34 s for a warm one-constant edit | Agent machine |
| The same constant in a technique still in `game/ambition_content` | 7.05 s to relink `ambition_app`, plus a restart | Agent machine |
| A data source read off disk (`boss_profiles.ron`, `sfx_registry.ron`, a demo move table) | about 0.4-0.5 s, no crate compiled; the same files embedded cost 5.8-7.2 s | Agent machine, warm |
| A game's own shader (`content_asset!`: `room_plate.wgsl` in `ambition_content`, `shimmer.wgsl` in Mary-O), then that crate's tests | 1.0 s, no crate compiled; the same shader compiled in (`embedded_asset!`) cost a build of `ambition_content` (58 s for its tests, under `app_it` load) and a relink | 2026-10-18, agent machine, debug build. A restart, not a live reload: the file is read when the app starts |
| A live-schema edit in a running demo (`scripts/measure/iteration_record.py`, `smash_rules.ron` `return_to_select_seconds`, the headless `smash_demo`) | edit to the published generation 0.010 s median (0.008-0.024 s, 7 edits; reverts the same); the host's pack compile 9.2 ms; no build, no link | 2026-10-16, agent machine, debug build. The headless loop is unpaced; a host at 60 Hz polls every 20 frames, so add up to 0.33 s. The published generation is acknowledged; the behaviour it changes is not observed |
| A sprite edit in a running development build (`ambition_platformer2d --features content_pack --test sprite_sheet_reload -- --ignored --nocapture`: the published tree's sheets, rigs, flipbooks, portraits and pack catalogs) | read and check the whole table 913-1200 ms median off the frame before the flipbooks joined, 2229-2897 ms with every flipbook decoded from an undecoded generation (under app_it load); 1136 ms median (799-1323) for the next edit, which parses only the changed files and keeps only the changed texts (2.1 s before that carry-over); the swap on the frame 18-38 ms; a look (a stat of each file) 0.12-0.42 s, now paid only when the operating system reports a change; no build, no link | 2026-10-16, agent machine, debug build. The look-to-played step waits for the end of the session; the behaviour it changes is witnessed (`sprite_sheet_reload`, `sheet_derived_cast`), not timed in a drawn host |
| The engine loop (`scripts/measure/compile_cost.py --scenario edit-cycle`: a private fn appended to `actor_monolith/src/lib.rs`, then the `app_it` test build, one targeted `app_it` module, the `capture_scene` build, and `capture_scene` drawing the hub's first frame) | build 33.8 s (warm no-op 0.9 s, 18 units); targeted test 2.9 s (warm 2.9 s); capture build 27.1 s (warm 1.0 s, 17 units); first frame 3.5 s (warm 3.2 s). Total about 67 s, and the two builds are 91% of it | One run, 2026-10-09, 14 cores, mold, `74bc999` in the measurements ledger (measured at `1e403a3`). Each warm phase compiled 0 units, and the script refuses a row where one compiles. An earlier row (`13b5acb`) had warm phases of 16.9 s and 12.5 s: they compiled during the control, and the row could not show it. Building `capture_scene` and then testing `app_it`, in either order, compiles nothing again (measured the same day) |
| Pre-push validation (the lanes `scripts/required_checks.py` names for an `ambition_app` test change: the package's default-features job and the repo tooling job), read from three pushes on 2026-10-10 | `app_it` 890-913 s of tests (1180 arms) after its build; the repo tooling job 1146-1161 s serial, and 231 s with `-n auto` (1778 passed, 19 skipped). The validation, not the build, was most of a push | One machine, 14 cores; the counts are from `run_tests.sh` output, not the cost ledger (a narrowed run records no cost) |

Compare only rows whose `warm_noop_seconds` is at the machine's warm floor. A
`warm_noop` that is not warm invalidates every duration beside it
(`dev/compile_telemetry_schema.md`).

## M1 - executable backend comparison

I6 uses the same I4/I5 mechanic, state schema, input sequence and request
outputs for a static native reference, a trusted native C ABI module and a WASM
module. Compare warm edit-build-load time, bootstrap cost, debugging and source
maps, invocation and batch overhead, allocation, reset cost, deterministic work
limits, failure behavior and target feasibility. The reference is a
correctness oracle for the fixture, not proof of general native and WASM float
equivalence.

Measure small frequent calls and batched entity processing separately. A
backend that compiles quickly but dominates the fixed-tick budget fails the
runtime criterion. Evaluate script bindings (Lua, Rhai) only when a scripting
customer exists, with the same contract and hidden-state tests.

### M1 results (wasmi, release, agent machine)

- One WASM call is about 30 us to instantiate plus the guest time. Guest time
  fell from 31.8 to 7.8 us per call when `export_modules!(list: ...)` started to
  build only the called module.
- A call that does nothing burns about 86k fuel in the ABI glue.
- Idle ticks: `IdlePolicy::ResetState` and `ResetStateExcept(keep)` reset
  records without a call. With them, the loaded road's per-tick overhead with
  one boss is within run-to-run noise (30-100 us).
- Every call starts from the file's starting image: that is what stops a
  guest static from carrying state across a rewind. Since 2026-11-28 the
  image is restored into a reused instance rather than built anew (below).
- Open: the image restore (6.4 µs for 4 pages) and the sentry's own guest
  work.

### M1 results: the native shared-library arm (2026-11-28, release, agent machine)

The bake-off (`fixtures/scripting_bakeoff`, `cargo run --release --bin
bakeoff`) runs one stream of 2000 sentry ticks through each backend. Two
arms were added: `native_sentry`, the game's sentry built as a `cdylib`
behind a trusted C entry table, and the game's own wasmi backend loading
the game's module file (`scripts/regen/build_extension_modules.sh`). The
entry table is versioned (`amb_native_abi`), takes fixed-width lengths,
writes into a caller-owned buffer, and turns a panic into a fault value. It
carries the `ambition-ext-3` wire bytes unchanged. The host keeps the
library loaded as long as the backend, which pins the code a call runs.

| Arm | Outputs byte-equal to the native reference | Cost per invocation (3 runs) |
| --- | --- | --- |
| native, linked (the reference) | — | 755–777 ns |
| native shared library | 2000 of 2000 | 770–789 ns |
| wasm (wasmi, instance per call) | 2000 of 2000 | 88.4–90.9 µs (27.3–29.3 µs after the stack, reuse and prepare changes below) |
| engine Rhai backend | 2000 of 2000 | ≈10.6 µs |

- The shared library's transport costs about 15 ns per call over the linked
  module: two buffer copies and an indirect call.
- wasmi costs about 115 times the native call here. Instantiation grew with
  the module file (935 kB, every content module) from the 30 µs measured
  above; the instance per call is that cost.
- A cold `cdylib` build took 26 s and a rebuild under 1 s; the wasm module
  file builds in about 11 s.
- Where the wasm call's time went: instantiation was 64 µs and the first
  allocation about 50 µs more. The module's memory started at 17 pages, a
  1 MiB stack that every fresh instance zeroes. Linked with a 128 KiB stack
  (`MODULE_STACK_BYTES`), memory starts at 3 pages, instantiation takes
  33 µs, the first allocation costs nothing extra, and a sentry call takes
  59 µs, byte-equal. The rest of instantiation scales with the module
  file's functions (every content module in one file).
- Instance reuse (2026-11-28): loading checks that an instance can be put
  back exactly (`restorable_globals`: every mutable global exported, no code
  that changes a table or a passive segment), and modules are linked to
  export Rust's one mutable global, `__stack_pointer`. After each call the
  backend copies the image back over memory and sets the globals back. Rust's
  allocator takes its heap with `memory.grow` on its first allocation, so
  every call from the bare instance grew memory (3 to 4 pages), and memory
  never shrinks. The image is therefore taken after one allocation at start,
  a fixed function of the file, the same for every call and every peer. A
  call that grows memory past it drops the instance. A sentry call takes
  34.0–35.9 µs (3 runs), byte-equal over the 2000 ticks; a refused call 13.7 µs against
  33 µs of instantiation. Witnesses: a write to an exported global or to
  memory is gone at the next call; a file with a hidden mutable global gets a
  new instance per call; a call that grows memory does not leave it grown.
- The module table in the image (2026-11-28): with the instance reused, each
  call still built its module's descriptor before decoding, 14k–112k fuel by
  module (sentry 26k, the bossofthegame 103k). A guest now exports
  `amb_prepare` (`export_modules!` writes it), which builds the table into a
  static; a host that reuses instances calls it before taking the image.
  Every module's refused call is then 9.2k fuel, its guest side 2.0 µs
  against 9.7 µs, and restoring the 4-page image (6.4 µs) is the larger part.
  A sentry call takes 27.3–29.3 µs (3 runs), byte-equal over the 2000
  ticks.

**Deployment choice: keep wasmi and Rhai as the loaded roads; do not ship
the native arm.** A native library has no sandbox, no fuel, no
fresh-instance guarantee (a static survives a rewind) and no float
determinism across platforms, and it cannot load on the browser target.
Its per-call cost matters only where the loaded call dominates a tick, and
the idle policies above keep the game's loaded overhead within noise. If a
desktop development build ever needs it, the entry table here is the
prototype; generation pinning before unload is then the remaining work.

## M2 - state, snapshots and scaling

Run the actual GGRS save, restore and resimulation path with populated
extension state. Compare safe row storage, chunked typed storage and dynamic
Bevy columns only where supported. Compare full copies with chunk COW or deltas
only after the correct baseline exists. Keep canonical values and result traces
identical.

Fixtures include sparse and dense writes, variable-size graphs, entity churn,
reference remapping, several independent modules, active-region records plus
dormant persistent data, and repeated rollback over a long history window.
Record entity and record counts, live bytes, changed bytes per tick, retained
snapshot bytes, allocations, checksum, save and restore time, resimulation CPU
and generation-reset cost. Vary one population axis at a time until the
scaling curve is visible. An unchanged dormant ledger adds no all-ledger
traversal to a local active step; visit and allocation counts can prove the
work set when timing is noisy.

A safe full-copy store that costs too much does not reopen whether
authoritative state rewinds; it selects a storage optimization. Host-managed
schema state stays the default unless a concrete customer proves that another
complete-state model is necessary and affordable.

## M3 - end-to-end developer and target validation

Use a clean out-of-workspace consumer, a separate lockfile and a documented
target. Do not make game sources dependencies of the engine host to share
fixtures. Verify data and procedural edits with no host build or link, plus raw
Bevy plugin use in its heavy lane. Test local reload, refused replacement,
peer-generation mismatch and installed read-only packaging.

Prove target support with that target's actual runtime and loader. A native
host experiment does not establish browser integration, mobile JIT permission,
console support or parity between numeric backends. Report each unsupported
target.

## Choices that remain open

| Question | Cheapest resolving evidence | What it gates | Default while open |
| --- | --- | --- | --- |
| Dominant current latency | M0 on move and procedural edits | Profile, link and cache optimizations | Independent artifact boundary |
| Production executable backend | M1 against the native shared-library arm | I7 commitment | wasmi, deterministic, fuel, every call from the starting image |
| State physical layout | M2 over real GGRS snapshots | I8 | Safe schema-backed host storage |
| Complete VM image fallback | One algorithm that host-managed state serves badly | An optional alternate state profile | Explicit host state; reset scratch |
| Artifact encoding and compression | Canonical roundtrip and M0 load breakdown | Encoding optimization | Simple versioned canonical section encoding |
| Parallel procedural scheduling | Serial reference against declared-independent batches | Parallel optimization | Stable serial execution |
| Public untrusted mod distribution | Maintainer decision when downloadable mods ship | Signing, permissions, distribution hardening | Trusted local native code; restricted portable imports; no sandbox claim |
| Cross-version save and code migration | Maintainer decision plus an old-save fixture | Public persistence compatibility | Refuse unsupported schema migration; keep the old save |
| State-preserving live reload | One owner-specific live-binding policy and a migration fixture | An optional seamless path | Scenario reconstruction; module records migrate by field tag (`StateSchema::migrate`); a changed attachment or save policy is refused; remote generations pinned |

The implementer runs the experiment for the first six rows and records a
decision; they are not maintainer questions. Product rows do not block local
trusted authoring.

## Primary references

These concern narrow platform facts, not evidence that engine features work.
Pin and verify dependency versions in each executable prototype.

- [Bevy ECS ComponentDescriptor](https://docs.rs/bevy_ecs/0.19.1/bevy_ecs/component/struct.ComponentDescriptor.html):
  a component need not be a Rust type; dynamic layouts have explicit unsafe
  layout, drop and thread-safety requirements.
- [Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html):
  an outer representation does not stabilize nested Rust types, so a native
  module boundary uses explicit wire values, not Rust containers.
- [Wasmtime: deterministic execution](https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html):
  deterministic imports, NaN and SIMD policy, growth and interruption need
  deliberate configuration.
- [Wasmtime: platform support](https://docs.wasmtime.dev/stability-platform-support.html):
  supported hosts and execution modes are runtime-specific.
- [Lua 5.4 reference](https://www.lua.org/manual/5.4/manual.html): environments,
  mutable values, coroutines and garbage collection are execution state that a
  binding must account for.
- [Rhai engine options](https://rhai.rs/book/engine/options.html): operation,
  depth and collection limits are configurable; a binding still supplies its own
  rollback contract.
