# Architecture consolidation census

- **Census baseline:** `662a9b56096a304ce9fcfe3ebcf1177403f2452d`
- **Planning-control refresh:** `2dbd81abc50f42a601d8e6177478ef0985002365`
- **Method:** static source, manifest, planning, and generated-inventory inspection. No Rust compiler or runtime was used. The later refresh updates only documentation-control-plane entries; other architecture claims retain the census baseline.

This document describes current architecture. It does not record the sequence of reviews that found it.
The stable machine entries are in [`consolidation-ledger.json`](consolidation-ledger.json).

## Executive map

Ambition already has several strong single-owner patterns:

1. `ActiveRollbackAuthority` owns rollback owner, timeline generation, contract, and health as one resource. Confirmation is derived from it.
2. `SessionRoot` owns major live world components. `SessionWorldRef` and `SessionWorldMut` query those Bevy components directly.
3. Content preparation separates active selection from a pending candidate. A pending generation does not become the active selection merely because preparation started.
4. The mechanical editor uses a shared Propose -> Admit -> Publish protocol for six production domains.
5. Typed construction owns stable simulation identity, provenance, relationships, and candidate entity visibility.

The main consolidation pressure is not the number of ECS objects. It is where one live fact can still move through more than one publication or lifetime road:

- room/session replacement is not one switch yet;
- local lifecycle identifiers still enter some canonical provenance;
- 32 process/App resources are explicitly documented by source as session- or generation-owned;
- direct-entry compatibility still gives some canonical values a second App-global fallback road;
- live content/session values can be updated separately around development reload.

The current A10 implementation should finish before another agent changes the room publication model.
The identity correction is also separate active work.

## Evidence and limits

`SOURCE_CONFIRMED` means explicit source establishes the claim.
`SOURCE_INFERRED` means static source shape supports the claim but does not prove the full production execution path.
`DOC_CLAIM` means a current architecture/planning document states it and this census did not independently prove it.
`NEEDS_COMPILED_VERIFICATION` and `NEEDS_RUNTIME_VERIFICATION` mark facts that static inspection cannot establish.

No count in this document is a limit. A count is only a baseline for later comparison.

## 1. Mechanical authority ledger

The table uses authority *families*. One row can cover an owner and its direct projections when those projections answer one fact.

| ID | Authority | Owner | Representation | Semantic lifetime | Authoritative for | Rollback relationship | Generation relationship | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AUTH-SESSION-SCOPE | Active gameplay session scope | ambition_platformer2d_shared_tangle::lifecycle | Resource + Components | App with gameplay-session sub-lifetime | which gameplay session scope is current for session-aware spawning | host-side local lifetime authority; not the peer-stable rollback identity | unrelated to content identity; one scope can own one activated generation at a time | one owner plus entity projections | SOURCE_CONFIRMED |
| AUTH-SESSION-WORLD | Canonical live platformer session world root | ambition_platformer2d_runtime + ambition_platformer2d_shared_tangle | Bundle on canonical entity | gameplay session | the live session world catalogs, room set, geometry, active-room metadata, initial-body policy, and session requests | canonical live world owner; rollback registration is per root component, not one bundle snapshot | contains both frozen generation projections and mutable session/timeline state | owner-scoped canonical state | SOURCE_CONFIRMED |
| AUTH-SHELL-SESSION | Shell gameplay session instance | ambition_game_shell | Resource with optional GameplaySessionInstance | shell activation / gameplay session | which shell activation owns the live gameplay session and world entity | host-only lifecycle authority above rollback simulation | correlates one shell activation with the activated prepared session; it is not content identity | shell lifecycle authority | SOURCE_CONFIRMED |
| AUTH-SESSION-MECHANICS | Frozen mechanics for the activated content generation | ambition_platformer2d_actor_monolith::session::mechanics | Resource | content generation within a gameplay session | which frozen registries and developer mechanical inputs live-session construction must consume | frozen host-side construction input; resulting runtime state can be rollback state | frozen generation input | generation authority with direct-entry fallback | SOURCE_CONFIRMED |
| AUTH-ROLLBACK | Active rollback authority | ambition_platformer2d_runtime::rollback::authority | Resource | gameplay session with rollback-timeline generations | who owns rollback, which timeline is current, what world it rewinds, and whether it may authorize confirmed effects | canonical host authority for the live rollback timeline; it governs snapshots rather than being ordinary snapshotted gameplay state | contract contains prepared content identity and snapshot schema for the governed timeline | already consolidated authority | SOURCE_CONFIRMED |
| AUTH-CONTENT-BINDING | Live session content binding | ambition_platformer2d_actor_monolith::world::rooms::transaction | Resource | content generation within gameplay session | which content generation a room transaction may publish into | commit-boundary host authority; no source evidence here that it is rollback-snapshotted | frozen generation identity projection: the ContentEpoch for the live session generation | canonical check today, but publication is split from other generation state | SOURCE_CONFIRMED |
| AUTH-CONTENT-IDENTITY | Prepared content identity | ambition_platformer2d_runtime::content_identity | Component value derived from PreparedContent | prepared generation / activation | the exact prepared-content activation and snapshot schema used by a session | identity input to rollback contract; the source registers related schema/content contracts, but this census does not claim whole-component snapshot behavior beyond explicit registration | frozen generation input; mixes peer-stable fingerprints with App-local epoch | mixed canonical content digest plus local activation lineage | SOURCE_CONFIRMED |
| AUTH-SELECTED-CONTENT | Selected authored content identity | ambition_platformer2d_runtime::content_identity + ambition_content | Resource + pending transaction resource | App active selection with transaction-local candidate inputs | which authored content the App selected, versus which candidate one load transaction is preparing | host-side content selection; not gameplay rollback state | names the active authored pack selection from which a generation can be prepared | legitimate active/candidate separation | SOURCE_CONFIRMED |
| AUTH-ROOM-SET | Live room selection and definitions | ambition_platformer2d_world::rooms on SessionRoot | Component on SessionRoot | gameplay session / room selection | which room set and active room the session uses | canonical rollback-registered component on SessionRoot | live projection of the admitted generation plus current room selection | canonical root state with non-atomic replacement today | SOURCE_CONFIRMED |
| AUTH-ROOM-GEOMETRY | Live room geometry | ambition_platformer2d_core on SessionRoot | Component on SessionRoot | active room | the room collision/world geometry used by simulation | canonical rollback-registered component on SessionRoot | runtime projection of the current admitted room/world geometry | — | SOURCE_CONFIRMED |
| AUTH-MOVING-PLATFORMS | Live moving-platform state | ambition_platformer2d_world + actor_monolith session lifecycle | Resource | active room within gameplay session | current moving-platform simulation state | canonical rollback-registered Resource | mutable timeline state initialized by room construction | — | SOURCE_CONFIRMED |
| AUTH-OCCURRENCE-CUSTODY | Occurrence and custody continuity state | ambition_platformer2d_shared_tangle + ambition_persistence + actor_monolith | Resources and typed baselines | gameplay session with durable continuity inputs | which authored occurrences exist, where custody resides, and which facts a rebuild must retain | mixed domain continuity state; some values are rollback projections and some are durable/session baselines; use owner-specific registrations | session continuity state; not a content-generation identity | distinct durable domain authorities; not one generic reset flag | SOURCE_CONFIRMED |
| AUTH-MOVEMENT-TUNING | Active movement tuning | ambition_platformer2d_core::movement::tuning | Resource | mutable gameplay-session mechanical state | movement tuning simulation consumes now | mechanical input consumed by rollback simulation; mechanical-edit admission prevents unsynchronized live mutation | mutable timeline mechanical state, not a frozen content-generation value | — | SOURCE_CONFIRMED |
| AUTH-FEEL-TUNING | Active platformer feel tuning | ambition_combat::feel | Resource | mutable gameplay-session mechanical state | combat and platformer feel tuning simulation consumes now | mechanical input consumed by rollback simulation; live edits pass through mechanical edit admission | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-PORTAL-TUNING | Active portal tuning | ambition_portal2d::tuning | Resource | mutable gameplay-session mechanical state | portal mechanics tuning simulation consumes now | mechanical input consumed by rollback simulation; live edits pass through mechanical edit admission | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-ABILITY-MASK | Admitted developer ability mask | ambition_dev_tools | Resource plus body-component projection | mutable gameplay-session developer mechanical state | the admitted developer ability mask | host-side admitted mechanical input; projected body ability state participates in simulation | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-BODY-PROFILE | Admitted developer body profile | ambition_dev_tools | Resource plus body-component projection | mutable gameplay-session developer mechanical state | the admitted developer body profile | host-side admitted mechanical input; projected body state participates in simulation | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-PLAYER-STATS | Live player stat components | ambition_dev_tools adapter + body component owners | Components with editable mirror and sync snapshot | entity / mutable gameplay session | the live player health, mana, and offense values | live body components are gameplay state; editor publication changes those components only after admission | mutable entity/timeline state | — | SOURCE_INFERRED |
| AUTH-CHECKPOINT-RESTORE | Accepted checkpoint restore operation | ambition_platformer2d_actor_monolith::session::checkpoint | Resources and operation keys | gameplay session / operation | which checkpoint restore has been admitted and which pinned facts it may apply | rollback-registered Resource with explicit checksum projection | session operation state; it pins reconstruction inputs but is not content identity | — | SOURCE_CONFIRMED |
| AUTH-LIFECYCLE-COMMIT | Pending lifecycle commit | ambition_platformer2d_actor_monolith::session::lifecycle_commit | Resource | gameplay session / lifecycle operation | which lifecycle transition/replay intent owns the pending commit slot | canonical rollback-registered Resource | mutable timeline/session state | — | SOURCE_CONFIRMED |

### Writer and reader map

These are representative writer/read classes. They are not a call-graph export.

