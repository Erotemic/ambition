# Architecture consolidation census

This page is the current authority and lifetime map. It says, for each live fact,
who owns it, how long it lives, and whether a second road can still write it.
The stable machine entries are in [`consolidation-ledger.json`](consolidation-ledger.json).
The campaigns that act on this map are in [`consolidation-plan.md`](consolidation-plan.md).

Method: static source, manifest, planning and generated-inventory inspection. No
compiler or runtime is used. The last column of each table is the row's evidence
class; this page owns it, and the ledger's copy must match
(`scripts/check_consolidation_ledger_states_are_live.py`).

## Current authority map

These single-owner shapes hold now. Code against them.

1. **Rollback.** `ActiveRollbackAuthority` owns the rollback owner, timeline
   generation, content/schema contract and health as one resource. Confirmation
   is derived from it, not stored.
2. **Session world.** `SessionRoot` carries the live session world as components
   (`PlatformerSessionWorld`, `PreparedContent`, `PreparedContentIdentity`,
   `ActiveContentBinding`). There is exactly one canonical live root. A prepared
   candidate carries `CandidateSessionRoot` and stays hidden until adoption.
3. **Live rooms.** Each live room is its own `RoomInstanceRoot` entity. Its
   `RoomGeometry`, `MovingPlatformSet` and `LiveRoomDefinition` are components on
   that root. `RoomSet` on the session root keeps the room definitions and the
   activation room.
4. **Replacement.** A room or session replacement is one candidate publication.
   `replace_live_world` stages the whole world, builds it hidden, verifies it and
   publishes or drops it; `settle_publication` reads the verdict once. A refusal
   leaves world N intact.
5. **Construction input.** Live construction reads only the activated generation
   (`GenerationMechanics::of` / `for_live_session`). There is no App-registry
   fallback.
6. **Content.** Active selection and a pending candidate are separate. A pending
   generation does not become the active selection because preparation started.
7. **Editor edits.** Every mechanical editor domain uses Propose -> Admit ->
   Publish (`MechanicalEditSet`), and the rollback owner can refuse or rebase.
8. **Identity.** Local lifetime and correlation ids stay local. Peer comparison
   uses projections (`TransactionId::peer_stable_checksum`, `PeerContentIdentity`)
   and stable `SimId`.

Open pressure:

- **34** process/App resources are explicitly documented by source as session- or
  generation-owned and are still App resources (section 3; campaign C03).
- Some optional reads of required authorities still mean both "capability not
  installed" and "authority went missing" (section 8; campaign C07).
- The facade crate and the large crates still need ownership review (sections 11
  and 12; campaigns C08 and C09).

No count on this page is a limit. A count is a measurement at a date.

## 1. Mechanical authority ledger

The table uses authority *families*. One row can cover an owner and its direct
projections when those projections answer one fact.

| ID | Authority | Owner | Representation | Semantic lifetime | Authoritative for | Rollback relationship | Generation relationship | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AUTH-SESSION-SCOPE | Active gameplay session scope | ambition_platformer2d_shared_tangle::lifecycle | Resource + Components | App with gameplay-session sub-lifetime | which gameplay session scope is current for session-aware spawning | host-side local lifetime authority; not the peer-stable rollback identity | unrelated to content identity; one scope can own one activated generation at a time | one owner plus entity projections | SOURCE_CONFIRMED |
| AUTH-SESSION-WORLD | Canonical live platformer session world root | ambition_platformer2d_runtime + ambition_platformer2d_shared_tangle | Bundle on canonical entity | gameplay session | the live session world catalogs, room set, geometry, active-room metadata, initial-body policy, and session requests | canonical live world owner; rollback registration is per root component, not one bundle snapshot | contains both frozen generation projections and mutable session/timeline state | owner-scoped canonical state | SOURCE_CONFIRMED |
| AUTH-SHELL-SESSION | Shell gameplay session instance | ambition_game_shell | Resource with optional GameplaySessionInstance | shell activation / gameplay session | which shell activation owns the live gameplay session and world entity | host-only lifecycle authority above rollback simulation | correlates one shell activation with the activated prepared session; it is not content identity | shell lifecycle authority | SOURCE_CONFIRMED |
| AUTH-SESSION-MECHANICS | Frozen mechanics for the activated content generation | ambition_platformer2d_actor_monolith::session::mechanics | Resource | content generation within a gameplay session | which frozen registries and developer mechanical inputs live-session construction must consume | frozen host-side construction input; resulting runtime state can be rollback state | frozen generation input | generation authority; live construction reads nothing else | SOURCE_CONFIRMED |
| AUTH-ROLLBACK | Active rollback authority | ambition_platformer2d_runtime::rollback::authority | Resource | gameplay session with rollback-timeline generations | who owns rollback, which timeline is current, what world it rewinds, and whether it may authorize confirmed effects | canonical host authority for the live rollback timeline; it governs snapshots rather than being ordinary snapshotted gameplay state | contract contains prepared content identity and snapshot schema for the governed timeline | already consolidated authority | SOURCE_CONFIRMED |
| AUTH-CONTENT-BINDING | Live session content binding | ambition_platformer2d_actor_monolith::world::rooms::transaction | Component on the session root | content generation within gameplay session | which content generation a room transaction may publish into | commit-boundary host authority; no source evidence that it is rollback-snapshotted | frozen generation identity projection: the content binding of the live session generation | projection of the one session publication; inserted on the session root in the same batch that builds the first room plan | SOURCE_CONFIRMED |
| AUTH-CONTENT-IDENTITY | Prepared content identity | ambition_platformer2d_runtime::content_identity | Component value derived from PreparedContent | prepared generation / activation | the exact prepared-content activation and snapshot schema used by a session | identity input to rollback contract; the source registers related schema/content contracts, but this census does not claim whole-component snapshot behavior beyond explicit registration | frozen generation input; mixes peer-stable fingerprints with App-local epoch | mixed canonical content digest plus local activation lineage | SOURCE_CONFIRMED |
| AUTH-SELECTED-CONTENT | Selected authored content identity | ambition_platformer2d_runtime::content_identity + ambition_content | Resource + pending transaction resource | App active selection with transaction-local candidate inputs | which authored content the App selected, versus which candidate one load transaction is preparing | host-side content selection; not gameplay rollback state | names the active authored pack selection from which a generation can be prepared | legitimate active/candidate separation | SOURCE_CONFIRMED |
| AUTH-ROOM-SET | Room definitions and the activation room | ambition_platformer2d_world::rooms on SessionRoot | Component on SessionRoot | gameplay session / room selection | which room set the session uses and which room it activates into; which room a live room instantiates is `LiveRoomDefinition` on that live room's own root | canonical rollback-registered component on SessionRoot | live projection of the admitted generation plus current room selection | canonical root state; its indices are private, and the setters and `RoomSet::try_from_parts` refuse an unknown room (`RoomSetRefused`) | SOURCE_CONFIRMED |
| AUTH-ROOM-GEOMETRY | Live room geometry | ambition_platformer2d_core on each live room root | Component on RoomInstanceRoot | live room | the room collision/world geometry used by simulation | rollback-registered component on the live room root | runtime projection of the current admitted room/world geometry | — | SOURCE_CONFIRMED |
| AUTH-MOVING-PLATFORMS | Live moving-platform state | ambition_platformer2d_world + actor_monolith session lifecycle | Component on each live room root | live room within gameplay session | current moving-platform simulation state | rollback-registered component on the live room root | mutable timeline state initialized by room construction | — | SOURCE_CONFIRMED |
| AUTH-OCCURRENCE-CUSTODY | Occurrence and custody continuity state | ambition_platformer2d_shared_tangle + ambition_persistence + actor_monolith | Resources and typed baselines | gameplay session with durable continuity inputs | which authored occurrences exist, where custody resides, and which facts a rebuild must retain | mixed domain continuity state; some values are rollback projections and some are durable/session baselines; use owner-specific registrations | session continuity state; not a content-generation identity | distinct durable domain authorities; not one generic reset flag | SOURCE_CONFIRMED |
| AUTH-MOVEMENT-TUNING | Active movement tuning | ambition_platformer2d_core::movement::tuning | Resource | mutable gameplay-session mechanical state | movement tuning simulation consumes now | mechanical input consumed by rollback simulation; mechanical-edit admission prevents unsynchronized live mutation | mutable timeline mechanical state, not a frozen content-generation value | — | SOURCE_CONFIRMED |
| AUTH-FEEL-TUNING | Active platformer feel tuning | ambition_combat::feel | Resource | mutable gameplay-session mechanical state | combat and platformer feel tuning simulation consumes now | mechanical input consumed by rollback simulation; live edits pass through mechanical edit admission | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-PORTAL-TUNING | Active portal tuning | ambition_portal2d::tuning | Resource | mutable gameplay-session mechanical state | portal mechanics tuning simulation consumes now | mechanical input consumed by rollback simulation; live edits pass through mechanical edit admission | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-ABILITY-MASK | Admitted developer ability mask | ambition_dev_tools | Resource plus body-component projection | mutable gameplay-session developer mechanical state | the admitted developer ability mask | host-side admitted mechanical input; projected body ability state participates in simulation | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-BODY-PROFILE | Admitted developer body profile | ambition_dev_tools | Resource plus body-component projection | mutable gameplay-session developer mechanical state | the admitted developer body profile | host-side admitted mechanical input; projected body state participates in simulation | mutable timeline mechanical state | — | SOURCE_CONFIRMED |
| AUTH-PLAYER-STATS | Live player stat components | ambition_dev_tools adapter + body component owners | Components with editable mirror and sync snapshot | entity / mutable gameplay session | the live player health and mana values | live body components are gameplay state; editor publication changes them only after admission | mutable entity/timeline state | one production editor writer, `publish_player_stats_edits` in `MechanicalEditSet::Publish`; `PlayerStatsSyncSnapshot` separates a developer edit from a gameplay change | SOURCE_CONFIRMED |
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

