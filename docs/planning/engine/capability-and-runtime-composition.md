# Capability and runtime composition

**2026-11-21 planned revision:** Move from full-engine installation followed by
omission toward explicit selected owner contributions. Keep compiled offers,
installed native services, experience requirements and room permission as
different facts. [Capability linking and prepared
facets](capability-linking-and-prepared-facets.md) owns the design and bounded
pilots. The profile receipts below describe the current implementation; this
revision does not claim new runtime or Cargo absence.

**2026-10-26 scope change:** Default minimum engine games should run deterministic fixed-step
gameplay **without GGRS/rollback or Avian in their resolved dependency closure**; opt-in
rollback is needed for Smash, while Mary-O/Sanic may choose level-scoped synchronization and
Ambition may need peer-hosted independent rooms. These are goals to test, not claims the current
default feature closure satisfies them. See [architecture
reassessment](architecture-reassessment-and-composable-multiplayer.md), [ADR
0036](../../adr/0036-synchronization-is-an-optional-game-capability.md), and [ADR
0039](../../adr/0039-evidence-based-crate-boundaries.md). Keep runtime installation, compile
closure, state ownership and public API as independent acceptance gates.
**Scope:** installation, scheduling, prerequisites and supported absence of
engine capabilities. **Doctrine:** [engine architecture](architecture.md).
**Profiles and closure work:** A9 in the
[packet catalog](actor-monolith-work-frontier.md). **Priority:**
[the queue](../queue.md). Readiness is assessed per authority; a claim that one
boundary is ready does not certify another.

## Measurements

Re-run these before you quote a number. Do not copy counts into other pages.

```bash
python3 scripts/measure/measure_foreign_system_ordering.py
python3 scripts/measure/measure_carveable_installations.py
python3 scripts/checks/check_facade_dependency_closure.py
python3 scripts/measure/measure_minimum_profile_closure.py --minimum
```

- `measure_foreign_system_ordering.py` is a syntactic inventory of foreign
  installations and orderings. A binary root counts as composition whatever its
  package is called. It does not prove semantic correctness.
- `measure_carveable_installations.py` lists upper-bound candidates. An
  "irreducible" label means two packages share a block without a suitable
  dependency edge; it does not prove semantic independence.
- `check_facade_dependency_closure.py` owns the facade closure count and fails
  any planning page that disagrees. It also fails if `ambition_render` re-enters
  the facade's mandatory graph.

The capability-footprint sentinel (`fixtures/minimal_game`) links 75 other
workspace packages besides the facade — the `ambition_closure` of
`scripts/baselines/capability-footprint-baseline.json`, a different subject
from the facade closure. Re-quote this sentence in the commit that changes the
baseline.

The November 10 measurement includes the new geometry observation service
from `21bab8c44` and the NPC planning contract from `dc206fea9`. The consumer
links the planning contract instead of the HTN backend. The contract remains
an unrequested NPC dependency. The baseline counts that cost explicitly.

S0.3 adds `ambition_hit_spec` as a shared value contract. It adds one package
to this closure. It has no ECS, SDK, or simulation dependency. Shared guard
values replace the authored and runtime guard twins. The new contact policy
serves moves, projectile ports, and body reactions.

## Four independently testable contracts

- **Authority:** private state and its transitions have one coherent owner.
- **Installation and lifetime:** an installed capability runs and retires with
  only its documented prerequisites.
- **Compile closure:** an absent optional capability is absent from the
  intended dependency and feature closure.
- **SDK:** an external consumer uses semantic entry points, not internal
  topology.

A passing optional-plugin suite does not prove compile closure. A green SDK
allowlist does not prove a good API. A falling foreign-installation count does
not prove the new installer owns the behavior.

## Selection and room permission

The current `shared_tangle::capability::Capability` enum serves three relations:
plugin omission, room system gates and a partial mapping to content schemas.
These relations are not one-to-one. Collection and held use can consume the
same item vocabulary; an installed capture service can be permitted in one
room and dormant in another. A compiler can understand a schema without
installing its native interpreter.

