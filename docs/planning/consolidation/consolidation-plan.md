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
| 2 | C02 | Separate local lifetime/correlation identity from peer-stable provenance | ACTIVE as [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity); the queue row owns its road count | large | — |
| 3 | C03 | Consolidate session-owned state and reduce reset-only App globals | OPEN — startable, not started | large | none |
| 4 | C04 | Activated generation mechanics are the only live construction source | ✅ DONE 2026-09-20 | medium | — |
| 5 | C05 | Collapse live content/session publication onto one admitted candidate | ⛔ DECIDED 2026-09-19: do not start; kept for its regression rule | none | — |
| 6 | C06 | Converge reconstruction roads on one materialization/publication engine | ✅ CONVERGED 2026-09-23 | — | — |
| 7 | C07 | Explicit composition contracts for required optional authorities | OPEN — startable | medium | none |
| 8 | C08 | Prune compatibility facades and forwarding mirrors | OPEN — later cleanup | medium | stay off identity surfaces while ID-PEER runs |
| 9 | C09 | Review crate boundaries by semantic ownership, not size | OPEN — later structural review | medium-large | after C03 and C07 settle owners |
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

### C10 — planning separation

`queue.md` holds open executable work. `status.md` is a short orientation.
Closed investigation history lives in Git. The ledger row
`TRANS-PLANNING-HISTORY` is the receipt. **Regression rule:** when a control-plane
file grows case files again, compress it in place. Do not add an archive page.

## 3. C03 — Consolidate session-owned state and reduce reset-only App globals

**State:** OPEN. Startable. Not started.

### Scope and current authority

Source explicitly groups **39** App resources as gameplay-session or
activated-generation state:

<!-- session-owner-census: SessionScopedResources=32 SessionOwnedCheckpointState=6 SessionMechanics=1 -->
- `SessionScopedResources` (**32**) in `actor_monolith/src/session/teardown.rs`;
- `SessionOwnedCheckpointState` (6) in `actor_monolith/src/session/checkpoint.rs`;
- `SessionMechanics` (1 resource with six fields; do not count its fields).

The HTML comment above is the machine-readable copy.
`scripts/check_session_owner_census_matches_source.py` compares it with source.
The census page lists the member names. The bundle also holds two optional
members that the guard does not count: `BossDefeatsSinceCheckpoint` (checkpoint /
restore state) and `BreakableRespawnSchedule` (a session-clock schedule).
Include them in the migration.

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
| current room / world / session state | `LastCutsceneRoom`, `LastQuestRoom`, `RoomTransitionCooldown`, `SlotInteractionState` | 4 |
| participant state | `ControlledSubject`, `PossessionState` | 2 |
| encounter state | `EncounterView`, `BossEncounterRegistry`, `AuthoredOccurrences` | 3 |
| simulation clocks / timeline state | `GameplayElapsed`, `LiveMatchTicks`, `SessionMatchOrdinal`, `ProjectileSeqCounter` | 4 |
| checkpoint / restore state | the six `SessionOwnedCheckpointState` members, `SaveRestored`, `CustodyBaseline`, `MintedItemBaseline`, `OccurrenceBaseline` | 10 |
| session request / admission queues | `CutsceneTriggerQueue`, `SwitchActivationQueue`, `PendingLifecycleCommit` | 3 |
| admitted mechanics / configuration | `SessionMechanics`, `BaseGravity` | 2 |
| cutscene / session gameplay state | `ActiveCutscene`, `ActiveConversation`, `CutsceneSkipHold` | 3 |
| transient progression | `QuestRegistry`, `StocksMatchSettled`, `SuddenDeathEntered` | 3 |

Leave `SessionMechanics` where it is (see C05). Decide `BaseGravity` together
with its ingress question (Q136 ruling: choose ingress by semantic ownership).

### The session-root aliases