- `AUTH-ROLLBACK` is a positive model: it combines facts that would be invalid if
  they drifted.
- `AUTH-SESSION-WORLD` is a positive model: live world facts are components on the
  canonical `SessionRoot`, read with normal Bevy queries.
- `AUTH-CONTENT-BINDING` is a projection of the one admitted session publication,
  not a separately queued write.
- `AUTH-SESSION-MECHANICS` is the one construction source for live rebuilds. Its
  remaining mismatch is storage kind only (see C05 in the plan).
- Editor authorities are mutable timeline state by design. Do not fold them into
  immutable content-generation identity to reduce object count.

## 2. Duplicate-truth families

The split is derived from the ledger, not counted by hand:
**0 open, 20 resolved, 4 legitimate separation.**

A resolved row is a receipt: the one owner now, and any standing rule. A
`LEGITIMATE_SEPARATION` row says two similar-looking owners hold different facts.
`scripts/check_collapsed_authorities_stay_collapsed.py` keeps deleted second
owners deleted, and `scripts/check_separated_authorities_stay_separated.py` keeps
the separations separate.

| ID | Family | State | Current state | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- |
| DUP-SESSION-CURRENT | Current gameplay-session identity family | RESOLVED — one collapse landed | `ActiveGameplaySession`, `ActiveSessionScope`, `SessionRoot` and `GameplaySessionWorldRoot` each answer a different layer's question. The one copy that was not a layer split, a one-entry resource that duplicated the live correlation, is deleted. | Keep the layer split. `ambition_platformer2d_shared_tangle` may not reach `ambition_game_shell` (`platformer-primitives-stays-a-foundation` in `scripts/check_absence_contracts.py`). | SOURCE_CONFIRMED |
| DUP-CONTENT-CURRENT | Live content-generation identity family | RESOLVED — published through the room verdict | `PreparedContent`, `PreparedContentIdentity` and `ActiveContentBinding` are Components on the session root from one lowering. The hot reload advances the content generation only behind `publication_succeeded`. | None on this axis. The peer-facing half of the identity is ID-PEER's. | SOURCE_CONFIRMED |
| DUP-GENERATION-MECHANICS | Activated generation mechanics versus App registries | RESOLVED — the fallback is deleted (2026-09-20) | `GenerationMechanics` holds one set of registries. Callers choose `of` or `for_live_session`, which refuses when no generation is active. No live rebuild, the LDtk world reload included, reads App registries. | A direct composition that rebuilds rooms installs its own scoped `SessionMechanics` (`install_direct_session_root`). `perception_extent_for` keeps an App fallback on purpose: it is a read at decision time, not construction. | SOURCE_CONFIRMED |
| DUP-LAUNCH-LAW | How far a hit launches its victim | RESOLVED — the second owner is deleted (2026-09-20) | One owner, `ambition_entity_catalog::launch::launch_speed`, in a crate that combat and the fighter brain both see. It takes the move's growth as an `Option`, and `LaunchConditions` carries the ruleset growth, so no caller decides "the ruleset decides" early. `ambition_combat` resolves the inputs (`ResolvedCombatTuning`) and does not own the arithmetic. | Rule: a factor that multiplies only part of each candidate's expression can reorder candidates even when it is common to all of them. Per-move staling is such a factor; queue.md's BRAIN row tracks it. | SOURCE_CONFIRMED |
| DUP-RESET-FACING | Which way a body looks after a reset | RESOLVED — the literal became a parameter (2026-09-21) | The caller answers through `ResetFacing::{Keep, Toward}`. The Smash respawn faces stage centre, the versus round boundary states its facing, and a room arrival or hazard respawn keeps its heading. | Rule: a parameter removes a duplicate only where each call site supplies a real answer. Audit the call sites, not the signature. | SOURCE_CONFIRMED |
| DUP-CHARACTER-KIT | A body's pre-equipment repertoire baseline | RESOLVED — the second owner is deleted (2026-09-21) | `IdentityKit` is the one baseline. `PreparedKit::baseline` is the one kit that the spawn grant and the re-wear both consume, with the character's own `ranged_execution`; `install_ranged_execution` installs charging from the character on every road that owns its kit. | Ruling (2026-09-23): an authored repertoire says what a character is. A progression mask is an explicit prepared ruleset policy, not an engine law; a body with no mask wears its authored kit ungated. | SOURCE_CONFIRMED |
| DUP-BODY-ABILITIES | What a body may do — the character's answer versus the constructor's | RESOLVED — the constructor stopped answering (2026-09-22) | `ActorBody::from_abilities` takes a resolved ability set and grants nothing. An unauthored character's abilities are its content provider's declaration, resolved during preparation. | Rule: a union cannot express a refusal. An authored `false` must reach the body. | SOURCE_CONFIRMED |
| DUP-DISMOUNT-REPERTOIRE | What a fallen rider swings | RESOLVED — the rediscovery and the invention are both deleted (2026-09-22) | A dismount changes only the brain. The rider keeps its own `IdentityKit`. A rider that authored no melee comes down without one. | None. | SOURCE_CONFIRMED |
| DUP-RESET-METER | What a reset does to a body's resource meter | RESOLVED — every banked resource resets to its declared start (2026-09-23) | A body's meters are slots of its `ActorResources` bank, declared by the ruleset or provider (`MatchRules::resources`, `HomeBodyResources`). Every reset road returns each slot to its declared start. A Smash seat is built with its Limit empty. No body holds Mana merely by existing. | Rule: a census of a defaulted fact lists its writers, not only its constructors. Owner: [composable actor resources](../engine/composable-actor-resources.md). | SOURCE_CONFIRMED |
| DUP-CROUCH-GEOMETRY | Which box a sheet-authored body stands in when the stance decision reads it | RESOLVED — the silhouette is published before the decision (2026-09-22) | The prepared-body grant publishes the standing box at construction. The pose pass runs in `PlayerInputSet::PosedGeometry`, before the stance decision, and projects the current pose only. `BodyMode::shape` is the one crouch rule. | A sheet's crouch rectangle measures the drawing; it is not a gameplay box, and no production code reads it. Do not make the sheet authoritative. | SOURCE_CONFIRMED |
| DUP-EFFECTIVE-REPERTOIRE | What a body can do right now | RESOLVED — one fold, and the reconstruction roads stopped answering (2026-09-22) | `ambition_characters::repertoire::effective_repertoire(identity, worn, hand)` is the one fold and returns the `ActionSet` + `ActorMoveset` pair. Every hand transition refolds for the hand it produces (`ambition_combat::hand`). Provocation, dismount and catalog restore change only the brain. Every repertoire-bearing body is built with an `ActorMoveset`. | Open: a hand that brings no melee (portal gun, javelin) leaves the wearer's `attack` verbs in the contract, because the action scheme offers the Attack slot from them. The fix is a hand-aware action scheme, not a narrower fold. | SOURCE_CONFIRMED |
| DUP-UNDESCRIBED-REPERTOIRE | What a body swings when nobody authored a repertoire for it | RESOLVED — the invention is deleted; the absence is the answer (2026-09-22) | A placement that names no character gets `ActionSet::peaceful()`. A provider declares its provoked default; a character with no resolved provoked policy is not provokable. | `default_player_action_set` survives only as a test fixture kit. | SOURCE_CONFIRMED |
| DUP-TRAVELED-PATH | How far a body travelled this tick | RESOLVED — one atomic path, read whole at a named settlement point (2026-09-22) | `SweepSample` is the one path. Every authority that moves a body after the kernel states its effect on it. Readers take it whole (`SweepSample::ending_at`) after `BodyPathSet::{Carry, Contacts, Crossing}`. | The writer-side `Option` is permanent: the fighter brain runs the movement kernel over a scratch body that is not an entity. | SOURCE_CONFIRMED |
| DUP-BOSS-DISPOSITION | Whether a boss is hostile right now | RESOLVED — construction sets the start, the runtime owns the rest (2026-09-22) | `boss_component_snapshot` is construction only (identity and the initial `Hostile`). Targeting and release own the disposition after that. The per-tick rewrite is deleted. | None. | SOURCE_CONFIRMED |
| DUP-SWITCH-STATE | Whether a switch is on | RESOLVED — the save is the one answer (2026-09-23); nothing projects it (2026-09-24) | The switch drain is the one press-time writer. Readers take the save by the activation id; the falling-sand room reads `FallingSandSpoutState::from_save`. | None. | SOURCE_CONFIRMED |
| DUP-PERSISTED-FATE | Whether an authored body the save says is dead / provoked / cleared starts that way | RESOLVED — deaths, cleared bosses and provocations are BUILT; the save mirror is deleted (2026-09-23) | The fate is a commit fact, not a plan fact. `ConstructionDomain::CommitFacts` carries `construction::PersistedFates`, read when the commit is requested, and the spawn roads take a `RecordedFate`. `NpcProvocationChanged` is announced by the two transitions that own provocation, and `record_npc_provocations` is the one writer of the durable flag. | None. | SOURCE_CONFIRMED |
| DUP-RELEASE-MIND | What a released NPC's body is | RESOLVED — release restores the mind and writes no body fact (2026-09-23) | A release restores the brain profile and brain only. `brain_builders::provoked_mind` is the one answer for what provocation produces, on the live flip and on construction from a save. A rollback load restores the brain and its binding together, so no post-load reconciler exists. | Open: when a mount-controlled ride ends, nothing resumes the recorded brain source (`Q76`). | SOURCE_CONFIRMED |
| DUP-EDITOR-STAGES | Mechanical editor desired/admitted/projection stages | LEGITIMATE_SEPARATION | Editable mirrors, pending proposals, admission, admitted authority, and runtime projection are intentionally different stages. The body-profile and ability repairs show that collapsing admission with projection loses state when a target entity is absent. | Consolidate protocol shape and registration, not the distinct values. New editable mechanical domains must use the same stages or explicitly document why a stage is not applicable. | SOURCE_CONFIRMED |
| DUP-CONTENT-CANDIDATE | Active content selection versus pending generation inputs | LEGITIMATE_SEPARATION | SelectedContentIdentity is active App selection. PendingGeneration and PendingGenerationInputs own candidate transaction values until activation. They must not overwrite the active selection during preparation. | Keep the active/candidate split. `PendingGenerationInputs::characters_for` returns a nested `Option` so a stranger's claim cannot fall through to the App registry; do not flatten it. | SOURCE_CONFIRMED |
| DUP-ROLLBACK-CONFIRMATION | Rollback authority versus confirmation answer | LEGITIMATE_SEPARATION | RollbackConfirmationState is deliberately not a Resource. It is derived from ActiveRollbackAuthority for a requested session scope. | Preserve this pattern. Do not promote derived answers into independently mutable resources. | SOURCE_CONFIRMED |
| DUP-CONSTRUCTION-DIAGNOSTICS | Construction authority versus last-result diagnostics | LEGITIMATE_SEPARATION | LastRoomConstructionCommit and LastConstructionVerification are documented as developer/test evidence. RoomSet and spawned authoritative entities remain live authority. | Keep diagnostics read-only. Do not let future systems use the last-result resources as simulation authority. | SOURCE_CONFIRMED |
| DUP-ROOM-PUBLICATION | Room replacement publishes through one candidate decision | RESOLVED — by A10's single publication decision | One room replacement is staged whole in `PendingWorldReplacement`, built as HIDDEN candidates, projected and validated, and published only on admission — root components, `MovingPlatformSet`, room entities and generation state are projections of that one decision. A refusal drops the candidates and the staged world and leaves world N untouched. | None. The single-publication shape this row asked for is what `replace_live_world` does. | SOURCE_CONFIRMED |
| DUP-MOVE-REACH | How far a move reaches, and what region it covers | RESOLVED — the second owner is deleted (2026-09-20) | `MoveSpec::frame_data()` folds captures as well as hit volumes, and `capture_candidate` is a legality gate that reads it. `MoveFrameData::hazard` is `Option<MoveHazard>`: `Spawned` for what the catalog measures, `OwnersRangedAction` for a ranged action whose numbers live on the body. `coverage_box()` is the one spelling of reach. | Rule: before you teach the fold a new road, ask whether the number answers "what can this do to them" or "where can this take me". Travel is not threat. | SOURCE_CONFIRMED |
| DUP-PRESENTATION-BINDING | Which art a body is finally bound to, and from which geometry | RESOLVED — on every shipped road (2026-09-23) | Worn-player binders finalize only from a `BodyPoseView` whose pose geometry is settled. An actor whose content declares its art waits for that sheet. | Two windows that no shipped road reaches are recorded, not fixed. The first road that spawns a boss outside room preparation, or re-wears a non-player actor, closes the matching one: demand the dedicated sheet before admission, or key the actor binding on the worn identity. | SOURCE_CONFIRMED |