| ID | Construction writers | Runtime writers | Editor/dev writers | Persistence writers | Main readers |
| --- | --- | --- | --- | --- | --- |
| AUTH-SESSION-SCOPE | — | ambition_game_shell::session activation and retirement | — | — | session spawn scoping, simulation authorization, session teardown, match activation, presentation scoping |
| AUTH-SESSION-WORLD | provider session activation, room construction and replacement | room transition and session systems mutate root-owned components | development reload replaces selected root-owned values | save/checkpoint adoption supplies selected construction inputs before reconstruction | simulation systems through SessionWorldRef/SessionWorldMut, presentation systems, room and quest policy |
| AUTH-SHELL-SESSION | — | ambition_game_shell session coordinator | — | — | shell route/session bridge, host tests and host lifecycle diagnostics |
| AUTH-SESSION-MECHANICS | platformer provider activation installs frozen prepared mechanics | — | developer mechanical values are captured before activation, not read live after activation | — | initial construction, room transition construction, reset construction, perception construction |
| AUTH-ROLLBACK | rollback timeline install and initial world adoption | rollback diagnostics, stand-down, and health updates | mechanical edit admission can rebase or replace the timeline through rollback host policy | — | confirmation policy, confirmed-effect gates, mechanical edit admission, diagnostics |
| AUTH-CONTENT-BINDING | session setup | — | development reload commit | — | room transaction verification |
| AUTH-CONTENT-IDENTITY | prepared content assembly, provider session root materialization | — | development content preparation creates the candidate value | — | rollback timeline contract, content diagnostics, session root consumers |
| AUTH-SELECTED-CONTENT | content pack selection | content generation commit can replace the active selection | development content selection/reload coordinator | — | provider preparation when no transaction-specific candidate claim overrides it |
| AUTH-ROOM-SET | session setup, room construction commit | room transition and reset construction | development reload construction | selected lifecycle/checkpoint inputs can choose reconstruction target | room transition policy, quest systems, host/presentation systems, construction and replay logic |
| AUTH-ROOM-GEOMETRY | session setup and room construction commit | room replacement | development reload construction | — | collision and movement, portal adapters, presentation, developer probes |
| AUTH-MOVING-PLATFORMS | session setup and room construction commit | moving-platform simulation | development reload construction can replace the set | — | collision composite, portal transit, movement and diagnostics |
| AUTH-OCCURRENCE-CUSTODY | session activation adopts admitted baseline facts | occurrence, pickup, custody, and entitlement domain systems | — | save and checkpoint adoption/update roads | room construction population policy, checkpoint restore, save projection |
| AUTH-MOVEMENT-TUNING | initial host/resource setup | mechanical edit Publish stage after admission | editable movement tuning proposal | — | movement simulation, damage/knockback policy, developer projections |
| AUTH-FEEL-TUNING | initial host/resource setup | mechanical edit Publish stage after admission | editable feel tuning proposal | — | combat and damage feel policy |
| AUTH-PORTAL-TUNING | initial plugin/resource setup | mechanical edit Publish stage after admission | editable portal tuning proposal | — | portal emission, input warp, transit, and wall ability policy |
| AUTH-ABILITY-MASK | developer-tools initialization | mechanical edit Admit/Publish path | editable ability proposal | — | ability projection to live bodies, developer UI/projections |
| AUTH-BODY-PROFILE | developer-tools initialization | mechanical edit Admit/Publish path | developer body profile proposal | — | body-profile projection, movement-profile and body policy application |
| AUTH-PLAYER-STATS | actor/body construction | combat/gameplay systems, mechanical edit Publish stage | editable player stats proposal | save/adoption roads where the owning stat domain participates | combat, HUD, abilities, simulation |
| AUTH-CHECKPOINT-RESTORE | checkpoint admission captures the reconstruction inputs | session checkpoint coordinator accepts and retires one operation | — | checkpoint/save source supplies pinned baselines | room preparation, checkpoint application, terminal outcome publication |
| AUTH-LIFECYCLE-COMMIT | — | simulation records lifecycle intents; host commit road consumes/clears them | — | — | rollback lifecycle bridge, eager/confirmed lifecycle commit paths |

### Authority conclusions

- `AUTH-ROLLBACK` is a positive model: it combines facts that would be invalid if they drifted.
- `AUTH-SESSION-WORLD` is another positive model: major live world facts are components on the canonical `SessionRoot` and are read with normal Bevy queries.
- `AUTH-CONTENT-BINDING` is a real commit-boundary authority today, but A10 can make it a projection of one admitted live candidate instead of a separately queued write.
- `AUTH-SESSION-MECHANICS` fixed a real generation-lifetime problem, but the fallback to App registries still creates a second construction source for supported direct compositions.
- Editor authorities are mutable timeline state by design. They must not be folded into immutable content-generation identity merely to reduce object count.

## 2. Duplicate-truth families

The census found eight important families. Four are current consolidation pressure. Four are valid stage/projection separation.

| ID | Family | Classification | Current state | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- |
| DUP-SESSION-CURRENT | Current gameplay-session identity family | NEEDS_SEMANTIC_REVIEW | ActiveGameplaySession, ActiveSessionScope, GameplaySessionLinks, SessionRoot, and GameplaySessionWorldRoot all carry pieces of the live session identity. Most answer different layer-specific questions, but several repeat scope/activation correlation. | Audit writers and readers after A10. Keep one owner per question and make other values projections or captured correlation. Do not collapse shell route identity into simulation identity. | SOURCE_INFERRED |
| DUP-CONTENT-CURRENT | Live content-generation identity family | SUSPECT_DUPLICATE_AUTHORITY | PreparedContent, PreparedContentIdentity, ActiveContentBinding, and hot-reload-owned prepared state are published through more than one write road. A10 comments state that these values can move independently of the room verification verdict. | Publish one admitted candidate session/world record and derive the live content binding, prepared identity, and prepared content references from that switch. | SOURCE_CONFIRMED |
| DUP-GENERATION-MECHANICS | Activated generation mechanics versus App registries | KNOWN_TRANSITIONAL_LAYER | GenerationMechanics reads SessionMechanics when a generation is active, but can read App registries for direct-entry compositions with no activated generation. | If all supported gameplay compositions gain a normal prepared-generation owner, delete the App-registry fallback from live construction. Otherwise keep the composition distinction explicit. | SOURCE_CONFIRMED |
| DUP-EDITOR-STAGES | Mechanical editor desired/admitted/projection stages | LEGITIMATE_STAGE_SEPARATION | Editable mirrors, pending proposals, admission, admitted authority, and runtime projection are intentionally different stages. The body-profile and ability repairs show that collapsing admission with projection loses state when a target entity is absent. | Consolidate protocol shape and registration, not the distinct values. New editable mechanical domains must use the same stages or explicitly document why a stage is not applicable. | SOURCE_CONFIRMED |
| DUP-CONTENT-CANDIDATE | Active content selection versus pending generation inputs | LEGITIMATE_STAGE_SEPARATION | SelectedContentIdentity is active App selection. PendingGeneration and PendingGenerationInputs own candidate transaction values until activation. They must not overwrite the active selection during preparation. | Keep the active/candidate split. A10 should align its scene candidate with this same lifecycle rather than create a parallel candidate state machine. | SOURCE_CONFIRMED |
| DUP-ROLLBACK-CONFIRMATION | Rollback authority versus confirmation answer | LEGITIMATE_STAGE_SEPARATION | RollbackConfirmationState is deliberately not a Resource. It is derived from ActiveRollbackAuthority for a requested session scope. | Preserve this pattern. Do not promote derived answers into independently mutable resources. | SOURCE_CONFIRMED |
| DUP-CONSTRUCTION-DIAGNOSTICS | Construction authority versus last-result diagnostics | LEGITIMATE_STAGE_SEPARATION | LastRoomConstructionCommit and LastConstructionVerification are documented as developer/test evidence. RoomSet and spawned authoritative entities remain live authority. | Keep diagnostics read-only. Do not let future systems use the last-result resources as simulation authority. | SOURCE_CONFIRMED |
| DUP-ROOM-PUBLICATION | Room replacement state published in separate stores | SUSPECT_DUPLICATE_AUTHORITY | One room replacement changes root components, MovingPlatformSet, room entities, and possibly content/session values on different ordered roads. Verification can withhold RoomLoaded but cannot make all of those stores one publication decision today. | A10 should make the replacement a single candidate state object or owner switch whose projections include non-entity room state and generation state. | SOURCE_CONFIRMED |

The four campaign-counted pressure families are:

- `DUP-SESSION-CURRENT`: semantic review is still needed. Shell activation, session scope, links, and root identity live at different layers. Most distinctions are valid, but the repeated correlation values need one ownership map.
- `DUP-CONTENT-CURRENT`: live content identity and live prepared/session values can move on separate write roads.
- `DUP-GENERATION-MECHANICS`: live-session construction prefers `SessionMechanics`, while direct compositions can still read App registries.
- `DUP-ROOM-PUBLICATION`: one room switch changes entity visibility, `RoomSet`, `RoomGeometry`, moving platforms, and sometimes content/session values through separate writes.

Do **not** consolidate the staged editor values, active-versus-pending content, rollback confirmation answer, or construction diagnostics into their underlying authorities. They answer different questions.

## 3. Lifecycle-state census

The intended lifetime ladder is:

```text
App/process
  -> shell transaction
  -> content generation
  -> gameplay session
  -> match / room
  -> entity
  -> presentation
```

`SessionRoot` already stores several session-world values at the session owner. The remaining high-value mismatch is explicit in source: process resources are reset on session activation because their semantic lifetime is one session or one activated generation.

| ID | Family | Semantic owner | Storage/representation | Current state | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| LIFE-SESSION-RESOURCE-AGGREGATE | SessionScopedResources process-storage aggregate | gameplay session | 25 process/App Resources accessed through one SystemParam | SessionScopedResources names 25 App resources that source states belong to one gameplay session. Activation resets them for correctness and retirement resets them for hygiene. | actual storage owner is broader than semantic owner | SOURCE_CONFIRMED |
| LIFE-CHECKPOINT-RESOURCE-AGGREGATE | SessionOwnedCheckpointState process-storage aggregate | gameplay session | 6 process/App Resources accessed through one SystemParam | SessionOwnedCheckpointState names six App resources for one gameplay-session checkpoint coordinator and resets all six at session activation. | actual storage owner is broader than semantic owner | SOURCE_CONFIRMED |
| LIFE-SESSION-MECHANICS | Generation-owned mechanics stored as App resource | content generation within gameplay session | Resource | SessionMechanics is an App Resource whose semantic owner is the activated generation. Retirement removes it; activation overwrites it. | actual storage owner is broader than semantic owner | SOURCE_CONFIRMED |
| LIFE-ROOT-OWNED-WORLD | Session-root-owned world components | gameplay session / room | Components on SessionRoot | RoomSet, RoomGeometry, ActiveRoomMetadata, initial-body policy, and session requests are already stored on the canonical SessionRoot instead of process-global resources. | owner-scoped state | SOURCE_CONFIRMED |

