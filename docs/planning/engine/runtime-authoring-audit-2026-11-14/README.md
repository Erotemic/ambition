# Ambition runtime authoring and architecture audit

**Source:** `34f03976b639`, supplied on 2026-11-14.  
**Task:** static engine and game architecture review, with selective source traces.  
**Focus:** composition, compile cost, scripting, and an SDK for LLM-authored 2D platformers.  
**Status:** review and proposed execution order. Existing owner plans and `queue.md` remain the implementation authority.

## Main conclusion

Make a prebuilt engine host the normal game-authoring product. A game project should be a set of content packs, behavior source files, asset references, and launch settings that the host can prepare and run. Changing an ordinary game mechanic should not require a Rust toolchain, a native link, or edits to engine source.

The source already has much of the necessary architecture. Keep the content compiler, prepared generations, common body simulation, domain-owned requests, extension SDK, rollback records, and Wasm backend. The next change is to complete their author-facing boundary and make it the default workflow. A new scripting language alone cannot supply the missing body, identity, custody, result, and lifetime contracts.

Use **Rhai as the first scripting-backend implementation candidate**, behind the current `ModuleBackend` interface. Prove it on one existing stateful module before a wide migration. Luau remains the strongest alternative considered here for static author feedback, but its common Rust binding's documented browser target is not the engine's current browser target. The choice is based on integration fit, not a claim that Rhai is the fastest runtime or that its adoption is already complete. See [the language decision](architecture-and-product-design.md#3-language-decision).

The resulting promise is specific: game data and procedural policy use a fixed engine binary. New collision solvers, render passes, and fundamental services remain native engine work. The script interface must be broad enough that a new game algorithm does not need a new central engine opcode.

## Six priorities

| Order | Direction | Source finding | Required result |
| --- | --- | --- | --- |
| 1 | Make existing-runtime launch normal | `run_game.sh` invokes Cargo; the app build label watches workspace sources and Git metadata | Content and script edits run against an identified, unchanged host executable |
| 2 | Complete the script binding contract | Ports have Rust values and wire codecs, but no shared structural description for script conversion and discovery | One domain-owned port description supplies bindings, validation metadata, and machine-readable help |
| 3 | Add the first text-language backend | The host already admits loaded code and owns its state; no source-language adapter is present | A stateful sentry runs from source text through the current host and rollback path |
| 4 | Extend composition through real mechanics | Current hooks favor AI/specials; body forms and effects still contain fixed native policy | Controller-independent behavior, admitted body operations, stable references, and independent effect lifetimes |
| 5 | Finish project revisions and lifecycle | Data and code have separate staging paths; body save support is narrower than the schema vocabulary | A project publishes code, data, and schemas together; each advertised state lifetime has a real restore path |
| 6 | Reduce the remaining native build closure | Runtime omission does not remove mandatory Cargo edges; broad foundations contain host and view implementation | One useful minimal external profile has both correct behavior and a measured dependency cut |

The first three directions can produce an authoring result before the full native dependency refactor. Forms and effects are bounded parallel customers, not prerequisites for the first script invocation. The richer acceptance case is the existing Primeval swallow/coconut behavior. It forces the SDK to handle cooperation, control intent, item custody, stable identity, and results without giving scripts direct ECS mutation.

## Most consequential source findings

**Avoidable rebuilds.** `game/ambition_app/build.rs:18` watches every package source directory under `crates` and `game`. It also watches Git metadata and emits a current timestamp into `AMBITION_BUILD_LABEL`, consumed with `env!` by the app. This can invalidate the app for changes outside its selected dependency closure. Preserve binary identity, but separate it from live workspace and content identity. The timing cost remains unmeasured in this audit. [Build finding B01](build-and-dependency-audit.md#b01--the-app-build-stamp-creates-a-workspace-wide-invalidation-path)

**A working loaded-module path is still opt-in.** `load_developer_modules` returns when no module source is selected. The game normally registers native descriptors. The Wasm helper narrows a Rust build to the module and pure ports; it does not provide source scripting or eliminate Rust compilation. Its watcher also accepts a package name but watches the Ambition module tree even for Mary-O or Smash. [Content audit, section 2](content-and-scripting-audit.md#2-the-path-still-defaults-to-native-game-code)

**The script language must bind the existing authorities.** `ModuleBackend` is generic over execution, while the host already owns state and phase admission. Keep that split. Add structural port metadata and typed values instead of a second script-only damage, movement, or save system. [Content audit, sections 3 and 8](content-and-scripting-audit.md)

**Composition gaps are concrete.** `BodyMode::shape` fixes morph-ball size at `14 × 14`, and both collision-safe transitions and posed-body geometry consume that function. Speed Shoes use a single `Option<MomentumBoost>`. Existing capability contributions already demonstrate source ownership, but their native static keys are not a sufficient identity scheme for arbitrary effect instances. [Composition audit, sections 2 and 3](composition-audit.md)

**Runtime state has honest limits.** The extension schema has primitive values, options, and bounded sequences. It supports body/session attachments, but the reviewed persistence adapter supplies only session-durable records. A checkpoint enum case is not a working checkpoint path. Submission also precedes domain application, so a script must observe an operation's outcome before recording its success. [Content audit, sections 5 and 6](content-and-scripting-audit.md)

**The reusable reload API retains every distinct generation.** `EmbeddedPack::publish` uses `Box::leak` to support static row borrows; Mary-O and Smash use this path. Main Ambition already uses an App-local `Arc` selection. Extend that ownership model to the SDK rather than treating the leak as a property of all content reload. [Content audit, section 7](content-and-scripting-audit.md#7-reload-and-packaging-need-a-project-level-path)

## Existing work to preserve

The audit found implemented mechanisms, not only plans:

- A generic content compiler outside Bevy, with domain-owned schemas and reference validation.
- Runtime disk content and explicit static distribution modes.
- A dependency-free extension SDK and small module-author dependency closures.
- Wasmi execution, declared phases and ports, host-managed records, rollback registration, and checked module replacement.
- Shared physical movement through `step_body` and `step_motion`, with explicit frames and body authority.
- Collision-safe body-mode changes and generic authored traversal actions.
- Native portal geometry, ordering, and admission integrated with common body motion.
- An independent Rust SDK consumer, Outlander, with its own state and behavior.
- LDtk semantic tooling, asset producers, a command catalog, geometry inspection, and headless/replay tools.
- Existing compiler-cost measurement tools, mold, development-profile tuning, and aggregated integration tests.

Do not restart those projects. Close the authoring path that joins them.

## What the census establishes

The root manifest contains **112 workspace members**. The generated declared index and ECS inventory cover 111; the omitted member is an extension fixture, and the generated resolved graph includes all 112. The bounded source census finds **2,179 Rust files and 964,837 raw lines under member `src` directories**, including comments, blanks, and inline tests. Separate member `tests` directories have 475 files. The source selects **Bevy 0.19.1**.

The regenerated ECS inventory lists **1,334 registered systems**, 885 components, and 626 resources. These are static navigation counts. They do not describe the number of systems active in one scene or the cost of one frame.

The facade's mandatory, target-independent internal manifest closure already contains **70 packages including itself**. The analogous content CLI closure contains 42; the generic pack compiler contains two; the extension SDK contains one. These are intentionally incomplete lower bounds. They are not feature-resolved consumer measurements or compile-time estimates. See [method and source map](source-map-and-method.md) and [census data](build_census.json).

## Reading order

1. [Architecture and product design](architecture-and-product-design.md): the proposed product boundary, language decision, SDK, and competitive acceptance.
2. [Execution plan](execution-plan.md): six dependent work packets, owners, and behavioral completion criteria.
3. [Build and dependency audit](build-and-dependency-audit.md): rebuild causes, dependency limits, and targeted native work.
4. [Composition audit](composition-audit.md): body, form, effect, portal, and native SDK traces.
5. [Content and scripting audit](content-and-scripting-audit.md): current implementation, missing semantic interfaces, and Rhai binding details.
6. [Source map and method](source-map-and-method.md): scope, exact denominators, and source anchors.
7. [Primary references](primary-references.md): external language, Cargo, and Godot evidence.
8. [Implementation handoff](implementation-handoff.md): a bounded first task for the next implementation agent.

## Audit limits

This is a static architecture audit. It does not claim to have read every implementation file, run the game, compiled the workspace, or measured current build/runtime performance. The three detailed passes inspected selected production paths and their plans. All root workspace manifests were parsed. The included authoring submodules are populated; their full internals were outside the deep sample. Historical timing results are labeled with their original scope.

This overlay adds review documents and a reproducible analysis utility. It changes no engine, game, asset, build, or policy source.
