# Architecture consolidation plan

This plan ranks the work that removes independent truths. The ranking is by
authority and lifetime leverage, not by LOC. A campaign can move down when a new
source reading shows that two values have different semantic owners.

The current authority map is [`architecture-census.md`](architecture-census.md).
Durable rulings are in [`maintainer-decisions.md`](../maintainer-decisions.md).
Executable slices go to [`queue.md`](../queue.md).

## Priority table

| Rank | ID | Opportunity | State | Size | Gate |
| --- | --- | --- | --- | --- | --- |
| 1 | C01 | One live room/session replacement transaction (A10) | ✅ DONE 2026-09-15 | large | — |
| 2 | C02 | Separate local lifetime/correlation identity from peer-stable provenance | ✅ DONE 2026-10-03 as [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity---done-2026-10-03): every road closed | large | — |
| 3 | C03 | Consolidate session-owned state and reduce reset-only App globals | OPEN — startable, not started | large | none |
| 4 | C04 | Activated generation mechanics are the only live construction source | ✅ DONE 2026-09-20 | medium | — |
| 5 | C05 | Collapse live content/session publication onto one admitted candidate | ⛔ DECIDED 2026-09-19: do not start; kept for its regression rule | none | — |
| 6 | C06 | Converge reconstruction roads on one materialization/publication engine | ✅ CONVERGED 2026-09-23 | — | — |
| 7 | C07 | Explicit composition contracts for required optional authorities | ✅ CONVERGED 2026-10-08 | medium | — |
| 8 | C08 | Prune compatibility facades and forwarding mirrors | OPEN — later cleanup | medium | stay off identity surfaces while ID-PEER runs |
| 9 | C09 | Review crate boundaries by semantic ownership, not size | OPEN — later structural review | medium-large | after C03 settles owners (C07 converged) |
| 10 | C10 | Separate current planning state from history | ✅ DONE 2026-09-14 | small | — |
| 11 | C11 | Authority polish: one owner per mechanical fact, no mirror in the rollback kernel | ACTIVE as [AUTHORITY-POLISH](../queue.md#authority-polish--one-owner-per-mechanical-fact-and-no-mirror-in-the-rollback-kernel); the queue row owns the order | medium | runs beside C03 and C07 |

## Converged shapes (closed campaigns)

These are receipts. Each one states the shape that now holds and the rule that
keeps it.

### C01 — one replacement transaction

There is one road into a live session and one publication authority per level.
`RoomConstructionPlan::replace_live_world` stages the whole replacement in
`PendingWorldReplacement`, builds it hidden under `InactiveCandidate`, verifies a
projected roster, and publishes or drops it. A refusal leaves world N intact. A
candidate session is prepared beside the live one and admitted through
`ShellActivationGates`. Root components, `MovingPlatformSet`, room entities and
content binding are projections of that one decision.

**Regression rule:** a new road that changes the authoritative world must cross a
publication verdict. ⛔ Do not add a flag that selects between a candidate road
and a live road.

Owner document: [construction and reconstitution](../engine/construction-and-reconstitution.md).

### C04 — one generation authority for live construction

`GenerationMechanics` holds one set of registries and no ranking. The caller
picks the authority at a named constructor: `of` (an activated generation) or
`for_live_session` (the live generation, or a refusal). No live rebuild reads App
registries. The LDtk world reload takes `for_live_session` like the other live
roads. A direct composition that rebuilds rooms installs its own scoped
`SessionMechanics` through `install_direct_session_root`, which freezes the
authority from that composition's own preparation.

**Deliberate exception:** `perception_extent_for` keeps its App fallback. It is a
read when a body decides, not a construction road.

**Regression rule:** no anonymous App-global construction fallback returns
(maintainer ruling 2026-09-19, Q144/Q146). Guard:
`an_ldtk_world_reload_rebuilds_the_room_from_the_generation_not_the_app`.

### C05 — decided: do not start

Five of the six values this campaign named are Components on the one session
root, lowered from one `PreparedPlatformerSource` (`PlatformerSessionWorld`,
`PreparedContent`, `PreparedContentIdentity`, `ActiveContentBinding`,
`LdtkRuntimeIndex`). The sixth, `SessionMechanics`, is one App resource with one
installer (`PreparedCandidateSession::adopt`) and one remover (the retirement
reset). Its value comes from the same frozen generation. Moving it onto the root
changes storage kind only, and it makes its optional readers worse.

**Regression rule:** a new value that means "this session is now generation N+1"
goes on the session root, not into a new App global. Re-open this row only on a
measured defect: a reader that sees mechanics disagree with the identity beside
them.

### C06 — one materializer, one verdict skeleton

- One private materializer, `spawn_contents_for`, under two public wrappers:
  `spawn_contents` (session activation) and `replace_live_world` (room
  transition, death and checkpoint restore, New Game, LDtk dev reload).
- Replay, checkpoint restore and New Game record a transition intent and ride the
  room-transition road. New Game is a checkpoint restore to the fresh baseline.
- Every road that waits on a room verdict calls `rooms::settle_publication`. It
  reads this publication's verdict once, runs the matching arm and retires the
  receipt on both arms.
- Activation is deliberately separate: `PreparedCandidateSession::adopt`
  finalizes, then promotes or discards a whole candidate session.

**Regression rule:** a new road calls `settle_publication`; it does not read a
verdict itself. Keep each road's retention filter explicit (`RoomResident` for
transitions and dev reload, the wider `RoomScopedEntity` sweep for New Game).
Do not merge those filters.