### Explicit narrower-lifetime App resources

`SessionScopedResources` names **25** process resources whose source says one gameplay session owns them:

`MovingPlatformSet, PossessionState, ControlledSubject, EncounterRegistry, EncounterView, BossEncounterRegistry, QuestRegistry, RoomTransitionCooldown, SlotInteractionState, SwitchActivationQueue, SaveRestored, AuthoredOccurrences, OccurrenceBaseline, CustodyBaseline, MintedItemBaseline, LastQuestRoom, LastCutsceneRoom, ProjectileSeqCounter, PendingLifecycleCommit, BaseGravity, ActiveCutscene, CutsceneTriggerQueue, ActiveConversation, CutsceneAdvanceRequest, CutsceneSkipHold`.

`SessionOwnedCheckpointState` adds **6** checkpoint-coordinator resources:

`SessionCheckpointOperations, SessionCheckpointOutcomes, AcceptedCheckpointRestore, AbandonedCheckpointOperation, SessionStartupResume, OutstandingCheckpointRequest`.

`SessionMechanics` is one more App resource whose semantic owner is the activated content generation.
The unique total is **32**.

The current compensation mechanisms are explicit activation reset, retirement cleanup, current-scope checks, generation presence checks, and direct-composition fallback.
These are not automatically defects. They are evidence that storage owner and semantic owner differ.

A later ownership campaign must classify each family before moving it:

- state needed before a `SessionRoot` exists can remain in a process coordinator if it has an explicit session owner key;
- state only meaningful inside a live session is a candidate for root-owned component/bundle storage;
- presentation-only latches can stay separate from canonical mechanical state;
- rollback-registered values need a migration that preserves snapshot registration and restore semantics.

## 4. Construction and reconstruction roads

Nine architecturally relevant roads were found.

| ID | Entry point | Owner | Mechanical inputs | Generation/session inputs | Construction API | Publication point | Failure semantics | Retirement | Shared primitive | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ROAD-INITIAL-SESSION | `prepare_candidate_platformer_session` -> `translate_shell_session_lifecycle` -> `adopt_candidate_platformer_session` | game shell + platformer provider | prepared content, `SessionMechanics`, prepared room/session world | `ShellActivationId`, `SessionScopeId`, load/prepared transaction data | `PlatformerSessionWorld` materialization plus prepared room construction | canonical `SessionRoot` plus activated provider/session state, promoted out of the hidden candidate population at adoption | the candidate session is built and VERIFIED while the route is pending and holds it; a first room that fails refuses the route through `ShellActivationGates`, so the session that is playing is never retired | shell session retirement and session-scope cleanup | normal session root and room construction primitives | SOURCE_INFERRED |
| ROAD-ROOM-TRANSITION | `begin_room_transition_load_system` -> authorization -> `commit_ready_room_transition_system` | platformer runtime room transition | prepared `RoomConstructionPlan`, current world continuity, target room data | current session scope, lifecycle intent, content binding | `RoomConstructionPlan::replace_live_world` | resource/root writes plus room transaction verification and `RoomLoaded` | preflight may refuse safely; after destructive replacement starts, old room is not retained | outgoing room retired before incoming verification on current normal road | same prepared room plan used by reset/reload paths | SOURCE_CONFIRMED |
| ROAD-SAME-ROOM-REPLAY | admitted `PendingLifecycleCommit` replay path | session lifecycle/reset + room construction | current frozen room plan inputs plus replay retention policy | current session, lifecycle intent, attempt/session retention | room replay through the same room construction model | admitted replay boundary, then normal room construction publication | admission can refuse; current room materialization still has the normal room transaction limits | attempt/room state according to replay policy | room plan and typed construction | SOURCE_CONFIRMED |
| ROAD-CHECKPOINT-RESTORE | `restore_checkpoint_on_session_start` / `resume_at_checkpoint_on_reset` and accepted checkpoint operation | session checkpoint coordinator + room lifecycle | accepted pinned occurrence, custody, item, intent, and room inputs | `CheckpointOperationKey`, session scope, accepted lifecycle intent | same-room replay or cross-room transition | exact operation commit and terminal restore outcome | request/admission/preparation can cancel or fail without treating raw request as authority | accepted operation retires after lifecycle intent settles | normal replay/transition constructors; checkpoint policy stays distinct | SOURCE_CONFIRMED |
| ROAD-NEW-GAME | `process_new_game_reset_request` | session reset | explicit new-game/default progress state plus current prepared room mechanics | current gameplay session | room replay/construction after preflight | `NewGameResetCommitted` plus rebuilt world | preflight precedes committed reset; normal room construction limits remain after reconstruction begins | old progress/session attempt state cleared by owner rules | normal room construction and reset sets | SOURCE_CONFIRMED |
| ROAD-DEV-RELOAD | content reload candidate -> app `dev_runtime` reconstruction | content reload + app development runtime | new `PreparedContent`, candidate cast/families, room plan, generation mechanics | reload request/load ids, content identity/epoch, current session | current `RoomConstructionPlan::replace_live_world` plus separate queued state writes | room verifier plus separate content binding/prepared/session state writes | content candidate can refuse before activation; scene replacement can still destroy N before N+1 verdict | old generation/room is retired on current destructive scene road | A10 target is one normal prepare -> validate -> publish -> retire road | SOURCE_CONFIRMED |
| ROAD-RUNTIME-ACTOR | typed construction plan executor / authoritative respawn helpers | shared construction protocol + actor construction owner | typed recipes, stable `SimId`, relationships, prepared services | `TransactionId`, `SessionSpawnScope`, construction provenance | typed `ConstructionPlan` commit paths and executor-owned root minting | normal live commit or inactive candidate publication when that path is selected | plan/roster validation can refuse; candidate entity path can retire hidden roots | transaction-owned candidate roots or normal entity lifecycle | generic typed construction primitive | SOURCE_CONFIRMED |
| ROAD-DYNAMIC-SPAWN | domain spawn requests for projectiles, summons, and similar runtime objects | owning gameplay domain | domain request plus parent/source mechanical state | stable parent `SimId`, rollback-state sequence/counter where required | domain materialization/spawn request seam | entity spawn into live simulation | domain-specific; static census does not claim one global transaction | normal entity/domain lifecycle | stable simulation identity and session spawn ownership | SOURCE_INFERRED |
| ROAD-A10-CANDIDATE | `ConstructionPlan::commit_inactive` | shared typed construction candidate protocol | verified typed construction plan and registered inactive-candidate filter | construction `TransactionId` and session scope | `commit_inactive`, `publish_candidate`, `retire_candidate` | remove `InactiveCandidate` from all roots in the transaction | retire candidate roots while live world stays visible for the entity slice this primitive owns | `retire_candidate` on refusal; old live world retirement is not yet part of the normal room road | candidate entity primitive; A10 must integrate room/session resource ownership | SOURCE_CONFIRMED |

### Construction model count

There are many entry roads, but fewer fundamental models:

1. **Prepared session activation.** A shell/provider transaction creates the canonical session root and installs prepared state.
2. **Live room replacement.** Transition, replay, reset, checkpoint, and development reload share room plans, but current publication still mutates live state before final room verification.
3. **Typed entity construction.** Domain plans mint stable authoritative roots and relationships.
4. **Inactive candidate entity construction.** `commit_inactive` + `publish_candidate` / `retire_candidate` is the bounded A10 primitive, but it is not yet the normal room/session publication model.
5. **Dynamic entity spawn.** Projectiles/summons use entity-level identity and lifecycle; they are not world-replacement transactions.

The main convergence target is model 2 onto model 4 **plus explicit candidate-owned non-entity state**. That is active A10 work, not work for this census.

## 5. Publication and admission roads

| ID | Who decides | Who writes | Validation separate from publication? | Can failure follow partial publication? | Other publication road for same truth? | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PUB-SHELL-SESSION | shell route/router and registered activation gates | `translate_shell_session_lifecycle` plus provider activation | yes: route readiness/gates precede activation | shell gate refusal cancels pending route; provider/session runtime behavior after activation needs runtime proof | shell session activation is distinct from content and room publication | session publication owner | SOURCE_CONFIRMED |
| PUB-CONTENT-GENERATION | `publication_boundary` through the shell gate | `commit_content_generation` and content/provider activation code | yes: pending candidate and gate verdict are separate from active selection | candidate can remain pending or be refused before active family publication; the shell/content A-supersedes-B race still owes its named production witness | content activation and scene/world replacement are still separate publication slices | candidate/admission publication road | SOURCE_CONFIRMED |
| PUB-ROOM-REPLACEMENT | room lifecycle authorization and prepared plan | `replace_live_world` + `PendingWorldReplacement` + transaction close | ✅ 2026-09-14: the whole replacement is STAGED and applied only on admission | no: a refusal drops the candidate roots AND the staged world; N is untouched | one bounded publication authority now writes the root components, the resource state and the entities together | explicit last-good-world boundary | SOURCE_CONFIRMED |
| PUB-CONSTRUCTION-CANDIDATE | typed construction transaction | `publish_candidate` removes `InactiveCandidate` | yes: inactive commit then validation then publish | candidate entity roots can be retired on failure; arbitrary resources/effects are outside this primitive | one bounded entity-publication road; A10 must widen ownership around it | bounded candidate entity publication | SOURCE_CONFIRMED |
| PUB-MECHANICAL-EDIT | rollback owner or default no-rollback policy | each domain Publish system | yes: Propose -> Admit -> Publish | refused/foreign/unhealthy timeline can keep desired value without moving admitted authority | same protocol is shared across six production editor domains | explicit admission road | SOURCE_CONFIRMED |
| PUB-CHECKPOINT | session checkpoint coordinator and lifecycle slot | accepted-operation commit and outcome systems | yes: request -> admission -> commit -> terminal outcome | refusal/cancellation is explicit and keyed to one operation | shares room replay/transition mechanics but keeps checkpoint state policy | explicit admission road | SOURCE_CONFIRMED |
| PUB-NEW-GAME | new-game reset preflight | `process_new_game_reset_request` and dependent reset systems | yes: decision precedes `NewGameResetCommitted` | preflight can stop commit; later room materialization has normal room limits | shares room reconstruction, but reset policy is distinct | explicit reset publication boundary | SOURCE_CONFIRMED |
| PUB-ROOM-LOADED | room transaction verifier | `verify_and_publish` writes `RoomLoaded` | ✅ 2026-09-14: the message and the live world move together — nothing is written before the verdict | a withheld message now means a withheld WORLD, not a notification lost behind changes already made | read in production by three `FreshAttempt` consumers (`ambition_damage`, Sanic `SpentMonitors`, Mary-O `BrokenBricks`) — the earlier "no production reader" row was wrong | publication notification AND the authority switch | SOURCE_CONFIRMED |

