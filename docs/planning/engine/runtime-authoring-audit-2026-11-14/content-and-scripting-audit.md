# Content, extension, and scripting audit

Source snapshot: `34f03976b639`, packaged 2026-11-14. This is a static review. No build, test, game, or timing experiment was run. Source paths and line numbers below refer to that snapshot. The generated navigation inventory was used to locate owners; implementation claims were checked in source.

## Main result

Ambition already has most of the hard architectural boundary that a runtime scripting language needs. It has a dependency-free procedural SDK, a host that owns extension state and admits declared ports, a working WebAssembly interpreter backend, a code replacement protocol, and migrated production game mechanics. Adding another general scripting authority or a second ECS would discard this work.

The next step should make that boundary the normal authoring path. Add one text-language binding to it; make game projects loadable by a prebuilt host; make port values machine-described; and expand the available behavior, identity, and persistence seams using mechanics that the present boundary does not express well. A faster Rust-Wasm loop is useful, but it is not the user's requested workflow with no external compiler operation.

The strongest next acceptance customer is the existing swallow/coconut mechanic, not another projectile fan. It requires coordinated actors, item custody, stable references, control intent, and state. Those are the seams that determine whether the SDK can support rich custom platformer mechanics.

## 1. Existing work to preserve

| Area | Source evidence | Consequence |
| --- | --- | --- |
| Pure content compiler | `crates/ambition_content_pack/src/lib.rs:1-6,47-60`; schema resolution starts at 63; capability validation at 118 | Content already parses, validates, resolves, canonicalizes, and fingerprints outside Rust compilation. Do not propose this as new work. |
| Extensible content schemas | `crates/ambition_content_pack/src/schema.rs:232-267` | Capability owners install schema handlers. There is no need for a central enum of every authored mechanic. |
| Dependency-free procedural SDK | `crates/ambition_extension_sdk/Cargo.toml:1-13` | Module algorithms can avoid Bevy, the engine implementation, and game host dependencies. Preserve this property. |
| Portable invocation contract | `crates/ambition_extension_sdk/src/invoke.rs:1-10,176-268` | An invocation reads its trigger and declared observations, changes its declared state, and stages requests. The engine retains domain authority. |
| Loaded backend seam | `crates/ambition_extension_host/src/admission.rs:39-50,61-74` | `ModuleBackend` accepts module/entry indices and input bytes, then returns output bytes. A script backend can use this seam without changing gameplay ownership. |
| Real Wasm backend | `crates/ambition_extension_wasm/src/lib.rs:65-118,148-170` | Wasmi loads bytes, rejects imports, provides fuel, and instantiates each invocation. This is real execution, not a placeholder. |
| Host state and rollback | `crates/ambition_extension_host/src/store.rs:1-17,35-43,98-115`; `crates/ambition_platformer2d_runtime/src/rollback/mod.rs:91` | Body and session records are canonical state. They participate in existing rollback; there is no separate script snapshot system. |
| Explicit phases | `crates/ambition_extension_sdk/src/module.rs:57-120`; `crates/ambition_platformer2d_runtime/src/extension_composition.rs:118-237` | Script callbacks can inherit existing timing contracts. Their meaning must stay tied to facts available and outcomes consumed in each phase. |
| Safe module replacement | `crates/ambition_extension_host/src/reload.rs:84-167,170-247` | Replacement re-admits the composition, retains the last valid generation on failure, migrates fields by tag, and removes records of departed schemas. |
| Game content migration | `game/ambition_content_modules/src/lib.rs:51-91` | Thirty module constructors cover specials, wielded mechanics, entities, and boss conductors. The algorithms already use the portable boundary. |
| In-game content refresh | `game/ambition_content/src/content_watch.rs:75-170`; `game/ambition_content/src/reload.rs:1640-1750` | A disk change uses the shared compiler and the route preparation transaction. This is much more than a raw file watcher. |
| Text inspection | `crates/ambition_extension_host/src/inspect.rs:16-95`; `game/ambition_app_tools/src/bin/extension_inspect.rs:67-128` | The current inspector already reports installed ports, entry order, code selection, generation, records, and replacement admission. Extend it instead of starting another diagnostic tool. |

