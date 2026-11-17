# Goal: first-class procedural game authoring

**Selection:** Maintainer goal, 2026-11-16. **Status:** Open until the acceptance gates below pass.  
**Command to an agent:** Complete this goal. Implement missing behavior, prove it in a real project, and commit the work. Do not stop after writing a plan.

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

Do not assume WASM is faster than Rhai. The WASM backend currently creates a new guest instance per call, so invocation overhead can dominate a small mechanic. Keep deterministic execution and bounded work. If a promoted implementation performs worse, explain the cause, change the implementation or batch size where justified, and state which backend is appropriate for that workload. Do not hide a regression behind a microbenchmark.

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

## Execution status

Contract inspection slice, Linux x86_64, debug host, 2026-11-16:

| Gate | Executed evidence | Remaining work |
| --- | --- | --- |
| A3 | Semantic comparison and refusal checks pass. The inspection tool matches all 30 linked content modules with WASM. Pulse Forge matches Rhai, WASM, and linked contracts. A real native rebuild at the same crate version changes its independently checked executable digest; a copy keeps it. | Browser and Android native artifact identity remains a loader limit in A8. |
| A4 | Pulse Forge's emitter runs as Rhai, WASM, and linked Rust. Both Rust paths use `portable/src/lib.rs`. Explicit selection keeps the same world, catalogs, and module key. Its second Rhai module remains admitted in every mode. The debug host and stock runtime were executed with no compiler on PATH. | Release-host build belongs to A8. |
| A5 | Two promotion tests pass. Six backend/rollback cases compare 280 input frames each. They compare initial and populated state, ordered requests, body and occurrence identities, cost admission, entity lifetime, projectile motion, and mana. A second release is refused for insufficient mana. Changed Rhai bolt speed produces a structured diff. | Keep this fixture when the authored actor and catalogs change. |
| A1, A6 | Pulse Forge owns its two character rows, starting-character rule, LDtk world, technique binding, and a stateful Rhai group patrol. The stock debug runtime loads it without a compiler. The parity fixture proves emitter effects, patrol steering, horizontal motion, and turn state. The guide lists the domain interfaces. | Self-contained delivery belongs to A8. |
| A2 | Project tests execute source edits, additions, removals, invocation faults, and failed whole-set reloads. The project publication trace records a repaired content/script pair reaching the running body together. | Keep the author guide and installed-port diagnostics current. |
| A7 | Project-state tests cover paired publication with a durable counter, old-backend retirement, body save/load, and refusal of changed attachment or field kind against live and saved records. Fresh starts refuse session activation with incompatible saves. The populated replay fixture remains green. | Keep these checks with changes to state or publication. |
| A8 | An independent source directory runs Rhai, builds only its WASM module, selects it on the unchanged stock engine, builds its linked release host, and packages its runtime files. Both package probes read game inputs only inside the artifact. | Desktop Linux is proved. Browser and Android loaders remain unavailable in this workflow. |
| A9-A10 | Existing foundation retained. These gates are not claimed complete by these slices. | Measure the three backends and finish the guide. |

Commands for this slice:

```bash
cargo test -p ambition_extension_sdk -p ambition_extension_rhai -p ambition_extension_host --lib contract
cargo test -p ambition_extension_rhai --lib an_independent_rhai_declaration
cargo build -p ambition_app_tools --bin extension_inspect
scripts/regen/build_extension_modules.sh ambition_content_modules
./target/debug/extension_inspect --compare target/extension-modules/wasm32-unknown-unknown/release/ambition_content_modules.wasm
./target/debug/extension_inspect --project examples/projects/pulse_forge --compare examples/projects/pulse_forge/modules/pulse_forge.rhai
cargo check -p ambition_app
```

The inspection executable SHA-256 was
`acef8cefe64643151d86d01746483fac0118d4fbe16067b8e6299fc618e043ba`.
The WASM SHA-256 was
`30d1fe400df2a575cbe87c4c4909b26702cd755163007b6634834c626c985798`.
Logs are under `target/procedural-contract-*`.

Selection slice checks:

```bash
cargo test -p ambition_extension_host --lib
cargo test -p ambition_app --lib app::project::tests
cargo test -p ambition_app --test app_it -- a_project_authors_a_technique
cargo check -p ambition_app
cargo build -p ambition_app_tools --bin extension_inspect
PATH=/nonexistent ./target/debug/extension_inspect --project target/procedural-selection-projects/rhai.ron
PATH=/nonexistent ./target/debug/extension_inspect --project target/procedural-selection-projects/wasm.ron
PATH=/nonexistent ./target/debug/extension_inspect --project target/procedural-selection-projects/linked.ron
```