⛔ Do not merge the staged editor values, active-versus-pending content, the
rollback confirmation answer or the construction diagnostics into their
underlying authorities. They answer different questions.

⚠ `0 open` is a statement about named families. Two ways have found a
duplicate since the list closed: a doc comment that argues a duplicate is
harmless, and a behavioural probe whose two sides must give a symmetric answer.
Use both when you review a new area.

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

`SessionRoot` already stores the session world. The remaining mismatch is
explicit in source: some process resources are reset at session activation
because their semantic lifetime is one session or one activated generation.

| ID | Family | Semantic owner | Storage/representation | Current state | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| LIFE-SESSION-RESOURCE-AGGREGATE | SessionScopedResources process-storage aggregate | gameplay session | **27** process/App Resources accessed through one SystemParam | SessionScopedResources names 27 App resources that source states belong to one gameplay session. Activation resets them for correctness and retirement resets them for hygiene. | actual storage owner is broader than semantic owner | SOURCE_CONFIRMED |
| LIFE-CHECKPOINT-RESOURCE-AGGREGATE | SessionOwnedCheckpointState process-storage aggregate | gameplay session | 6 process/App Resources accessed through one SystemParam | SessionOwnedCheckpointState names six App resources for one gameplay-session checkpoint coordinator and resets all six at session activation. | actual storage owner is broader than semantic owner | SOURCE_CONFIRMED |
| LIFE-SESSION-MECHANICS | Generation-owned mechanics stored as App resource | content generation within gameplay session | Resource | SessionMechanics is an App Resource whose semantic owner is the activated generation. Retirement removes it; activation overwrites it. | actual storage owner is broader than semantic owner | SOURCE_CONFIRMED |
| LIFE-ROOT-OWNED-WORLD | Session-root-owned world components | gameplay session / room | Components on SessionRoot and on each live room root | `RoomSet`, initial-body policy and session requests are on the canonical `SessionRoot`; `RoomGeometry` and `MovingPlatformSet` are on each live room's own root. None is a process-global resource. | owner-scoped state | SOURCE_CONFIRMED |