<!-- alias-split: SessionWorldRef=22/12 SessionWorldMut=10/9 live_session_world_root=3/1 session_root_for_scope=2/2 SoleLiveRoom=19/15 SoleLiveRoomSpec=11/11 -->
| spelling | what it is | production uses / files |
| --- | --- | ---: |
| `SessionWorldRef<T>` | `Single<Ref<T>, With<SessionRoot>>` | 22 / 12 |
| `SessionWorldMut<T>` | `Single<&mut T, With<SessionRoot>>` | 10 / 9 |
| `live_session_world_root` | the root whose scope is the active scope | 3 / 1 |
| `session_root_for_scope` | a named scope's root, through the disabling marker | 2 / 2 |
| `SoleLiveRoom<T>` | `Single<Ref<T>, With<RoomInstanceRoot>>`; one-live-room debt, not a session alias | 19 / 15 |
| `SoleLiveRoomSpec` | the authored spec of the one live room; same debt | 11 / 11 |

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

Do not begin by moving all 39 values. Work owner by owner:

1. Re-run `python3 scripts/architecture_census.py` and confirm the list.
2. For each family, state whether the value must exist before `SessionRoot`, only
   during a live session, or only for presentation.
3. For rollback-registered values, record the registration key and restore
   boundary before you change storage. A rollback key is a wire identity. Do not
   rename a key to match a type (`RoomTransitionCooldown` registers as
   `resource.sandbox_sim_state` on purpose).
4. Pick one coherent family with one owner. `SessionOwnedCheckpointState` is a
   good first review unit.
5. Choose storage from semantics: a `SessionRoot` component for live-session
   state; an explicit session-keyed coordinator when the value must exist before
   the root; an App resource only when process lifetime is real.
6. Delete the old reset and retirement compensation only after the new owner is
   the sole authority.
7. Keep direct Bevy queries and system parameters. Do not add a generic state
   container.

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

## 7. C07 — Explicit composition contracts for required optional authorities

**State:** OPEN. Startable.

### Ruling (2026-09-19, Q146/Q144)

There are two supported composition modes, direct and shell-hosted, and the game
is essentially the same in both. Capabilities stay optional and composable. When
authored production content requires a capability that the composition lacks,
the composition refuses that content or its admission. Reduced tools and tests
may omit capabilities explicitly. No anonymous App-global fallback state returns.

The ruling's "greyed-out return-to-shell row" has no composition to apply to:
every composition that shows the system menu is shell-hosted. Do not build a
disabled menu state until a shell-less composition exists.

### Current state

- `python3 scripts/architecture_census.py` reports 745 optional `Res`/`ResMut`
  occurrences over 200 type spellings (2026-10-02). It is a discovery index, not a
  defect list.
- Three high-authority cases already use a composition discriminator
  (`SessionGatedSimulation`): session scope, generation mechanics and content
  binding. The shell-routed side refuses.
- `ActiveSessionScope` is the largest single population of optional reads.
- Eleven session-owned members (C03's list) are read optionally in production:
  `ControlledSubject`, `AuthoredOccurrences`, `SessionMechanics`,
  `OccurrenceBaseline`, `CustodyBaseline`, `MintedItemBaseline`, `BaseGravity`,
  `ActiveConversation`, `StocksMatchSettled`, `PendingLifecycleCommit`,
  `AcceptedCheckpointRestore`. This is the sharpest entry population, because
  each `None` arm reads past a declared session owner.
- ✅ The fallback no `Option` scan could see is closed (2026-10-03):
  `insert_session_world_component` refuses in a session-gated composition
  with no root and no active scope. A direct host (no gate) builds its one root
  at the named `DIRECT_HOST_SESSION_SCOPE`. Measured first: the branch was
  reached only by ungated lib-test fixtures, never by `app_it` or the demo
  suites. Witness:
  `a_gated_composition_with_no_active_scope_refuses_to_mint_a_session_root`
  (poison: drop the assert).

### Work

Triage per call site, not per type. Ask what the `None` arm does: does it read
or write state that a second coexisting session could legitimately hold
differently? Many sites document their reason at the parameter; read that
comment first. (`BaseGravity` in the kaleidoscope menu is a legitimate optional
read: its `None` arm renders "n/a".)

**Acceptance:** each optional canonical authority has one documented reason for
absence, and a production profile cannot represent a live state without its
required owners.

**Risk:** medium. Making every resource required fights Bevy and removes
supported compositions. Leaving required state optional fails open.

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