### C07 — explicit composition contracts (converged 2026-10-08)

Capabilities are optional and composable (Q146/Q144), and a production
composition cannot hold a live state without its required owners:

- Each optional read of a session-owned canonical authority names what its
  `None` means, in a closed class vocabulary:
  [session-authority-none-arms.md](session-authority-none-arms.md), held to source
  by `scripts/check_session_authority_none_arms.py`.
- Each supported profile installs every capability it does not omit and
  validates its session-edge parameters, and refuses content that needs a
  capability it omits (A9, `ambition_platformer2d_host/tests/supported_profiles.rs`).
- A session-gated composition with no active scope refuses to mint a session root.
- A root that a room publishes into refuses the room when it holds no
  `ActiveContentBinding`, in a direct composition too (2026-10-08). Measured first,
  with a probe on the arm: the SDK host tests verify 29 rooms in direct
  compositions, and the shipped app and the three demo suites 2,302 shell-routed
  ones; each root held a binding. Only monolith lib fixtures published into a root
  without one, and they now state it. Witness:
  `stage::tests::a_direct_root_with_no_content_binding_refuses_the_room` (poison:
  the refusal only when shell-routed; red, the room publishes). A direct fixture
  with no session root states no binding: there is no generation to be stale
  against.

**Regression rules:** a new optional read of a canonical authority gets a row, or
the check is red. Do not make a required canonical authority optional so that a
fixture can skip it; the fixture installs it. The ruling's greyed-out
return-to-shell row has no composition to apply to (every composition that shows
the system menu is shell-hosted); do not build it until a shell-less one exists.

### C10 — planning separation

`queue.md` holds open executable work. `status.md` is a short orientation.
Closed investigation history lives in Git. The ledger row
`TRANS-PLANNING-HISTORY` is the receipt. **Regression rule:** when a control-plane
file grows case files again, compress it in place. Do not add an archive page.

## 3. C03 — Consolidate session-owned state and reduce reset-only App globals

