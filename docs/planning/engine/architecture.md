# Engine architecture: current shape, doctrine and target authorities

**2026-10-26 review note:** The current code facts and guard references below remain useful, but
the *target* design and old D33/A9 ordering are open to reassessment: [composable multiplayer
and crate-boundary review](architecture-reassessment-and-composable-multiplayer.md). Existing
global GGRS lifecycle and platformer composition are current implementations, not requirements
on every future game. New proposals [ADRs 0035-0039](../../adr/README.md) separate deterministic
stepping, synchronization, region authority, persistence, physics and package boundaries. No
island-host implementation is claimed.
**Scope:** the planning owner for engine architecture. It states the shape the
engine has converged on, the decomposition doctrine, the target logical
authorities and the rules that workspace policies cite.
**Implementation reference:** [the durable engine architecture](../../architecture/engine-architecture.md)
and [package and capability boundaries](../../architecture/package-and-capability-boundaries.md).
**Priority:** [the queue](../queue.md) alone selects work.
**Packets:** [ownership migration packets](actor-monolith-work-frontier.md) (A1-A12)
and [actor residual-kernel decomposition](actor-monolith-decomposition.md).

Workspace-policy rows (`source_doc`) and Rust comments cite this path. When you
move a rule out of this page, move the argument with it and update the citing
rows in the same commit.

## Structural direction from the 2026-11-21 review

**State:** planned changes, reviewed against source
`6895e516863872eb6d38093a27225ed8e2973d70`. No new runtime or compile-time
result is claimed. [The queue](../queue.md) selects implementation.

The next work removes repeated interpretations of the same facts. Three
distinctions guide it:

- A definition, an application of that definition, and the contributions
  owned by that application have different identities.
- Code compiled into a program, services installed in an App, and behavior
  permitted in one experience are different selections.
- A speculative local result and a committed fact another history may use
  have different lifetimes.

The current engine already preserves these distinctions in several strong
boundaries: hidden candidate publication, stable object occurrences, domain
action receipts and confirmed world posts. Apply those meanings where
central structs, category bits or undo logs currently discard them.

| Direction | Knowledge or duplicate path to remove | Owner and first proof |
| --- | --- | --- |
| Link capability-owned prepared facts | The central character/context must know each optional domain's fields; reference validation is separate from the lowered value it claims to prove | [Capability linking and prepared facets](capability-linking-and-prepared-facets.md): character/repertoire binding and a bounded pure-format consumer |
| Preserve contribution occurrence identity | A category bit or one source/attribute slot stands in for several independent grants and terms | [Actor effects](actor-effects-and-modifier-composition.md): overlapping script holds/poses and a native transform, with typed reducer laws |
| Commit cross-history consequences | Shared-store inverse logs try to repair decisions made by independent readers | [Causal world commit](causal-world-commit.md): dependent grant/spend and a held compare-and-set refusal |
| Bind reusable authored instances | Scripts repeat child creation and relationship wiring that construction already knows how to verify | [World composition](reusable-authored-world-composition.md): existing local bindings, then two different authored structures |
| Reuse preparation from observed inputs | A reference graph or broad repository digest is treated as the complete reason a product is current | [Content generation](content-generation-and-reload.md), [asset delivery](asset-library-editor-and-delivery.md): distinct use, computation and product edges |

These directions reinforce one another without requiring one framework.
The content linker resolves immutable domain inputs. The constructor gives
one live occurrence those inputs. A temporary contribution has its own
issuer and retirement. A world operation states when another authority can
observe its result. Do not merge these into one generic object, transaction
or registry type.

### Revision to dependency and ownership work

Move pure schemas and binders below their native ECS adapters when a real
authoring consumer needs that boundary. Select services positively and keep
room permission separate. A domain supplies its own typed preparation,
construction, install, rollback and port participation; the composition root
supplies the selected combination and cross-domain milestones.

The target removes catalog-specific knowledge from common construction.
Moving the same catalog union to another crate does not achieve it. An
absent boss service, for example, must not require an empty boss catalog
merely to satisfy a common body-building context.