The current extension plan is consistent with the user's new direction. `docs/planning/engine/extension-model.md:222-243` already distinguishes offline authoring and simulation scripting and names Lua/Rhai candidates. The prior plan waited for a scripting customer. This request supplies one.

## 2. The path still defaults to native game code

### Observation

The application enables the Wasm module feature (`game/ambition_app/Cargo.toml:203`). However, `load_developer_modules` returns without loading anything when neither the `ExtensionModuleFiles` resource nor `AMBITION_EXTENSION_MODULES` is supplied (`crates/ambition_platformer2d_runtime/src/extension_composition.rs:294-303`). The content plugin registers native module descriptors from `ambition_content_modules::modules()` (`game/ambition_content/src/bosses/specials/mod.rs:80-93`). The module crate explicitly says a linked edit rebuilds the host (`game/ambition_content_modules/src/lib.rs:8-11`).

Thus backend availability and normal use are separate. A developer who follows the ordinary host run without the module selection still pays native invalidation for a procedural edit. The no-host-relink route exists, but it is an opt-in developer overlay over statically linked game code.

The build helper runs `cargo rustc --target wasm32-unknown-unknown --release --crate-type cdylib` (`scripts/regen/build_extension_modules.sh:35-44`). This narrows the build closure. It still requires a Rust toolchain and a compile action for an ordinary behavior change. The new requirement is stronger.

### Recommendation

Make a project manifest the normal development entry point. It should identify the content pack, behavioral source modules, engine profile, dependency lock, and entry scenario. A prebuilt host must read it. Native linking can remain a reference and distribution option, but should not be the default content-author workflow.

Do not claim that a single prebuilt host supports a new native capability without an engine build. The product target is that ordinary custom game mechanics can combine a sufficiently rich installed platformer vocabulary and script algorithms. A change to the collision solver or to an engine service remains engine development.

### Concrete tool defect

The watch helper accepts a package argument at line 29 but always watches `game/ambition_content_modules` and a fixed list of port crates at lines 51-54. A run with `--watch ambition_demo_mary_o_modules` does not watch that module's own directory. This is a small repair with direct iteration value. Derive its source closure from the selected module's manifest or build dependency inputs; do not maintain another list per game.

The helper documentation also uses `cargo run -p ambition_app` at line 9. Even if this produces a no-op build, it does not communicate the prebuilt-host contract. The normal author-facing launcher should run an existing binary and report a missing engine installation explicitly.

## 3. Ports need one machine-readable value schema

### Observation

`Port` provides `KEY`, `ROLE`, a Rust `Value` type, and hand-written wire encode/decode functions (`crates/ambition_extension_sdk/src/port.rs:63-75`). `PortOffer` adds role, phase, and owner (`crates/ambition_extension_host/src/admission.rs:118-127`). The code has good prose port cards. It does not expose a generic description of every port value's fields, enum cases, units, references, or constraints through this contract.

For example, `ActorThink` repeats its field shape in a Rust struct and manual codec (`crates/ambition_combat_port/src/think.rs:25-67`). The current extension inspector can state that a port exists, but cannot emit its full value schema from `PortOffer` (`crates/ambition_extension_host/src/inspect.rs:29-37`).

A script binding built by hand on top of these types would add another representation. Rust values, binary codecs, script object conversion, documentation, completion data, and CLI examples would then drift separately. This is a direct LLM authoring cost: the agent must inspect source to learn the accepted record shape and discover failures late.

### Recommendation

Add a domain-owned description for port payloads. Use one declarative Rust source or derive to generate the wire codec, reflection descriptor, script conversion, type metadata, and CLI schema. Preserve explicit domain semantics in the port card: scope, coordinate frame, time, refusal, and result meaning. Derivation should remove encoding duplication; it cannot infer those semantics.

