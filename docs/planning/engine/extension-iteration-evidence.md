# Extension iteration: evidence, measurements and decisions

**Inspection baseline:** `d81a7ae1d2db1fc5caa49efc807a39ea6b1ca266`, 2026-09-11.
**Owner:** [extension model](extension-model.md). This page records evidence and
experiments, not another work queue. [Packets](fast-iteration-implementation.md)
consume the results. Recheck locators on a newer head.

## What was and was not established

The investigation read the uploaded source, manifests, relevant planning and
repository checks. A temporary Python manifest traversal ran outside the source
tree. It found 79 workspace members. There was no cargo or rustc executable in
the investigation environment. No Cargo resolution, Rust compilation, linking,
hot reload, runtime profile or rollback acceptance test ran here.

Existing timing results in Cargo comments and the build plan remain historical
results from their stated machines. They are not new measurements. The user
reports severe iteration latency; the source establishes several coupling paths,
but it does not identify today's dominant wall-clock cost.

### Source map

| ID | Source and locator | Established fact | Architectural consequence |
| --- | --- | --- | --- |
| E01 | `Cargo.toml`, workspace members and profiles; `.cargo/config.toml` | Broad workspace; deliberate per-package dev settings, incremental builds and Linux clang/mold configuration | Do not treat a generic linker/profile recommendation as a new discovery |
| E02 | `crates/ambition_entity_catalog/Cargo.toml`; `crates/ambition_entity_catalog/src/lib.rs`, MoveSpec, TechniqueFlow, EffectRef | Move values already live in a Bevy-free serde/RON crate | Start authoring extraction here, not at the engine facade |
| E03 | `crates/ambition_characters/src/moveset_authoring.rs`, imports and helper functions | Mostly pure value construction; imports slash VFX constants from moveset_prefabs | Move the precise pure closure and audit constants, not the whole character domain |
| E04 | `game/ambition_demo_smash/src/moveset.rs`, imports; `game/ambition_content/src/authored/` | Authored Rust move data is built under game providers; Smash helpers import through the facade | A builder fixture must prove an independent compilation path |
| E05 | `crates/ambition_content_pack/src/prepared.rs`, PreparedContentPack and lowered | Lowered sections contain Arc of type-erased Any values | Current prepared packs are in-process objects, not a portable executable/content ABI |
| E06 | `game/ambition_content/src/pack.rs`, PACK_MANIFEST_RON, embedded_sources, compile_pack, prepared | Several inputs use include_str; prepared pack is process-global OnceLock; some generated inputs have disk/embed selection | Disk source support is not coordinated last-good generation reload |
| E07 | `crates/ambition_content_cli/Cargo.toml` | Offline CLI directly composes several owning domain schemas | Its name alone does not establish a small compilation closure |
| E08 | `crates/ambition_characters/src/prepared.rs`, stage_character_revision, activate_staged_revision, RevisionOutcome | Candidate cast validation can retain last-good definitions and generation on refusal | Reuse this path; expand coordination to the complete admitted bundle |
| E09 | `crates/ambition_combat/src/technique.rs`, InstalledTechniques; `crates/ambition_platformer2d_runtime/src/combat_schedule.rs` | Installed technique support and the preparation barrier exist | Remove the old extension plan's claim that production validation is wholly absent |
| E10 | `crates/ambition_entity_catalog/src/lib.rs`, TechniqueOffer, NestedReferences, TechniqueSupport | Installed offer data includes executable validators/reference callbacks | Do not serialize those function tables as artifact data |
| E11 | `crates/ambition_platformer2d_runtime/src/content_identity.rs`, PreparedContentBuilder, PreparedContentIdentity, ContentEpochSequence | Versioned BLAKE3 sections and App-local activation epochs already exist | Add module/state/profile identity sections; do not create a third content identity authority |
| E12 | `crates/ambition_platformer2d_core/src/snapshot.rs`, RollbackRegistrar; `crates/ambition_platformer2d_rollback_ggrs/src/registrar.rs` | Domain registration uses concrete generic types and existing backend installation | A dynamic schema store needs concrete registered storage, not only metadata |
| E13 | `crates/ambition_platformer2d_runtime/src/rollback/registry.rs`, RollbackRegistrationDescriptor, RollbackRegistry | Registry describes installed state and checksum/restore obligations | Descriptors do not themselves snapshot arbitrary new schemas |
| E14 | `game/ambition_content/src/bosses/specials/rollback.rs` | Content-owned boss state has repeated handwritten canonical codecs and registrations | Real candidate customer for generic schema-backed extension state |
| E15 | `crates/ambition_platformer2d_shared_tangle/src/sim_id.rs`; `crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs` | Semantic identity, per-spawner counters and session ownership already have homes | Preserve their semantics when extracting portable values |
| E16 | `crates/ambition_platformer2d_runtime/src/session_world.rs`; `crates/ambition_platformer2d_runtime/src/rollback/authority.rs` | Prepared source is distinct from live world; rebase/session health semantics are explicit | Do not promise arbitrary world undo or clear unhealthy diagnosis during reload |
| E17 | `crates/ambition_asset_manager/src/lib.rs`; `docs/planning/engine/asset-preparation-and-residency.md` | Asset handling and target-specific source/residency policy already exist | Load content through that source boundary, not a second asset manager |
| E18 | `AGENTS.md`; `docs/recipes/cheapest-sufficient-check.md`; `scripts/check_absence_contracts.py` | Narrow checks and dependency/profile guards already exist | Extend these mechanisms instead of adding a competing all-tests launcher |

