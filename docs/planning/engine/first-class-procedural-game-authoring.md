# Goal: first-class procedural game authoring

**Selection:** Maintainer goal, 2026-11-16. **Status:** Complete, 2026-11-17, for the desktop Linux acceptance slice.
Browser and Android loader limits are stated below. They are not claimed from desktop results.

## Product result

A developer can make most of a game in Rhai, play it on a built Ambition runtime, and change the game without an engine build. When a mechanic is ready, the developer can implement that mechanic in Rust, build it as a loadable WebAssembly module, and later link that same Rust implementation into a release executable.

**The game does not need a rewrite when the implementation changes.** Its entities, catalog bindings, modules, state contracts, interactions, and authored content keep the same meaning. The developer changes the selected implementation and its build step, not the game design or the engine's domain rules.

This is a goal for the **portable procedural tier**, not a request to replace Bevy or the native engine-plugin tier. A new engine service may still require Rust and a host rebuild. Ordinary game logic must not require that work.

### The three paths

| Path | Main use | Author edit loop | Runtime contract |
| --- | --- | --- | --- |
| Rhai source | Rapid design, tuning, and playtesting | Edit script; validate and reload the project; no compiler | Shared procedural module descriptor, phases, ports, state, and authority |
| Rust to `.wasm` | Optimize a selected mechanic without relinking the engine | Compile the module only; load or reload the artifact | The same procedural module descriptor, phases, ports, state, and authority |
| Rust linked into the host | Ship a selected mechanic in the final binary | Compile and link the release host | The same procedural module descriptor, phases, ports, state, and authority |

Rhai source does **not** have to transpile into Rust. Porting an algorithm can be a deliberate manual or assisted translation. The contract, observable behavior, and content bindings must be portable. A Rust implementation should have one algorithm source that can build for both the WASM and linked paths, except for narrow backend glue.

## Existing foundation: preserve it

At source snapshot `158aeb5dcb27` (2026-11-16), the engine has:

- `ambition_extension_sdk` for module descriptors, typed ports, state schemas, and invocation values.
- `ambition_extension_host` for admission, deterministic invocation, state writes, faults, and module replacement.
- `ambition_extension_rhai` for Rhai modules, including `fn declare()`, source reload, and script records.
- `ambition_extension_wasm` for the `ambition-ext-1` ABI with wasmi, fuel limits, and fresh guest instances.
- Linked Rust extension modules in `game/ambition_content_modules`, which also use `export_modules!` for WASM builds.
- `--project` and `project.ron`, with `examples/projects/pulse_forge` as a scripted technique and LDtk world.
- Port descriptions from domain owners and a generated script port reference.

These facts are a starting point, not a claim that the goal is complete. Inspect the current branch before each change. Reuse these mechanisms; do not create a new extension runtime or second gameplay architecture.

## Definition of done

**All gates A1-A10 must pass before calling this goal complete.** Record exact commands, platform, build identity, and results. A source-code demonstration alone is not enough.

### A1. A real external project

Create or extend one small game project that is separate from the flagship game's content. It must contain its world, game-specific catalogs, module selection, and scripts. It must start through a prebuilt runtime. It must not use a game-specific Rust callback or require editing `ambition_app` for its mechanics.

The project must demonstrate at least two stateful mechanics from different gameplay domains:

- One new technique or interaction that creates or changes world state.
- One actor or group behavior that observes its world and makes decisions over several ticks.

Use the existing physics, actor, combat, item, and lifecycle owners. The example must not depend on a fixed Ambition-only equipment slot, item ID, or compiled behavior declaration. Small original or placeholder assets are sufficient.

**Pass:** A clean project can be loaded, played, and inspected without a Cargo invocation after installation. A README gives all steps. A headless test proves both mechanics affect the simulation.

### A2. Rhai is a normal authoring path

An author can declare and install a new script module with its own entry points, state schema, and required ports. The project can add, remove, and replace source files. It can bind the mechanic to game-authored data without a game-specific host rebuild.

Script authors can inspect the installed phases, triggers, observations, requests, numeric kinds, reference kinds, limits, and save roads. Errors name the project file and the relevant field or line. A port known in source but not installed in this runtime must be reported as unavailable, not silently accepted.

**Pass:** Change, add, remove, and break a script in a running project. Valid changes take effect. A rejected candidate leaves the last good generation usable. Show the effect through a simulation trace, not only a log message.

### A3. One stable semantic contract across backends

For a portable mechanic, the following values must mean the same thing in Rhai, WASM Rust, and linked Rust:

- Provider, module key, entry keys, phase, trigger selector, and dependencies.
- Read and request port versions, capability requirements, work limits, and result semantics.
- State schema field tags, value kinds, reference identities, attachment, lifetime, and save eligibility.
- Invocation identity, simulation-time units, coordinate frames, ordering, and fault boundaries.

Code identity **must differ** across implementations. The host must distinguish a Rhai revision, a WASM artifact, and a linked native build. It must also be able to compare their **normalized semantic declarations** without confusing source or binary identity with gameplay compatibility.

Use the existing descriptors as the contract. It is acceptable to produce a descriptor separately from Rhai `declare()` and Rust builders if validation proves that they match. Prefer shared domain-owned schema descriptions and generated language bindings over parallel handwritten conversions. Do not introduce a central enum that names every game operation.

**Pass:** Inspection reports each implementation's identity and a clear match or mismatch of its semantic declaration. A deliberate missing port, wrong field type, or mismatched state attachment is refused with an actionable diagnostic.

### A4. Promote one mechanic through all three paths

Select one of A1's mechanics and implement it in Rhai and Rust. Compile the Rust implementation to `.wasm` and also link it into a release/test host. Both Rust modes should use the same portable module code and SDK interfaces. Do not make a separate gameplay algorithm for each Rust deployment path.

Allow the project to choose **one active implementation for each module key**. A single project must be able to mix Rhai and WASM modules, then select linked Rust for a release build. Select Rhai or WASM without changing content IDs, entity bindings, or gameplay catalogs. Linked Rust is a deliberate build choice, and must not masquerade as hot reload. Reject duplicate active providers unless an explicit replacement selection resolves them.

**Pass:** Demonstrate Rhai -> WASM -> linked Rust on one project and one module key, with a second Rhai module active beside the WASM implementation. The same authored world and content files work in all three runs. Document the exact change the developer makes to select each path.

### A5. Prove gameplay parity, not syntax parity

Run the same deterministic input sequence against each implementation. Compare:

- Normalized module declarations and initial state.
- Requests submitted, including occurrence identities and order where the domain requires it.
- Requests applied or refused by domain owners.
- State evolution, creation and retirement of entities, and final world facts.
- A populated rollback and replay, including a request that is refused.

Require exact agreement for identities, integer state, accepted actions, and domain outcomes. Define and justify any floating-point tolerance **per measured field**, with a check that it cannot change discrete gameplay decisions in the fixture. Do not claim general cross-backend bitwise float identity. Preserve stable simulation-tick and coordinate-frame semantics.

**Pass:** A focused test runner executes one fixture through Rhai, WASM, and linked Rust and reports a structured diff if any required observable behavior differs. Include a negative test that deliberately changes one backend's logic and makes this gate fail.

### A6. The SDK supports the chosen game without private adapters

The example game must not need new game-specific Rust integration to create its actors, attach and retire behavior, read needed world facts, send control intent, request actions, and observe their results. Add the smallest **general-purpose domain ports** needed to meet that test.

Keep the domain owner responsible for validation and effects. A script or portable module must not mutate raw Bevy `World`, write private components, or bypass item custody, collision, damage arbitration, or control authority. New observation ports must have bounded scope, stable ordering, and clear cost. A trigger field is enough when it meets the use case; do not add a universal query API only for completeness.

**Pass:** List every interface used by A1. For each one, identify the domain-owned implementation, the SDK schema, and its Rhai and Rust access path. No production behavior is wired by a hardcoded check of the demonstration game's module name.

### A7. State, reload, and save behavior are explicit

Use the existing registered state and rollback system. State that influences future gameplay may not live in a Rhai global or a persistent WASM/native static. A failed invocation must commit no state writes or requests. A failed script or content update must not publish half a project generation.

Build a single project candidate for related scripts, schemas, and content, and activate it at the supported mechanical-edit boundary. Preserve the known last good generation on rejection. Document when an existing session must be reconstructed. A backend swap is **not** proof that live state can migrate across arbitrary code changes.

Demonstrate the save lifetimes this project advertises. Incompatible schema or save policies must refuse at admission instead of dropping state. Retire unused generations after they are no longer referenced.

**Pass:** A script and content edit succeed together or fail together. A rollback test covers populated module state. A save/load test covers one supported durable record. A deliberately incompatible replacement gives a useful refusal and keeps the usable old generation.

### A8. The build and delivery steps are real

Provide commands or noninteractive tools for:

1. Validate the project and inspect installed SDK capabilities.
2. Run it from a prebuilt engine with Rhai only.
3. Compile only the selected portable Rust module to WASM and run it with the same engine binary.
4. Build a release host with the selected Rust module linked in.
5. Package and run at least one self-contained desktop artifact without paths into the engine source checkout.

A module compilation is allowed in step 3; a host recompilation is not. Step 4 deliberately compiles the final executable. State the differences plainly. Test that the package contains the selected code, content, and assets. Check the game identity and module identity in the running artifact.

For browser and Android, report actual tested loader support and missing prerequisites. Do not claim those targets pass from desktop results. Reuse P5's asset-provider work when those targets are available; do not build a separate platform-specific gameplay system.

**Pass:** An independent directory can perform the five steps above using documented inputs. Capture which Cargo target or package was compiled for each step. No hidden source-checkout fallback is allowed for the packaged desktop artifact.

### A9. Performance-guided promotion

Measure the same mechanic and representative game workload in Rhai, WASM Rust, and linked Rust. Report invocation count, host and guest time, total simulation-tick cost, and memory or allocation behavior where observable. Include module build time, engine relink time, and edit-to-visible-result time separately. Measure a small frequent callback and a batched workload when both are relevant.

Do not assume WASM is faster than Rhai. The WASM backend starts every call from the module file's starting image (about 28 µs for the sentry against 10.6 µs in Rhai, 2026-11-28), so invocation overhead can dominate a small mechanic. Keep deterministic execution and bounded work. If a promoted implementation performs worse, explain the cause, change the implementation or batch size where justified, and state which backend is appropriate for that workload. Do not hide a regression behind a microbenchmark.

**Pass:** A repeatable benchmark or profiling recipe compares all three implementations on the same machine, inputs, and host profile. Its report states measured gains and costs. It does not label an unmeasured Rust or WASM implementation an optimization.

### A10. The authoring experience is coherent

Provide one short guide that starts with Rhai and then moves the same mechanic to portable Rust and linked Rust. It must explain project selection, module declarations, state, domain calls, outcomes, reload, build cost, and what is **not** available to portable code. Generate or derive API reference material from installed port descriptions where possible.

When a selected port is unavailable, when a candidate conflicts with another module, or when a build target lacks a prerequisite, the tool must explain what the author can change. Do not leave authors to read private engine code to understand an ordinary integration failure.

**Pass:** A developer can follow the guide without editing the engine. A reviewer can reproduce A1-A9 from the guide and test commands.

## Work order: complete one vertical slice at a time

1. **Normalize and inspect contracts.** Compare Rhai and Rust declarations, and report semantic drift. Reuse domain port descriptions. Do not rewrite the existing host.
2. **Make selection explicit.** Let a project choose a source script or WASM artifact for one module key. Preserve native fallbacks only through an intentional selection rule. Prove the unchanged host binary.
3. **Build the promotion fixture.** Port one Rhai mechanic to portable Rust. Run it as WASM and linked Rust against the same project data. Add A5's parity fixture now, not after expanding the SDK.
4. **Close actual SDK gaps.** Use the second game mechanic and its integration needs to add domain observations and requests. Remove fixed game-catalog assumptions exposed by the project.
5. **Finish project transactions and state.** Wire code, bindings, content, and save lifetimes through the existing generation owner. Test failure and replay.
6. **Measure, package, and document.** Compare the backends on the same workload, prove the clean external desktop build, and write the three-path guide. Then run applicable target checks and report limits.

Advance to the next step when its focused acceptance case passes. The sequence can change when the source demands it; preserve the gates.

## Engineering limits

- Keep **one simulation authority**. The domain owner decides whether a requested action applies.
- Keep **one portable procedural API**. Rhai, WASM, and linked Rust are backends of that API.
- Do not invent a second ECS, physics system, inventory, combat resolver, or rollback manager.
- Do not expose raw Bevy mutation through the portable SDK. Advanced Bevy plugins remain a separate engine tier.
- Do not require Rhai-to-Rust source translation or a common language syntax. Require common semantics and testable behavior.
- Do not hide engine rebuilds in the development loop. Measure the actual build work.
- Do not preserve a broken API only for pre-release compatibility. Migrate users and remove the obsolete path.
- Use focused, high-value acceptance tests. Do not run the full multi-hour test suite after each edit.
- Work in the assigned checkout. Do not create or switch worktrees. Follow `AGENTS.md` and commit attribution rules.

## Completion protocol for an autonomous agent

This is a multi-session implementation goal. Continue the work across normal agent sessions until **all A1-A10 gates pass**. Keep the repository buildable and commit logical vertical slices. Do not substitute a new roadmap or a series of speculative prototypes for the working result.