The core SDK needs only the common value vocabulary and schema interfaces. Each domain owns its payload types. The script adapter registers selected domain bindings. Do not place all ports in a central enum or import every domain into the dependency-free SDK.

A useful schema must describe:

- exact numeric kinds and finiteness/bounds;
- named records and sum types;
- sequences and their bound/order semantics;
- stable content or instance references;
- spatial units and frames where they affect correctness;
- required versus optional values;
- effects or requests that are submitted, applied, refused, or deferred;
- installed version and legal phases.

Use the same descriptions for an author-facing `describe` operation, diagnostics, examples, and script bindings. This is infrastructure with a concrete consumer, not a generic schema campaign.

## 4. The current procedural surface is shaped by its first mechanics

### Observation

The current runtime installs seven named phase families: technique execution, wielded use, module entity tick, boss conduct, actor think, hit reaction, and body contact (`crates/ambition_extension_sdk/src/module.rs:57-120`). This is a useful set, and `Phase` is a namespaced value rather than a closed enum. A new phase still requires native scheduling and adapters; a script cannot invent the facts and barriers that give a phase meaning.

The `actor_think` trigger serves custom AI. It selects `CharacterBrain::Custom`, excludes `PlayerEntity` and `DrivingParticipant`, and runs before the normal control gate (`crates/ambition_combat/src/extension/think.rs:42-58,74-107`). Excluding driven bodies is appropriate for an AI controller that must yield control. It becomes a limitation when an author needs attached physical behavior that should continue under possession or human control.

The `body_contact` trigger also uses `CharacterBrain::Custom` as its binding selector. It excludes driven bodies and considers only `PlayerEntity` touchers (`crates/ambition_combat/src/extension/contact.rs:75-85,101-133`). This ties a body interaction rule to a brain choice and player marker. It does not express a controller-independent contact mechanic between arbitrary actors.

`ActorThink` provides position, velocity, size, scalar facing, on-ground state, health, target position, and placement ID (`crates/ambition_combat_port/src/think.rs:25-38`). It does not provide the gravity, acceleration, or body frame that a general traversal script would need. This matters in an engine where world-relative and body-relative movement deliberately differ.

The current custom brain requests include fire and velocity (`crates/ambition_platformer2d_runtime/src/extension_composition.rs:70-72`). A general controller intent interface is more useful than adding another isolated request each time a new AI needs jump, crouch, aim, posture, or interaction.

### Recommendation

Separate three roles in the public SDK:

1. A controller produces semantic intent and yields when another controller owns the actor.
2. A body behavior continues according to its authored attachment and capability policy, regardless of controller kind.
3. A domain rule observes shared facts and submits requests to the domain that owns the outcome.

Bindings should identify an authored behavior attachment, not overload the AI brain key. Composition must allow several behaviors on one actor, with explicit state namespaces and request arbitration. Keep the same native movement, contact, damage, possession, and custody authorities.

For spatial APIs, expose a small immutable body frame/read model, typed frame conversion helpers, and selected geometric queries. Do not make scripts infer gravity from presentation rotation or reimplement collision sweeps in script. A custom movement modifier should submit intent or a defined movement contribution at the appropriate phase. A new physical constraint should use a native service with script-defined parameters or policy.

Do not solve limited expressiveness by exposing raw `World`, mutable components, or unbounded arbitrary ECS queries. That would make the script language the new ownership mechanism and duplicate the engine's existing guarantees. Supply semantic, bounded queries that meet actual mechanic needs.

## 5. State is rollback-aware, but its authoring and persistence vocabulary is narrow

### Observation

The state schema can express `Bool`, `U32`, `I32`, `U64`, finite `F32`, `Vec2`, optional values, and bounded sequences (`crates/ambition_extension_sdk/src/schema.rs:86-100`). Its attachment is either body or session (`schema.rs:62-72`). New loaded descriptors can provide new schemas and entries; there is no fixed native type roster required for each new state record.

This already supports useful dynamic gameplay state. It is not yet a convenient schema for nested records, enums, stable references, or strings. Those can be encoded manually into primitive sequences, but that moves schema meaning out of the host and makes diagnosis, migration, and authoring worse.

