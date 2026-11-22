# Compiler boundaries and build products

**State:** PLANNED. Reviewed source: `3ed50e318392b9c2aedc7e900a3358f75f716474`.
This review inspected source and manifests. No engine build, runtime witness
or binary measurement was run. It establishes candidate cuts, not measured savings.
[The queue](../queue.md) selects packets from this owner.

**Related owners:** [project builds](project-build-and-distribution.md),
[performance](performance-and-iteration.md),
[capability linking](capability-linking-and-prepared-facets.md),
[extension execution](extension-state-and-execution.md), and
[execution specialization](execution-specialization-and-work-selection.md).

## Selected boundaries

Let the executable select its dependencies. A schema library should not
import its checker, and a file-report command should not activate capture.
Within native code, retain types where they express different values or
system identities. Convert them before implementation work that depends only
on ordinary values. The first customers already have these conversion points;
they need no new registry, composition language or request interpreter.

Keep four build subjects distinct:

| Subject | Required work | A different subject |
| --- | --- | --- |
| Independent schema consumer | Parse, validate and lower selected authored vocabulary | Installing its native interpreter |
| Simulation tool | Compose real gameplay, step it and observe results | Selecting a render device or capturing a frame |
| Visible/capture tool | The same gameplay plus its presentation/capture adapters | Every unrelated report binary |
| Native game with authoring commands | Gameplay and its deliberately shipped `content` command | An Android library entry with no desktop command dispatcher |

Runtime plugin omission, compiled dependency absence, retained executable code
and packaged asset bytes need different evidence. A host that supports several
experiences may legitimately retain all their providers even while one room
is active. Do not prune that product by its current room selection.

## What the source establishes about cost

| Candidate | Current evidence | Missing cost evidence |
| --- | --- | --- |
| CB1 schema/checker inversion | A normal schema-library dependency reaches the CLI and aggregate native schema owners | Resolved closure and edit-to-build difference for an independent schema consumer |
| CB2 installation conversion | Port metadata and codecs are already ordinary values, but schedule installation remains generic over port/system types | Emitted function families, codegen and warm consumer-edit cost |
| CB3 phase adapters | Projectile lowering has five phase bindings; status lowering has four. Their phase marker only selects the drain | How much reducer code survives separately in the selected artifact |
| CB4 tool activation | One bin package enables desktop/capture dependencies for every command; the harness also inherits facade defaults | Each command's resolved features, build work and retained bytes |
| Curved-search experiment | The acceptance closure propagates through the world query into the numerical search | Duplicated emitted search code and the runtime cost of a borrowed callback |

A conservative source census found 146 explicit external extension-port
registration sites in 25 files, before test modules and excluding host
forwarders/macros; seven additional body-phase macro invocations each emit
seven bindings. This is source fanout, not a count of compiler instantiations
or retained functions. Do not derive a percentage saving from it.

Existing profile/linker choices, source embedding gates and the corrected
native-development build stamp remain in force. The historical
[compile journal](../../../dev/journals/compile-time-and-disk-2026-08-07.md)
records that removing much of the then-current rollback generic surface did
not measurably improve `cargo check`. Generic spelling alone is not a cost
model. Its [full-build follow-up](../../../dev/journals/compile-cost-what-actually-drives-it-2026-08-08.md)
did find a large code-generation difference while type-check time changed
little. The lesson is to use an instrument that sees the proposed cost;
neither dated result is a baseline for the current code.

Current [room snapshots](../../../crates/ambition_platformer2d_runtime/src/rollback/room_snapshot.rs)
already put typed ECS/clone/codec access behind function tables and share the
save/restore orchestration. Current
[GGRS registration](../../../crates/ambition_platformer2d_rollback_ggrs/src/registration.rs)
still installs per-type snapshot/checksum/probe systems. Measure those emitted
families before a new backend change. Another erased layer around room rows
would repeat an existing cut; relocating generic registration may only move
the work. None of the packets below selects broad rollback or Serde erasure.

## CB1 — Put the game checker above its schema library

**Execution owner:** `ambition_content_schemas` and the game-owned checker
frontend. The pure-domain extraction in capability-linking L4 is separate;
this packet can remove the reverse frontend edge before that work lands.