**State:** IN PROGRESS, AND NO LONGER A CAMPAIGN. Seven families have landed (see
"Landed families" below). From 2026-10-09 (Jon's integration reorder) a remaining
member moves only when a two-session witness shows its value leaks from one
session into the next; the count is not the goal.

### Scope and current authority

Source explicitly groups **29** App resources as gameplay-session or
activated-generation state (46 until the checkpoint family and the room memories
left on 2026-10-07; on 2026-10-08, 38 until the session clock left, 36 until the
door countdown left, 35 until the switch queue left, 34 until the encounter
view left and 33 until the four checkpoint baselines left):

<!-- session-owner-census: SessionScopedResources=28 SessionMechanics=1 -->
<!-- session-root-family: SessionCheckpointState=6 -->
- `SessionScopedResources` (**28**) in `actor_monolith/src/session/teardown.rs`;
- (`SessionOwnedCheckpointState`, the third bundle of six, is DELETED: its values are
  components of the session root, `SessionCheckpointState` (6) in
  `actor_monolith/src/session/checkpoint.rs`, and no reset runs for them.)
- `SessionMechanics` (1 resource with six fields; do not count its fields).

The HTML comment above is the machine-readable copy.
`scripts/check_session_owner_census_matches_source.py` compares it with source.
The census page lists the member names. The guard counts the optional
(`Option<ResMut<..>>`) members too, for example `BossDefeatsSinceCheckpoint`.

### Measured facts that shape the campaign

- **There is no reset-only subset.** Every `SessionScopedResources` member has a
  reader outside its reset (`scripts/measure_session_scoped_resource_readers.py`).
  The campaign is a migration, not a move.
- **The two reset lists are a partition.** Their intersection is empty. Merging
  them removes a struct, not a truth.
- **Most members are rollback state.** Of the 27, 23 are state-registered, 2
  declare themselves derived (`ControlledSubject`, `EncounterView`), and 2 carry
  no rollback decision (`BossEncounterRegistry`, `CutsceneTriggerQueue`). The
  checkpoint family is 5 registered plus `AbandonedCheckpointOperation`, which is
  host-side by design.
- **The reset exists only to compensate for process storage.** Both bundles reset
  at `SessionScopeSet::Activate`, before any provider builds the session world. A
  value stored on the session root needs no reset: a new root carries new
  components. The real deliverable is to delete that reset for each migrated value.

### Governing rule (maintainer ruling 2026-09-19, Q132)

> If mutable state can legitimately hold different values for two sessions,
> generations, participants or timelines that could coexist during preparation,
> handoff, rollback, multiplayer or testing, it must carry the appropriate
> explicit scope rather than relying on anonymous App-global singleton identity.

- The test is "could two coexisting sessions differ here?", not "is it a
  resource?" or "does it reset?".
- Explicit scope does not mean a child entity of `SessionRoot`. A keyed or scoped
  resource also satisfies it.
- Out of scope: prepared immutable data, true application infrastructure, and
  user settings and save data (separate authorities that enter a session only
  through admission).
- There is exactly one canonical live `SessionRoot`. A candidate carries
  `CandidateSessionRoot` and must never count as a canonical root.

### Triage order (the ruling's categories)

| category | members | n |
| --- | --- | ---: |
| current room / world / session state | ~~`LastCutsceneRoom`, `LastQuestRoom`, `RoomTransitionCooldown`~~ (LANDED, on the root), `SlotInteractionState` | 1 |
| participant state | `ControlledSubject`, `PossessionState` | 2 |
| encounter state | ~~`EncounterView`~~ (LANDED, on the root), `BossEncounterRegistry`, `AuthoredOccurrences` | 2 |
| simulation clocks / timeline state | `GameplayElapsed`, `LiveMatchTicks`, `SessionMatchOrdinal`, `ProjectileSeqCounter` | 4 |
| checkpoint / restore state | ~~the six `SessionOwnedCheckpointState` members~~, ~~`CustodyBaseline`, `MintedItemBaseline`, `OccurrenceBaseline`, `OwnedItemsBaseline`~~ (LANDED, on the root), `SaveRestored` | 1 |
| session request / admission queues | `CutsceneTriggerQueue`, ~~`SwitchActivationQueue`~~ (LANDED, on the root), `PendingLifecycleCommit` | 2 |
| admitted mechanics / configuration | `SessionMechanics`, `BaseGravity` | 2 |
| cutscene / session gameplay state | `ActiveCutscene`, `ActiveConversation`, `CutsceneSkipHold` | 3 |
| transient progression | `QuestRegistry`, `StocksMatchSettled`, `SuddenDeathEntered` | 3 |

Leave `SessionMechanics` where it is (see C05). Decide `BaseGravity` together
with its ingress question (Q136 ruling: choose ingress by semantic ownership).

### The session-root aliases

<!-- alias-split: SessionWorldRef=43/19 SessionWorldMut=48/26 live_session_world_root=3/1 session_root_for_scope=2/2 SoleLiveRoom=9/9 SoleLiveRoomSpec=5/5 -->
| spelling | what it is | production uses / files |
| --- | --- | ---: |
| `SessionWorldRef<T>` | `Single<Ref<T>, With<SessionRoot>>` | 43 / 19 |
| `SessionWorldMut<T>` | `Single<&mut T, With<SessionRoot>>` | 48 / 26 |
| `live_session_world_root` | the root whose scope is the active scope | 3 / 1 |
| `session_root_for_scope` | a named scope's root, through the disabling marker | 2 / 2 |
| `SoleLiveRoom<T>` | `Single<Ref<T>, With<RoomInstanceRoot>>`; one-live-room debt, not a session alias | 9 / 9 |
| `SoleLiveRoomSpec` | the authored spec of the one live room; same debt | 5 / 5 |

`SoleLiveRoomMut<T>` is deleted: its last user, Smash's respawn platforms, writes each protected body's own live room.

Method: parameter form (`Name<` for aliases, `name(` for functions) over
`crates/` and `game/`, test files dropped, `#[cfg(test)]` modules and comments
stripped. `scripts/check_alias_census_agrees_with_source.py` compares the marker
with a live count. The census row `BEVY-SESSION-ROOT` owns the session-alias
total. The `SoleLiveRoom*` readers belong to
[open-world runtime and residency](../engine/open-world-runtime-and-residency.md).

The `Single` semantics is the engine's meaning (Q132). The scope-aware helpers are
for lifecycle code that sees both sides of a handoff. Guards:
`the_shipped_app_never_holds_two_session_roots_across_a_handoff`,
`a_prepared_candidate_never_counts_as_a_canonical_session_root`, and
`scripts/check_session_root_construction_is_declared.py`.

### Sequence

Do not begin by moving all 29 values. Work owner by owner:

1. Re-run `python3 scripts/architecture_census.py` and confirm the list.
2. For each family, state whether the value must exist before `SessionRoot`, only
   during a live session, or only for presentation.
3. For rollback-registered values, record the registration key and restore
   boundary before you change storage. A rollback key is a wire identity. Do not
   rename a key to match a type (`RoomTransitionCooldown` registers as
   `resource.sandbox_sim_state` on purpose).
4. Pick one coherent family with one owner. (`SessionOwnedCheckpointState` was the
   first review unit and has landed; see "Landed families".)
5. Choose storage from semantics: a `SessionRoot` component for live-session
   state; an explicit session-keyed coordinator when the value must exist before
   the root; an App resource only when process lifetime is real.
6. Delete the old reset and retirement compensation only after the new owner is
   the sole authority.
7. Keep direct Bevy queries and system parameters. Do not add a generic state
   container.

### Landed families

**1. The checkpoint coordinator, 2026-10-07.** `SessionCheckpointOperations`,
`SessionCheckpointOutcomes`, `AcceptedCheckpointRestore`,
`AbandonedCheckpointOperation`, `SessionStartupResume` and
`OutstandingCheckpointRequest` are components of the session root. They were
process-global resources that `reset_checkpoint_coordinator_on_activation`
overwrote at each activation; that system, the `SessionOwnedCheckpointState`
`SystemParam` and the `SessionScopeActivated` registration made for it are deleted.

- **Owner:** the session root. `SessionCheckpointHorizonPlugin` registers each as a
  required component of `SessionRoot` (`require_checkpoint_state_on_session_root`),
  so a root has all six from the moment a candidate is published, a direct host's
  root has them too, and a hidden candidate has none (nothing reads a coordinator
  before its session is live). A plugin installed after a root exists backfills it,
  because every reader is a `Single` on the root and would otherwise skip in silence.
- **Rollback identities did not move.** The five registered values keep their keys
  (`resource.session_checkpoint_operations` and the rest; a key is a wire identity).
  The kind in each row is `component-clone-custom-checksum` now, and
  `GGRS_ROLLBACK_SCHEMA_VERSION` went 318 -> 319 (the readable baseline, the JSON
  baseline and the codec-shape record follow). `AbandonedCheckpointOperation` is
  still deliberately not registered; its waiver moved from `RESOURCE_WAIVED` to
  `WAIVED` in `rollback_coverage.rs`.
- **Witness that two sessions disagree without contamination:**
  `checkpoint::tests::two_session_roots_hold_two_checkpoint_coordinators`. Session A
  is live with three spent operations and an owed restore; a candidate root beside
  it holds nothing and leaves A's coordinator untouched; after the swap the live
  coordinator is fresh and its first operation has sequence zero. Its guard of
  the plugin's list against the bundle:
  `a_session_root_carries_the_whole_checkpoint_coordinator`.
- **Counts:** the App-resource census is 40 (39 + `SessionMechanics`), down from 46.
  The root family has its own marker above; RULE 3 (every member registers or is
  declared deliberately unregistered) and RULE 4 (the census name list) still hold
  for it.
- The restore's readers (`room_transition` loading and commit, the confirmed commit
  in `rollback_ggrs`) are `SessionWorldRef`/`SessionWorldMut` params or
  `session_world_component` reads. The terminalizer holds the accepted operation
  and the outcomes through one `get_components_mut` borrow.