Results: 46 host tests, 4 project tests, and 5 gameplay tests passed. The app
check passed. Each runtime manifest listed the same Sentry script and content
WASM artifact. Only its `implementations` choice changed. The manifest syntax
is in the [module recipe](../../recipes/writing-a-procedural-module.md).
Moving an identical selected artifact to another directory keeps generation
identity. Selection changes require a restart; selected-file edits still reload.

`cargo test -p ambition_workspace_policy` had 33 passes and 2 failed tests.
It reported nine violations outside this slice: four dependency allowlist
entries, three direct body writes, and two optional-catalog diagnostics. The full
diagnostics are in `target/procedural-selection-policy.log`. Other selection
logs use the same prefix.

The selection executable SHA-256 was
`296479b1b9fa00351df40b21227c4378a453876d5921847cd60b39bb29bcfb39`.

Promotion slice checks, Linux x86_64, debug host and release WASM:

```bash
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
cargo test -p ambition_combat_port -p ambition_extension_host --lib
cargo test -p ambition_app --test app_it -- a_project_authors_a_technique
cargo check -p ambition_app
cargo build -p ambition_app -p ambition_app_tools -p pulse_forge_host --bin ambition_game_bin --bin extension_inspect --bin pulse_forge_host
python3 scripts/checks/check_absence_contracts.py --check
```

Results: 2 promotion tests, 35 port tests, 46 host tests, and 5 project tests
passed. The app check and all 53 absence contracts passed. The generated
script reference includes the existing mana-cost port. The minimal consumer
lock was refreshed for the persistence dependency merged from upstream.
The policy check still had 33 passes and 2 failed tests. It reported ten
violations outside the promotion change, including the new upstream
persistence edge's allowlist. Logs use `target/procedural-promotion-*`.

The [example guide](../../../examples/projects/pulse_forge/README.md) gives
the exact build, copy, selection, and run commands. The stock executable ran
both authored-file modes for 30 ticks with `PATH=/nonexistent`. Its SHA-256
was unchanged between runs:
`b3d4b0333281671c271fdf5517e01064fa4030d442568b7badc5d102521a1c90`.
The linked example host also ran for 30 ticks and printed both native emitter
entries and the loaded Rhai sentinel. Its SHA-256 was
`814c4b7b4c0d2527a900bf04cb599311d34332abeff02d3d23eb0e6b9ee1e31d`.
The WASM SHA-256 was
`2465155e4fa0d304a740e34e0d30eee6d539f8cc94dadcfff97743f378986a66`.
The semantic declaration matched as `46370e7146c047f6`.

Complete traces are under `target/pulse-forge-parity`. The negative fixture's
diff is `target/pulse-forge-parity-negative/diff.json`. The guide states the
fixture's tick origin and exact float comparisons. These results do not
establish parity for other algorithms, profiles, or platforms. The next step
was the authored actor and an innate technique binding, through the character
catalog and the shared action scheme. The next receipt records that work.

Independent cast slice, Linux x86_64, debug host, 2026-11-16:

The example uses its own character catalog and an innate Projectile technique.
It has no Eye Laser item binding. The room places a tagged sentinel. Its Rhai
group rule reads room observations, keeps patrol state, and sends steering to
the common control owner. The fixture compares horizontal motion and two turns
alongside the emitter and mana refusal in all six backend/replay cases.
Character grants and retraction use the shared body path (`fb341a507`). An
explicit empty boss or cutscene catalog is valid (`e4593e3a4`). The rollback
schema is 477 because character technique declarations now change at runtime.

Focused checks passed: 20 action-scheme tests, 22 projection tests, 12 boss
schema tests, 7 cutscene schema tests, 112 Sanic tests, 74 Smash tests, 5 project
tests, 4 rollback-schema checks, 21 rollback coverage checks, and 2 group tests.
The app check and 53 absence contracts passed. Six diagnostic Smash probes and
two rollback probes were ignored. The workspace policy check had 33 passes and
2 failures, with eight findings outside this slice: five dependency allowlist
entries and three direct body writes. The group's required catalog resolves
its two earlier findings. Logs use `target/procedural-innate-*`.

```bash
cargo test -p ambition_cutscene -p ambition_boss_encounter --features ambition_cutscene/content_pack,ambition_boss_encounter/content_pack --lib content_schema
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
PATH=/nonexistent ./target/debug/extension_inspect --project examples/projects/pulse_forge --ticks 280
PATH=/nonexistent ./target/debug/ambition_game_bin --project examples/projects/pulse_forge/project-rhai.ron --headless-ticks 60
PATH=/nonexistent ./target/debug/ambition_game_bin --project examples/projects/pulse_forge/project-wasm.ron --headless-ticks 60
PATH=/nonexistent ./target/debug/pulse_forge_host --project examples/projects/pulse_forge/project-linked.ron --headless-ticks 60
```

