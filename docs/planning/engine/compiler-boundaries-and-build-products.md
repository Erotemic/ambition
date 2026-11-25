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

**Done (2026-11-22).** `game/ambition_content_check` now owns the
`ambition_content_check` binary and its executable test
(`the_shipped_pack.rs`, with the engine-only refusal control). It depends on
`ambition_content_cli` and `ambition_content_schemas`. The schema library
has no normal or dev dependency on the CLI and no binary target, and
`cargo tree -p ambition_content_schemas -e normal` names no
`ambition_content_cli` or `ambition_engine_schemas`. Repointed to
`-p ambition_content_check`: the package guard route, the editor build
command, the packaged-asset-guard doc and the validation recipe.
`AMBITION_CONTENT_BIN` and the binary name are unchanged. No compile-time or
size result is claimed. The Android adapter audit is still a separate
subtask.

**Retained boundary:** schema definitions, lowerings and the sole
`ambition_schemas()` list stay in
[the schema library](../../../game/ambition_content_schemas/src/lib.rs).
[The checker entry](../../../game/ambition_content_check/src/main.rs) and
[its executable integration test](../../../game/ambition_content_check/tests/the_shipped_pack.rs)
now live in the checker package. The test retains the engine-only refusal
control. Do not move the checker into the desktop-heavy app-tools package
or restore the schema library's reverse CLI dependency.

The completed move is `29720575`. The native game's prebuilt content
command remains intentional; the Android adapter below is a separate open
subtask. A dependency cut is not a measured compile-time or size saving.
Use an independent consumer's resolved graph and actual compiler work for
that claim.

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

**Done (2026-11-22).** `install_request::<P, M>` now only builds the checked
`PortOffer`, the typed `RequestCodec` and `adapter.into_configs()`. The
non-generic `install_request_values` owns the offer and codec insertion,
the `last_request` chain, the `Lower`/`Retire` parent, the first retirement
adapter's `holds_phase` gate and the `LowerPort` ordering. Triggers (with
or without their own collector, after E3's shared fact collector) go through
`install_trigger_values`. Observations keep typed boxing and `encode_erased::<P>`
at the entrypoint and pass both to `install_observation_values`. The host
witnesses (installation-order lowering and phase isolation among them) pass,
51 of 51. No emitted-symbol count is claimed yet.

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

**First two customers done (2026-11-22).** `lower_projectile_spawns::<L>`
now only drains its port for `L::PHASE`, makes its own lazy writer state
after the empty check, and calls the non-generic `apply_projectile_spawns`.
That function validates, writes, applies and answers each submission in
order, using the wrapper's `SpawnState`. `lower_held_statuses::<L>` drains
and calls the non-generic `apply_held_statuses`, which stages per body in
submission order (a let-go then a hold in one drain keeps the hold), writes
each final set once, and retracts the clocks of released statuses. Phase
marker types, system wrappers and per-phase local state are unchanged. No
emitted-code measurement is claimed.

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

**First slice (2026-11-22).** `ambition_app_tools` has an explicit `game`
feature family holding the app, facade, harness, content, Smash, Bevy UI,
image and Avian dependencies, all now optional. The 22 bins that simulate or
capture declare `required-features = ["game"]`. `item_report` imports
`ambition_persistence` from its owner and builds with
`--no-default-features`. That graph has no app, facade, content or renderer
crate (529 `cargo tree` lines against 3224 for the default selection), and
the binary runs and refuses a missing save as before. Persistence still
brings core, input and windowed Bevy, so it is not called renderer-free.

`game` is still a default, so every existing launch descriptor works
unchanged. Making it genuinely default-off means updating those descriptors
to name it; that is the next step, together with the harness's facade
defaults and one real harness consumer. No compile-time result is claimed.

**Harness slice (2026-11-25).** `ambition_sim_harness` no longer takes the
facade's defaults (`all_capabilities`); it asks for `rollback`, the one
capability its runtime drives, and `capture` still adds the renderer. The
harness's own normal graph (`cargo tree -e normal`, unique crates) fell from
626 to 549 and no longer contains `ambition_render`, `bevy_render`, `wgpu`,
the menus, the settings menu, touch input, the inventory UI, the portal
presentation or the LDtk crate. Its 36 lib tests and its integration tests
pass, and the tools package checks.

No current consumer's graph changes. The app selects facade defaults
itself, the tools package asks for `capture` and `game`, and
`capability_demo` takes the harness as a dev-dependency beside its own
facade. The first beneficiary is a consumer of the harness alone, which is
what the harness manifest promises. Still open: `game` default-off in the
tools launch descriptors (about 60 direct cargo invocations in `scripts/`
and the catalog).

**Default-off slice (2026-11-25).** `game` is no longer a default of
`ambition_app_tools`. The plain selection's normal graph is 247 unique
crates against 939 with `game`, and `cargo build -p ambition_app_tools`
builds `item_report` alone.

- **Bins.** The three bins cargo had been discovering by file name
  (`officer_probe`, `whiff_drill`, `project_export`) are declared with
  `required-features = ["game"]`. Discovery gives a bin no gate, so a
  default build would otherwise have failed on their optional
  dependencies.