Bevy plugins remain the native installation mechanism. The backend-neutral
rollback registrar has generic typed methods; do not make it a dynamic
service locator to force all contributions through one object-safe trait.
The all-schema CLI and full-game preset may intentionally select a broad
set. Their convenience does not define the minimum consumer.

### Order and evidence

Correct the causal contract when extending independent-room world writes.
The effect-ownership pilot can run independently. The linker pilot removes
one repeated interpretation, then one justified format/adapter dependency
cut proves the compile claim. Authored assemblies reuse the existing local
reference pass. General incremental reuse follows complete input evidence.

Keep BOSS-CHARACTER's common body seed as a current customer; do not make
every remaining actor carve a prerequisite. Do not reopen completed
script-first, part-first, editor-shell or numeric-effect work. The new proof
is composition across those boundaries.

Use one source-derived explanation of each selected definition and binding
to improve exploration: authored field, selected provider, domain binder,
prepared facet, native consumer and refusal. Feed that projection to the
existing query/editor tools. Do not create a second manually maintained
concept catalog or require a new planning document to author each mechanic.

### Portal review: accepted operations retain their meaning

The source review at `b91b29d35935` applies the same ownership principle to
spatial state. The [portal delivery plan](portal-parts-similarity-and-chirality.md)
keeps the existing material algebra and replaces reconstruction from reduced
facts: channel as lifetime, end pose as trajectory, static mapped vector as
moving-frame contact velocity, and camera rectangle as presented observer.

A domain's accepted operation retains its scope, selected geometry, physical
time and result. Completed trajectory and next-step initialization have
separate consumers. Topology occurrence, topology revision, sampled pose and
source image are separate identities. The view's visibility eye and final
projection are separate facts. These distinctions allow owners to share
prepared data and remove repeated decisions without one global spatial graph.

P1–P6 name current writers, crossings, failures and removals. Pure query
specializations and prepared-input reuse address repeated runtime work;
per-observer demand and complete image-source reuse address redundant capture.
Do not claim a new compile-time boundary or speedup until its actual consumer
and resolved dependencies are measured.

## Converged shape

Code against these facts. Each one is in source and has a guard.

| Fact | Where it is |
| --- | --- |
| **Crate tiers.** Engine crates live under `crates/`. Named games, providers, content and app shells live under `game/`. | workspace `Cargo.toml`; policy `repository.tier-manifests-exist` |
| **One session root.** Live platformer world state (`RoomSet`, `RoomGeometry`, `StartingCharacter`, `LdtkRuntimeIndex`, encounter music requests) is a set of components on the exact `SessionRoot` entity. A running session has one active `SessionRoot`; hidden candidate sessions carry `CandidateSessionRoot` until publication. No process `Resource` and no projection bridge holds this state. | `ambition_platformer2d_shared_tangle::lifecycle::session::{SessionRoot, session_world_component}`; policy `engine.canonical-session-world-only` |
| **Candidate-world publication.** A reload or room transaction builds a hidden candidate. Candidate roots carry the disabling component `InactiveCandidate`, so ordinary queries do not see them. A candidate session carries `CandidateSessionRoot`, not `SessionRoot`. Publication removes the component (`publish_candidate`, `publish_candidate_session`). Recipes receive `ConstructionRootCtx` / `RootScope`, which expose no `Commands`. | `ambition_platformer2d_shared_tangle::construction`; absence contracts `only-the-candidate-builder-hides-a-root`, `only-the-publication-authority-publishes-a-candidate` |
| **Frozen session cast.** A running session reads the cast its generation was prepared against (`ActiveSessionCast`), through `SessionCast`. With no session, readers see the published cast. Generation inputs that a candidate must see at N+1 travel in `PendingGenerationInputs`, not through global publication. | `ambition_characters::session_cast`; `ambition_platformer2d_runtime::content_identity::PendingGenerationInputs` |
| **Several live rooms.** Each live room is a `RoomInstanceRoot` entity with a `LiveRoomInstance` and a `LiveRoomDefinition`. Entities carry `InRoomInstance`. `LiveRooms` answers which room an entity is in. `RoomSet::activation` is a prepared fact, read only when a session activates. | `ambition_platformer2d_shared_tangle::lifecycle::room_instance`; `game/ambition_app/tests/two_players_two_live_rooms.rs` |
| **Content packs are data.** A pack manifest lists each source file with its schema (`character_catalog`, `moveset`, `item_catalog`, `boss_profiles`, `boss_encounter`, `cutscene_library`, `music_cue_catalog`, `sfx_registry` and others). `ambition_content_cli` validates a pack without a Rust rebuild. Authored movesets are content, not Rust tables. | `game/ambition_content/assets/pack.ron`; `crates/ambition_content_pack`; `crates/ambition_content_cli/tests/fast_validation.rs` |
| **App-local authority.** Character, boss, item and audio catalogs are resources assembled in each Bevy `App`. No process-global install or override seam exists. | policies `engine.character-authority-is-app-local`, `engine.audio-authority-is-app-local` |
| **Procedural extension tier.** A module builds against `ambition_extension_sdk` (no dependencies) and port value leaves. `ambition_extension_host` admits and invokes modules. `ambition_extension_wasm` is one backend. Game modules live in `game/ambition_content_modules`. | [extension model](extension-model.md) |
| **One observation boundary.** Observers read plain-data read models from `ambition_sim_view`. `ambition_render` never mutates the simulation. | crate headers of `ambition_sim_view`, `ambition_render` |
| **One rollback backend adapter.** Domains declare rollback state through the backend-neutral `RollbackRegistrar`. Only `ambition_platformer2d_rollback_ggrs` knows `bevy_ggrs`. | `ambition_platformer2d_core::snapshot`; `crates/ambition_platformer2d_rollback_ggrs` |