The host can retain transient body or session state. For saved state, `SaveRoad` currently contains only `SessionDurable` (`crates/ambition_extension_host/src/admission.rs:18-35`). A schema that requests another attachment/save combination is refused (`admission.rs:442-469`). `Checkpoint` exists in the SDK enum, but its presence does not establish a working checkpoint road. `crates/ambition_platformer2d_runtime/src/module_save_road.rs:1-18,28-42` implements the actual durable session road.

The host snapshots `BodyRecords` and `SessionRecords` with deep clones of logical values (`crates/ambition_extension_host/src/store.rs:1-17`). Invocation staging reads only declared records (`exec.rs:349-350`), so it does not clone the entire world store for each call. The current storage is a sound baseline. Large nested world ledgers would need measured scaling work before adoption.

### Recommendation

Extend state kinds in response to concrete mechanics. Start with explicit stable instance/content references and nested record/enum forms. Keep field tags stable. Keep finite numeric checks, bounded containers, and canonical ordering. Do not serialize a language heap as the default save or rollback format.

Add body or persistent-occurrence save participation through the existing persistence authority before promising that all script state survives room unload or save/load. Define whether a behavior is attached to a live body, persistent occurrence, room, or session. A body record that retires with its live entity is different from a persistent object's record. This lifetime distinction must be in the SDK and inspect output.

Treat numbers deliberately. A script language's default number is not a safe encoding for every `U64` ID or record field. Use opaque typed references and exact host conversion. Rhai can use custom Rust values; do not force stable IDs through floating point. The simulation math profile should define f32 behavior, finite input/output checks, transcendental functions, integer overflow, and deterministic RNG. The current invocation API gives tick, dt, occurrence, trigger, observations, state, and requests (`invoke.rs:176-268`); it does not itself supply a random stream. Supply an explicit stream/state operation rather than a hidden VM-global RNG.

## 6. Requests are not transactionally applied outcomes

### Observation

The host validates an invocation's output, commits its own state, and then queues requests (`crates/ambition_extension_host/src/exec.rs:349-390`). The phase is collected, invoked, cost-settled, lowered, and retired in order (`crates/ambition_extension_host/src/lib.rs:400-434`). `submit` returns success when a request is staged (`crates/ambition_extension_sdk/src/invoke.rs:247-268`). A domain can later refuse an invocation's remaining requests (`crates/ambition_extension_host/src/exec.rs:176-182`).

This does not mean that arbitrary engine effects and script state share one global transaction. The local state may legitimately record an attempt. A downstream refusal does not rewind that state. The current API documents submitted-versus-applied, so this is not by itself a correctness defect.

It is an authoring hazard if an LLM reads `submit` as proof that a teleport, spawn, pickup, resource spend, or strike happened. Complex mechanics need the next decision to depend on the outcome. Logging a refusal is not always enough.

### Recommendation

Expose structured result observations for operations whose outcome affects later script state. Make the timing explicit: same phase only if the domain supports it, otherwise a named later phase or next tick. Include stable request identity or occurrence identity where correlation is required. Use existing domain outcome facts rather than adding a universal immediate-effect API.

Do not market an entry as all-or-nothing beyond its actual guarantee. Its staged writes and requests are discarded on an entry fault. Accepted requests still obey domain ownership, cost settlement, and conflict rules. Explain this directly in generated SDK documentation and show examples that record attempted versus completed actions.

## 7. Reload and packaging need a project-level path

### Observation

The procedural loader is currently filesystem-oriented. It enumerates Wasm files from supplied paths/directories once at plugin build, reads them with `std::fs`, then polls known files every twenty frames (`crates/ambition_platformer2d_runtime/src/extension_composition.rs:273-366,376-439,502-523`). New files in a watched directory are not discovered by the later poll, which iterates the existing file list. The path-list format splits on `:`, which is not a portable project declaration.