- **Launch sites.** Every launch site names the feature: CI, `run_headless.sh`,
  `run_game.sh`'s match profiler, the capture and measure scripts, the
  moveset inspector, `commands.json`, the bins' own usage lines and the user
  docs.
- **Workspace lanes.** `required-features` SKIPS a target and its tests
  without a word, so every workspace-wide lane names
  `ambition_app_tools/game` (`WORKSPACE_FEATURES` in `scripts/run_tests.py`):
  the default and serial test lanes, clippy, the feature-union lane, CI
  clippy and the target sweep's marker. Without it a green run would cover 25
  fewer targets.

`cargo check -p ambition_app_tools --features game --all-targets` is clean.
No compile-time result is claimed.

What CB4 still names, and what this slice did not do:

- ~~**Harness consumer.**~~ Landed 2026-11-25, below.
- ~~**Persistence.**~~ Its windowed Bevy is cut (2026-11-25, below).
- ~~**Command families.**~~ The simulation family is split from capture
  (2026-11-25, below).

**Harness consumer (2026-11-25).** `fixtures/harness_alone` is the first
consumer of the harness alone. It is its own workspace with its own
lockfile, like the other fixtures, so nothing unifies its features with the
app's. It depends on the harness at the harness's own selection (no
`capture`), and on the facade at `rollback` only, for the room and control
types its composition names. It repeats the root `bevy_ggrs` patch, as
`external_consumer` does. Its test composes one room, steps it ten times and
reads the observation back.

Its normal graph has 372 unique crates. Absent: `bevy_render`,
`bevy_core_pipeline`, `bevy_winit`, `wgpu`, `winit`, `ambition_render`,
`ambition_app`, `ambition_content`. CPU-side Bevy crates survive, by name:
`bevy_ui`, `bevy_text` and `bevy_sprite`, through `ambition_platformer2d_host`
(which asks Bevy for `bevy_ui`) and `ambition_sprite_sheet` (which asks for
`bevy_sprite`). `scripts/tests/test_harness_alone_has_no_renderer.py` holds
the absence and the fixture's separate workspace. `run_tests.py` runs the
fixture's test with the other external consumers. This is the harness
result only. The full Ambition provider still compiles its presentation
adapters (below). No compile-time result is claimed.

**Persistence cut (2026-11-25).** `ambition_persistence` asked Bevy for
`bevy_window`, but nothing in it names `bevy::window`: its display-mode
vocabulary is plain enums and one `Resource`. The facade feature also turned
on `bevy_image`, `bevy_a11y`, `bevy_asset` and `bevy_color`, with their async
and hashing crates. Dropping it took the offline report's normal graph
(`cargo tree -p ambition_app_tools --no-default-features -e normal`, unique
crates) from 188 to 144. The `bevy_window` crate itself stays, through
`ambition_input`'s direct dependency (input reads `CursorMoved`); no winit,
renderer or asset server is in that graph. The workspace checks with every
target, persistence's 140 lib tests pass, and `item_report` still refuses a
missing save (exit 2). No compile-time result is claimed.

**Simulation family (2026-11-25).** `ambition_app_tools` has a `sim`
feature beside `game`: the composed game at `ambition_app/rl_sim` with no
`visible` persona, the facade and harness without `capture`, Bevy, content
and Avian. `game` is `sim` plus the desktop development persona, both
`capture` features, the cube menu, `image` and the Smash demo, which is what
every bin had before. `headless`, `rl_smoke`, `rl_random_walker`,
`room_census`, `fight_discovery` and `trace_replay` require only `sim`;
`moveset_export` still names the Smash roster and stays on `game`. The
tools depend on `ambition_app` without its defaults, so `game` names
`ambition_app/desktop_dev` itself. A workspace lane selecting
`ambition_app_tools/game` unions both and builds every bin as before.

For this the app compiles at `rl_sim` alone, which it did not: the launcher
styling (needs `basic_shell_presentation`), the menu's departures (needs a
menu backend), three debug-overlay drawers (need `input`) and the
no-window plugin groups' `WinitPlugin` disable (there is no winit without a
platform feature) are gated on what defines them. The app root allows
unused helpers when `visible` is off, since the menu and overlay modules
still compile for their always-on parts.

Normal graphs (`cargo tree -p ambition_app_tools --no-default-features
--features F -e normal`, unique crates): `sim` 503, `game` 665. Absent
from `sim`: winit, `bevy_winit`, the wayland and x11 crates, kira/cpal
audio, egui and the inspector, lunex, yarnspinner and the Smash app.
Surviving in `sim`, by name: `wgpu`, `bevy_render` and `ambition_render`,
through `ambition_content`, which selects the facade's `all_capabilities`
(the provider cut above). Behaviour: `headless 300 --start-room
goblin_encounter --dump-trace` gives the same trace (every frame and event;
only the dump's timestamp differs) built under `sim` and under `game`.
No compile-time result is claimed.

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