Two publication roads need special care:

- Room replacement is a current implicit transaction across several stores. `RoomLoaded` is not the full publication boundary because earlier root/resource writes can already be live.
- Development generation replacement has a mature content candidate/gate road, but scene/world replacement and several session values are not yet one candidate-owned switch.

## 6. Editor and mechanical mutation architecture

The shared mechanical-edit admission protocol is a **completed foundation at this snapshot**, not a current blocker.
The production census found six editor domains and all six use the shared admission protocol.

| ID | Domain | Current stage map | Coverage | Admitted authority | Sources |
| --- | --- | --- | --- | --- | --- |
| EDIT-MOVEMENT | Movement tuning | `EditableMovementTuning` -> `PendingMechanicalEdits` -> `MechanicalEditAdmission` -> `ActiveMovementTuning` -> simulation reads | complete for current production path | ActiveMovementTuning | `crates/ambition_dev_tools/src/dev_tools/editable.rs`<br>`crates/ambition_platformer2d_core/src/movement/tuning.rs` |
| EDIT-ABILITIES | Developer ability mask | editable ability set -> pending domain -> admission -> `ActiveEditableAbilityMask` -> body abilities projection | complete for current production path | ActiveEditableAbilityMask then body projection | `crates/ambition_dev_tools/src/lib.rs`<br>`crates/ambition_dev_tools/src/dev_tools/editable.rs` |
| EDIT-BODY-PROFILE | Developer body profile | `DeveloperTools` desired profile -> pending domain -> admission -> `ActivePlayerBodyProfile` -> body/movement projection | complete for current production path | ActivePlayerBodyProfile then body projection | `crates/ambition_dev_tools/src/dev_tools/editable.rs` |
| EDIT-PLAYER-STATS | Player stats | editable stats -> pending domain -> admission -> live body stat components; reverse mirror sync is separate | complete for current production path | live body stat components | `crates/ambition_dev_tools/src/dev_tools/editable.rs` |
| EDIT-FEEL | Platformer feel tuning | `EditableFeelTuning` -> pending domain -> admission -> `Platformer2dFeelTuningMonolith` -> combat/sim reads | complete for current production path | Platformer2dFeelTuningMonolith | `crates/ambition_combat/src/feel.rs` |
| EDIT-PORTAL | Portal tuning | `EditablePortalTuning` -> pending domain -> admission -> `PortalTuning` -> portal systems | complete for current production path | PortalTuning | `crates/ambition_portal2d/src/tuning.rs` |

`MechanicalEditAdmission` is intentionally optional in a host that has no rollback history to protect.
That is a valid capability distinction, not a fail-open production bug by itself.
A future live mechanical editor domain should be added to the stable ledger with its stage map. Do not add a second admission protocol.

## 7. Local identity and canonical deterministic identity

| ID | Identity | Class | Lifetime | Current use | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ID-CONTENT-FINGERPRINT | ContentFingerprint | CANONICAL_MECHANICAL / intended peer-stable content identity | content definition | ContentFingerprint is a BLAKE3 digest over versioned canonical prepared-content sections. Source excludes timestamps, handles, entity ids, map iteration order, and mutable session state. | — | SOURCE_CONFIRMED |
| ID-CONTENT-EPOCH | ContentEpoch | LOCAL_LIFETIME | App activation lineage | ContentEpoch is an App-local gap-tolerant activation lineage token. It is equality-only and is not a content fingerprint. | — | SOURCE_CONFIRMED |
| ID-PREPARED-CONTENT | PreparedContentIdentity | MIXED_RESPONSIBILITY | prepared activation | PreparedContentIdentity packages canonical fingerprints with the local ContentEpoch. It is exact for one App activation, but the local epoch is not peer-stable by contract. | Do not use the local epoch half where peer-stable canonical identity is required. Keep exact local activation identity separate from peer comparison. | SOURCE_CONFIRMED |
| ID-SESSION-SCOPE | SessionScopeId | LOCAL_LIFETIME | gameplay session | SessionScopeId is minted by an App-local monotonic allocator for gameplay-session ownership. Planning notes identify prior-session count as a source of cross-peer divergence when this local token enters canonical construction identity. | — | SOURCE_CONFIRMED |
| ID-SHELL-ACTIVATION | ShellActivationId | LOCAL_LIFETIME | shell activation | ShellActivationId names one local shell activation. Planning notes flag using it to mint canonical session SimId as a local-history leak into peer identity. | — | SOURCE_CONFIRMED |
| ID-SHELL-REQUEST | ShellRequestId | LOCAL_CORRELATION | shell request transaction | ShellRequestId is caller-minted transaction correlation for shell requests. Content reload uses it before the router later assigns LoadId. | — | SOURCE_CONFIRMED |
| ID-LOAD | LoadId | LOCAL_CORRELATION | load transaction | LoadId is router-minted load transaction correlation. PendingGenerationInputs keys candidate preparation inputs by this id. | — | SOURCE_INFERRED |
| ID-ROLLBACK-TIMELINE | RollbackTimelineGeneration | LOCAL_LIFETIME | rollback timeline | RollbackTimelineGeneration is a process-monotonic identity for one rollback timeline. It distinguishes restarted timelines whose frame numbers begin at zero. | — | SOURCE_CONFIRMED |
| ID-SIM | SimId | CANONICAL_MECHANICAL / PEER_STABLE | logical simulation object | SimId is stable semantic simulation identity used for snapshot, replay, netcode, and deterministic ordering. Dynamic descendants derive from stable parent SimId plus rollback state counter. | — | SOURCE_CONFIRMED |
| ID-TRANSACTION | TransactionId | MIXED_RESPONSIBILITY | construction transaction / rollback-visible provenance | TransactionId stamps authoritative construction roots. The current construction identity includes incoming content binding and SessionScopeId. Active planning records that host-local ContentEpoch and SessionScopeId can therefore change canonical transaction identity for mechanically identical peer worlds. | Split local correlation from peer-stable construction provenance. Build canonical provenance from content fingerprints, stable world/room identity, canonical SimIds, and shared match/session identity where needed. | SOURCE_CONFIRMED |
| ID-ROOM-PLAN | RoomConstructionPlanId | CANONICAL_MECHANICAL within same-build plan semantics | prepared room plan | RoomConstructionPlanId is a stable same-build identity from the frozen room spec and deterministic construction plan. It excludes SessionSpawnScope, TransactionId, Entity, and process-local values. | — | SOURCE_CONFIRMED |
| ID-MECHANICAL-DOMAIN | MechanicalDomain | LOCAL_CORRELATION | host editor proposal batch | MechanicalDomain uses TypeId as a host-local editor-domain key. Source explicitly states that this is valid because PendingMechanicalEdits is host-side and outside rollback. | — | SOURCE_CONFIRMED |

Three local values are known to feed canonical/peer-visible provenance on some current path:

- `ContentEpoch` — App-local generation lineage;
- `SessionScopeId` — App-local gameplay-session ownership;
- `ShellActivationId` — App-local shell activation.

This does **not** make those local types wrong. They are useful local lifetime/correlation identities.
The correction is to stop using them as substitutes for peer-stable mechanical identity.
`PreparedContentIdentity` and `TransactionId` are the two current mixed-responsibility types in the manual ledger.
The active identity campaign owns the fix.

## 8. Optional canonical authorities and capability composition

A raw `Option<Res<T>>` is not evidence of a defect. The static helper finds **732** optional resource accesses over **196** unique type spellings; this is a discovery index only.

| ID | Capability/authority | Classification | Current state | Possible consolidation | Evidence |
| --- | --- | --- | --- | --- | --- |
| CAP-SESSION-SCOPE-OPTIONAL | Optional ActiveSessionScope in mixed compositions | capability legitimately absent today, with compatibility semantics | SessionSpawnScope::for_optional_active_session interprets missing ActiveSessionScope as a direct/legacy process-resident composition and present-with-no-current as a shell frontend where gameplay spawning must sleep. | If session lifecycle becomes universal, remove the missing-resource meaning. Until then, do not make it required without migrating direct/headless compositions. | SOURCE_CONFIRMED |
| CAP-SESSION-MECHANICS-OPTIONAL | Optional SessionMechanics for live room construction | required canonical authority in shell production; capability absent in supported direct compositions | GenerationMechanics::for_live_session refuses missing SessionMechanics in shell-routed live rebuilds but permits App-registry fallback in direct/headless compositions. | Use explicit composition contracts rather than bare Option at each reader. Candidate for removal if all supported gameplay compositions activate a generation. | SOURCE_CONFIRMED |
| CAP-CONTENT-BINDING-OPTIONAL | Optional ActiveContentBinding at room verification | required canonical authority in shell production; explicit direct fixture absence | Room verification permits no ActiveContentBinding in direct fixtures, but refuses it when SessionGatedSimulation marks a shell-routed session. | Make the binding part of candidate/live session ownership under A10 so shell production cannot represent a live session without it. | SOURCE_CONFIRMED |
| CAP-MECHANICAL-ADMISSION-OPTIONAL | Optional MechanicalEditAdmission in non-rollback compositions | capability legitimately absent | Editor publishers treat absent MechanicalEditAdmission as Publish. Source states this is intentional because a composition with no rollback host has no history to protect; a host that can refuse installs the resource. | — | SOURCE_CONFIRMED |
| CAP-INITIAL-READINESS | Optional InitialGameplayReadiness | optional presentation/startup capability | Visible direct-entry hosts can install a closed startup readiness gate. Apps that omit it keep the normal behavior. | — | SOURCE_CONFIRMED |
| CAP-LDTK-INDEX | Optional LDtk session-world index | capability legitimately absent | PreparedPlatformerSource carries an installed LDtk index only when an authoring format installs one. RON-authored sessions legitimately carry none. | — | SOURCE_CONFIRMED |
| CAP-OPTIONAL-RES-CENSUS | Repository optional Res/ResMut surface | NEEDS_SEMANTIC_REVIEW | The static helper counts hundreds of Option<Res<T>> and Option<ResMut<T>> occurrences, but the count is only a discovery surface. Each canonical type needs composition-aware classification before any change. | — | SOURCE_INFERRED |