**Start from the actual dependency and callers.**
[The schema manifest](../../../game/ambition_content_schemas/Cargo.toml)
normally imports `ambition_content_cli`. Its
[library](../../../game/ambition_content_schemas/src/lib.rs) only defines the
schema modules and `ambition_schemas()`; the CLI call is in
[the checker main](../../../game/ambition_content_schemas/src/main.rs).
[Ambition content](../../../game/ambition_content/Cargo.toml) imports that
library for real runtime values. Consequently the schema library reaches
[the CLI](../../../crates/ambition_content_cli/Cargo.toml), which reaches
[aggregate engine schemas](../../../crates/ambition_engine_schemas/Cargo.toml)
and full native domain owners.

Give the checker a narrow game-owned command package. Keep all schema
definitions, lowerings and the sole `ambition_schemas()` list in the library.
The generic engine CLI must not depend on the game's schemas. Moving the
checker into the current desktop-heavy `ambition_app_tools` package would
substitute a larger dependency problem.

Move [the executable integration test](../../../game/ambition_content_schemas/tests/the_shipped_pack.rs)
with the checker: it uses `CARGO_BIN_EXE_ambition_content_check` and proves
that the engine-only CLI refuses the game-owned schema. Schema-only tests
remain with their values. Update the package route in
[package_asset_guard.py](../../../scripts/package/package_asset_guard.py)
and the supported command descriptors/docs; retain `AMBITION_CONTENT_BIN`
as the already-built checker path.

**Delete:** the schema library's normal CLI dependency and its checker target.
Keep a CLI dev-dependency only if an actual remaining test needs it. A gated
bin with a nonoptional dependency does not make this cut. An optional feature
can narrow an isolated build, but a separately owned frontend avoids unioning
its command dependencies back into the reusable schema package.

**Accept:** the checker accepts/refuses the same packs, reports the same schema
identity, and preserves advisory/strict asset and package-input behavior.
An independent normal schema consumer no longer reaches the CLI. Editing CLI
argument handling does not rebuild that consumer. Use its resolved graph and
actual compiler work, not an empty App or the all-workspace feature union.

**Stop/falsifier:** list any surviving path before claiming a consumer cut.
If native owners remain through another intended dependency, do not report
their absence or duplicate their schema types to force the result. Timing
and binary-size improvements remain separate measurements.

### Preserve the native validator; scope Android separately

[The app manifest](../../../game/ambition_app/Cargo.toml) explicitly enables
facade `content_check` on non-wasm targets. The
[desktop main](../../../game/ambition_app/src/bin/ambition_game_bin.rs) calls
[the content dispatcher](../../../game/ambition_app/src/app/content_check.rs)
before starting the game. `content …` and `--schema-identity` let an author
validate with a prebuilt runtime. CB1 preserves this product behavior and does
not claim to remove CLI from the native authoring game.

After removing the reverse schema edge, audit Android as a separate subtask.
Its [shared-library entry](../../../game/ambition_app/src/lib.rs) calls
`run_visible()` directly, while the current `not(wasm32)` manifest/module
conditions still activate the desktop command adapter. Verify the packaging
entrypoints, then narrow that adapter's compilation/dependency selection to
the frontend that uses it. Keep normal content preparation and game schemas.
Accept the same Android launch and desktop content command, with the Android
CLI dependency path removed. Stop if another supported Android entry actually
uses the command. Dead stripping may already eliminate its executable text;
a dependency cut is not a claim about saved APK bytes.

## CB2 — Convert typed installation inputs before schedule registration

**Execution owner:** `ambition_extension_host`, inside its existing
`ExtensionAppExt` implementation. No new public registration protocol.

**Start:** [install_request](../../../crates/ambition_extension_host/src/lib.rs)
(`install_request<P, M>`, source lines 561–601) currently combines port-role
checking, codec selection, system conversion and schedule topology. Only the
first three require the caller's concrete types.
[RequestCodec::of](../../../crates/ambition_extension_host/src/exec.rs)
already returns ordinary function pointers. Bevy's converted
`ScheduleConfigs<ScheduleSystem>` is already used by
[clash_arbitration](../../../crates/ambition_projectile_combat/src/clash.rs).