## Rules that workspace policies cite

Each rule below has a policy row whose `source_doc` is this page.

- **Foundation crate.** `ambition_platformer2d_core` holds geometry, the body
  contract and control-frame vocabulary. It never names the runtime
  composition tier, the content crate or the app shell.
- **Load and shell.** `ambition_load` owns headless load truth, barriers,
  cancellation and commit authorization. It never renders and never names a
  game destination. `ambition_game_shell` routes top-level experiences, gives
  each one a scoped activation identity and owns no route names and no
  rendering. `ambition_load_presentation` is a replaceable presentation. It
  depends downward on load and shell, never on content, an app shell or a demo.
  All three crates stay free of Ambition content, app and demo route
  identities.
- **Audio authority.** Authored audio is App-local through
  `AudioCatalogRegistry`. The retired process-global install and override
  functions must not return. Every SFX emission goes through `SfxWriter`, which
  captures the exact `AudioContextOwner` at emission time. Every shell
  activation has an explicit frontend, gameplay or direct audio context.
- **Character authority.** Playable characters, hostile archetypes, boss
  behavior and encounters, presentation, dialogue, barks, collision and attack
  geometry come from validated resources assembled in each `App`. Production
  code must not reinstall process-global authority, substitute an empty
  catalog, make these resources optional or use implicit lookup wrappers.
- **Providers are host-ignorant.** A provider expresses one game. The host
  decides home and initial routes, the launcher and process exit. A provider
  must not name another host's route, a host crate or issue host routing itself.
  `ambition_demo_pocket` proves that a provider can depend on the umbrella
  facade alone.
- **Canonical session world.** See "One session root" above.

## Decomposition doctrine

Decompose by ownership and dependency value, not by line count. A move must
remove state, policy or lifecycle knowledge from a consumer, or deliver a
specific installation, SDK or measured build benefit. A move does not by itself
improve frame time, startup time or memory use.

### Two dimensions

**Authority decomposition** asks who owns a fact, which writers may change it,
which invariant coordinates them, when others observe the result, and how it is
created, restored and retired.

**Capability composability** asks whether a consumer can select or omit a
capability with only its declared prerequisites.

Authority and composition must both be proven, but neither has a universal sequencing prerequisite; a minimal consumer can reveal a needed authority boundary before an extraction campaign finishes. Authority decomposition alone does not prove composability.
Measure composability along four independent axes:

| Axis | Falsifiable question |
| --- | --- |
| Semantic authority | Can the owner be understood without the sibling's internal policy and writers? |
| Runtime installation and lifetime | Does the capability run with only declared prerequisites, including teardown and re-entry? |
| Compile-time closure | Is an absent capability absent from the selected normal dependency and feature closure? |
| Public API | Can an external game use the supported path without internal re-exports? |

A no-render plugin configuration can still compile renderer dependencies.
`scripts/checks/check_facade_dependency_closure.py` owns the facade closure count; do
not restate the number on a planning page.

### What moves with an authority

A packet accounts for: state and writers; construction and defaults; stable
identity; queries and read models; private systems and public phases; run
conditions; deferred-command visibility; rollback declaration, encoding and
checksum; teardown; session, match, attempt, body and stock lifetime; external
effects. A type plus a re-export is not a transfer. A `SystemParam` that makes
the caller assemble every old dependency is not a transfer.

### Boundary selection test

Before you add a trait, message, adapter, registry or shared value, answer:

1. Which knowledge of the other domain disappears from the caller?
2. Is the dependency already semantically downward, so a direct call is simpler?
3. Does the operation need same-tick execution, deferred observation, rollback
   replay or confirmed external delivery? Does the new seam change that?
4. Who owns failure, cancellation, stale identity and lifetime cleanup?
5. Which supported customer needs substitution or independent installation?

If the only answer is a smaller SCC or fewer imports, do not add the
abstraction. A cycle inside one control or custody authority can be correct.

### Absence, readiness and defaults

- A supported absent capability must not require dummy sibling resources.
  Validate real prerequisites at installation or preparation, or represent
  absence in the consuming API.
- Do not wrap every system parameter in `Option` so that required work
  disappears.
- A capability initializes its own state. One that cannot is one the
  composition layer must know about.
- A default that is a plausible member of the value space it replaces turns a
  composition error into a silent measurement error. Represent absent, pending,
  ready and invalid when the difference affects behavior.
- Scope readiness to the selected content revision, session and device. A
  ready resource left over from a retired session is not evidence.

### Scheduling and registration

- Capability owners install their private systems and order their internal
  phases. Composition orders independent capability phases. Zero foreign
  installs is not a requirement.