These paths establish native local development. Wasmi's ability to compile for another target does not prove the browser or packaged Android module-loading road. This audit found no package loader for extension modules in the sampled packaging and app paths. Report target delivery as unproved here, not as impossible or absent from the entire repository.

Code reload and content reload have separate staging paths. Code replacement uses `StagedModuleReplacement`, `PendingMechanicalEdits`, and `remint_session_content` (`extension_composition.rs:428-499`). Pack reload uses `CandidateGeneration`, `PendingGeneration`, and route re-preparation (`game/ambition_content/src/reload.rs:1640-1750,1930 onward`). The plan calls for one code/data/schema generation (`docs/planning/engine/extension-model.md:245-261`). The reviewed source does not establish a single author-selected revision spanning a set of changed script files and a changed pack. Treat coordinated publication as an audit gap to close, not a demonstrated race or corruption bug.

Module replacement is already careful: it removes the old artifact's exported modules, composes pending replacements, re-admits, migrates records, and respects the mechanical-edit boundary (`crates/ambition_extension_host/src/reload.rs:75-167,189-247`). Reuse it.

### Recommendation

Make source discovery and project revision explicit. The project manifest lists script modules and imports. A changed revision consists of content, behavior source, schemas, engine profile, and binding/runtime version. Prepare it off the simulation path, then publish one admitted generation through the existing boundary. Invalid revisions keep the running generation. Inspect output should identify source digest, prepared generation, admitted generation, and currently active scenario separately.

Use a byte-oriented artifact provider over the asset/publication system for native files, embedded packages, Android assets, and browser fetch. Script parsing may happen when a source revision loads; an author should not run Cargo or a separate compiler. Cached AST/bytecode is an implementation detail keyed by the entire semantic generation and runtime profile.

Preserve remote-session generation pinning. Development code replacement must not invalidate an active shared rollback history silently. Use the existing policy for local rebase or scenario reconstruction and report which outcome occurred.

### SDK demo generation lifetime