### Explicit narrower-lifetime App resources

`SessionScopedResources` names **27** process resources whose source says one
gameplay session owns them:

`PossessionState, ControlledSubject, EncounterView, BossEncounterRegistry, QuestRegistry, RoomTransitionCooldown, SlotInteractionState, SwitchActivationQueue, SaveRestored, AuthoredOccurrences, OccurrenceBaseline, CustodyBaseline, MintedItemBaseline, LastQuestRoom, LastCutsceneRoom, ProjectileSeqCounter, PendingLifecycleCommit, BaseGravity, ActiveCutscene, CutsceneTriggerQueue, ActiveConversation, CutsceneSkipHold, StocksMatchSettled, SuddenDeathEntered, LiveMatchTicks, SessionMatchOrdinal, GameplayElapsed`.

`SessionOwnedCheckpointState` adds **6** checkpoint-coordinator resources:

`SessionCheckpointOperations, SessionCheckpointOutcomes, AcceptedCheckpointRestore, AbandonedCheckpointOperation, SessionStartupResume, OutstandingCheckpointRequest`.

`SessionMechanics` is one more App resource whose semantic owner is the activated
content generation. The unique total is **34** — the three lists are disjoint, so
it is their sum. `scripts/check_session_owner_census_matches_source.py` checks
both name lists and every restated count against source.

⚠ That guard counts only required `ResMut` fields. `SessionScopedResources` also
holds two optional members, `BossDefeatsSinceCheckpoint` and
`BreakableRespawnSchedule` (`Option<ResMut<..>>`, present only when their
capability is installed). `scripts/architecture_census.py` counts them and
reports 36.

The compensation mechanisms are activation reset, retirement cleanup,
current-scope checks and generation presence checks. They are evidence that the
storage owner and the semantic owner differ, not defects by themselves. Classify
each family before you move it:

- state needed before a `SessionRoot` exists can stay in a process coordinator if
  it has an explicit session owner key;
- state that is meaningful only inside a live session is a candidate for
  root-owned storage;
- presentation-only latches can stay separate from canonical mechanical state;
- rollback-registered values need a migration that keeps their registration and
  restore semantics.

## 4. Construction and reconstruction roads

| ID | Entry point | Owner | Mechanical inputs | Generation/session inputs | Construction API | Publication point | Failure semantics | Retirement | Shared primitive | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ROAD-INITIAL-SESSION | `prepare_candidate_platformer_session` -> `translate_shell_session_lifecycle` -> `adopt_candidate_platformer_session` | game shell + platformer provider | prepared content, `SessionMechanics`, prepared room/session world | `ShellActivationId`, `SessionScopeId`, load/prepared transaction data | `PlatformerSessionWorld` materialization plus `RoomConstructionPlan::spawn_contents` | canonical `SessionRoot` plus activated provider/session state, promoted out of the hidden candidate population at adoption | the candidate session is built and verified while the route is held; a first room that fails refuses the route through `ShellActivationGates`, so the playing session is never retired for an unverified one | shell session retirement and session-scope cleanup | normal session root and room construction primitives | SOURCE_CONFIRMED |
| ROAD-ROOM-TRANSITION | `begin_room_transition_load_system` -> authorization -> `commit_ready_room_transition_system` | platformer runtime room transition | prepared `RoomConstructionPlan`, current world continuity, target room data | current session scope, lifecycle intent, content binding | `RoomConstructionPlan::replace_live_world` | one candidate publication, whose projections include the resource/root writes and `RoomLoaded` | a refusal at ANY point leaves world N intact and unchanged; there is no window in which neither world is authoritative | the outgoing room stands WHILE the candidate is built — it is what the candidate is validated against — and is swept only after the candidate publishes | same prepared room plan used by reset/reload paths | SOURCE_CONFIRMED |
| ROAD-SAME-ROOM-REPLAY | admitted `PendingLifecycleCommit` replay path | session lifecycle/reset + room construction | current frozen room plan inputs plus replay retention policy | current session, lifecycle intent, attempt/session retention | room replay through the same room construction model | admitted replay boundary, then normal room construction publication | admission can refuse; current room materialization still has the normal room transaction limits | attempt/room state according to replay policy | room plan and typed construction | SOURCE_CONFIRMED |
| ROAD-CHECKPOINT-RESTORE | `restore_checkpoint_on_session_start` / `resume_at_checkpoint_on_reset` and accepted checkpoint operation | session checkpoint coordinator + room lifecycle | accepted pinned occurrence, custody, item, intent, and room inputs | `CheckpointOperationKey`, session scope, accepted lifecycle intent | same-room replay or cross-room transition | exact operation commit and terminal restore outcome | request/admission/preparation can cancel or fail without treating raw request as authority | accepted operation retires after lifecycle intent settles | normal replay/transition constructors; checkpoint policy stays distinct | SOURCE_CONFIRMED |
| ROAD-NEW-GAME | `resume_at_checkpoint_on_reset` on `NewGameRequested` (a host intent) | session checkpoint coordinator | the fresh baseline pinned at admission: empty occurrences, custody and mints, the starter bag, the start room's spawn | current gameplay session | the checkpoint restore's room transition, committed on the confirmed frame | `CheckpointRestoreOutcome` plus rebuilt world | since 2026-09-29 a New Game IS a checkpoint restore (NEW-GAME-RESYNC); it differs from a death only in destination, pinned inputs and the `fresh` flag | old progress cleared by each domain's fresh-run reducer in `CheckpointDomainApply` | normal room construction and checkpoint verification | SOURCE_CONFIRMED |
| ROAD-DEV-RELOAD | content reload candidate -> app `dev_runtime` reconstruction | content reload + app development runtime | new `PreparedContent`, room plan, the live generation's frozen mechanics | reload request/load ids, content identity/epoch, current session | `RoomConstructionPlan::replace_live_world` | `rooms::settle_publication` on the room verdict; the content binding moves only behind `publication_succeeded` | content candidate can refuse before activation; the scene replacement cannot destroy N before the N+1 verdict | the old room is retired only after the candidate publishes | the same materializer and verdict skeleton as room transition | SOURCE_CONFIRMED |
| ROAD-RUNTIME-ACTOR | typed construction plan executor / authoritative respawn helpers | shared construction protocol + actor construction owner | typed recipes, stable `SimId`, relationships, prepared services | `TransactionId`, `SessionSpawnScope`, construction provenance | typed `ConstructionPlan` commit paths and executor-owned root minting | normal live commit or inactive candidate publication when that path is selected | plan/roster validation can refuse; candidate entity path can retire hidden roots | transaction-owned candidate roots or normal entity lifecycle | generic typed construction primitive | SOURCE_CONFIRMED |
| ROAD-DYNAMIC-SPAWN | domain spawn requests: `ProjectileSpawnRequest` and `SpawnActorRequest` | owning gameplay domain | domain request plus parent/source mechanical state | stable parent `SimId`, rollback-state sequence/counter where required | domain materialization/spawn request seam | entity spawn into live simulation | domain-specific; no global transaction | normal entity/domain lifecycle | stable simulation identity and session spawn ownership; every spawn-request seam is a `Message` raised and read inside the simulation | SOURCE_CONFIRMED |
| ROAD-A10-CANDIDATE | `ConstructionPlan::commit_inactive` | shared typed construction candidate protocol | verified typed construction plan and registered inactive-candidate filter | construction `TransactionId` and session scope | `commit_inactive`, `publish_candidate`, `retire_candidate` | remove `InactiveCandidate` from all roots in the transaction | retire candidate roots while live world stays visible for the entity slice this primitive owns | `retire_candidate` on refusal; the old live world is swept by the same publication that admits the candidate | candidate entity primitive, integrated with room/session resource ownership through `PendingWorldReplacement` | SOURCE_CONFIRMED |

### Construction model

There are many entry roads and four models:

1. **Prepared session activation.** A shell/provider transaction builds a hidden
   candidate session beside the live one, verifies it, and adopts it.
2. **Live room replacement.** Room transition, replay, death and checkpoint
   restore, New Game and the LDtk dev reload all use `replace_live_world` and
   `settle_publication`. Replay, checkpoint restore and New Game ride the
   room-transition road.
3. **Typed entity construction.** Domain plans mint stable authoritative roots
   and relationships. `commit_inactive` + `publish_candidate` / `retire_candidate`
   is the candidate entity primitive under models 1 and 2.