**2. The room-entry memories, 2026-10-07.** `LastQuestRoom` (the rooms the quest
producer last announced) and `LastCutsceneRoom` (the same for entry cutscenes) are
components of the session root. Each is a single-writer edge detector that a reset
cleared at every session edge, because a new game that starts in the room the last
session ended in would otherwise skip its first room's quest event and cutscene.

- **Owner:** the session root; each is required by `SessionRoot`
  (`require_on_session_root`, the helper family 1's six values now use too, which
  also backfills a root that exists when a plugin installs late).
- **Rollback identities did not move:** `resource.quest_last_room` and
  `cutscene.last_room` keep their keys; their kinds are
  `component-clone-custom-checksum` and `component-canonical`. The schema version
  moves 319 -> 320.
- **Deleted:** the two members of `SessionScopedResources` and their two reset lines.
  `SessionScopedResources` was 37 and the App-resource total 38.
- **Witnesses:** `quest::tests::a_new_session_in_the_same_room_announces_it_again` and
  `cutscene::tests::a_new_session_in_the_same_room_queues_its_entry_cutscene_again`
  (A live and having announced `hall`; a candidate beside it has no memory and
  changes none; after the swap B announces `hall`). Each poisoned by giving B A's
  memory at the swap: the arm fails on "B was born with A's memory".