The useful pattern is explicit composition semantics:

```text
capability installed
  -> owner resource/component is installed
  -> its systems are installed
  -> absence has one documented meaning
```

The risky pattern is a canonical production authority that is simply absent and causes the consumer to continue with current App state.
The shell-routed `SessionMechanics` and `ActiveContentBinding` checks already distinguish those cases from direct fixtures.

## 9. Correctness-sensitive plugin and schedule coupling

The raw source has many `.before()` and `.after()` edges. The census records only seven relationships where ordering carries an architectural invariant.

| ID | Producer | Consumer | Schedule/set or mechanism | Invariant | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ORDER-SESSION-LIFECYCLE | session lifecycle messages/scope owner | cleanup and activation consumers | `SessionScopeSet`: RetireAuthority -> Cleanup -> Activate -> Presentation | old session state cannot become input to the next activation | NORMAL_PIPELINE_ORDER | SOURCE_INFERRED |
| ORDER-MECHANICAL-EDIT | editor proposal systems | rollback admission and domain publishers | `MechanicalEditSet`: Propose -> Admit -> Publish before rollback advance | simulation never observes an editor change before rollback policy admits it | NORMAL_PIPELINE_ORDER | SOURCE_INFERRED |
| ORDER-ROOM-TRANSACTION | transaction open/baseline capture | construction + verify/publish tail | deferred Commands queue order | verification compares the result to the baseline that belonged to this transaction | SUSPECT_IMPLICIT_TRANSACTION | SOURCE_INFERRED |
| ORDER-ROOM-REPLACE | outgoing retirement | root/resource writes + incoming construction + verification | straight-line `replace_live_world` order plus queued commands | current road must not leave two live rooms, but this creates a destructive gap | LIKELY_REPLACEABLE_BY_OWNERSHIP | SOURCE_INFERRED |
| ORDER-CONTENT-ACTIVATION | content publication gate | shell activation and content commit | exclusive shell gate evaluation at otherwise-ready activation | content gate answer and route activation cannot be separated by another system | NORMAL_PIPELINE_ORDER | SOURCE_INFERRED |
| ORDER-CHECKPOINT | checkpoint admission | room/replay commit and domain restore | lifecycle/checkpoint sets and exact operation key | domain restore cannot treat an unadmitted request as authority | NORMAL_PIPELINE_ORDER | SOURCE_INFERRED |
| ORDER-NEW-GAME | new-game preflight | dependent reset systems | `NewGameResetDecided` then `NewGameResetCommitted` | dependent state resets only after the reset is accepted | NORMAL_PIPELINE_ORDER | SOURCE_INFERRED |

`ORDER-ROOM-TRANSACTION` and `ORDER-ROOM-REPLACE` are the main consolidation candidates.
The other rows are normal pipeline order with useful public phase/set vocabulary.
Do not replace clear Bevy set ordering with a custom scheduler abstraction.

## 10. Compatibility and fallback roads

| ID | Road | What it is | Current state | Deletion/consolidation condition | Blocked by | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| TRANS-DIRECT-SESSION-FALLBACK | Unscoped direct/headless session spawning | supported composition answer, not a fallback | `live_scope_of` BRANCHES on `SessionGatedSimulation`: shell-routed selects the root the activation names and answers `None` when no scope is active (a lingering retired root is not a candidate); direct-entry has no activation, so its single root IS the authority. | Remove only if direct/headless hosts are required to adopt the shell session lifecycle — a composition decision, not a correctness one. | — | SOURCE_CONFIRMED 2026-09-16 |
| TRANS-DIRECT-GENERATION-FALLBACK | App registry values for construction with no activated generation | supported composition answer, not a fallback | `GenerationMechanics` is a THREE-STATE contract, asserted per state: shell + generation takes the generation (the App loses); direct-entry + no generation takes the App override, and is the one composition entitled to supply it; **shell + no generation REFUSES** rather than falling back, so a room cannot come out part generation-derivative and part App. | Remove only if direct compositions are required to carry a prepared generation — a composition decision. | — | SOURCE_CONFIRMED 2026-09-16 |
| TRANS-DIRECT-CONTENT-BINDING | Missing content-binding allowance in direct fixtures | supported composition answer, not a gap | `verify_and_publish` discriminates on `SessionGatedSimulation`: a shell-routed session missing `ActiveContentBinding` REFUSES the room; a direct-entry fixture states no binding and means it. | Delete only if direct-entry compositions are required to own a content binding — a composition decision, not a correctness one. | — | SOURCE_CONFIRMED 2026-09-15 |
| TRANS-FACADE-MIRRORS | Umbrella facade and compatibility re-export mirrors | migration/compatibility layer | The responsibility map records facade re-export and legacy module mirrors as compatibility surface, while ownership remains in lower crates. | Remove internal mirror paths as consumers move to the canonical public surface. Keep useful external facade ergonomics. | — | DOC_CLAIM |
| TRANS-PLANNING-HISTORY | Historical closure prose in live queue | resolved documentation debt | The semantic-preservation cleanup at source snapshot `2dbd81abc50f` returned `queue.md`, `status.md`, and the decision ledger to current-state roles. Closed/retracted case-file prose now lives in Git history instead of the live queue. | Keep the queue role structural: open executable rows only, with owner/current state/next action/blocker/acceptance. | — | SOURCE_CONFIRMED |

The baseline snapshot contained **8** transitional roads/families. **Four are now
closed** and their rows are deleted rather than annotated: `TRANS-PLANNING-HISTORY`
(documentation consolidation at `2dbd81abc50f`), and three closed by A10 and
verified against source at `4a243c8ec`:

- `TRANS-ROOM-DESTRUCTIVE` — `apply_world_replacement` has exactly ONE call site,
  inside `finalize_room_publication`, reachable only from a successful verdict.
  Nothing retires N before N+1 is verified.
- `TRANS-CANDIDATE-FLAG` — `ROOM_CANDIDATE_BRACKET` is DELETED, not `true`. Every
  caller passed `true`; the alternate road had no selector.
- `TRANS-HOT-RELOAD-SPLIT` — the reload publishes `ActiveContentBinding` behind
  `publication_succeeded` on its exact publication (`dev_runtime.rs`).

The remaining rows still require their replacement conditions. Some are supported
direct-composition behaviour today. Do not delete them until the replacement
composition exists.

## 11. Crate and package architecture

The root manifest declares **80** workspace packages.
The static helper records Rust LOC and Rust file count for every package. It also records exact direct and reverse workspace dependency names, direct Bevy dependency, a public-API source-text clue, and a responsibility clue from `package.description` or crate-root docs.
Run `python3 scripts/architecture_census.py --crate-table` for the full exact graph, or read `workspace_crates` in the machine ledger.

### Largest packages by nonblank Rust LOC

Size is a navigation signal only. It is not a merge or split rule.