- `ambition_platformer2d_runtime` owns installation and scheduling policy.
  Cross-domain simulation adapters may later move to a separate
  `ambition_platformer2d_integration` implementation crate, once a coherent
  room-publication packet needs it. Domain crates do not import that upper
  layer. A crate carve does not create a new ECS system by default. See the
  [decomposition plan](actor-monolith-decomposition.md#target-package-and-integration-boundary).
- A public phase describes an observable guarantee (custody settled, body
  geometry published). Do not publish one set per private function. Do not
  chain the whole tick to hide ordering failures.
- Preserve phase ancestry, run conditions and deferred flushes when you move a
  system. Two identical `.before` edges can behave differently under different
  parent sets.
- App-local provider registration is valid for real independent providers. A
  closed recipe set uses typed dispatch and metadata registration. None of
  these justifies a service locator, a generic message bus, type-erased
  dependency injection or global plugin discovery.
- Shared vocabulary needs a semantic owner and a bounded interpreter. Two
  consumers of one type is not a reason to put it in `shared_tangle`.

### Public API and naming

Keep `ambition_platformer2d` as the primary ergonomic semantic facade and allow focused reviewed SDK/host surfaces when real independent consumers justify them. Migrate consumers off internal namespace mirrors, then delete the mirrors. `actor_monolith` and `shared_tangle` are historical warning names, not permanent barriers to a justified rename. Rename or divide them based on current ownership, dependent customers and actual package closures, not simply when an old carve packet is checked off. `features` should not be an owner category. `runtime`, `characters`, `combat` and `core` are not by themselves evidence of coherent boundaries; neither are small crate sizes or an SCC count.

Completion evidence is a simpler responsibility map, fewer unrelated writers, a
production behavior witness and the relevant profile or API benefit. SCC counts
are diagnostics only.

## Target logical authorities

Names are logical responsibilities, not approved crate names. Start with
modules and visibility inside existing crates. Extract a crate only for a real
consumer, independent installation, a closure benefit or a measured build
benefit.

| ID | Authority | Owns | Does not own |
| --- | --- | --- | --- |
| T1 | Simulation primitives and tick protocol | geometry values, tick and frame identity, time domains, stable identity vocabulary | session selection, room transitions, movement tuning |
| T2 | Platformer body motion and contacts | kinematics, motion model, contacts, attachment, transit, collision response | player identity, brain strategy, score, animation assets |
| T3 | Accepted control and custody relations | participant-to-body driver relation, handoff, body claims with mount, possession and held relations | device bindings, brain policy, ability eligibility, camera |
| T4 | Action execution and victim reactions | action progression and teardown, attack and hurt geometry publication, accepted reactions | action selection, stock rules, save |
| T5 | Projectile travel and contact | projectile lifetime, path segments, world-hit policy, reflection, contact selection | boss phases, rewards, family-string dispatch |
| T6 | Destructible and interactive world objects | destructible health, break and respawn transitions, mechanism state | every item, NPC or checkpoint |
| T7 | Physical items, custody and accounting | occurrence lifecycle, collection, custody transfer, held use and throw, inventory reconciliation | checkpoint routing, reward policy, UI |
| T8 | Spatial world and residency | room geometry, live-instance membership, dynamic collision, query views, residency sets | character catalogs, sprite preparation, camera targeting |
| T9 | Lifecycle and reconstruction coordinator | session generation, admitted lifecycle intent, transition state machine, checkpoint routing, retirement | item internals, AI, shrine healing |
| T10 | Definitions, preparation and construction adapters | authored schema, validation, prepared immutable values, lowering into typed plans | live-world mutation, participant selection, device textures |
| T11 | Observation and intent producers | perception budget, attention, memory, decision policy, input translation | body integration, world mutation |
| T12 | Ruleset outcomes and game content | stock, round and match outcome, rewards, named characters and bosses, story orchestration | body physics, collision kernels |
| T13 | Presentation, assets and external effects | rendering, drawable geometry, per-view composition, audio and VFX delivery, asset residency, confirmed effects | damage geometry, stock state, input |
| T14 | Host, composition and public SDK | provider selection, profile prerequisites, backend and window selection, cross-capability ordering | body algorithms, service locator |

Each authority needs an identifiable writer or coordinator, an explicit
lifetime and an output that other domains consume without its policy. The
dependency shape is:

```text
host / game composition
  selects providers, capabilities, profiles, backends, lifecycle policy
       +--> preparation adapters --> immutable prepared definitions
       +--> domain installers    --> deterministic simulation authorities
       +--> lifecycle coordinator --> domain checkpoint / retire operations
       +--> presentation adapters <-- read models / confirmed effects

spatial / motion / math / time primitives
  never depend on game content, named bosses, UI, device residency or the host
```

The engine stays 2D-first and platformer-capable. Do not parameterize present
code over spatial dimension to advertise parity.

## Standing decisions

- Construction and live mutation are separate operations. Keep body construction
  below live body execution. Provoking,
  querying or rebuilding an existing actor is not spawning.
- Construction, transit and resource hydration are different operations even
  when one reset triggers all three. Rebuild an occurrence with its canonical
  constructor. Relocate a body through transit. Restore inventory through its
  domain.
- Checkpoint restoration is a session-owned operation. A rest point detects
  interaction, heals and requests capture only.
- Contact selection, accepted damage and presentation are separate stages.
  None of them reclassifies a selected victim by feature-family strings.
- Possession eligibility is ability policy. The accepted driver relation and
  input projection are control authority. Keep them understandable together,
  even if an internal cycle remains.
- Rollback registration does not drag a backend into a domain.
- World-to-domain lowering belongs at a preparation boundary. World geometry
  does not depend on character materialization.
- Agent authoring is a control plane: inspectable source, typed schemas,
  preparation, validation, a headless fixture and explicit publication. Model
  reasoning is never part of a simulation tick.
- Trusted native code is not sandboxed by a typed registration API. Data,
  procedural modules and raw engine plugins are three trust tiers.
- The supported reload path builds and verifies a hidden candidate before it
  retires the active scenario. General unsafe-plugin recovery and arbitrary
  save-schema migration are out of scope.

## Program map

| Program | Acceptance | Owner |
| --- | --- | --- |
| E1 simulation authority and lifetime | canonical writers, stable identity, explicit tick and confirmation, correct retirement | [simulation authority](simulation-authority-and-determinism.md), [instance lifetime](instance-lifetime-provenance-and-persistence.md) |
| E2 construction and reconstitution | typed domain plans, singular occurrence construction, distinct transit and hydration | [construction](construction-and-reconstitution.md), [immutable content](immutable-content-and-transactional-construction.md) |
| E3 persistent systemic world | occurrences, custody, durable disposition, nonresident state | [open world](open-world-runtime-and-residency.md), [items](item-custody-and-accounting.md), [progression](capability-progression-and-world-gating.md) |
| E4 body kernel and domain ownership | A-packets; no SCC target | [decomposition](actor-monolith-decomposition.md), [packets](actor-monolith-work-frontier.md) |
| E5 capability and host composition | explicit prerequisites, owner installation, absence and re-entry, measured closure | [composition](capability-and-runtime-composition.md) |
| E6 public SDK | an external game needs no internal module map | [SDK](public-sdk-1.0.md) |
| E7 runtime, assets and iteration | qualified budgets, quality and residency, build measurements | [performance](performance-and-iteration.md), [assets](asset-preparation-and-residency.md), [distribution](project-build-and-distribution.md) |
| E8 multiplayer and multiview | participants, bodies and views stay distinct | [multiplayer](multiplayer-and-multiview.md), [netcode](netcode.md) |
| E9 agent-native authoring | data and module edits without a host relink | [authoring](authoring-and-tools.md), [world tools](ldtk-authoring-and-world-tools.md), [extension](extension-model.md) |
| E10 world facts and orchestration | bounded observations, typed domain operations | [facts](world-facts-observations-and-memory.md), [orchestration](authored-gameplay-logic-and-orchestration.md), [agentic runtime](agentic-character-runtime.md) |
| E11 presentation and observability | read models, body-owned drawable geometry, per-view composition | [presentation](render-animation-and-vfx.md), [inspection](inspection-diagnostics-and-workbench.md) |
| E12 competitive capability | real authored game slices | [capability bar](godot-class-2d-capability.md) |

Program exit: a small independent game selects a supported headless profile,
authors a world, body, action and object, drives them through semantic input,
tests the outcome, adds presentation and produces a platform artifact. The
flagship uses the same authorities.

## Open questions

- Public cross-version save compatibility, untrusted mod distribution,
  concurrent world scheduling and hardware targets are product decisions.
- Projectile leg reconstruction uses current position, velocity and `dt`.
  Acceleration, returning motion and portals can need real path segments.
  Build a witness before you call it a bug.
- Prepared content fingerprints do not fingerprint native function behavior.
  Same-build networking identifies compatible executables.

## Forbidden regressions

- Do not move a lifecycle slot into `shared_tangle` to erase an edge. Move the
  consumer that owns the operation.
- Do not split accepted control, possession projection and custody into
  services that need a registry round trip to keep one invariant.
- Do not create a `features` plugin or crate.
- Do not make the host a gameplay kernel. Composition may order owners; it must
  not hide their algorithms behind a context object.
- Do not replace a valid direct call with a message to remove an import.
  Deferred delivery changes visibility, ordering and rollback meaning.
- Do not declare success from fewer SCCs, re-exports or private-system names
  while one state is still multiply interpreted.
- Do not keep internal compatibility aliases. Public compatibility is a separate
  product choice.
- Do not copy Godot's scene tree or Unity's editor. The test is whether a game
  can be expressed, inspected, tested, iterated and shipped.
- Do not add a repository-wide scanner for every new rule. Prefer the smallest
  behavioral witness.