The code paths above were read, not executed. Source-backed reuse is not a
passing acceptance report. Some old focused plans contain historical baseline
paragraphs followed by closure corrections. Follow the current code and the
local correction; do not resurrect closed A11/A12 work from the older paragraph.

### Manifest-only dependency observations

Method: traverse top-level normal, nonoptional internal path dependencies.
Exclude the root package from each count. Exclude dev/build/target-specific
sections and all feature activation. This is not a Cargo-resolved graph and is
not a proxy for milliseconds, code size or installed systems.

| Root | Other required internal packages under this method |
| --- | ---: |
| ambition_entity_catalog | 0 |
| ambition_registry_core | 0 |
| ambition_content_pack | 0 |
| ambition_characters | 4 |
| ambition_combat | 18 |
| ambition_content_cli | 25 |
| ambition_platformer2d | 48 |
| ambition_platformer2d_runtime | 43 |
| ambition_content | 54 |
| ambition_demo_smash | 49 |

Examples of inspected paths: content directly depends on render; Smash depends
on the facade and therefore the host; content_cli reaches audio. The old mandatory
facade -> host -> render path was already removed from the inspected normal
nonoptional closure. Do not cite the older 51-package/render claim as current.

A content edit may rebuild its provider and relink a dependent executable without
recompiling unchanged Bevy dependencies. Conversely, a feature, flags or profile
change can invalidate much more. M0 must distinguish these cases. A graph walk
cannot say which one currently dominates the user's loop.

Reproduce the static count without fetching dependencies:

```bash
python3 - <<'PY'
from pathlib import Path
import tomllib
root = Path('.')
w = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']
graph = {}
for member in w['members']:
    d = tomllib.loads((root / member / 'Cargo.toml').read_text())
    graph[d['package']['name']] = {
        spec.get('package', key)
        for key, spec in d.get('dependencies', {}).items()
        if isinstance(spec, dict) and 'path' in spec
        and not spec.get('optional', False)
    }
for start in ('ambition_entity_catalog', 'ambition_registry_core',
              'ambition_content_pack', 'ambition_characters',
              'ambition_combat', 'ambition_content_cli',
              'ambition_platformer2d', 'ambition_platformer2d_runtime',
              'ambition_content', 'ambition_demo_smash'):
    seen, todo = set(), list(graph[start])
    while todo:
        name = todo.pop()
        if name not in seen:
            seen.add(name)
            todo.extend(graph.get(name, ()))
    print(start, len(seen - {start}))
print('members', len(graph))
PY
```

Use actual Cargo metadata/tree for the resolved guard. Treat command failure or
an empty unexpected graph as a failed measurement, never as zero dependencies.
Do not rewrite the existing feature-resolved absence guard into this simpler walk.

## Measurement program

M0 can run beside I1. Only backend/layout/profile decisions wait on measurements.
If the toolchain or representative machine is unavailable, record that blocker,
continue DO work, and leave performance claims open.

### M0 - edit-to-observable-result baseline and comparison

Owner: B7 in [build and distribution](project-build-and-distribution.md).
Use the normal developer machine and the real selected host/profile. Record
commit and dirty diff; toolchain, target, CPU, memory, filesystem and free disk;
features, profile, jobs, linker, flags, cache state and asset prerequisites.
Do not clean the shared target or change profiles between a before/after pair.
Use a separately prepared cold-cache target only when measuring cold builds.

Run these edit classes independently:

| Loop | Current specimen | Required post-migration observation |
| --- | --- | --- |
| Move timing/damage | One scalar in the actual Smash moveset | Artifact validator and load only; changed value affects a controlled exchange |
| Character configuration | One actual authored character field | Only the responsible schema/source compiler runs; new definition appears at supported activation boundary |
| Procedural mechanic | One boss special algorithm, then one new state field | Small module build and activation; no host linker invocation |
| Presentation | A real loose asset edit with no mechanical effect | Existing source/asset path; simulation digest and replay unchanged |
| Engine | A scoped runtime implementation edit | Normal heavyweight lane, measured separately |
| Validation | Content tests versus module tests versus host tests | Correct selected lane; no implicit workspace suite |

For each specimen: warm the exact command, record an unchanged no-op, make a
small semantically visible edit, build/prepare/admit, and wait for an automated
host acknowledgment tied to the new generation and observed behavior. Undo the
edit and repeat paired runs. Preserve before/after bytes and raw command logs.
Start with at least seven pairs and report median, range and sample count. Do
not claim a stable p95 from that small set; use at least thirty samples when a
tail-latency release gate is needed. Record competing machine load.

Split elapsed time into source compile/check, builder/module link, preparation,
transfer/load, host admission, reconstruction, first simulation result and visual
readiness. Use Cargo timings and verbose freshness reasons for compilation;
observe actual linker process invocations with a matched tracing/wrapper setup.
An unchanged executable hash does not prove the linker was never run. Capturing
only cargo check omits codegen/linking and cannot close edit-to-play acceptance.

Existing commands to start inspection on a configured developer machine:

```bash
scripts/setup/target_bindmount.sh --status
python3 scripts/check_disk_headroom.py
rustc -Vv
cargo -V
cargo metadata --locked --format-version 1 --no-deps > /tmp/ambition-metadata.json
cargo tree -e normal --no-default-features -p ambition_platformer2d
cargo tree -e normal -p ambition_content_cli
cargo build --locked -p ambition_demo_smash_app --timings -vv
```

The last command is a representative existing build, not the benchmark harness
or a required check for applying this documentation. Record the actual normal
run command and features for the user workflow. Do not benchmark an unrelated
minimal executable and present it as the full game's old cost.

I0 adds a narrow measurement helper if current tools cannot record these phases.
Its output contract includes:

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

Null means unmeasured, not zero. A successful artifact publication must be
correlated with the exact edited input and admitted digest. Include raw log paths,
input traces and result checksums in the real output. A faster failure or loading
the previous generation is not a faster iteration.

### M1 - executable backend comparison

I6 uses the same I4/I5 mechanic, state schema, input sequence and request outputs
for a static native reference, a separately built trusted native C ABI module,
and a WASM prototype. Compare warm edit-build-load time, bootstrap/dependency
cost, debugging/source maps, invocation/batch overhead, allocation behavior,
reset cost, deterministic work limits, failure behavior and target feasibility.
The reference is a correctness oracle for the specified fixture, not proof of
universal native/WASM float equivalence.

Measure small frequent calls and batched entity processing separately. Include
new algorithm code and new schema admission, not only replacing a constant.
A backend that is quick to compile but dominates the fixed-tick budget fails the
runtime criterion. A fast backend with unsupported reload on the required target
needs an explicit target-specific deployment policy.

Prefer one production procedural backend after this comparison. A native path
must justify its ABI and unload maintenance cost. A WASM path must justify its
runtime/target and marshaling costs. Do not require Lua and Rhai prototypes before
delivering the data path. Evaluate those script bindings when a scripting syntax
customer exists, using the same contract and hidden-state tests.

### M2 - state, snapshots and scaling

Run the actual GGRS save/restore/resimulation path with populated extension state.
Compare safe row storage, chunked typed storage and dynamic Bevy columns only
where supported. Compare full copies versus chunk COW/deltas only after obtaining
the correct baseline. Keep canonical values and result traces identical.

Fixtures include sparse and dense writes, variable-size graphs, entity churn,
reference remapping, several independent modules, active-region records plus
dormant persistent data, and repeated rollback over an appreciable history window.
Record entity/record counts, live bytes, changed bytes per tick, snapshot retained
bytes, allocations, checksum time, save time, restore time, resimulation CPU and
the cost of generation reset. Exercise multiple sizes until the scaling curve,
not just one average, is visible. Select sizes from actual game populations plus
stated stress multipliers; do not present stress values as product requirements.