The facade's `EmbeddedPack` retains its live generation as `&'static PreparedContentPack` (`crates/ambition_platformer2d/src/lib.rs:130-144`). Its `publish` function uses `Box::leak` for each new generation (`lib.rs:440-442`). The demo watcher calls this publication at `content/live_reload.rs:612`. Mary-O and Smash install that watcher (`game/ambition_demo_mary_o/src/lib.rs:1179`; `game/ambition_demo_smash/src/lib.rs:2127`). Mechanically unchanged saves are filtered, but each distinct published revision stays allocated until process exit. The source shows no retention cap. Existing static row borrows stay valid, which explains the implementation, but also leaves old and new generation views in the process.

This finding must not be generalized to the main game's pack selection. `game/ambition_content/src/pack.rs:377-398` has a one-time boot `Arc` and an App-local `SelectedContentPack(Arc<PreparedContentPack>)`. `install_selection` updates the App resource and identity together (`pack.rs:480-494`), and `selected` reads that App's resource (`pack.rs:519-523`). The main game already has a better ownership seam.

Bring the reusable SDK/demo path to explicit App/session-owned generation handles. Return owned handles or borrows tied to a retained generation instead of promising arbitrary static references to reloadable rows. Readers that need old definitions can retain an `Arc`; reclamation then follows actual owners. This is a lifetime and multi-project composition improvement needed by a long-running prebuilt authoring host. It is not a measured memory bottleneck or a claim of an observed mixed-generation gameplay failure.

## 8. Recommended first language binding: Rhai on the existing backend

This is a recommendation, not an implemented capability or measured performance result. [The integrated language decision](architecture-and-product-design.md#3-language-decision) compares the alternatives and target constraints.

### Why the current boundary fits

`ModuleBackend` is already independent of Wasm and requires `Send + Sync` (`crates/ambition_extension_host/src/admission.rs:43-50`). A Rhai backend can own immutable prepared ASTs, module descriptors, binding schema, and entry lookup. It decodes the existing invocation, creates fresh mutable invocation context, evaluates the selected function, validates returned state/requests, and returns the existing output encoding. No change to simulation scheduling or rollback ownership is necessary for the first implementation.

Rhai's official `sync` feature makes `Engine`, `Scope`, and `AST` `Send + Sync`. Its options include `f32_float` and `no_time`. Its documented evaluation model begins each invocation without retained global state unless a Scope is deliberately reused. This is a better fit for host-managed records than a design whose default is one mutable VM object per actor. Sources: [Rhai features](https://rhai.rs/book/start/features.html) and [Scope](https://rhai.rs/book/engine/scope.html), checked 2026-11-14.

This does not make arbitrary Rhai code deterministic automatically. Registered Rust closures, loaded module variables, shared custom values, mutable function captures, random sources, and numeric functions can retain or introduce state outside the host records. The adapter must make those choices explicit. Keep immutable compiled code shared; make every mutable gameplay value invocation-local or host-owned. A fresh Scope is necessary but is not a proof if registered values contain shared mutable storage.

`Engine::call_fn` normally evaluates the AST before calling the entry and removes newly introduced scope values afterwards. `call_fn_with_options` permits skipping AST evaluation. Do not select that optimization without an explicit immutable import and initialization model. Source: [Calling Rhai functions](https://rhai.rs/book/engine/call-fn.html), checked 2026-11-14.

Current Rhai docs also list an optional Grain bytecode VM. Therefore avoid the claim that Rhai can only interpret ASTs. The initial integration should use the smallest stable API that proves the gameplay contract; choosing a different VM is later measured work, not a condition for the first script.

### Preparation contract

At source-load time:

1. Resolve a module and its imports within the selected project revision.
2. Parse the source; compile it to an internal representation without an external toolchain.
3. Read or derive descriptors for entry functions, trigger bindings, state schemas, and declared ports.
4. Validate value kinds, installed port versions, legal phases, schema ownership, attachment/save support, and ordering.
5. Retain source positions so syntax, admission, and runtime diagnostics point to the author's file and field/function.
6. Give the prepared module a code identity containing source/import digest, interpreter version, numeric profile, binding schema versions, and budget policy.
7. Submit it to existing generation admission and replacement.

A script must not have to declare every low-level glue detail manually. It should name semantic hooks and typed operations; the preparation layer can derive read/request requirements from registered calls or give precise static diagnostics. When a request set cannot be inferred soundly, use an explicit compact manifest. Do not infer permission from merely observing which branch happened to run during a sample invocation.

### Execution contract

Keep the current phase/read-cut semantics. A script gets immutable facts and explicit state, then emits requests. Each entry runs against a prepared program. Domain effects are lowered by existing native owners. No Rust code generation, Wasm compilation, or relink belongs in the normal edit path.

Use a deterministic operation limit. Rhai documents that a host Rust call counts as one operation even when it does substantial work. Therefore an interpreter instruction count alone does not bound a spatial query or a request that processes many bodies. Add deterministic result-count/work bounds at those semantic operations. Avoid wall-clock cutoffs for authoritative decisions. Source: [Rhai maximum operations](https://rhai.rs/book/safety/max-operations.html), checked 2026-11-14.

Do not save Rhai Scope directly. In particular, the official Scope documentation says custom values are not serialized as their full custom data through that convenience route. Serialize the engine's state records. The host already has the authoritative shape and lifetime.

### Current cost risks to measure

The host currently checks each admitted entry against pending invocations in its phase (`crates/ambition_extension_host/src/exec.rs:332-337`). That is an entries-times-pending matching loop. Loaded calls allocate/encode trigger and observation buffers before execution (`exec.rs:467-490`). Wasmi instantiates a store, linker, instance, and memory for every call (`crates/ambition_extension_wasm/src/lib.rs:108-118,148-165`). None of these is measured as a present bottleneck by this audit.

A script backend should first preserve the exact semantics. Then measure many small actor callbacks and batched callbacks separately. Cache an index of admitted entries by `(phase, trigger, selector)` at admission if matching cost is material. Reuse immutable read projections within a phase when scope and schema permit it. Consider a typed in-process invocation adapter after the byte-based binding works, if buffer conversion is material; both paths must retain the same host validation and authority.

Fresh mutable state does not require rebuilding all immutable interpreter infrastructure every invocation. Conversely, pooling a VM is not correct if hidden globals or captures survive a rollback. The implementation must prove that reuse changes allocation cost without changing persistent state.

## 9. Vertical samples and next packets

### Existing sample: sentry

`game/ambition_content_modules/src/sentry.rs` is a complete small scripting customer. The use entry spends mana and spawns a module-owned turret. The tick entry owns cadence through a schema-backed record, reads the entity trigger, and requests a projectile and sound. State is scoped to each turret (`sentry.rs:38-68,105-140`). Domain code owns identity, side, room, lifetime, and projectile execution. A script translation should preserve all of this.

This is a good first adapter proof. It is not enough to prove the full platformer authoring product, because its lifecycle and operations already fit existing ports.

### New expressive sample: swallows and coconuts

`game/ambition_content/src/primeval.rs` remains native game behavior. It uses shared item-custody operations and normal ActorControl (`primeval.rs:3-16,103-110`). It stores `FetchFlight { goal: Option<String>, phase, elapsed }` and registers its rollback participation by hand (`primeval.rs:54-60,460-469`). Its plugin schedules the body decisions after actor publication and before the control gate (`primeval.rs:472-488`).

Porting this requires meaningful SDK work: stable item/actor references, item read projections and custody requests, outcome observation, control-intent submission, and convenient state schemas. It should not require giving scripts direct component writes or inventing a swallow-specific core command. Keep the algorithm and its named tuning in the game project. Reuse domain custody, capacity, cooperation, and control rules.

### Recommended packet order

| Packet | Concrete result | Evidence to require |
| --- | --- | --- |
| A. Project source modules | A prebuilt host opens a manifest, reports loaded/native backend, and discovers changed/new/removed modules | One game runs from a project path with no Cargo in the edit loop; invalid source keeps previous code |
| B. Port schema and Rhai binding | Existing sentry runs as text source under the existing extension host | Independent turrets retain independent cadence; fault discards staged output; rollback uses host records |
| C. Body behavior and reference seams | Behavior attachments are independent of controller selection; stable references and frame-aware views are available | Same custom body behavior survives controller/possession change and behaves in turned gravity |
| D. Coordinated mechanic | Swallow/coconut behavior moves out of native game code through shared custody and control APIs | Carry/release/failed pickup/cooperation outcomes remain correct without native game edits |
| E. Persistent script state | The selected live/persistent attachment has the promised checkpoint and durable save roads | Save, room unload/reload, and rollback each preserve the correct distinct lifetime |
| F. Unified revision and target loading | Content plus code plus schemas publish as one project generation, through native/browser/mobile byte providers | Changed code/data pair either both activates or neither does; target run demonstrates actual loading |
| G. Cost and authoring evidence | Edit-to-observed-behavior timings and structured diagnostics over the real samples | Report parse/prepare/admit/reconstruct/first-effect time separately; count any unexpected Cargo units |

The first two packets should be small enough to expose a wrong language or binding choice before a wide migration. Later packets should not wait for a universal language, plugin marketplace, or GUI. They should add only the domain seams demanded by a concrete rich mechanic.

## 10. Evidence limits and retained measurements

The repository records prior measurements in `docs/planning/engine/extension-iteration-evidence.md:78-123`: a warm one-constant Wasm module rebuild of 0.34 seconds, a 7.05-second host relink for the compared content edit, and per-call Wasm setup costs on an agent machine. These show that prior work addressed real iteration paths. They do not establish current machine latency, browser/mobile support, or Rhai speed. This audit does not rerun them.

The source supports a clear next direction: use the existing engine authority boundary as a source-language execution boundary and finish the project-level authoring product. It does not support claims that all custom mechanics can already be authored without compilation, that a new language alone removes remaining engine edits, or that a current VM choice is proven optimal across targets.