The inspection run reports two patrol turns. All three prebuilt runs succeed.
The stock executable's SHA-256 stays
`7cb0474e9af45d3a570eb6e061190b75665f2ff1ff3a1b9683cd0a3f4570129e`
across the source and WASM runs. The linked host SHA-256 is
`68b971c66a433f856464fa95bd75eec0a4e7134a81df970cd6eaeab93c00d985`.
The WASM remains
`2465155e4fa0d304a740e34e0d30eee6d539f8cc94dadcfff97743f378986a66`.
The project still reads installed shared catalogs, tuning, and assets through
repository paths. A8 must remove those delivery paths. Native build identity
is still an A3 gap. The next acceptance case pairs a content edit with a script
edit and exercises a durable body record through save/load.

Project publication and durable record slice, Linux x86_64, debug host,
2026-11-17:

A character reload stages only the selected pack's cast. It no longer rebuilds
other providers' characters under the selected provider. The paired test reads
the catalog, live body tuning, and script request in one invocation. Its trace
contains only `(120, 120, 48)` and `(180, 180, 96)`, including a failed content
candidate and a repair of only that content. The replaced backend is released.

The optional `recipes/progress.rhai` counter uses the existing durable body road
on the authored sentinel. It saves at 61, loads and reaches 71, then continues
to 192 through two refused replacements. Changed attachment and changed kind on
field tag 1 both leave the live code and record usable. Compatible additions,
renames, and field reordering still migrate by tag. A changed kind on an
existing live tag now refuses instead of resetting its value.

```bash
cargo test -p ambition_extension_host --lib
cargo test -p pulse_forge_host --test project_state -- --test-threads=1
cargo test -p ambition_app --test app_it -- an_edit_reaches_the_shipped_game::an_edited_pack_reaches_the_cast_the_shipped_composition_plays
cargo check -p ambition_app
```

Results: 46 host tests, 2 project-state tests, the shipped reload test, and the
app check passed. Artifacts are `target/procedural-project-state/paired-trace.json`,
`save.ron`, and `save-observation.json`. Logs use `target/procedural-project-*`.
The guide states the durable identity requirement: the current home avatar has
no durable construction provenance and does not use this body save road. This
slice did not prove rejection of an incompatible saved schema at a fresh
session admission. The next slice adds that check.

Saved-schema admission, Linux x86_64, debug host, 2026-11-17:

One compatibility check now covers live replacements and saved rows. It
refuses changed attachment, save policy, or kind on an existing field tag.
It permits compatible additions, removals, renames, and field reordering.
Code replacement checks the active save before staging. Candidate session
admission checks its own frozen save before activation. Fresh headless starts
refuse activation with a useful diagnostic. A direct restore also checks saved
schemas before it applies state. The save file stays intact. Unknown module rows stay opaque.

The project fixture saves and loads an authored body's counter. It rejects
incompatible fresh starts and an incompatible module added after its live
schema has left. The paired content/script test also keeps this counter
active. Publication succeeds while the record changes each tick.

```bash
cargo test -p ambition_extension_host --lib
cargo test -p pulse_forge_host --test project_state -- --test-threads=1
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
cargo test -p ambition_platformer2d_provider --lib the_candidate_gate_refuses_rather_than_retiring_a_playing_session
cargo test -p ambition_app --test app_it -- a_durable_session_record_comes_back_with_its_save
cargo check -p ambition_app
cargo check -p ambition_app_tools --bin extension_inspect
```

Commit `bb812e8d3` adds saved-schema admission. Results: 46 host tests,
2 project-state tests, 2 promotion tests, the session gate test, the existing
durable-session save test, both compile checks, documentation links, and all
53 absence contracts passed. Workspace policy had 33 passes and 2 failures
with the same eight findings listed in the earlier character slice. No waiver
was added. Logs use `target/procedural-save-final-*`. The saved-schema artifact
is `target/procedural-project-state/save-admission-refusals.json`. Native build
identity, release packaging, backend measurements, and platform loader checks
remain.

Native build identity, Linux x86_64, debug host, 2026-11-17:

Commit `25f88d451` binds linked code to the running executable's BLAKE3 digest,
read once before simulation. Crate name and version alone could not distinguish
two native builds. Descriptor identity includes the build digest; semantic
comparison excludes it. Portable declarations keep `build: None`.