- `RoomTransitionCooldown`, the third of the group, landed as family 4. The
  reason first given for leaving it (its readers sit below `SessionRoot`) did not
  hold: the type and `SessionRoot` are both in `shared_tangle`.
  `SlotInteractionState` lives in `ambition_characters::control`, below
  `shared_tangle`, so it does need a seam.

**3. The session clock, 2026-10-08.** `GameplayElapsed` (the session's sum of the
scaled simulation dt, which the brain reads for its reaction-latency lookback) and
`WorldTimeSchedule` (OW5: when each gone occurrence comes back, on that clock) are
components of the session root, each required by `SessionRoot`.

- **Rollback identities did not move:** `resource.gameplay_elapsed` and
  `feature.world_time_schedule` keep their keys; their kinds are
  `component-canonical` and `component-clone-custom-checksum`. The schema version
  moves 331 -> 332.
- **Deleted:** the two members of `SessionScopedResources` and their two reset lines.
  `SessionScopedResources` was 35 and the App-resource total 36.
- **Witness:**
  `world_time_schedule::tests::two_session_roots_hold_two_clocks_and_two_schedules`
  (A runs and schedules a return; a candidate beside it has neither and changes
  neither; after the swap B counts from zero with no record).
- **The match family is not started, and it is larger than the bundle shows:**
  `StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks` and
  `SessionMatchOrdinal` are bundle members, and each is stamped with the
  `MatchInstance` that `ActiveMatch` mints. `ActiveMatch` is not a bundle member (the
  teardown removes it), but it is the same family. Measured 2026-10-08: about 360
  references in 40 files (189 for `ActiveMatch` alone), the Smash demo's presentation
  reads included.