| Package | Nonblank LOC | .rs files | Direct workspace deps | Reverse deps | Direct Bevy | Public items* | Root pub use/mod* | Responsibility clue |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ambition_platformer2d_actor_monolith | 104,962 | 242 | 34 | 7 | yes | 804 | 4 | Ambition gameplay core: content-free simulation systems, runtime state, schedules, and compatibility facades. |
| ambition_app | 99,420 | 245 | 7 | 1 | yes | 384 | 8 | Ambition app shell: assembly, binaries, host glue, headless/RL drivers. |
| ambition_content | 52,454 | 157 | 37 | 2 | yes | 469 | 29 | Ambition's named game content: quests, bosses, items, dialogue, intro, banter. |
| ambition_combat | 52,112 | 88 | 12 | 17 | yes | 634 | 47 | Reusable, content-free combat MODEL: Damage / Hitbox / Hurtbox / DamageVolume, the AttackIntent / AttackPhase / AttackSpec attack vocabulary + intent resolution, and the action-slot kinds... |
| ambition_platformer2d_core | 45,460 | 76 | 1 | 40 | yes | 794 | 21 | ECS-native, content-free pure-logic core (geometry / movement / collision / player_state / abilities / ledge_grab / world / player_clusters + the shared BodyKinematics body component) ext... |
| ambition_characters | 35,660 | 72 | 4 | 26 | yes | 853 | 17 | Unified actor system for Ambition: actor control/AI vocabulary + the universal brain (player, NPC, enemy, boss) + the character catalog. Bosses are actors (ADR 0016). |
| ambition_render | 30,137 | 61 | 18 | 4 | yes | 450 | 14 | Ambition's Bevy presentation layer: the sandbox's default renderer (sprite/world sync, parallax, HUD overlay, screen-space FX, cutscene UI, fonts). Consumes sim read-models and lower pres... |
| ambition_demo_smash | 24,539 | 39 | 2 | 3 | yes | 260 | 21 | Stocks-based platform-fighter demo. The first consumer of the S4 stocks loop, and the oracle for whether a stocks game is expressible through the facade. |
| ambition_platformer2d_shared_tangle | 21,569 | 65 | 4 | 35 | yes | 661 | 34 | Reusable, content-free platformer runtime primitives (lifecycle vocabulary + schedule sets) extracted from ambition_platformer2d_actor_monolith. See docs/planning/status.md and docs/archi... |
| ambition_demo_mary_o | 16,599 | 28 | 2 | 2 | yes | 214 | 1 | Tiny Mary-O demo content home used as an oracle for the Ambition facade crate. |
| ambition_boss_encounter | 14,345 | 49 | 15 | 9 | yes | 219 | 9 | Ambition's boss-fight DOMAIN: the boss data model (BossConfig / BossEncounter / BossOverrides + the borrow views), the authored boss catalog and behavior profiles, the phase machine tick,... |
| ambition_entity_catalog | 14,096 | 27 | 0 | 23 | no | 376 | 26 | Reusable, content-free entity-contract + moveset vocabulary: EntityDef contract bundles and the Smash-model MoveSpec timeline (windows / hit volumes / events on the owner's proper time), ... |
| ambition_app_tools | 13,268 | 18 | 7 | 0 | yes | 18 | 0 | Ambition's non-game binaries: headless/RL drivers, trace replay, scene capture, previews. |
| ambition_platformer2d_runtime | 11,947 | 35 | 37 | 6 | yes | 227 | 5 | The platformer ENGINE face (demo plan E5): a `PlatformerEnginePlugins` plugin group that assembles the content-free simulation plugins, so a game (Ambition, or a demo) builds an engine Ap... |
| ambition_demo_smash_app | 11,708 | 25 | 2 | 1 | yes | 33 | 2 | The smash demo's thin SHELL. The first place a stock is actually spent in a running session rather than in a unit test. |
| ambition_sim_view | 11,407 | 18 | 22 | 7 | yes | 210 | 18 | [the observation boundary] (E4): the SimView read-model — per-body pose + velocity views, the feature/actor/boss/nameplate indexes, the item/hud/prop fact resources, and the follow-camera... |
| ambition_sprite_sheet | 10,138 | 30 | 8 | 14 | yes | 357 | 18 | Reusable, content-free sprite-sheet metadata vocabulary: the SheetRecord / AnimationMetrics / PixelRect / FrameRect schema + the SheetRegistry resource that parses a baked `(filename_root... |
| ambition_game_shell | 9,541 | 20 | 8 | 3 | yes | 216 | 14 | Renderer-independent routing and scoped lifecycle for top-level game experiences, plus neutral sequences and a minimal launcher. |
| ambition_input | 9,308 | 21 | 2 | 20 | yes | 280 | 8 | Device -> engine-owned ControlFrame input adapter for ambition: Platformer2dInputActionMonolith (leafwing), MenuControlFrame, keyboard/gamepad presets, active-input-kind tracking, and inp... |
| ambition_portal2d_presentation | 9,080 | 18 | 4 | 4 | yes | 127 | 11 | Reusable default renderer for the ambition_portal2d mechanic: placed-portal quads + labels, the held/pickup gun sprite, mid-transit body pieces, the disorientation indicator, and the thro... |

`*` Public-item and root re-export counts are source-text heuristics. They are not a resolved Rust API.

### High-impact boundary review

| ID | Crate | Static size/fanout clue | Boundary classification | Current responsibility | Review direction | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CRATE-ACTOR-MONOLITH | ambition_platformer2d_actor_monolith | 104,962 LOC; 242 .rs; 34 direct workspace deps; 7 reverse deps; Bevy | multiple domain services and remaining migration hub | Large and multi-responsibility, but durable architecture explicitly says it is not awaiting size-driven carving. Extract only proven owner boundaries that remove dependency/change fanout. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-RUNTIME | ambition_platformer2d_runtime | 11,947 LOC; 35 .rs; 37 direct workspace deps; 6 reverse deps; Bevy | runtime/session/transition/rollback coordination | High fan-in composition/service crate. Review lifecycle coordinator versus provider/host assembly boundaries, not LOC alone. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-SHARED-TANGLE | ambition_platformer2d_shared_tangle | 21,569 LOC; 65 .rs; 4 direct workspace deps; 35 reverse deps; Bevy | cross-domain neutral protocols and residual shared vocabulary | The responsibility map treats this as a warning label. Keep proven neutral protocols such as construction/lifecycle/SimId; move domain ownership out when a concrete owner exists. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-FACADE | ambition_platformer2d | 3,705 LOC; 8 .rs; 56 direct workspace deps; 13 reverse deps; Bevy | public facade/re-export composition surface | A facade can be useful, but compatibility mirrors can hide ownership. Review forwarding surface separately from the public SDK role. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-GAME-SHELL | ambition_game_shell | 9,541 LOC; 20 .rs; 8 direct workspace deps; 3 reverse deps; Bevy | route/session lifecycle and activation transaction owner | The shell owns a semantic lifecycle not supplied by Bevy: route activation, transaction correlation, session bridge, and publication gates. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-ROLLBACK-GGRS | ambition_platformer2d_rollback_ggrs | 6,611 LOC; 12 .rs; 11 direct workspace deps; 1 reverse deps; Bevy | GGRS adapter and rollback timeline owner | The crate adapts external rollback hosting to engine authority, mutation admission, and lifecycle commit. This is a useful independent capability boundary. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-REGISTRY-CORE | ambition_registry_core | 216 LOC; 1 .rs; 0 direct workspace deps; 4 reverse deps; no direct Bevy | generic registry conflict classification utility | Planning/source use it to make silent replacement impossible. Small size alone is not a merge reason. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-BINDING | ambition_binding | 747 LOC; 1 .rs; 0 direct workspace deps; 2 reverse deps; no direct Bevy | small binding/protocol boundary | Static package size alone does not establish whether this crate is a useful semantic unit or forwarding residue. Manual API/consumer review is needed before a merge decision. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-BODY-SEED | ambition_body_seed | 1,686 LOC; 4 .rs; 8 direct workspace deps; 5 reverse deps; Bevy | body-seeding capability boundary | Static package size and dependency fanout are evidence only. Review whether the crate owns one reusable body-construction concept before any merge decision. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |
| CRATE-MOUNT | ambition_mount | 1,820 LOC; 1 .rs; 4 direct workspace deps; 8 reverse deps; Bevy | mount capability | The architecture plan uses mount as a model of a capability that owns its plugin/system installation and publishes ordering vocabulary rather than exposing foreign system names. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_INFERRED |

The durable architecture is explicit that `ambition_platformer2d_actor_monolith` is **not** waiting for a size-driven carve.
A later extraction must name a semantic owner and show that dependency/change fanout improves.
Small crates are also not merge candidates merely because they are small.

### Full workspace package summary

This compact table gives the whole package surface. Exact dependency names are in `workspace_crates` in the JSON ledger and in `--crate-table` output.

| Package | Nonblank LOC | .rs | Direct deps | Reverse deps | Bevy | Public items* | Responsibility clue |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ambition_abilities | 4,981 | 28 | 11 | 4 | yes | 68 | The WIELDED ability kit: ranged (beam, meteor, shockwave, volley, vortex, sentry, bomb), thrown gravity grenades, the... |
| ambition_app | 99,420 | 245 | 7 | 1 | yes | 384 | Ambition app shell: assembly, binaries, host glue, headless/RL drivers. |
| ambition_app_tools | 13,268 | 18 | 7 | 0 | yes | 18 | Ambition's non-game binaries: headless/RL drivers, trace replay, scene capture, previews. |
| ambition_asset_manager | 5,726 | 28 | 0 | 7 | yes | 219 | Ambition logical asset catalog + source/profile policy + Bevy integration. See docs/systems/asset-manager.md. |
| ambition_audio | 6,518 | 23 | 2 | 6 | yes | 301 | Authored-audio stack: data schema, Kira playback library, adaptive music director. |
| ambition_binding | 747 | 1 | 0 | 2 | no | 37 | The binding resolution boundary: an authored reference resolves through the authority that knows it, and what fails t... |
| ambition_body_seed | 1,686 | 4 | 8 | 5 | yes | 31 | The actor body seed: the prepared body facts every construction path spawns from (ActorClusterSeed, its two construct... |
| ambition_boss_encounter | 14,345 | 49 | 15 | 9 | yes | 219 | Ambition's boss-fight DOMAIN: the boss data model (BossConfig / BossEncounter / BossOverrides + the borrow views), th... |
| ambition_causal | 1,496 | 8 | 0 | 4 | yes | 77 | Structured causal facts and the tick explainer. Domains publish typed facts; `explain_tick` composes them into one ch... |
| ambition_character_sprites | 3,100 | 7 | 5 | 3 | yes | 29 | Reusable, content-free derivations FROM a character sheet: the animation-row picker over body state, the sheet-author... |
| ambition_characters | 35,660 | 72 | 4 | 26 | yes | 853 | Unified actor system for Ambition: actor control/AI vocabulary + the universal brain (player, NPC, enemy, boss) + the... |
| ambition_combat | 52,112 | 88 | 12 | 17 | yes | 634 | Reusable, content-free combat MODEL: Damage / Hitbox / Hurtbox / DamageVolume, the AttackIntent / AttackPhase / Attac... |
| ambition_content | 52,454 | 157 | 37 | 2 | yes | 469 | Ambition's named game content: quests, bosses, items, dialogue, intro, banter. |
| ambition_content_cli | 510 | 3 | 2 | 0 | no | 6 | The content-pack validator's diagnostic front door. Composes the same schema registry the app composes and calls the ... |
| ambition_content_pack | 3,664 | 11 | 0 | 10 | no | 138 | Content-pack compiler. [`compile`] is the single parse/resolve/validate/canonicalize/fingerprint path used by tests, ... |
| ambition_conversation | 3,053 | 19 | 8 | 4 | yes | 72 | Conversation continuity: the rollback-owned ActiveConversation authority, the hold projected from it, the break rule ... |
| ambition_cutscene | 850 | 2 | 1 | 6 | yes | 27 | Reusable, content-free cutscene SCRIPT format + runtime stepper: the CutsceneBeat vocabulary, a CutsceneScript (order... |
| ambition_damage | 3,769 | 2 | 11 | 3 | yes | 13 | ⛔ AND IT RE-EXPORTS NOTHING. Three `pub use` lines forwarded `ambition_combat::util` and `_core::hit_response` items ... |
| ambition_demo_mary_o | 16,599 | 28 | 2 | 2 | yes | 214 | Tiny Mary-O demo content home used as an oracle for the Ambition facade crate. |
| ambition_demo_mary_o_app | 8,050 | 28 | 2 | 0 | yes | 7 | The Super Mary-O demo's thin SHELL: foundation + PlatformerEnginePlugins + PlatformerHostPlugins + the demo's content... |
| ambition_demo_pocket | 247 | 1 | 1 | 0 | yes | 8 | Tiny fourth-provider acceptance fixture for Ambition's provider authoring surface. |
| ambition_demo_sanic | 7,956 | 10 | 2 | 2 | yes | 85 | Tiny Sanic-style demo content home used as an oracle for the Ambition facade crate. |
| ambition_demo_sanic_app | 3,962 | 17 | 2 | 0 | yes | 5 | The Sanic demo's thin SHELL: foundation + PlatformerEnginePlugins + PlatformerHostPlugins + the demo's content and ru... |
| ambition_demo_smash | 24,539 | 39 | 2 | 3 | yes | 260 | Stocks-based platform-fighter demo. The first consumer of the S4 stocks loop, and the oracle for whether a stocks gam... |
| ambition_demo_smash_app | 11,708 | 25 | 2 | 1 | yes | 33 | The smash demo's thin SHELL. The first place a stock is actually spent in a running session rather than in a unit test. |
| ambition_demo_twintrack | 7,096 | 8 | 1 | 1 | yes | 125 | TwinTrack: a character-driven 2D special-relativity plaza and teaching prototype. |
| ambition_demo_twintrack_app | 1,723 | 4 | 2 | 0 | yes | 4 | Standalone shell and acceptance tests for the TwinTrack special-relativity festival. |
| ambition_dev_tools | 6,437 | 14 | 5 | 4 | yes | 170 | Reusable developer-tooling STATE + logic (E1d): the DeveloperTools debug/gizmo toggle resource, keyboard-preset index... |
| ambition_dialog | 2,655 | 10 | 4 | 5 | yes | 76 | Reusable dialogue runtime: the poll-based DialogState view model (typewriter reveal + option selection), the typewrit... |
| ambition_encounter | 3,898 | 21 | 6 | 9 | yes | 126 | Reusable encounter wave/lockdown vocabulary and headless state machine. |
| ambition_encounter_features | 2,300 | 8 | 11 | 3 | yes | 18 | Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not scripted) — distinct from `ambition_boss_encou... |
| ambition_engine_schemas | 72 | 1 | 7 | 2 | no | 1 | The schemas the ENGINE itself owns — the one list, in one place. ⛔⛔ THERE WERE TWO HAND-KEPT COPIES AND A TEST HOLDIN... |
| ambition_entity_catalog | 14,096 | 27 | 0 | 23 | no | 376 | Reusable, content-free entity-contract + moveset vocabulary: EntityDef contract bundles and the Smash-model MoveSpec ... |
| ambition_game_shell | 9,541 | 20 | 8 | 3 | yes | 216 | Renderer-independent routing and scoped lifecycle for top-level game experiences, plus neutral sequences and a minima... |
| ambition_gameplay_trace | 1,766 | 6 | 2 | 5 | yes | 62 | Reusable, content-free gameplay flight-recorder format: the per-frame trace schema (GameplayTraceFrame / GameplayTrac... |
| ambition_geometry | 3,046 | 7 | 0 | 3 | yes | 101 | General geometry and reference-frame primitives. This crate contains platformer-independent shapes, combat volumes, s... |
| ambition_held_items | 3,396 | 4 | 9 | 6 | yes | 42 | The PRESSED collectible: a GroundItem in the world, an empty hand that takes it, the held spec it overlays on the act... |
| ambition_input | 9,308 | 21 | 2 | 20 | yes | 280 | Device -> engine-owned ControlFrame input adapter for ambition: Platformer2dInputActionMonolith (leafwing), MenuContr... |
| ambition_interaction | 333 | 1 | 2 | 11 | yes | 17 | Reusable, content-free interactive-world-object MODEL: the Interactable / InteractionKind vocabulary, Pickup / Chest ... |
| ambition_inventory_ui | 226 | 3 | 2 | 1 | yes | 13 | Reusable inventory menu-navigation state for Ambition-style platformers. |
| ambition_items | 1,924 | 8 | 5 | 7 | yes | 60 | Reusable item catalog and shop primitives for Ambition-style platformers. |
| ambition_load | 1,331 | 6 | 0 | 6 | yes | 52 | Headless load coordination: work evidence, activation barriers, streaming/prefetch roles, cancellation, supersession,... |
| ambition_load_presentation | 1,774 | 7 | 4 | 2 | yes | 32 | Contributor-neutral loading foreground lifecycle with shell adaptation, semantic progress, activities, ready-hold, an... |
| ambition_match | 2,510 | 7 | 8 | 2 | yes | 91 | The versus match, prepared: the participant roster and its three staging shapes, the rules of the stage, prepare_matc... |
| ambition_menu | 4,468 | 12 | 6 | 5 | yes | 161 | Reusable Bevy inventory/menu UI data model and interaction primitives. |
| ambition_menu_kaleidoscope | 2,763 | 9 | 1 | 1 | yes | 22 | The bevy_lunex 3D OoT-style cube renderer for the ambition_menu page model — the FIRST engine extension crate (E1e). ... |
| ambition_mount | 1,820 | 1 | 4 | 8 | yes | 37 | Ambition's MOUNT PAIR: two linked bodies where one carries the other. Owns the pair's components (Mountable / CanPilo... |
| ambition_persistence | 6,765 | 18 | 2 | 18 | yes | 286 | Ambition saved-shape and settings persistence model. |
| ambition_platformer2d | 3,705 | 8 | 56 | 13 | yes | 109 | Facade crate for composing Ambition-derived platformer games from one engine surface. |
| ambition_platformer2d_actor_monolith | 104,962 | 242 | 34 | 7 | yes | 804 | Ambition gameplay core: content-free simulation systems, runtime state, schedules, and compatibility facades. |
| ambition_platformer2d_actor_spawn | 3,617 | 8 | 13 | 5 | yes | 67 | Actor construction and spawn realization for Ambition platformer bodies. Owns spawn requests, spawn/materialization r... |
| ambition_platformer2d_core | 45,460 | 76 | 1 | 40 | yes | 794 | ECS-native, content-free pure-logic core (geometry / movement / collision / player_state / abilities / ledge_grab / w... |
| ambition_platformer2d_host | 3,421 | 6 | 10 | 1 | yes | 30 | The windowed-HOST face (decomposition E5 step 5): a `PlatformerHostPlugins` plugin group that assembles the wiring on... |
| ambition_platformer2d_ldtk | 6,771 | 20 | 5 | 5 | yes | 220 | LDtk backend adapter for Ambition authored world IR. |
| ambition_platformer2d_provider | 4,014 | 4 | 12 | 1 | yes | 50 | The platformer experience-provider protocol: authored catalog identity, one shared preparation/activation lifecycle, ... |
| ambition_platformer2d_rollback_ggrs | 6,611 | 12 | 11 | 1 | yes | 92 | GGRS rollback backend for the Ambition platformer runtime. |
| ambition_platformer2d_runtime | 11,947 | 35 | 37 | 6 | yes | 227 | The platformer ENGINE face (demo plan E5): a `PlatformerEnginePlugins` plugin group that assembles the content-free s... |
| ambition_platformer2d_shared_tangle | 21,569 | 65 | 4 | 35 | yes | 661 | Reusable, content-free platformer runtime primitives (lifecycle vocabulary + schedule sets) extracted from ambition_p... |
| ambition_platformer2d_world | 5,311 | 19 | 5 | 19 | yes | 198 | Backend-agnostic authored world IR: room graph, placement records, moving-platform math, and room metadata. |
| ambition_portal2d | 6,277 | 25 | 2 | 8 | yes | 165 | Reusable, content-free portal mechanic (portal-gun placement, aperture transit math, carve publishing, pieces geometr... |
| ambition_portal2d_presentation | 9,080 | 18 | 4 | 4 | yes | 127 | Reusable default renderer for the ambition_portal2d mechanic: placed-portal quads + labels, the held/pickup gun sprit... |
| ambition_projectile_spec | 48 | 1 | 0 | 2 | yes | 1 | Authored projectile intent — content-free spawn data, and nothing else. Lower authored-intent vocabulary consumed by ... |
| ambition_projectiles | 2,428 | 14 | 9 | 8 | yes | 78 | Reusable, content-free projectile MODEL: shot vocabulary, charge state, ECS projectile components, the unified Projec... |
| ambition_registry_core | 216 | 1 | 0 | 4 | no | 10 | The protocol every canonical registry repeats: what counts as a registration's stable identity, how a second registra... |
| ambition_relativity | 664 | 1 | 0 | 2 | no | 30 | Dimension-independent special-relativity mathematics for Ambition. |
| ambition_relativity2d | 3,263 | 5 | 5 | 1 | yes | 104 | Opt-in 2D spacetime clocks, analytic null signals, and observer measurements for Ambition. |
| ambition_render | 30,137 | 61 | 18 | 4 | yes | 450 | Ambition's Bevy presentation layer: the sandbox's default renderer (sprite/world sync, parallax, HUD overlay, screen-... |
| ambition_settings_menu | 2,455 | 7 | 2 | 1 | no | 53 | The renderer-agnostic settings + system menu IR (E1e): SettingsMenuModel / SettingsOption / apply_settings_option bui... |
| ambition_sfx | 1,029 | 3 | 1 | 14 | yes | 57 | SFX runtime contract for Ambition: SfxId, SfxClip, SfxProvider trait, and Bank/Filesystem/Silent/Layered providers. |
| ambition_sfx_bank | 484 | 1 | 0 | 1 | no | 28 | Reader + format spec for Ambition's binary SFX bank file (.sfxbank). Pure data; no audio or Bevy deps. |
| ambition_sim_harness | 4,246 | 14 | 1 | 2 | yes | 162 | Programmatic simulation harness: reset/step, typed AgentAction, AgentObservation, example reward shaping, and a rando... |
| ambition_sim_view | 11,407 | 18 | 22 | 7 | yes | 210 | [the observation boundary] (E4): the SimView read-model — per-body pose + velocity views, the feature/actor/boss/name... |
| ambition_sprite_fx | 1,021 | 2 | 0 | 2 | yes | 15 | Simple per-sprite visual manipulations (tint, hue shift, saturation, silhouette) as one engine concept. |
| ambition_sprite_sheet | 10,138 | 30 | 8 | 14 | yes | 357 | Reusable, content-free sprite-sheet metadata vocabulary: the SheetRecord / AnimationMetrics / PixelRect / FrameRect s... |
| ambition_time | 911 | 5 | 1 | 17 | yes | 38 | Reusable, content-free time vocabulary + producer (WorldTime / ClockState / named-clock dt accessors / TimePlugin) ex... |
| ambition_touch_input | 4,310 | 8 | 8 | 1 | yes | 81 | Mobile / touch input for ambition: an always-built pure touch-state vocabulary plus a mobile_touch-gated HUD and Leaf... |
| ambition_ui_nav | 840 | 4 | 1 | 8 | yes | 63 | Reusable, content-free UI/menu navigation primitives: windowed list math (visible window + discrete scroll-to-row), p... |
| ambition_vfx | 719 | 4 | 3 | 12 | yes | 47 | Reusable, content-free effect vocabulary: the Effect enum + EffectRequest seam + the generic executor, plus the world... |
| ambition_workspace_policy | 3,682 | 25 | 0 | 0 | no | 65 | Sequestered workspace-policy tests: dependency boundaries, source scans, module-size, architecture ratchets, and thei... |
| ambition_world_items | 1,235 | 4 | 4 | 4 | yes | 26 | The physical life of a collectible in the world: its presence, its motion, and touch-to-collect. |

## 12. Bevy leverage census

| ID | Mechanism | Classification | Why the custom part exists | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- |
| BEVY-SESSION-ROOT | SessionRoot plus SessionWorldRef/Mut | THIN_USEFUL_BEVY_ADAPTER | Uses Bevy components and Single queries directly. The custom part supplies Ambition session ownership, not an ECS replacement. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |
| BEVY-SESSION-COMMANDS | SessionCommands | THIN_USEFUL_BEVY_ADAPTER | Wraps Commands with captured session ownership and reduces system-parameter count. It still exposes normal Bevy Commands through Deref. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |
| BEVY-CONSTRUCTION | Typed construction plans and candidate filters | JUSTIFIED_AMBITION_SEMANTICS | Bevy supplies entities, commands, disabling components, and queries. Ambition adds deterministic SimId/provenance, typed plans, transaction ownership, roster verification, and publication semantics. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |
| BEVY-ROLLBACK | ActiveRollbackAuthority | JUSTIFIED_AMBITION_SEMANTICS | Bevy does not define rollback owner, timeline generation, snapshot schema contract, or confirmation health. The custom resource is domain authority, not an ECS abstraction. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |
| BEVY-CONTENT-GENERATION | Prepared content and generation identity | JUSTIFIED_AMBITION_SEMANTICS | Bevy assets/resources do not define immutable mechanical generation identity, candidate admission, schema fingerprint, or content publication. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |
| BEVY-MECHANICAL-EDIT | Mechanical edit admission protocol | JUSTIFIED_AMBITION_SEMANTICS | Bevy change detection can detect editor writes but does not decide whether a write may change mechanics under rollback. The custom proposal/admission protocol supplies that invariant. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |
| BEVY-FACADE-REEXPORTS | Facade and convenience mirrors | NEEDS_REVIEW | Convenience re-exports can be useful, but they can also hide actual ownership. This is a package/API review, not a reason to hide Bevy itself. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_INFERRED |

The strongest current pattern is **Bevy mechanism + Ambition semantic invariant**:

- entity/component/query storage with session ownership;
- Commands plus captured session scope;
- normal schedule sets plus rollback/edit admission;
- normal entity visibility/filtering plus candidate transaction identity.

The census did not find evidence for a broad replacement of Bevy ECS, schedules, resources, or plugins.
The review target is custom forwarding that hides Bevy without adding an Ambition invariant, not custom semantics such as rollback, content generations, or candidate publication.

## 13. Test architecture

| ID | Test family | Current role | Limit / verification still needed | Evidence |
| --- | --- | --- | --- | --- |
| TEST-PROD-COMPOSITION | Production composition witnesses | App-level tests exercise shell/session/content/room behavior through shipped plugin composition rather than only direct helpers. | — | SOURCE_CONFIRMED |
| TEST-HELPERS | Unit and helper tests | The generated inventory reports thousands of tests. Most are local behavior tests and are not architecture witnesses by themselves. | — | SOURCE_CONFIRMED |
| TEST-POLICY | Policy and source scanners | Static policy tests and scripts enforce source-shape rules, dependency rules, registration rules, and documentation contracts without runtime simulation. | — | SOURCE_CONFIRMED |
| TEST-SCHEDULE | Schedule-graph witnesses | Tests such as sim_phase_pins and scripts such as measure_foreign_system_ordering inspect declared schedule ownership/order. They establish graph shape, not runtime outcome by themselves. | — | SOURCE_CONFIRMED |
| TEST-TWO-APP | Two-App / peer determinism witnesses | The active identity work calls for a two-App witness where local history differs but canonical mechanical snapshots must agree. This census did not run it. | — | DOC_CLAIM |
| TEST-ROLLBACK | Rollback canaries and reversion/poison witnesses | Rollback tests cover state registration, checksum behavior, lifecycle rebase, and mechanical edit admission. Poison tests are valuable at major authority boundaries but should not be multiplied for ordinary details. | — | SOURCE_INFERRED |

The current architecture has many good tests, but tests are not the architecture owner.
Where a future ownership change makes an invalid state impossible by type/storage shape, keep the production-composition witness and remove only tests that become pure duplication.
Do not add poison tests to every census row.
Use them at major authority boundaries where a passing test might otherwise exercise only a helper or the wrong composition.

## 14. Planning and documentation architecture

| ID | Planning layer | Classification | Current role/state | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- |
| DOC-QUEUE | queue.md execution ledger | current executable planning | The cleanup at `2dbd81abc50f` rewrote the queue to open executable rows with owner, current state, next implementation, blockers and acceptance. Closed investigations were removed from the live control plane. | Maintain this role; do not append completion diaries. | SOURCE_CONFIRMED |
| DOC-OWNER-PLANS | Focused owner plans | durable current design within live planning | Focused owner documents define current authority, topology, executable work, acceptance, and forbidden regressions for major engine domains. | — | SOURCE_CONFIRMED |
| DOC-ARCHITECTURE | Durable architecture document | durable design | engine-architecture.md states stable layer ownership, content/construction flow, session/rollback lifetimes, identity rules, and capability composition principles. | — | SOURCE_CONFIRMED |
| DOC-ADR | Architecture decision records | decision record | ADRs record explicit architectural decisions such as immutable prepared content, exact session identity, spawn provenance, and construction planning. | — | SOURCE_CONFIRMED |
| DOC-HISTORY | Engineering journals and Git history | historical evidence, not current-state authority | Repository policy places investigation/history in Git and engineering memory under dev rather than in current planning. The census should not duplicate that narrative. | — | SOURCE_CONFIRMED |

The planning contract already states the target structure: queue for executable work, focused owner docs for current design, ADRs for decisions, and Git/dev for history.
The live control-plane mismatch identified by this census was corrected in the documentation consolidation at `2dbd81abc50f`. Future reviewers should treat renewed historical growth in `queue.md` or `status.md` as a regression of the planning contract, not as a reason to add another archive layer.

## Current complexity classification

### Essential complexity

Keep these semantic distinctions:

- rollback owner, timeline generation, schema/content contract, and health as one rollback authority;
- gameplay session lifetime separate from process and shell transaction lifetime;
- peer-stable simulation identity separate from Bevy `Entity` and local correlation IDs;
- immutable prepared content generation separate from mutable timeline state;
- active content selection separate from a pending candidate;
- editable desired value, admission decision, admitted authority, and runtime projection;
- checkpoint admission/pinned continuity separate from generic room construction;
- typed deterministic construction and explicit publication/retirement;
- direct Bevy ECS, resources/components, systems, schedules, plugins, queries, messages, and entity lifecycle where those already express the semantics.

### Suspected accidental complexity

The strongest current cases are:

1. split room publication across outgoing retirement, root components/resources, candidate entities, and later session/content writes;
2. local lifecycle/correlation IDs entering peer-stable/canonical construction identity;
3. live content/session generation values that can move through separate write roads;
4. session-current correlation repeated across shell/session/root layers where the layer distinction is not always explicit;
5. process storage for state that source itself declares session- or generation-owned, where resets and stale-owner guards compensate for the lifetime mismatch.

### Transitional complexity

- compatibility/facade mirrors.

⚠ **THE THREE "DIRECT-*" ROWS ARE NO LONGER TRANSITIONAL AND THE LIST NO LONGER
CLAIMS THEY ARE.** Measured 2026-09-16, each turns out to be a composition
DISCRIMINATOR with the shell-routed side REFUSING, not a compatibility road
waiting to be deleted: `live_scope_of` and `live_session_world_root` branch on
`SessionGatedSimulation`, `GenerationMechanics` asserts its three states
separately, and `verify_and_publish` refuses a shell session missing
`ActiveContentBinding`. What is left to decide about them is whether direct-entry
hosts should be *required* to carry a session and a generation — a composition
decision, not a correctness gap. `TRANS-FACADE-MIRRORS` is the only genuinely
transitional row still standing (C08).

### Uncertain areas

- whether every current session-current projection should remain separate after A10 and peer identity settle;
- which of the 32 narrower-lifetime resources should move to `SessionRoot`, which should become explicit session-keyed process services, and which are presentation-only;
- whether small boundary crates such as `ambition_binding` and `ambition_body_seed` should stay independent after a full consumer/API review;
- which custom facade/re-export layers are valuable external ergonomics versus internal ownership camouflage.

## Facts static inspection cannot establish

This census does not claim:

- successful Rust type checking, macro expansion, trait resolution, or feature compilation;
- that every declared plugin/system registration is on the shipped production path under all feature sets;
- runtime schedule execution beyond explicit source ordering;
- rollback restore behavior for any state whose registration/restore contract was not explicit in source;
- A10's final last-good-world guarantee on the production room road;
- the two-App peer identity witness with different local histories;
- the pending shell/content A-supersedes-B hold-race witness;
- runtime absence of hidden observer/hook/external side effects outside the bounded candidate protocol;
- that any existing test passes at this snapshot.
