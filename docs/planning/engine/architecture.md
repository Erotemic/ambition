# Engine architecture: current shape, doctrine and target authorities

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

## Converged shape

Code against these facts. Each one is in source and has a guard.

| Fact | Where it is |
| --- | --- |
| **Crate tiers.** Engine crates live under `crates/`. Named games, providers, content and app shells live under `game/`. | workspace `Cargo.toml`; policy `repository.tier-manifests-exist` |
| **One session root.** Live platformer world state (`RoomSet`, `RoomGeometry`, `ActiveRoomMetadata`, `StartingCharacter`, `LdtkRuntimeIndex`, room and encounter music requests) is a set of components on the exact `SessionRoot` entity. There is exactly one `SessionRoot`, alive or hidden. No process `Resource` and no projection bridge holds this state. | `ambition_platformer2d_shared_tangle::lifecycle::session::{SessionRoot, session_world_component}`; policy `engine.canonical-session-world-only` |
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

Authority decomposition is necessary for composition but does not prove it.
Measure composability along four independent axes:

| Axis | Falsifiable question |
| --- | --- |
| Semantic authority | Can the owner be understood without the sibling's internal policy and writers? |
| Runtime installation and lifetime | Does the capability run with only declared prerequisites, including teardown and re-entry? |
| Compile-time closure | Is an absent capability absent from the selected normal dependency and feature closure? |
| Public API | Can an external game use the supported path without internal re-exports? |

A no-render plugin configuration can still compile renderer dependencies.
`scripts/check_facade_dependency_closure.py` owns the facade closure count; do
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

Keep one ergonomic facade (`ambition_platformer2d`). Migrate consumers off
internal namespace mirrors, then delete the mirrors. `actor_monolith` and
`shared_tangle` are honest warning labels until their exits are met. Retire
them by removing the accidental responsibilities, not by renaming them.
`features` must disappear as an owner category. `runtime`, `characters`,
`combat` and `core` are not evidence of coherent boundaries.

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

- Construction and live mutation are separate operations. Keep the corrected
  spawn boundary (`scripts/tests/test_actor_spawn_boundary.py`). Provoking,
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