**4. The door countdown, 2026-10-08.** `RoomTransitionCooldown` (one countdown per
seat: the seat whose body crossed a door waits before it crosses again) is a
component of the session root, required by `SessionRoot`.

- **Rollback identity did not move:** `resource.sandbox_sim_state` keeps its key;
  its kind is `component-canonical`. The schema version moves 332 -> 333.
- **Deleted:** the member of `SessionScopedResources` and its reset line.
  `SessionScopedResources` was 34 and the App-resource total 35.
- **Witness:**
  `room_transition_cooldown_tests::two_session_roots_hold_two_door_countdowns`
  (A's seat 0 waits and counts down; a candidate beside it has no countdown and
  changes none; after the swap every seat of B is free). Poisoned by a default
  that holds seat 0: the arm fails on "B was born inside A's door countdown".
- **An instrument fix it needed:** the session-world writer census matched one
  lifetime in `SessionWorldMut<..>`, and the alias takes two. The `RoomClock`
  field was invisible, and so was a ninth `EncounterMusicRequest` writer (the
  audio context reset). Both are recorded with verdicts.

**5. The switch queue, 2026-10-08.** `SwitchActivationQueue` (the switch presses
waiting for the encounter drain, which runs one tick after the producer) is a
component of the session root, required by `SessionRoot`.

- **Rollback identity did not move:** `resource.switch_activation_queue` keeps
  its key; its kind is `component-clone-custom-checksum`. The schema version
  moves 333 -> 334.
- **Deleted:** the member of `SessionScopedResources` and its reset line.
  `SessionScopedResources` was 33 and the App-resource total 34.
- **Witness:**
  `one_drain_one_author::a_press_queued_in_one_session_is_not_delivered_into_the_next`
  (A's press is drained on A, the control; a press waiting on A at the swap is
  not drained on B). It reads the presses the drain resolved, not the switch
  flag: the first poison passed against a flag reading, because a toggle applied
  twice reads like one never applied. Poisoned by a default that holds a press:
  the arm fails on "A's waiting press was delivered into session B".