At each handoff, leave a compact status record with the gate ID, what passed, what remains, concrete test commands/results, and the next implementation step. Do not mark a gate complete from code inspection alone when it asks for an execution check. Distinguish missing toolchains from code failures. Do not block work on nonessential design questions; choose the smaller reversible design that preserves this contract.

The final handoff must contain:

- The example project and both versions of its promoted mechanic.
- The working Rhai, WASM, and linked-Rust run/build commands.
- The semantic declaration and gameplay parity test results.
- Last-good reload, rollback, save, and packaged-artifact evidence.
- The SDK and user documentation, known platform limits, benchmark evidence, and the commits.

**Failure condition:** If a developer still needs to modify game-specific Rust code in the host for an ordinary new mechanic, or must rewrite content and bindings to move that mechanic from Rhai to portable Rust, this goal is not complete.

## Acceptance receipt

Executed on Linux x86_64. Each gate below has runtime evidence. The
[authoring guide](../../../examples/projects/pulse_forge/AUTHORING.md) routes
through the [independent-directory commands](../../../examples/projects/pulse_forge/STANDALONE.md),
[example reference](../../../examples/projects/pulse_forge/README.md), and
[measured report](../../../examples/projects/pulse_forge/PERFORMANCE.md).

| Gate | Result and implementation | Guard or retained evidence |
| --- | --- | --- |
| A1 | Pulse Forge owns its world, two character rows, technique binding, emitter, and stateful group patrol. The stock runtime runs it without a compiler. | Five project gameplay tests; promotion fixture observes emitter effects and patrol motion and turns. |
| A2 | Scripts declare modules through `declare()`. File edits and membership changes use the existing project candidate and mechanical-edit publication. Invalid candidates keep the last good set. | Project gameplay tests cover edits, additions, removals, faults, and whole-set rejection. `project_state` observes paired script/content publication in the simulation. |
| A3 | Semantic declarations compare independently of code identity. Linked identity includes the running executable's digest (`25f88d451`). Copies keep it; a real native constant edit changes it. | 13 SDK, 46 host, and 3 Rhai tests passed. All 30 content module contracts match WASM. Changed field kinds and missing capabilities refuse with field or port diagnostics. |
| A4 | One portable Rust source builds as WASM and linked Rust. Manifest selection changes only the active emitter implementation. The patrol stays Rhai. | Stock-engine hash stays fixed across external Rhai/WASM runs. Independent linked release host matches semantic declaration `46370e7146c047f6`. |
| A5 | The same 280-frame input runs through all three backends, with and without populated GGRS rollback. Ordered requests, state, identities, costs, entity lifetime, projectile motion, and patrol motion agree exactly in this fixture. | Two `promotion` tests pass. A deliberately changed Rhai speed produces a structured diff. The guide states the float scope and tick origin. |
| A6 | Shared character grants (`fb341a507`), explicit empty optional catalogs (`e4593e3a4`), and domain-owned ports support the game without private game adapters. | The example reference lists every domain interface. The authored sentinel and keeper use the common body/control path. |
| A7 | One compatibility check covers live and saved schemas (`bb812e8d3`). Paired publication, durable body state, refusal, and old-generation retirement are exercised. | Two `project_state` tests pass, including a changing durable counter, fresh-start refusal, and incompatible readdition after unload. Forty-six host tests pass. |
| A8 | Export and byte verification use existing project, pack, library, and packaging owners (`31949354d`). The source-kit helper and manifest-aware module builder prove an independent workspace (`80290fbe1`). File probes include ordinary project inputs (`8702d6b50`). | All five standalone steps executed. The package passes 30 headless ticks and 600 no-window frames from `/`, without a compiler on PATH. |
| A9 | The actual workload is measured through all three backends (`9c1ccf62d`). Optional counters observe invocation boundaries without affecting state. | Eighteen processes run 50,400 measured ticks. Nine speed edits reach a projectile at 241 units/s. The report retains counts, host/backend time, full-step cost, memory, builds, and restart latency. |
| A10 | One short guide covers declarations, state, domain effects, outcomes, reload, promotion, build cost, saves, and capability limits. Port fields come from the generated reference. | Guide commands match the executed project, parity, state, build, packaging, and profiling paths. Documentation links pass. |

Final focused commands:

```bash
cargo test -p ambition_extension_host --lib
CARGO_INCREMENTAL=0 cargo test --release -p pulse_forge_host --no-default-features --features desktop_player --test promotion --test project_state -- --test-threads=1
cargo check -p ambition_app
python3 scripts/checks/check_absence_contracts.py --check
python3 scripts/checks/check_doc_links.py
python3 examples/projects/pulse_forge/profile.py --out target/pulse-forge-profile-final --samples 3 --cycles 10
python3 examples/projects/pulse_forge/measure_edits.py --samples 3
python3 scripts/package/package_project.py --project /tmp/pulse-forge-authoring --out /tmp/pulse-forge-release --verify-only --ticks 30 --presentation-frames 600
```

Results: 46 host tests, two release promotion tests, two release project-state
tests, the assembled app check, and all 53 absence contracts pass. The
workspace policy lane has 33 passes and two failures. Before the upstream
merge it reports eight findings: five dependency allowlist entries and three
direct body writes. The unchanged policy test binary reports ten findings on
the merged source: upstream `d9022dd41` adds two optional-catalog findings at
one group binding. No waiver was added. This is a focused lane; the full
workspace suite and visual acceptance were not run. Check logs use
`target/procedural-authoring-final-*` and `target/procedural-authoring-merged-*`.
The merged app check and all four release project tests pass again after
`af3ac0f6e`, including upstream's rollback schema 481.

The independent source workspace is `/tmp/pulse-forge-authoring`. The stock
engine SHA-256 stayed
`2af71b81af56bcf0d7dd7c59a0c1ac0726b8a99837baa3a694a41cac37e8b358`
through source and WASM runs. A forced WASM build compiled only the module and
public value dependencies. Its SHA-256 is
`7cdfa44fedf283fbf8304c510d8baa8d8fc212ae8d845c3f8494cecf5e844856`.

The verified desktop artifact is `/tmp/pulse-forge-release`. Its host SHA-256
is `2db3637166c3a019b0da754ed2b4a1de161b02beec027c808a1c441ed5e113dd`.
It contains 28,365 payload files. The broader input probe records 9,601 inputs
for 30 ticks and 10,394 inputs for 600 no-window frames. All observed game
inputs are inside the artifact. A focused external-script probe confirms that
ordinary source reads outside the artifact are visible to rejection. Logs use
`target/procedural-standalone-*` and `target/procedural-package-input-roots.log`.

Median complete-step costs are 648.9 us for Rhai, 740.8 us for WASM, and
629.5 us for linked Rust. Native and Rhai ranges overlap; a whole-game native
speedup is not established. WASM is slower for this small mechanic. Retain
four-projectile request batches and use Rhai for this edit loop. Warm module
edits build in 0.421–0.470 s; linked release-host edits build in 3.734–3.818 s.
Restart-to-observed results and the separate native probe build are stated in
the report. The [raw observations](../../../dev/journals/2026-11-17-procedural-backends.json)
retain the machine, toolchain, profile, hashes, commands, and all samples.

Standing limits:

- Browser and Android project loaders and native artifact identity are not
  established. SDK cross-target compilation proves neither loader.
- No-window verification loads presentation assets but draws no pixels.
- The package includes the full selected shared library, about 5.7 GB with
  its host and metadata. Asset reduction remains open.
- Selection changes require restart. Arbitrary live state migration is not
  supported. Transient records stay out of saves. Durable body state needs
  stable construction provenance; the current home avatar lacks it.
- Native linking is a deliberate host build. The measured restart loop is
  not live reload latency or visual readiness. Process RSS is not an isolated
  guest allocation count. No backend is an optimization merely because it is
  implemented in Rust.

## Owner documents and source locations

This goal refines, rather than replaces, the current owners:

- [Extension model](extension-model.md): the three tiers, one domain authority, and portable module contract.
- [Extension state and execution](extension-state-and-execution.md): state, ABI, identity, rollback, and limits.
- [Domain contracts](extension-domain-contracts.md): reads, requests, outcomes, and scope.
- [Runtime authoring execution plan](runtime-authoring-audit-2026-11-14/execution-plan.md): P2-P5 and their existing completion criteria.
- [Public SDK](public-sdk-1.0.md): independent consumer and supported profiles.
- [Project build and distribution](project-build-and-distribution.md): B6 and external packages.
- [Fast-iteration implementation](fast-iteration-implementation.md): I4-I7 and backend parity.

Implementation starts at `crates/ambition_extension_{sdk,host,rhai,wasm}`, `crates/ambition_platformer2d_runtime/src/extension_composition.rs`, `game/ambition_app/src/app/project.rs`, `game/ambition_content_modules`, and `examples/projects/pulse_forge`. The source on the current branch takes priority over this dated inventory.