4. **Dynamic entity spawn.** Projectiles and summons use entity-level identity
   and lifecycle. They are not world-replacement transactions.

## 5. Publication and admission roads

| ID | Who decides | Who writes | Validation separate from publication? | Can failure follow partial publication? | Other publication road for same truth? | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PUB-SHELL-SESSION | shell route/router and registered activation gates | `translate_shell_session_lifecycle` plus provider activation | yes: route readiness/gates precede activation | shell gate refusal cancels pending route; provider/session runtime behavior after activation needs runtime proof | shell session activation is distinct from content and room publication | session publication owner | SOURCE_CONFIRMED |
| PUB-CONTENT-GENERATION | `publication_boundary` through the shell gate | `commit_content_generation` and content/provider activation code | yes: pending candidate and gate verdict are separate from active selection | a candidate can stay pending or be refused before active family publication; a candidate superseded while pending is discarded and cannot publish (`a_candidate_session_replaced_while_pending_is_discarded`, `a_superseded_transaction_cannot_publish_in_the_shipped_app`) | content activation and scene replacement are separate slices of one session adoption | candidate/admission publication road | SOURCE_CONFIRMED |
| PUB-ROOM-REPLACEMENT | room lifecycle authorization and prepared plan | `replace_live_world` + `PendingWorldReplacement` + transaction close | yes: the whole replacement is staged and applied only on admission | no: a refusal drops the candidate roots and the staged world; N is untouched | one bounded publication authority writes the root components, the live-room state and the entities together | explicit last-good-world boundary | SOURCE_CONFIRMED |
| PUB-CONSTRUCTION-CANDIDATE | typed construction transaction | `publish_candidate` removes `InactiveCandidate` | yes: inactive commit then validation then publish | candidate entity roots can be retired on failure; arbitrary resources/effects are outside this primitive | one bounded entity-publication road, widened to room and session state by `PendingWorldReplacement` and `FrozenPublicationEffects` | bounded candidate entity publication | SOURCE_CONFIRMED |
| PUB-MECHANICAL-EDIT | rollback owner or default no-rollback policy | each domain Publish system | yes: Propose -> Admit -> Publish | refused/foreign/unhealthy timeline can keep desired value without moving admitted authority | same protocol is shared by every production editor domain | explicit admission road | SOURCE_CONFIRMED |
| PUB-CHECKPOINT | session checkpoint coordinator and lifecycle slot | accepted-operation commit and outcome systems | yes: request -> admission -> commit -> terminal outcome | refusal/cancellation is explicit and keyed to one operation | shares room replay/transition mechanics but keeps checkpoint state policy | explicit admission road | SOURCE_CONFIRMED |
| PUB-NEW-GAME | new-game restore | the checkpoint commit (`apply_committed_checkpoint_restore`) and the fresh-run reducers it runs | yes: the room transaction verifies before `CheckpointDomainApply` runs, as for every checkpoint restore | the commit fails closed like any restore | shares the checkpoint restore; the fresh-run reducers are the only distinct policy | the checkpoint operation's terminal outcome | SOURCE_CONFIRMED |
| PUB-ROOM-LOADED | room transaction verifier | `verify_and_publish` writes `RoomLoaded` | yes: the message and the live world move together; nothing is written before the verdict | a withheld message means a withheld world | read in production by `FreshAttempt` consumers | publication notification and the authority switch | SOURCE_CONFIRMED |

## 6. Editor and mechanical mutation architecture

The shared mechanical-edit admission protocol is a completed foundation. Each
production domain has one proposer and one publisher.

| ID | Domain | Current stage map | Coverage | Admitted authority | Sources | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| EDIT-MOVEMENT | Movement tuning | `EditableMovementTuning` -> `PendingMechanicalEdits` -> `MechanicalEditAdmission` -> `ActiveMovementTuning` -> simulation reads | `propose_editable_movement_tuning` -> `publish_editable_movement_tuning` | ActiveMovementTuning | `crates/ambition_dev_tools/src/dev_tools/editable.rs`<br>`crates/ambition_platformer2d_core/src/movement/tuning.rs` | SOURCE_CONFIRMED |
| EDIT-ABILITIES | Developer ability mask | editable ability set -> pending domain -> admission -> `ActiveEditableAbilityMask` -> body abilities projection | `propose_editable_abilities` -> `admit_editable_abilities` | ActiveEditableAbilityMask then body projection | `crates/ambition_dev_tools/src/lib.rs`<br>`crates/ambition_dev_tools/src/dev_tools/editable.rs` | SOURCE_CONFIRMED |
| EDIT-BODY-PROFILE | Developer body profile | `DeveloperTools` desired profile -> pending domain -> admission -> `ActivePlayerBodyProfile` -> body/movement projection | `propose_developer_body_profile` -> `sync_developer_body_profile` | ActivePlayerBodyProfile then body projection | `crates/ambition_dev_tools/src/dev_tools/editable.rs` | SOURCE_CONFIRMED |
| EDIT-PLAYER-STATS | Player stats | editable stats -> pending domain -> admission -> live body stat components; reverse mirror sync is separate | `propose_player_stats_edits` -> `publish_player_stats_edits` | live body stat components | `crates/ambition_dev_tools/src/dev_tools/editable.rs` | SOURCE_CONFIRMED |
| EDIT-FEEL | Platformer feel tuning | `EditableFeelTuning` -> pending domain -> admission -> `Platformer2dFeelTuningMonolith` -> combat/sim reads | `propose_editable_feel_tuning` -> `publish_editable_feel_tuning` | Platformer2dFeelTuningMonolith | `crates/ambition_combat/src/feel.rs` | SOURCE_CONFIRMED |
| EDIT-PORTAL | Portal tuning | `EditablePortalTuning` -> pending domain -> admission -> `PortalTuning` -> portal systems | `propose_editable_portal_tuning` -> `publish_editable_portal_tuning` | PortalTuning | `crates/ambition_portal2d/src/tuning.rs` | SOURCE_CONFIRMED |

`MechanicalEditAdmission` is intentionally optional in a host that has no rollback
history to protect. That is a capability distinction, not a fail-open bug.

⚠ `scripts/architecture_census.py` now finds a seventh production domain,
`ExtensionModuleCode` in `crates/ambition_platformer2d_runtime/src/extension_composition.rs`
(behind the `wasm_modules` feature). It has no ledger row yet. Add one with its
stage map. Do not add a second admission protocol.

## 7. Local identity and canonical deterministic identity