Keep the typed `install_*::<P, M>` entrypoints. They create the checked
`PortOffer` and typed codec, call `into_configs()` once on the adapter, then
pass those values, `Phase` and `RequestStage` to a non-generic internal
installer. That installer owns metadata insertion and the existing schedule
relationships. Preserve `ensure_phase`; it is already non-generic.

Apply the same bounded conversion to trigger installation. For observations,
retain typed boxing/encoding at the entrypoint and pass the existing supplier
and encoding function to ordinary installation code. Do not introduce a
second runtime registry or add an indirect gameplay call: the systems already
enter Bevy through converted configurations, and the codec boundary exists.

**Delete:** port/system parameters from the request topology-installing body,
including the retirement gate closure, and the corresponding repeated trigger
and observation installation bodies. Keep the typed payload codecs: different
wire layouts are real work, not accidental specialization.

**Accept:** installation order still determines request-lowering order within
each phase/stage. Preserve `Lower` versus `Retire`, `LowerSubmitted`, the first
retirement adapter's `holds_phase` gate, distinct phase/system identities,
port-key/type rules and owner metadata. Reuse
[host witnesses](../../../crates/ambition_extension_host/src/tests.rs),
including `request_ports_are_lowered_in_the_order_they_were_installed` and the
two-phase isolation case. Inspect the same installation family in emitted
symbols/mono items and record a real consumer edit as well as a host edit.

`RequestStage::Retire` is request lowering, gated by submitted work. It is
not an effect-lifetime maintenance stage. Effects6a must withdraw retired
sources even on a tick with no request. Preserve this gate in CB2 and give
the effect owner its required lifecycle boundary in that separate packet.

**Stop/falsifier:** if the compiler already shares this work, report little or
no code-size benefit. Keep the refactor only if ownership remains simpler;
do not compensate with a more elaborate abstraction. No startup-speed claim
follows from fewer generic source bodies, and no blanket `inline(never)` rule
is selected to force a desired report.

## CB3 — Share domain lowering after the phase-specific drain

**Execution owner:** combat's projectile-emission and status adapters. Their
native systems and payloads stay in their current domains.

**Start:** [LowersIn](../../../crates/ambition_extension_host/src/exec.rs)
exists for a real reason: every phase needs a distinct adapter system and an
isolated drain. Registering one system repeatedly loses unambiguous ordering
by system type. Keep those marker types and system wrappers.

[lower_projectile_spawns](../../../crates/ambition_combat/src/projectile_spawn/extension.rs)
has five phase registrations. Its marker is used only in the drain; the body
then validates shape/identity, uses `ProjectileWriter` and publishes action
results through the same concrete types.
[lower_held_statuses](../../../crates/ambition_combat/src/extension/status.rs)
likewise selects its phase at the drain, then stages an ordered update per
body and retracts obsolete clocks. These are the first two customers.

For each operation, keep `lower_*::<L>` as the Bevy system boundary. Drain its
concrete port for `L::PHASE`, then pass the resulting
`Vec<Submitted<ConcreteValue>>` to a non-generic domain function with the
existing world/query/command access. `ExtensionOutbox::drain` already creates
this vector; this extraction needs no additional allocation, `Any` conversion
or per-request virtual dispatch.

For projectile emission, pass access to the wrapper's existing
`Local<Option<SpawnState>>`. Preserve its lazy initialization after the empty
check, its independent instance for each phase, and each `state.apply(world)`
boundary. A shared function body must not become shared mutable system state.
For status updates, preserve submission order and the final staged value per
body, including a let-go followed by a hold in the same drain.

**Delete:** validation, staging and publication loops from the generic
wrappers, leaving typed fetch/drain and the call. Keep one implementation per
domain operation, not one universal reducer for projectile/status requests.
Do not sweep every `LowersIn` function: some already delegate their substantive
work. [Body-query lowering](../../../crates/ambition_combat/src/extension/body_query.rs)
is a counterexample to a crude line count: its geometric selector is already
non-generic, and its result explicitly carries the phase.