**6. The encounter view, 2026-10-08.** `EncounterView` (the camera zoom each live
room's encounters want, republished every tick) is a component of the session
root, required by `SessionRoot`. Its publisher takes `SessionWorldMut`; the
camera reads `Option<SessionWorldRef<..>>`, so with no session there is no zoom.

- **Rollback identity did not move:** `derived.encounter_view` is a derived
  component under the same key. The schema text prints `derived` for both forms,
  so the version does not move.
- **Deleted:** the member of `SessionScopedResources` and its reset line.
  `SessionScopedResources` was 32 and the App-resource total 33.
- **Witness:** `entity::tests::two_session_roots_hold_two_encounter_views` (A's
  encounter zooms A's room, the control; a candidate has no view; after the swap
  B's room is not zoomed). Poisoned by a default that zooms the room: the arm
  fails on "B was born framing A's encounter".

**7. The four checkpoint baselines, 2026-10-08.** `OccurrenceBaseline`,
`CustodyBaseline`, `MintedItemBaseline` and `OwnedItemsBaseline` (what a death
restores to) are components of the session root, required by `SessionRoot`
through the lifecycle and item checkpoint offers. They moved together because
the checkpoint offer pins all four as one tuple.

- **Rollback identity did not move:** the four keys stay; their kind is
  `component-clone-custom-checksum`. The schema version moves 334 -> 335.
- **Deleted:** the three members of `SessionScopedResources`, the optional
  fourth, and their reset lines. `SessionScopedResources` was 32 and the
  App-resource total 33.
- **A hazard the move exposed:** a load installs the candidate's durable
  horizon BEFORE the candidate root is promoted, while the outgoing root is
  still the `SessionRoot`. A direct port wrote the file's checkpoint onto the
  session that ends. `CandidateDurableHorizon::install` now installs only the
  ledger and gives back the checkpoint half, which
  `CandidateCheckpointBaselines::adopt_onto` writes onto the candidate's root
  after the promotion.
- **Witness:**
  `durable_horizon::tests::a_load_writes_its_checkpoint_onto_the_session_it_builds`
  (B holds the file's checkpoint and none of A's rows; A's own checkpoint is
  untouched, the control). Poisoned by writing the half through the live root
  at `install`: the arm fails on "the new session was born without the file's
  checkpoint". `session_isolation` (Sanic) also asserts that session B is born
  without A's three baselines.
- **An instrument fix it needed:** the C07 `None`-arm scan matched
  `Option<SessionWorldRef<..>>` only with no path before the alias. With the
  path allowed it found eight older optional reads with no row; they are rows
  now.

### Constraints

- The reset is legal today because it lands before frame zero. Storage on a root
  built inside the provider step is on the other side of that boundary. Re-derive
  the rollback-mutator answer per value.
- ⛔ Do not register `AbandonedCheckpointOperation`. It is a local preparation
  fact that two peers need not agree on.
- `Q128` landed 2026-10-03: `SimTick` and `ImpactHitstop` are members of
  `SessionScopedResources`, reset at activation. A migration that moves the
  tick moves the timeline's start; keep it reset on the activation edge.

**Acceptance:** a reviewer can name one owner for each migrated fact, and session
activation no longer overwrites a process-global copy to make the next session
safe.

**Risk:** medium-high. Moving rollback state or pre-root coordinator state to the
wrong owner can break startup, restore or snapshots.

## 8. C08 — Prune compatibility facades and forwarding mirrors

**State:** OPEN. Later cleanup.

The facade is one crate: `ambition_platformer2d` holds most of the workspace's
cross-crate `pub use` statements. The census rows `TRANS-FACADE-MIRRORS` and
`BEVY-FACADE-REEXPORTS` hold the measurement: the facade re-exports and does not
own, and every rename can be mapped back to its owner.

**Work:** remove internal mirror paths as consumers move to the owning crate.
Keep deliberate public facade ergonomics.

**Constraint:** do not run during a large ownership migration. Stay off session
and canonical identity while ID-PEER runs.

**Acceptance:** internal imports name the owner, and the public facade is small
enough to describe.

## 9. C09 — Review crate boundaries by semantic ownership, not size

**State:** OPEN. Later structural review.

**DO NOT START BEFORE:** C03 and C07 have settled their owners.

Size is a navigation signal, not a split rule. The durable architecture rejects a
size-only carve of `ambition_platformer2d_actor_monolith`, but it also names the
target: a small residual actor kernel. Split only at a proven independent owner.
Merge only a boundary that forwards one tightly coupled owner. A package's LOC
mixes source and tests; `game/ambition_app` is large mostly because of its
integration tests. The census `CRATE-*` rows hold the per-crate review.

**Acceptance:** the package graph follows ownership and change boundaries.

**Risk:** medium. A wrong merge increases dependency fan-out. A wrong split adds
wiring and compile churn.