| ID | Identity | Class | Lifetime | Current use | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ID-CONTENT-FINGERPRINT | ContentFingerprint | CANONICAL_MECHANICAL / intended peer-stable content identity | content definition | ContentFingerprint is a BLAKE3 digest over versioned canonical prepared-content sections. Source excludes timestamps, handles, entity ids, map iteration order, and mutable session state. | — | SOURCE_CONFIRMED |
| ID-CONTENT-EPOCH | ContentEpoch | LOCAL_LIFETIME | App activation lineage | ContentEpoch is an App-local gap-tolerant activation lineage token. It is equality-only and is not a content fingerprint. `ContentBinding` carries it beside a `PeerContentIdentity` digest, and `peer_binding_term` folds the digest, so the epoch does not reach a peer comparison. | — | SOURCE_CONFIRMED |
| ID-PREPARED-CONTENT | PreparedContentIdentity | MIXED_RESPONSIBILITY | prepared activation | PreparedContentIdentity packages canonical fingerprints with the local ContentEpoch. It is exact for one App activation, but the local epoch is not peer-stable by contract. | Do not use the local epoch half where peer-stable canonical identity is required. Keep exact local activation identity separate from peer comparison. | SOURCE_CONFIRMED |
| ID-SESSION-SCOPE | SessionScopeId | LOCAL_LIFETIME | gameplay session | SessionScopeId is minted by an App-local monotonic allocator for gameplay-session ownership. `TransactionId::peer_stable_checksum` drops the session field, so it does not reach a peer comparison. | — | SOURCE_CONFIRMED |
| ID-SHELL-ACTIVATION | ShellActivationId | LOCAL_LIFETIME | shell activation | ShellActivationId names one local shell activation. The session root's canonical `SimId` does not name it: there is one mint, `SimId::singleton("session", "root")`. Its live job is host-local correlation; the candidate slot is keyed on it. | — | SOURCE_CONFIRMED |
| ID-SHELL-REQUEST | ShellRequestId | LOCAL_CORRELATION | shell request transaction | ShellRequestId is caller-minted transaction correlation for shell requests. Content reload uses it before the router later assigns LoadId. | — | SOURCE_CONFIRMED |
| ID-LOAD | LoadId | LOCAL_CORRELATION | load transaction | LoadId is load-transaction correlation with two production minters in separate namespaces: the shell router (`shell.<route>.<n>`) and the room transition (`room-transition:<seq>:<src>-><dst>`). `PendingGenerationInputs` keys candidate inputs by it, and `content_identity_for` derives the key from the transaction so a caller cannot misspell it. | — | SOURCE_CONFIRMED |
| ID-ROLLBACK-TIMELINE | RollbackTimelineGeneration | LOCAL_LIFETIME | rollback timeline | RollbackTimelineGeneration is a process-monotonic identity for one rollback timeline. It distinguishes restarted timelines whose frame numbers begin at zero. | — | SOURCE_CONFIRMED |
| ID-SIM | SimId | CANONICAL_MECHANICAL / PEER_STABLE | logical simulation object | SimId is stable semantic simulation identity used for snapshot, replay, netcode, and deterministic ordering. Dynamic descendants derive from stable parent SimId plus rollback state counter. | — | SOURCE_CONFIRMED |
| ID-TRANSACTION | TransactionId | LOCAL stamp with a PEER PROJECTION | construction transaction / rollback-visible provenance | TransactionId stamps authoritative construction roots as `{binding}\t{room}\t{session}`, so the whole stamp is host-local. It is registered with `TransactionId::peer_stable_checksum`, which drops the session field and reduces the binding to the content digest, so peers compare content and room only. The type still snapshots whole, because a rewind must restore local ownership. | The projection reads the rendered stamp, so the formatter and the reader can drift; a round-trip arm holds them together. | SOURCE_CONFIRMED |
| ID-ROOM-PLAN | RoomConstructionPlanId | CANONICAL_MECHANICAL within same-build plan semantics | prepared room plan | RoomConstructionPlanId is a stable same-build identity from the frozen room spec and deterministic construction plan. It excludes SessionSpawnScope, TransactionId, Entity, and process-local values. | — | SOURCE_CONFIRMED |
| ID-MECHANICAL-DOMAIN | MechanicalDomain | LOCAL_CORRELATION | host editor proposal batch | MechanicalDomain uses TypeId as a host-local editor-domain key. Source explicitly states that this is valid because PendingMechanicalEdits is host-side and outside rollback. | — | SOURCE_CONFIRMED |

The local types are correct local identities and stay load-bearing in that role.
The remaining mixed-responsibility type is `PreparedContentIdentity`: it packages
canonical fingerprints with the local epoch. Do not use its local half where
peer-stable identity is required. The live owner of this road is
[ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).

## 8. Optional canonical authorities and capability composition

A raw `Option<Res<T>>` is not evidence of a defect.

| ID | Capability/authority | Classification | Current state | Possible consolidation | Evidence |
| --- | --- | --- | --- | --- | --- |
| CAP-SESSION-SCOPE-OPTIONAL | Optional ActiveSessionScope in mixed compositions | capability legitimately absent today, with compatibility semantics | SessionSpawnScope::for_optional_active_session interprets missing ActiveSessionScope as a direct/legacy process-resident composition and present-with-no-current as a shell frontend where gameplay spawning must sleep. | If session lifecycle becomes universal, remove the missing-resource meaning. Until then, do not make it required without migrating direct/headless compositions. | SOURCE_CONFIRMED |
| CAP-SESSION-MECHANICS-OPTIONAL | Optional SessionMechanics for live room construction | required for live room construction in every composition | `GenerationMechanics::for_live_session` refuses a live rebuild when no `SessionMechanics` is installed. A direct composition that rebuilds rooms installs its own scoped `SessionMechanics`. | — (closed by C04) | SOURCE_CONFIRMED |
| CAP-CONTENT-BINDING-OPTIONAL | Optional ActiveContentBinding at room verification | required canonical authority in shell production; explicit direct fixture absence | Room verification permits no ActiveContentBinding in direct fixtures, but refuses it when SessionGatedSimulation marks a shell-routed session. | Keep the discriminator. Whether direct entry must own a binding is a composition decision (C07). | SOURCE_CONFIRMED |
| CAP-MECHANICAL-ADMISSION-OPTIONAL | Optional MechanicalEditAdmission in non-rollback compositions | capability legitimately absent | Editor publishers treat absent MechanicalEditAdmission as Publish. Source states this is intentional because a composition with no rollback host has no history to protect; a host that can refuse installs the resource. | — | SOURCE_CONFIRMED |
| CAP-INITIAL-READINESS | Optional InitialGameplayReadiness | optional presentation/startup capability | Visible direct-entry hosts can install a closed startup readiness gate. Apps that omit it keep the normal behavior. | — | SOURCE_CONFIRMED |
| CAP-LDTK-INDEX | Optional LDtk session-world index | capability legitimately absent | PreparedPlatformerSource carries an installed LDtk index only when an authoring format installs one. RON-authored sessions legitimately carry none. | — | SOURCE_CONFIRMED |
| CAP-OPTIONAL-RES-CENSUS | Repository optional Res/ResMut surface | discovery index, classified per call site | `python3 scripts/architecture_census.py` reads 745 occurrences over 200 type spellings (2026-10-02), with test items and comments stripped. | Classify per call site, not per type. One type can be a hard composition fact at one site and a tooling guard at another (`SimTick`), and some absences are temporal ("not yet" versus "never"). Most sites state their reason at the parameter; read it first. | SOURCE_CONFIRMED |

The useful pattern is explicit composition semantics:

```text
capability installed
  -> owner resource/component is installed
  -> its systems are installed
  -> absence has one documented meaning
```

The risky pattern is a required production authority that is simply absent, so
the consumer continues with current App state.

## 9. Correctness-sensitive plugin and schedule coupling

The raw source has many `.before()` and `.after()` edges. These rows are the
relationships where ordering carries an architectural invariant.

| ID | Producer | Consumer | Schedule/set or mechanism | Invariant | Classification | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| ORDER-SESSION-LIFECYCLE | session lifecycle messages/scope owner | cleanup and activation consumers | `SessionScopeSet`: RetireAuthority -> Cleanup -> Activate -> Presentation, in `Update`, declared once by `SessionScopePlugin` | old session state cannot become input to the next activation; `Cleanup -> Activate` is the load-bearing edge and `RetireAuthority` is hygiene. Held by `the_retired_scopes_sweep_precedes_the_incoming_sessions_construction` | NORMAL_PIPELINE_ORDER | SOURCE_CONFIRMED |
| ORDER-MECHANICAL-EDIT | editor proposal systems | rollback admission and domain publishers | `MechanicalEditSet`: Propose -> Admit -> Publish, in `PreUpdate`, declared once by `configure_mechanical_edit_sets`; the rollback host adds `Publish.before(RunGgrsSystems)` | simulation never observes an editor change before rollback policy admits it. Held by `the_mechanical_edit_chain_completes_before_the_timeline_advances` | NORMAL_PIPELINE_ORDER | SOURCE_CONFIRMED |
| ORDER-ROOM-TRANSACTION | transaction open/baseline capture | construction + verify/publish tail | not an ordering: `PendingConstructionBaseline` and `PendingConstructionReceipt` are Components on the publication entity, so baseline, staged world, effects and verdict name the same publication by construction | verification compares the result to the baseline that belongs to this transaction | RESOLVED_BY_A10 | SOURCE_CONFIRMED |
| ORDER-ROOM-REPLACE | candidate construction and validation | publication, then the outgoing sweep | `replace_live_world` stages the whole replacement and publishes on admission | two rooms' bodies coexist for the length of ONE command flush, which no scheduled system can observe; a refusal publishes nothing | RESOLVED_BY_A10 | SOURCE_CONFIRMED |
| ORDER-CONTENT-ACTIVATION | content publication gate | shell activation and content commit | atomicity, not order: `advance_pending_route` is an exclusive system that runs each gate evaluator inside its own world access; on refusal it cancels, then releases | a gate answers `Hold` / `Admit` / `Refuse` and never releases itself; a gate that answered yes earlier does not authorize a later activation | NORMAL_PIPELINE_ORDER | SOURCE_CONFIRMED |
| ORDER-CHECKPOINT | checkpoint admission | room/replay commit and domain restore | a schedule only the commit executor runs (`CheckpointDomainApply`), plus an operation key that is a session stamp and an admission-advanced sequence | domain restore cannot treat an unadmitted request as authority; `scripts/check_commit_only_schedules_have_one_runner.py` keeps the executor the only runner | NORMAL_PIPELINE_ORDER | SOURCE_CONFIRMED |
| ORDER-NEW-GAME | new-game admission | the fresh-run reducers | the confirmed-frame lifecycle commit, which runs `CheckpointDomainApply` with `FreshRunRestore` installed | domain state resets only inside an authorized commit; a New Game is admitted by the checkpoint coordinator like a death | NORMAL_PIPELINE_ORDER | SOURCE_CONFIRMED |