**Accept:** preserve submission order, state scope versus execution scope,
occurrence/move-use identity, action admission and terminal results, deferred
command boundaries and phase isolation. Exercise requests in two phases with
the existing owner witnesses. Projectile refusal and queued-success results
must remain correlated with the same submission; status clock removal and
same-drain let-go/hold must retain their current outcomes. Coordinate with
effect-claim work without changing its semantics in this compiler packet.

**Stop/falsifier:** thin phase wrappers and distinct Bevy system types remain
intentionally. Inspect whether meaningful reducer work was emitted repeatedly
and is now shared. Release inlining can duplicate a non-generic body again;
evaluate actual codegen/runtime results instead of enforcing global no-inline
attributes. Stop if sharing requires another interpreter, cross-phase batching
or changed Local lifetime. Those are different behavioral changes.

## CB4 — Select command dependencies and the harness's actual services

**Execution owner:** `ambition_app_tools` command packaging and
`ambition_sim_harness` dependency selection, coordinated with supported launch
and test lanes. The Ambition provider's presentation cut is a separate proof.

**Start:** [the tools manifest](../../../game/ambition_app_tools/Cargo.toml)
has `default = []`, but normally depends on default-enabled `ambition_app`,
facade capture, harness capture, Bevy UI, image, Avian and the Smash app. Every
bin shares this activation. The historical app-bin split in the same manifest
avoided unrelated relinks during app integration tests; preserve that win.

Separate the actual command requirements into offline reports, real simulation
and visible/capture work. First implement genuinely default-off optional
dependency families inside this existing bin-only package. Give each bin the
appropriate `required-features`; update supported launch descriptors to select
them explicitly. The old failed `required-features` attempt used gates that
were already on through defaults, so it is not evidence against this cut.

Use [item_report](../../../game/ambition_app_tools/src/bin/item_report.rs) as the
first offline witness: it reads a save through persistence APIs and never
constructs an App. Import those APIs from their owner instead of the facade.
Do not call persistence pure or renderer-free without resolving its remaining
closure. Keep its saved-value format, audit results and platform path behavior.

[Headless](../../../game/ambition_app_tools/src/bin/headless.rs) and
[extension_inspect](../../../game/ambition_app_tools/src/bin/extension_inspect.rs)
need real gameplay composition. Keep their current adapter until the lower
dependency cut exists. Captures keep their render/capture requirements.
[moveset_export](../../../game/ambition_app_tools/src/bin/moveset_export.rs)
currently boots the composed visible App to read prepared characters, provider
overrides, the Smash roster and art metadata. Do not replace that with a second
interpretation of source move tables. A later pure preparation consumer belongs
to the existing capability-linking work and must read the same admitted values.

Split coherent command families into packages only if feature union or target
test ownership prevents the required independent build subject. Do not make
one crate per bin. A schema checker should not automatically share a package
with persistence reports just because both print text; compare their activated
dependencies first. Selecting capture and simulation together legitimately
unions features; report the exact selection under which isolation is claimed.

### Finish the harness cut before claiming a headless product

[The harness manifest](../../../crates/ambition_sim_harness/Cargo.toml) makes
capture optional but leaves facade defaults enabled in normal and development
dependencies. [Facade defaults](../../../crates/ambition_platformer2d/Cargo.toml)
select `all_capabilities`, including rendering, portal presentation and menus.
Thus capture gating alone does not establish renderer absence.

Disable those implicit defaults and name the harness's real requirements.
Its [runtime](../../../crates/ambition_sim_harness/src/runtime.rs) genuinely
uses rollback functions and public result types, so retain that capability in
this slice. Trace the remaining normal/test consumers before selecting the
minimal supported set; do not change their API to make a manifest look small.
First prove a useful independent harness composition that steps and observes
gameplay without selecting the renderer.

The full Ambition composition is a second subject:
[its manifest](../../../game/ambition_content/Cargo.toml) selects facade
`all_capabilities` and directly imports render.
[Its provider](../../../game/ambition_content/src/provider.rs) conditionally
installs presentation at runtime, but those implementation types still compile.
The later provider cut must make presentation adapters optional while retaining
the same gameplay inputs. The clock/light split in
[day_cycle](../../../game/ambition_content/src/day_cycle.rs), renderer-owned
[area cards](../../../game/ambition_content/src/area_cards.rs), and art-catalog
publication in [reload](../../../game/ambition_content/src/reload.rs) are real
remaining edges. Start with optional modules/dependencies; a presentation crate
needs an independent consumer justification. Keep mechanically significant
sheet/body facts headlessly and preserve atomic mechanical-generation admission.