The artifact probe copied the same executable to two paths and checked each
reported digest against an independent file hash. It then changed only
`BOLT_SPEED` from `240.0` to `241.0` in `portable/src/lib.rs`, rebuilt with the
same command and version `0.1.0`, and repeated inspection. The digest changed
from `19abf154906767ad45b7cf3885525472170834c0dc9bbbf8da867f3883dfdd0b`
to `6741ae97a4d4ef1692b7b35606145394fa7fe7146f8e3238424044437863b288`.
Both match semantic declaration `46370e7146c047f6`. Restoring the source
reproduced the original executable bytes. Each edit/relink took about 2.9 s
on this warm target. This is build evidence, not a gameplay performance result.

```bash
cargo test -p ambition_extension_sdk -p ambition_extension_host -p ambition_extension_rhai --lib
cargo build -p pulse_forge_host -p ambition_app_tools --bin pulse_forge_host --bin extension_inspect
PATH=/nonexistent ./target/debug/pulse_forge_host --project examples/projects/pulse_forge/project-linked.ron --headless-ticks 0 --compare examples/projects/pulse_forge/build/pulse_forge.wasm
PATH=/nonexistent ./target/debug/extension_inspect --compare target/extension-modules/wasm32-unknown-unknown/release/ambition_content_modules.wasm --ticks 0
cargo test -p pulse_forge_host --test promotion -- --test-threads=1
cargo check -p ambition_app
cargo check -p ambition_extension_sdk --target wasm32-unknown-unknown
cargo check -p ambition_extension_sdk --target aarch64-linux-android
```

Results: 13 SDK tests, 46 host tests, 3 Rhai tests, both promotion tests, and
the compile checks pass. All 30 content module contracts match. The linked
host refuses a changed field kind with status 1 and a field path. Workspace
policy has the same eight findings: 33 tests pass and 2 fail. The consumer
lockfiles now include the host's hash dependency; all 53 absence contracts
pass. Logs use `target/procedural-native-*`; artifact observations are in
`target/procedural-native-builds/observations.json`. Browser and Android host
loaders still need native artifact identity. The SDK cross-target checks do
not prove either loader. Release delivery and backend measurements remain.

Desktop delivery, Linux x86_64, release without incremental compilation,
2026-11-17:

Commit `31949354d` exports declared project, content, world, module, and library
inputs through their existing owners. It verifies the exported catalog against
the original. The package tool uses the existing byte contract and file-open
probe. Embedded installed content can still reload an external project's
paired content and scripts. Default shell libraries no longer override the
project's selected library.

The [standalone guide](../../../examples/projects/pulse_forge/STANDALONE.md)
contains the five executed steps. `/tmp/pulse-forge-authoring` has its own
Cargo workspace, module source, host source, and runtime files. The SDK source
is an explicit Rust build input. The workspace copies the SDK's dependency
patches. Runtime loading needs no SDK source.

The stock engine ran Rhai and WASM with `PATH=/nonexistent`. Its SHA-256 stayed
`2af71b81af56bcf0d7dd7c59a0c1ac0726b8a99837baa3a694a41cac37e8b358`.
A forced WASM build compiled only the module and public value dependencies.
The module SHA-256 is
`7cdfa44fedf283fbf8304c510d8baa8d8fc212ae8d845c3f8494cecf5e844856`.
The independently built native host matches semantic declaration
`46370e7146c047f6`. It keeps the Rhai patrol active.

`/tmp/pulse-forge-release` contains 28,365 payload files. Its host SHA-256 is
`2db3637166c3a019b0da754ed2b4a1de161b02beec027c808a1c441ed5e113dd`.
Both the temporary package and final package pass byte and identity checks.
From `/`, with no compiler on PATH, 30 simulation ticks read 9,568 game inputs;
600 no-window frames read 10,361. All are inside the artifact. The presentation
probe loads assets but draws no pixels. The package includes the whole selected
shared library, about 5.7 GB with its host and metadata; asset minimization is
not implemented. Browser and Android project loaders and native artifact
identity are not proved by this desktop result.

Focused checks pass: two release project-state tests, two release promotion
tests, the app check, 53 absence contracts, documentation links, shell syntax,
and Python compilation. The prior policy lane has 33 passes and two failures
with the same eight findings outside this slice. Logs and Cargo artifact lists
use `target/procedural-standalone-*` and `target/procedural-package-final-*`.
Release builds use `CARGO_INCREMENTAL=0`; an earlier incremental build is not
performance evidence.

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