The target composition selects native service owners and their prerequisites.
An App installs the union needed by its mounted experiences. Each experience
declares its requirements and each room retains local use policy. Native
plugins do not load and unload when a body changes rooms.

Schema, port, construction and rewind offers must describe that selected
composition. They must not advertise an executable operation solely because
its metadata occurs in the repository. Keep the current four proof axes:
deriving related offers from one selected owner does not prove dependency
absence or runtime behavior by itself.

### SC1 — select rule subjects inside the work gate

**State:** DONE 2026-11-23 for the named writer. `walk_by_default_run_while_held(scope)`
takes the installer's scope, the one `add_scoped_systems` gates its set with,
and throttles only a body whose live room that scope governs (`LiveRuleRooms`,
`RulesScope::governs`; `EveryRoom` resolves no room). The system keeps its
name for the phase pins (`with_name`). `RulesScopeSet`'s description now says
it is a work gate, not a subject filter. A temporary two-room probe (the
world crate's `two_game_session`, scope `Mode("smash")`) measured the
stage body throttled to 0.6 with run released and 1.0 held, and the hall
body unchanged at 1.0 in both; it was removed. The spark writer is unchanged:
it acts only on a `run` press of a body wearing a ranged grant.
Planned from the static audit at `00bf92c22142`, 2026-11-22.
Selected under
[ARCHITECTURE-COMPOSITION](../queue.md#architecture-composition--link-owned-facts-and-preserve-causal-boundaries).
This is a remaining caller of the existing per-subject room contract. Keep
the completed S2 and profile receipts; no gameplay run or cost measurement
was made for this finding.

A gate answers whether any room needs work. The system must still select
the subjects that its rule governs. These are different relations:

| Relation | Existing owner | Meaning |
| --- | --- | --- |
| Installed service | Host and capability installer | The App can execute the operation |
| Work gate | `CurrentRoom::in_scope`, `RulesScopeSet` | Some live room matches the scope |
| Subject permission | `LiveRuleRooms`, `RulesOf<T>`, `RulesScope::governs` | This body's room admits this rule |
| Observation interval | Public schedule and room publication | Which published population the answer describes |

**Source evidence.** [`add_scoped_systems`](../../../crates/ambition_platformer2d_runtime/src/mode_scope.rs)
adds a set condition. It does not change a system's query. The underlying
[`CurrentRoom::in_scope`](../../../crates/ambition_platformer2d_world/src/governing_rules.rs)
deliberately asks whether **any** live room matches. In contrast,
[`walk_by_default_run_while_held`](../../../game/ambition_demo_mary_o/src/movement.rs)
iterates every player body with control and resolved technique edges. It
takes no subject identity or room policy. A missing `run` edge reads as not
held, so this system applies Mary-O's walk throttle to that body too.

The [native registration](../../../game/ambition_demo_mary_o/src/lib.rs)
places this mutation after `BodyControlGate` and before integration.
[`gate_body_control`](../../../crates/ambition_platformer2d_actor/src/control/acceptance.rs)
already resolves room-granted techniques through `RulesOf<DrivenTechniques>::of(entity)`.
With one Mary-O room and one untagged room, the first room can wake the
gait system while the foreign body receives no Mary-O room grant. If that
body has no `run` of its own, the gait still changes its control. This is
a supported composition derived from source, not a reproduced route in
the shipped Mary-O world.

**Change at the writer.** Capture the installer's `RulesScope` once and
use it both for the coarse set gate and for subject selection. Add subject
identity to the gait query. Resolve its room with the existing borrowed
`LiveRuleRooms::{room_of, of}`, then ask `scope.governs` before changing
control. A scope-taking system factory or install-time closure is sufficient.
An existing domain rule declared through `DeclaredRules<T>` should instead
use `RulesOf<T>::of` to select its actual rule. Mary-O's pack tuning does
not need a second rule registry merely to ask which rooms its system serves.

Keep the current held-edge rule after this selection. Absence and an idle
edge are different facts, but this fix does not require a new edge API.
Do not infer room permission from a missing or held technique. The existing
body gate remains the owner of technique grants and action acceptance.

Preserve `EveryRoom` with no world, named-mode absence, untagged rooms,
and the existing unstamped-subject behavior. Keep mode-owner lifetime and
the presentation gate's extra retirement frame with their current owners.
A scope-independent `EveryRoom` check can avoid room resolution. For named
scopes, reuse existing room access; it currently searches live roots and
is not proven constant time. Reuse a resolved room within an existing room
batch where available. Do not add a persistent marker mirror, whole-world
scan, callback registry or generalized scoped-query interpreter.

**Removal and proof.** Remove the implicit assumption that membership in
`RulesScopeSet` restricts the query. Correct its public description to say
what the helper actually guarantees; a rename alone does not fix the writer.
A reader should find both the installed scope and the subject selection on
the direct registration-to-writer path.

For implementation, use two live rooms and let both bodies have nonzero
control: the foreign body's control must stay unchanged, while Mary-O's
held/released run states retain their existing throttle. Keep the
`EveryRoom` no-session control. These discriminate subject selection from
merely disabling the system. This audit specifies those cases; it did not
execute them. No new input pipeline, movement kernel or capability split is
needed.

### Room gates have a stable observation interval

The existing scheduling contract also applies to a cached room answer.
Mary-O's registration expressly shares one condition per schedule run.
Its `RulesScopeSet` includes early `WorldPrep` gait and late
`GameplayEffects` work. The eager fixed-step path can publish a room
change between them in
[`RoomTransitionSet::Apply`](../../../crates/ambition_platformer2d_runtime/src/room_transition/mod.rs).
That path includes command application and
[`finalize_committed_room_transition`](../../../crates/ambition_platformer2d_runtime/src/room_transition/commit.rs).

Do not promise that a shared condition observes a new population after
publication without checking its evaluation contract in the pinned Bevy
source. The repository's once-per-run contract makes that a specific
static review question. It is not an executed mixed-mode crossing failure.
If late work must use the new population, give the groups on either side
of the **existing** publication boundary separate shared gates. If a domain
intentionally waits until the next tick, state that entry contract instead.
Do not restore one live-room scan per private system or invent a new phase
graph. The rollback host's delayed publication and mode-owner creation
must follow their actual cuts. Subject selection in SC1 remains necessary
even when a gate is evaluated at the correct cut.

## Capability installation contract

A capability declares required resources and services, optional integrations,
private systems, public milestones, state scope, rollback participation and
retirement. It owns its internal ordering. An installer is an ordinary Bevy
function or plugin; no new trait hierarchy.

The planned unit is the owner's complete contribution. Select its systems,
preparation support, ports, typed rewind declaration and retirement together.
Use direct typed owner functions and the existing registrars. Do not add an
independent table that claims what an opaque plugin should have installed.
`ModuleDraft::capability` currently stores such opaque closures; improving that
surface must keep ordinary Bevy plugins valid while making supported service
composition inspectable before content admission.

A missing required prerequisite fails with a concrete installation or
preparation diagnostic. Model supported absence on purpose. Do not require
dummy resources or hide required behavior behind an `Option` parameter. Do not
detect render readiness by a proxy such as `AssetPlugin` when the system needs
render-device resources.

**Q73 (ruled):** opaque installation is prohibited, not the Bevy `Plugin` type.
A capability may install its private systems through its own plugin when the
host requests the capability explicitly, the systems sit in documented public
milestones and the composition root controls whether the capability exists and
the order between published boundaries. A plugin that silently installs
unrelated capabilities or hides scheduling dependencies is not allowed.

**Portal clip startup, 2026-10-28:** the embedded registry did not prove that
mesh assets existed. Clip setup now requires both stores. Asset-only hosts
create no clip quad; repeated mesh-capable setup creates one valid quad.
Guards: clip material tests, Smash stage and body startup, and the app check.

## Procedural port installation

The [extension contract](extension-state-and-execution.md) adds executable
consumers of published domain ports. The owning capability installs each port's
schema, read projection or request reducer, phase guarantee and prerequisites
together. A metadata-only declaration cannot authorize a call. The runtime
composition root wires the host to selected domain offers; the host never
depends back on the runtime crate or enumerates game algorithms.

Modules write only their own registered state. Engine state keeps its owner.
Resolve phase dependencies at admission, reject cycles and preserve `Commands`
flush and run-condition contracts. Module invocation is stable and serial;
parallel invocation needs declared access and a merge proof.

[Domain contracts](extension-domain-contracts.md) keep operation schemas at the
domain owner and common wire and state primitives in the SDK. No central
all-requests enum, and the executor never imports every domain. An installed
port includes implementation, scope and grant, observation cut, consume barrier
and result. Bevy messages are not rollback queues.

## Composition owns integration, not every algorithm

A host selects capabilities and orders their public milestones. The lifecycle
coordinator owns admission, loading and commit state machines. Those roles can
share the runtime crate and still differ.

A composition block is justified when it states a real relation between
independent owners (capture after custody settlement, presentation after body
geometry publication). It is suspect when it lists one owner's private steps,
performs that owner's transitions or knows its special cases. Do not move
gameplay into runtime to escape Cargo direction. A wrapper that forwards the
same private calls has not reduced knowledge.

## Scheduling contract

For every public milestone, state what is true on entry and exit, the enclosing
phase, run conditions, whether `Commands` have been applied and the population
the guarantee covers. Order only on a real read/write or semantic prerequisite.

Examples that one `.before` cannot capture: reset admission runs before replay
admission in `PlayerInput`; startup checkpoint restoration can run before
gameplay is enabled; item capture observes settled custody. Empty optional
phases must not stop required phases.

## Separation mechanisms

| Mechanism | Legitimate use | Reject |
| --- | --- | --- |
| Direct Cargo dependency | body execution using geometry; a domain adapter using prepared definitions | assuming an acyclic graph proves correct ownership |
| Bevy plugin or installer | owner installs private systems against documented phases | a wrapper around foreign algorithms to move a metric |
| Published `SystemSet` | ordering and visibility between independent owners | one public set per private function |
| Typed intra-tick message | explicit observation with known delivery | replacing a required synchronous result with next-tick delivery |
| Rollback state and effect journal | speculative state restored on rewind; confirmed effects released once per process | treating every message buffer as rollback history |
| App-local provider registry | independent providers registered and frozen before use | dynamic service discovery during a tick |
| Schema or metadata registry | validate IDs, schemas, revisions and conflicts | metadata-only registration as an executable extension |
| Backend-neutral registrar | a domain declares rewind state without importing GGRS | a backend-owned list of every domain type |
| Shared values or `SystemParam` | small owned values; borrow grouping for one operation | bags that carry siblings' private resources |

`ambition_registry_core` is a canonical-registration helper with explicit
New/Idempotent/Conflict behavior. It is not a service locator.

## Compile-time optionality

Measure a real independent consumer manifest under the intended feature set.
Cargo features unify across paths, so an opt-out on one edge does not remove
another edge's request. Check normal, build, dev and target closures
separately. Do not infer binary bytes from graph counts.

The hard cut can be in interpretation rather than plugin installation.
`ActorConstructionServices` and `SessionMechanics` currently require boss
catalogs even where boss plugins are omitted. `ambition_engine_schemas` obtains
schemas from complete native capability crates. Adding a Cargo flag above
these paths leaves the dependency present. Move a selected pure authored/linked
contract below its native adapter, and let construction consume only the
selected owner values. Do this with the existing common-body work; do not
create a second boss body contract to preserve the old dependency.

L3 and L4 in [capability linking](capability-linking-and-prepared-facets.md)
define the bounded cutscene pilot and actual consumer graphs. An all-schema
tool may intentionally select all contracts. A minimum runtime does not have
to inherit that preset. Measure rebuild time separately from graph absence.

**The graph the suite compiles is not the graph the game ships.** `relativity`
is out of `all_capabilities`, and `cargo tree -e normal -p ambition_app` has no
`ambition_relativity2d`. A `--workspace` invocation unifies features and links
it, because `ambition_demo_twintrack` asks for it. So the suite compiles and
exercises a capability that the shipped game omits, and a defect that appears
only when the capability is absent is invisible to a workspace lane.
`the-featureless-facade-links-none-of-these` walks a per-package
feature-resolved tree and measures the right graph.

## Supported profiles

**Current implementation, 2026-10-08 (A9).** The profile set is a registry, not a convention:
`ambition_platformer2d_runtime::profile` declares each `EngineProfile` (name, face,
the `Capability` set it omits) and `PlatformerEnginePlugins::for_profile` builds the
group it names.

Preserve these supported behaviors while the selected-contribution path is
introduced. The subtraction model is an implementation to migrate, not the
target native service taxonomy. Full/default should become a positive preset
over the same owners that smaller compositions select. Do not drop a profile
promise merely because the representation changes.

| profile | face | omits |
| --- | --- | --- |
| `headless-body-world` | headless | nothing |
| `windowed-body-world` | windowed (drawing host) | nothing |
| `combat-without-inventory-boss-dialogue` | headless | `Inventory`, `HeldUse`, `BossEncounters`, `Dialogue` |
| `collection-without-held-use` | headless | `HeldUse` |
| `encounters-without-named-bosses` | headless | `BossEncounters` |
| `world-without-cutscenes` | headless | `Cutscenes` |
| `minimal-platformer` | headless | every removable capability |
| `level-platformer` | headless | `Dialogue`, `BossEncounters`, `HeldUse`, `Trails` |
| `stage-fighter` | headless | `Dialogue`, `BossEncounters`, `Trails` |

**The demo hosts compose these (S3, 2026-10-21).** Mary-O's and Sanic's hosts
build `level-platformer`, Smash's `stage-fighter`; each names it as its
`ENGINE_PROFILE` and compiles its own pack against the profile's schemas
(`EmbeddedPack::compile_for`) before the first frame. A guard in each
(`the_host_omits_only_what_the_game_does_not_use`) holds that the profile
leaves out only what the game's rooms declare unused, and that none of it is
installed. Mounts, wave encounters, portals, fighter specials and grabs
(`Captures`, added 2026-10-21) are gated capabilities, not removable units:
each game declares whether its rooms use them, and their systems sleep where
a room does not.

`Trails` (the player trail, S5, 2026-10-21) is a removable unit: leaving it
out drops `PlayerTrailPlugin` and the four room gizmo groups it registers,
each of which runs Bevy's per-frame and per-tick gizmo systems.

**Three claims per profile, none of which means anything alone**
(`ambition_platformer2d_host/tests/supported_profiles.rs`):

1. it STEPS A REAL BODY: sixty frames expend sixty ticks and the primary body
   moved. A probe that only builds certified an engine that took zero fixed steps;
2. each omitted capability is NOT INSTALLED (`Capability::is_installed`: its plugins
   are not added and the resource only it owns does not exist), and each kept one is;
3. the CONTROL arm builds the full group and requires every capability to read
   installed, so a probe that is always false cannot make a profile pass.

A fourth check covers the edges a ticking fixture never reaches:
`session_edge_params_validate` asks Bevy whether the session-teardown bundle and
the room-transition finalizer validate in the composition. The first probe of the
`Dialogue` omission stepped cleanly and would have failed at the first room
transition and the first session end.

**What the probe found, and what changed.** Omitting a capability failed on its
first tick in each case below. Each is a coupling repaired in source, not a profile
excluded:

| omission | failing parameter | repair |
| --- | --- | --- |
| `HeldUse` | `AuthoredOccurrences` (required by the session teardown and the room loader) installed by `HeldItemSimulationPlugin` | the ledger is session lifecycle state; `LifecycleCheckpointHorizonPlugin` installs it |
| `HeldUse` | `fire_puppy_slug_gun_system` ran ungated | the item-pickup kernel nests `CoreHeldItems` in `PlayerSimulation` itself, as the held-items domain also does |
| `Dialogue` | the conversation UI bridge required `DialogState` | `ConversationPlugin` is a member of the group (a plugin added from another plugin's `build` is invisible to `disable`) |
| `Dialogue` | the narrative ledger's release, the interaction system's `DialogueDispatch`, `space_the_talkers`, the transition finalizer, the teardown bundle, `rebuild_dialog_view` | each takes the conversation authority or `DialogState` as `Option`; C07's table records the reason |
| `Dialogue` | `speak_conversation_cut_barks` read a message only conversation registered | the consumer registers what it reads |
| `BossEncounters` | `simulation_world` required `BossCatalog` | the empty catalog is core construction input (`SimCoreResourcesPlugin`) |
| `BossEncounters` | `populate_boss_encounter_registry` was registered by the progression plugin every composition carries | the boss plugin registers it |
| `BossEncounters`, `Cutscenes` | the teardown bundle required their state | the members are `Option` |
| `Cutscenes` (with bosses kept) | `update_boss_encounters` required `CutsceneTriggerQueue` to request a boss intro | the queue is `Option`: with no cutscenes nothing plays the intro, and its banner still shows (`an_intro_requests_its_cutscene_where_there_is_a_queue`) |

**What a profile does NOT claim.**

* It claims *not installed*, not *not linked*. The crates behind these capabilities are
  unconditional dependencies (`Q106`); `scripts/checks/check_engine_profiles.py` asserts they
  ARE in the closure, so the day one becomes optional that check goes red and the
  contract can be upgraded.
* The bag itself (`OwnedItems`) is core state. `Inventory` removes collecting and the
  authored conditions that ask about the bag, not the bag.
* The closure claim that holds is the facade's: the headless host closure contains no
  `ambition_render` and no `ambition_menu`; the windowed one contains `ambition_render`.
* Not omittable today, each for a reason the probe printed: `dev_tools_sim` (the
  feel-tuning proposal system requires its `PendingMechanicalEdits`). It is not in the
  list.
* Content that requires an omitted capability must refuse at admission (`Q146`/`Q144`).
  A pack that requires an omitted pack capability is refused: a profile's schemas are
  `ambition_engine_schemas::engine_schemas_without(profile.omitted_content_capabilities())`
  (witness `a_profile_refuses_content_that_needs_a_capability_it_omits`). A Yarn script
  asked of a game with no dialogue is not refused: dialogue has no pack capability.
* Re-entry: a second session of each headless profile in one process steps as the
  first, bit for bit (`a_second_session_in_one_process_steps_as_the_first`). The
  windowed profile is not compared.

**Dropping one of these profiles' promises from the registry is a decision; a
witness that stopped iterating the registry would pass for any list**, so the guard
requires the iteration and the control arm to still be in the file.

## Rollback and external effects

A domain declares its authoritative rewind state; the backend implements the
registrar. Register an optional domain only when its profile includes it.
Loading or unloading arbitrary rewind-owning native plugins at runtime is out of
scope. Preserve wire IDs and encoding during an ownership-only move; the
same-build policy stands.

The current `register_engine_rollback_state` still composes broad built-in
declarations independently of runtime omissions. A broad schema is not itself
a desync. The planned selection must drive both the schema and concrete backend
passes. Keep them typed: `RollbackRegistrar` has generic methods and cannot be
used as an object-safe `dyn` service. Do not add type-erased rollback lookup to
make contribution storage convenient.

`crates/ambition_platformer2d_runtime/src/external_effects.rs` is a bounded
confirmed-effects journal: replacement on resimulation, empty-frame
replacement, session reset and delivery after confirmation. Exactly-once
in-process release is not durable exactly-once delivery at a disk or network
sink.

## Ruleset and session scope

Games choose policy and capabilities; sessions own active lifetime; hosts
choose platform and backends. Restore the prior process policy when a scoped
ruleset leaves. A demo plugin never becomes the permanent owner of a setting
another experience uses. Re-entry, not only first startup, is a profile test.

## Completion evidence

Every claimed optional capability needs explicit prerequisites, a minimal
positive case, a supported absence case, correct re-entry and retirement,
rollback declaration and a resolved closure statement. Cross-owner composition
has a reason and a phase guarantee. Unsupported configurations report
unsupported; they do not borrow a plausible sibling default.