**Delete:** unused normal dependency activation from each selected command
subject, accidental facade defaults, and obsolete package/feature routes.
No parallel save parser, headless provider registry or content compiler.

**Accept:** the offline report, real simulation and capture execute unchanged.
Preserve `causal`, `profile` and `perf_probe` forwarding. Feature-gated bins with
tests must be selected by their supported test lane; `test = false` cannot
silence real tests. Record the resolved graph and compiler work for each named
command independently. Report harness renderer absence separately from full
Ambition-provider renderer absence, naming surviving paths in the latter.

**Stop/falsifier:** package movement without reduced activation is not a cut.
If the current game provider still requires rendering, finish the bounded
command/harness result and state that limit. Do not require every capability
to become optional before delivering it, or claim no-render compilation from
a no-window runtime test. Keep the existing main game bin/app-test separation.

## Conditional experiment: the curved-search callback boundary

This remains a second-wave experiment within
[portal P4](portal-parts-similarity-and-chirality.md#p4--conservative-query-planning-and-proven-fast-cases),
not a new solver owner or a blanket collision-interface rewrite. Start only
with emitted-code evidence that the selected artifact duplicates substantial
curved-search work.

[World::first_material_contact](../../../crates/ambition_platformer2d_core/src/world.rs)
passes a domain predicate through
[against_rigid_where](../../../crates/ambition_platformer2d_core/src/frame/material_cast.rs)
into [curved::cast](../../../crates/ambition_platformer2d_core/src/frame/material_cast/curved.rs).
Keep the generic outer/static path initially. At the existing curved call,
borrow the local acceptance closure through one callback into a non-generic
search. No heap allocation is needed. Interval, witness, SAT and motion-bound
work remains concrete; the indirect call occurs at contact proposals.

Preserve the exact callback sequence and numerical operations. Rejection must
continue along the same recorded curve and may accept a later approach;
post-filtering the first geometric hit or collecting every contact changes the
operation. Retain initial-overlap rules, interval order and the work/precision
refusal budget. The existing
[rejected-then-later-contact witness](../../../crates/ambition_platformer2d_core/src/frame/material_cast/tests.rs)
is relevant. Leave `first_material_sweep_prefix<F: Copy>` unchanged: its retries
copy the predicate, whereas borrowing one mutable predicate across retries
could change captured-state behavior.

**Delete only if justified:** concrete predicate specialization of the curved
search body. Compare static, accepted curved, rejected-then-later curved and
refusal workloads, plus emitted code. Stop if current compilation already
shares the work or candidate-heavy runtime regresses enough to outweigh the
measured build/size result. This experiment does not change P4's geometry,
query preparation, eligibility selection or correctness obligations.

## Evidence and completion records

Use [compile_cost.py](../../../scripts/measure/compile_cost.py) and
[monomorphization_report.py](../../../scripts/measure/monomorphization_report.py).
Keep their existing target-bind/headroom workflow. No new checker or permanent
source-text gate is required. Record the exact target, package/bin, features,
profile, toolchain, linker, cache state and changed source for each comparison.

For CB1/CB4, show the resolved normal dependency/feature cut before attributing
time to it. For CB2/CB3, distinguish source sites, compiler mono items and
emitted function families; preserve intended typed wrappers. Pair codegen/link
results with cold, no-op and representative warm edits on the same subject.
Check time alone does not measure executable code generation or linking.

Separate executable text, readonly metadata/content, debug/compiler metadata,
retained target variants and appended packaged assets. Removing cache variants
does not prove less monomorphized code; removing a dependency can leave retained
binary bytes unchanged. The existing static-content gates and sprite metadata
compiler remain. The [prevalidated-artifact hold](prevalidated-content-artifacts.md)
remains explicit. No general cache, dynamic Serde tree or erased rollback model
is selected. Close a packet with its deleted path, preserved behavior and actual
cost evidence, including a zero or negative result where that is what occurred.
