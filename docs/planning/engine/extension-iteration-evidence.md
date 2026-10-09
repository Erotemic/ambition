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
| Graph position (`scripts/compile_cost.py --scenario check` against `--scenario check-leaf`: one file edited in `actor_monolith` or in `platformer2d_core`, then `cargo check -p ambition_app`) | about 2x: 2.14x on a 30-core host capped at `-j2`; 1.91x on a 6-core VM | Two hosts that differ in cap and core count; one edit class (`append-private-fn`); consistent with a property of the dependency graph, not established |
| Module road (an edit to a loadable `.wasm` with `scripts/build_extension_modules.sh`) | 1.36 s for the first edit after a cold module build; 0.34 s for a warm one-constant edit | Agent machine |
| The same constant in a technique still in `game/ambition_content` | 7.05 s to relink `ambition_app`, plus a restart | Agent machine |
| A data source read off disk (`boss_profiles.ron`, `sfx_registry.ron`, a demo move table) | about 0.4-0.5 s, no crate compiled; the same files embedded cost 5.8-7.2 s | Agent machine, warm |
| The engine loop (`scripts/compile_cost.py --scenario edit-cycle`: a private fn appended to `actor_monolith/src/lib.rs`, then the `app_it` test build, one targeted `app_it` module, the `capture_scene` build, and `capture_scene` drawing the hub's first frame) | build 33.8 s (warm no-op 0.9 s, 18 units); targeted test 2.9 s (warm 2.9 s); capture build 27.1 s (warm 1.0 s, 17 units); first frame 3.5 s (warm 3.2 s). Total about 67 s, and the two builds are 91% of it | One run, 2026-10-09, 14 cores, mold, `74bc999` in the measurements ledger (run `a85c8fabfd06`, measured at `1e403a3`). Each warm phase compiled 0 units, and the script refuses a row where one compiles. An earlier row (`13b5acb`) had warm phases of 16.9 s and 12.5 s: they compiled during the control, and the row could not show it. Building `capture_scene` and then testing `app_it`, in either order, compiles nothing again (measured the same day) |

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
- A fresh instance per call is kept: it is what stops a guest static from
  carrying state across a rewind.
- Open: the per-call glue cost (instance reuse with a restored image; a lighter
  input encoding) and the native shared-library arm.

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
| Production executable backend | M1 against the native shared-library arm | I7 commitment | wasmi, deterministic, fuel, a new instance per call |
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