Do not replace clear Bevy set ordering with a custom scheduler abstraction.

## 10. Compatibility and fallback roads

| ID | Road | What it is | Current state | Deletion/consolidation condition | Blocked by | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| TRANS-DIRECT-SESSION-FALLBACK | Unscoped direct/headless session spawning | supported composition answer, not a fallback | `live_scope_of` branches on `SessionGatedSimulation`: shell-routed selects the root the activation names and answers `None` when no scope is active; direct-entry has no activation, so its single root is the authority. | Remove only if direct/headless hosts must adopt the shell session lifecycle — a composition decision. | — | SOURCE_CONFIRMED |
| TRANS-DIRECT-GENERATION-FALLBACK | App registry values for construction with no activated generation | deleted road | Deleted 2026-09-20 with C04. A live rebuild reads the activated generation or refuses, in every composition. A direct composition that rebuilds rooms installs a scoped `SessionMechanics`. | — | — | SOURCE_CONFIRMED |
| TRANS-DIRECT-CONTENT-BINDING | Missing content-binding allowance in direct fixtures | supported composition answer, not a gap | `verify_and_publish` discriminates on `SessionGatedSimulation`: a shell-routed session missing `ActiveContentBinding` refuses the room; a direct-entry fixture states no binding and means it. | Delete only if direct-entry compositions must own a content binding — a composition decision. | — | SOURCE_CONFIRMED |
| TRANS-FACADE-MIRRORS | Umbrella facade and compatibility re-export mirrors | migration/compatibility layer | `crates/ambition_platformer2d/src/lib.rs` re-exports and does not own: almost every `pub use` is rooted in another crate. The one local re-export, manual stepping, is SDK surface. | Remove internal mirror paths as consumers move to the owning crate (C08). Keep useful external facade ergonomics. | — | SOURCE_CONFIRMED |
| TRANS-PLANNING-HISTORY | Historical closure prose in live queue | resolved documentation debt | `queue.md`, `status.md` and the decision ledger hold current state; closed case files live in Git history (C10). | Keep the queue role structural: open executable rows only. | — | SOURCE_CONFIRMED |

The baseline had four more transitional rows. A10 closed three of them (the
destructive retire-before-verify road, the candidate mode flag, the split
hot-reload writes) and C10 closed one. The three `DIRECT-*` rows are composition
discriminators with a refusing shell side, not compatibility roads.

## 11. Crate and package architecture

Size is a navigation signal, not a merge or split rule. For the whole package
graph run `python3 scripts/architecture_census.py --crate-table`, or read
`workspace_crates` in the ledger. The size clues below were measured on
2026-10-02.

| ID | Crate | Static size/fanout clue | Boundary classification | Current responsibility | Review direction | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CRATE-ACTOR-MONOLITH | ambition_platformer2d_actor_monolith | 118,574 LOC (53,805 tests); 253 .rs; 35 direct workspace deps; 7 reverse deps; Bevy | multiple domain services and remaining migration hub | The durable architecture calls it a migration-era hub, not the permanent boundary. Its target is a small residual actor kernel. Carve by semantic ownership and dependency closure, not by size. The actor-monolith pages under `docs/planning/engine/` own the carve. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-RUNTIME | ambition_platformer2d_runtime | 16,512 LOC; 45 .rs; 40 direct workspace deps; 6 reverse deps; Bevy | runtime/session/transition/rollback coordination | An assembler with high fan-out, not a high fan-in hub. Review the lifecycle coordinator versus provider/host assembly boundary. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-SHARED-TANGLE | ambition_platformer2d_shared_tangle | 26,809 LOC; 68 .rs; 4 direct workspace deps; 36 reverse deps; Bevy | cross-domain neutral protocols and residual shared vocabulary | The high fan-in crate. The name is a deliberate warning label until its exits are met. Keep proven neutral protocols (construction, lifecycle, `SimId`); move domain ownership out when a concrete owner exists. No new catch-all slot, command enum or shared context. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-FACADE | ambition_platformer2d | 4,296 LOC; 8 .rs; 60 direct workspace deps; 13 reverse deps; Bevy | public facade/re-export composition surface | The review is `BEVY-FACADE-REEXPORTS`: the facade does not hide ownership. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-GAME-SHELL | ambition_game_shell | 9,042 LOC; 21 .rs; 8 direct workspace deps; 3 reverse deps; Bevy | route/session lifecycle and activation transaction owner | Each responsibility is an owned type: route activation (`ShellRouter`, `PendingShellRoute`), transaction correlation (`PreparedSessionRegistry`), session bridge (`ActiveGameplaySession`), and publication gates (`ShellActivationGates` with a `Hold` / `Admit` / `Refuse` verdict). The carrier is Bevy's one-shot system id; the verdict is what the shell adds. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-ROLLBACK-GGRS | ambition_platformer2d_rollback_ggrs | 8,413 LOC; 11 .rs; 10 direct workspace deps; 1 reverse deps; Bevy | GGRS adapter and rollback timeline owner | The only crate that depends on `bevy_ggrs`. Domains declare rollback state against the neutral `RollbackRegistrar` trait in `ambition_platformer2d_core`; this crate's implementation installs it into the concrete host. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-REGISTRY-CORE | ambition_registry_core | 216 LOC; 1 .rs; 0 direct workspace deps; 4 reverse deps; no direct Bevy | generic registry conflict classification utility | `Classification` is `New` / `Idempotent` / `Conflict`, with no `Replace`, so a silent overwrite cannot be the default for a registry that adopts it. A registry that declines `classify` says why in place. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-BINDING | ambition_binding | 747 LOC; 1 .rs; 0 direct workspace deps; 2 reverse deps; no direct Bevy | small binding/protocol boundary | A semantic unit, not forwarding residue: no `pub use`, one idea (resolve an authored name in a namespace, with ambiguous and unresolved as outcomes), and no dependency but `tracing`. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-BODY-SEED | ambition_body_seed | 2,258 LOC; 5 .rs; 8 direct workspace deps; 5 reverse deps; Bevy | body-seeding capability boundary | One concept (the values a body is built from, carried by `ActorClusterSeed`), but not reusable outside this domain: it depends on eight domain crates. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |
| CRATE-MOUNT | ambition_mount | 1,904 LOC; 1 .rs; 4 direct workspace deps; 8 reverse deps; Bevy | mount capability | The model capability: it installs its own systems and publishes ordering sets, each with one member. Consumers order against the sets, never against its systems. | Use dependency closure, ownership, public API meaning, and change fanout. Do not merge or split from size alone. | SOURCE_CONFIRMED |

A later extraction must name a semantic owner and show that dependency or change
fan-out improves. Small crates are not merge candidates because they are small.

## 12. Bevy leverage census