Default safe full-copy storage may prove too costly. That does not reopen whether
authoritative state rewinds; it selects storage/snapshot optimization. Dirty-page
experiments must report target support and write-barrier coverage. Host-managed
schema state remains the default unless a concrete guest-state customer proves
that an alternate complete-state model is necessary and affordable.

### M3 - end-to-end developer and target validation

Use a clean out-of-workspace consumer, separate lockfile and documented target.
Do not make game/provider sources dependencies of the engine host merely to
share fixtures. Verify normal data and procedural edits with no host build/link,
plus raw Bevy engine plugin use in its heavy lane. Test local reload, rejected
replacement, peer-generation mismatch and installed read-only packaging.

Prove target support with that target's actual runtime and loader. A native
Wasmtime host experiment does not establish a browser integration, mobile JIT
permission, console support or parity between different numeric backends.
One unsupported product target cannot be silently omitted from the report.

## Choices that remain open

| Question | Why still open | Cheapest resolving evidence | What it gates | Default while open |
| --- | --- | --- | --- | --- |
| Dominant current latency | No representative toolchain run here | M0 on move and procedural edits | Profile/link/cache optimizations and quantitative gain claim | Implement the independent artifact boundary |
| Production executable backend | ABI/runtime/target tradeoffs unmeasured | M1 with one real stateful fixture | I7 backend commitment, not I1-I5 | Static semantic reference plus WASM-first portable prototype |
| State physical layout | Copying/query/checksum curve unknown | M2 over real GGRS snapshots | I8 optimization | Safe schema-backed host storage |
| Complete VM image fallback | No fixture yet needs persistent opaque heap | One algorithm failing host-managed-state ergonomics, then complete-image replay and cost | Optional alternate state profile only | Explicit host state; reset scratch |
| Artifact encoding/compression | No package/load bottleneck measured | Small canonical roundtrip and M0 load breakdown | Encoding optimization, not ownership | Simple versioned canonical section encoding |
| Parallel procedural scheduling | Access patterns and merge costs unknown | Serial reference versus declared-independent batches | Parallel optimization | Stable serial execution |
| Public untrusted mod distribution | Product trust/installation policy undecided | Maintainer decision when shipping downloadable mods | Signing, permissions UX, distribution hardening | Trusted local native code; portable imports restricted, no sandbox marketing |
| Cross-version save/code migration promise | Compatibility horizon is a product promise | Maintainer decision plus concrete old-save fixture | Public persistence compatibility commitment | Reject unsupported schema migration; keep old save intact |
| Seamless state-preserving live reload | No requirement for arbitrary mid-session state migration | Explicit workflow customer plus migration/rollback failure tests | Retaining old code or live migration | Supported local reconstruction; remote generations pinned |

Do not send the first six questions to the maintainer as ordinary architecture
choices. The implementer runs the specified experiment and records a decision.
Product rows do not block local trusted authoring or the current data migration.

## External technical verification

These primary sources were consulted on 2026-09-11. They verify narrow platform
facts; the architecture decisions above are this investigation's recommendations.
Pin actual dependency versions in each executable prototype.

- [Bevy ECS 0.19.1 ComponentDescriptor](https://docs.rs/bevy_ecs/0.19.1/bevy_ecs/component/struct.ComponentDescriptor.html): components need not correspond to a Rust type; dynamic layout construction has explicit unsafe layout, drop and thread-safety requirements. This supports a possible host implementation, not an ABI or rollback guarantee.
- [Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html): layout guarantees depend on representation, and an outer representation does not stabilize arbitrary nested Rust types. The native module recommendation therefore uses an explicit boundary rather than exporting Rust containers.
- [Wasmtime: deterministic execution](https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html): deterministic imports, NaN/SIMD policy, growth behavior and deterministic interruption need deliberate configuration. The proposed restricted execution profile follows those obligations rather than assuming all WASM execution is deterministic.
- [Wasmtime: platform support](https://docs.wasmtime.dev/stability-platform-support.html): supported hosts and execution modes are runtime-specific. Check the chosen target, not just the portable module format.
- [Lua 5.4 reference](https://www.lua.org/manual/5.4/manual.html): the language includes environments, mutable values, coroutines and garbage collection. A host-state binding must account for that mutable execution state rather than treating globals as automatically rewindable.
- [Rhai engine options](https://rhai.rs/book/engine/options.html): operation, depth and collection limits are configurable; some restrictions are compile-time settings. A binding must apply a consistent configuration to compilation and execution and still supply its own rollback contract.