| ID | Mechanism | Classification | Why the custom part exists | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- |
| BEVY-SESSION-ROOT | SessionRoot plus SessionWorldRef/Mut | THIN_USEFUL_BEVY_ADAPTER | `SessionWorldRef<T>` and `SessionWorldMut<T>` are `pub type` aliases for `Single<.., With<SessionRoot>>`; `SessionRoot(SessionScopeId)` is an ordinary component. The Ambition content is the choice of filter: a system that names one cannot fall back to process-resident state. This row owns the alias population: the marker below, counted in parameter form (`SessionWorldRef<`, `SessionWorldMut<`) over production `crates/` and `game/` with test modules and comments stripped. | Keep direct Bevy mechanisms visible. The `SoleLiveRoom*` aliases are a separate one-live-room debt owned by the open-world residency plan. | SOURCE_CONFIRMED |
| BEVY-SESSION-COMMANDS | SessionCommands | THIN_USEFUL_BEVY_ADAPTER | A two-field `SystemParam` (`Commands` plus the optional active scope) with one method and `Deref` to `Commands`. It saves a parameter slot. The spawn verb is `SpawnSessionScopedExt` on plain `Commands`; this type is one of two ways to get its scope value. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_CONFIRMED |
| BEVY-CONSTRUCTION | Typed construction plans and candidate filters | JUSTIFIED_AMBITION_SEMANTICS | `InactiveCandidate` is a Bevy disabling component (`register_disabling_component`), so ordinary queries skip candidates by Bevy's rules. Ambition adds a nesting refcount, `SimId` provenance, typed plans, `TransactionId` and roster verification. Roster verification checks after the fact; it does not prevent a recipe from changing its root. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_CONFIRMED |
| BEVY-ROLLBACK | ActiveRollbackAuthority | JUSTIFIED_AMBITION_SEMANTICS | Bevy does not define rollback owner, timeline generation, snapshot contract or health. The resource is one completed authority collapse: five private fields, one constructor that decides carry-forward from the owner, and `Timeline::StoodDown` for a timeline that stopped while its session did not. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_CONFIRMED |
| BEVY-CONTENT-GENERATION | Prepared content and generation identity | JUSTIFIED_AMBITION_SEMANTICS | Two ids, because one must be peer-stable and one must not: `ContentEpoch` (App-local activation lineage) and `PeerContentIdentity` (which content). A Bevy asset handle is App-local and cannot be the second. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_CONFIRMED |
| BEVY-MECHANICAL-EDIT | Mechanical edit admission protocol | JUSTIFIED_AMBITION_SEMANTICS | Change detection can see an editor write but cannot decide whether it may change mechanics under rollback. For player stats it cannot even detect the edit: the editor resource has two writers, so the proposer compares against `PlayerStatsSyncSnapshot`. Admission (`decide_mechanical_edit_admission`) is shared and domain-blind. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_CONFIRMED |
| BEVY-FACADE-REEXPORTS | Facade and convenience mirrors | REVIEWED_2026_09_18 | The facade's renames are crate-alias prefix strips that map back to the owner by rule. The two item renames are written at the owner as well as at the facade. Two drivers that arrive from two crates are a feature selection, not a second owner. | Keep direct Bevy mechanisms visible. Retain custom code only for the stated Ambition invariant or a small ergonomic adapter. | SOURCE_CONFIRMED |

<!-- alias-census: parameter_form=32 files=20 -->
The line above is `BEVY-SESSION-ROOT`'s machine-readable count.
`scripts/check_alias_census_agrees_with_source.py` compares it with a live
measurement. The per-spelling split is in the plan, under C03.

The strongest pattern is **Bevy mechanism + Ambition semantic invariant**:
entity/component/query storage with session ownership; `Commands` plus captured
session scope; normal schedule sets plus rollback and edit admission; normal
entity visibility plus candidate transaction identity. Review custom forwarding
that hides Bevy without adding an invariant. Do not replace Bevy ECS, schedules,
resources or plugins.

## 13. Test architecture

| ID | Test family | Current role | Limit / verification still needed | Evidence |
| --- | --- | --- | --- | --- |
| TEST-PROD-COMPOSITION | Production composition witnesses | App-level tests exercise shell/session/content/room behavior through shipped plugin composition rather than only direct helpers. | — | SOURCE_CONFIRMED |
| TEST-HELPERS | Unit and helper tests | The generated inventory reports thousands of tests. Most are local behavior tests and are not architecture witnesses by themselves. | — | SOURCE_CONFIRMED |
| TEST-POLICY | Policy and source scanners | Static policy tests and scripts enforce source-shape rules, dependency rules, registration rules, and documentation contracts without runtime simulation. | — | SOURCE_CONFIRMED |
| TEST-SCHEDULE | Schedule-graph witnesses | Tests such as sim_phase_pins and scripts such as measure_foreign_system_ordering inspect declared schedule ownership/order. They establish graph shape, not runtime outcome by themselves. | — | SOURCE_CONFIRMED |
| TEST-TWO-APP | Two-App / peer determinism witnesses | `game/ambition_app/tests/shell_host_lifecycle.rs` builds two hosts that reach Ambition by different route histories, asserts the histories differ, and compares the canonical entity census, the mechanical values and the GGRS component checksums. A third arm asserts disagreement against a different session, so a constant projection fails. | — | SOURCE_CONFIRMED |
| TEST-ROLLBACK | Rollback canaries and reversion/poison witnesses | Rollback tests cover state registration, checksum behaviour, lifecycle rebase and mechanical edit admission. Use poison tests at major authority boundaries; do not add them for ordinary details. | — | SOURCE_CONFIRMED |

Tests are not the architecture owner. When an ownership change makes an invalid
state impossible by type or storage shape, keep the production-composition
witness and remove only tests that become pure duplication.

## 14. Planning and documentation architecture

| ID | Planning layer | Classification | Current role/state | Consolidation direction | Evidence |
| --- | --- | --- | --- | --- | --- |
| DOC-QUEUE | queue.md execution ledger | current executable planning | The queue holds open executable rows with owner, current state, next action, blockers and acceptance. Closed investigations are not in the live control plane. | Maintain this role; do not append completion diaries. | SOURCE_CONFIRMED |
| DOC-OWNER-PLANS | Focused owner plans | durable current design within live planning | Focused owner documents define current authority, topology, executable work, acceptance, and forbidden regressions for major engine domains. | — | SOURCE_CONFIRMED |
| DOC-ARCHITECTURE | Durable architecture document | durable design | engine-architecture.md states stable layer ownership, content/construction flow, session/rollback lifetimes, identity rules, and capability composition principles. | — | SOURCE_CONFIRMED |
| DOC-ADR | Architecture decision records | decision record | ADRs record explicit architectural decisions such as immutable prepared content, exact session identity, spawn provenance, and construction planning. | — | SOURCE_CONFIRMED |
| DOC-HISTORY | Engineering journals and Git history | historical evidence, not current-state authority | Repository policy places investigation/history in Git and engineering memory under dev rather than in current planning. The census should not duplicate that narrative. | — | SOURCE_CONFIRMED |

## Current complexity classification

### Essential complexity

Keep these distinctions:

- rollback owner, timeline generation, schema/content contract and health as one
  rollback authority;
- gameplay session lifetime separate from process and shell transaction lifetime;
- peer-stable simulation identity separate from Bevy `Entity` and local
  correlation ids;
- immutable prepared content generation separate from mutable timeline state;
- active content selection separate from a pending candidate;
- editable desired value, admission decision, admitted authority and runtime
  projection;
- checkpoint admission and pinned continuity separate from generic room
  construction;
- typed deterministic construction and explicit publication/retirement;
- direct Bevy ECS, resources, components, systems, schedules, plugins, queries
  and messages where those already express the semantics.

### Remaining accidental complexity

1. Process storage for state that source declares session- or generation-owned,
   where resets and stale-owner guards compensate for the lifetime mismatch (C03).
2. Optional reads that do not say whether absence is a capability choice or a
   missing required authority (C07).
3. Readers that assume exactly one live room (`SoleLiveRoom*`), owned by the
   open-world residency plan.

### Transitional complexity

- Compatibility and facade mirrors (`TRANS-FACADE-MIRRORS`, C08).

### Uncertain areas

- which of the 34 narrower-lifetime resources should move to `SessionRoot`, which
  should become explicit session-keyed process services, and which are
  presentation-only;
- which facade re-exports are valuable external ergonomics and which are internal
  ownership camouflage.

## Facts static inspection cannot establish

This census does not claim:

- successful type checking, macro expansion, trait resolution or feature
  compilation;
- that every declared plugin or system registration is on the shipped path under
  all feature sets;
- runtime schedule execution beyond explicit source ordering;
- rollback restore behaviour for state whose registration contract is not
  explicit in source;
- the absence of hidden observer, hook or external side effects outside the
  bounded candidate protocol;
- that any test passes at the current commit.
