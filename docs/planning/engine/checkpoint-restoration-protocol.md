# Checkpoint restoration: admission, preparation and commit

**Scope:** restoration of one session's checkpoint: admission, pinned inputs,
preparation, commit, verification and the terminal outcome. It does not choose
multiplayer save ownership or define arbitrary ECS transactions.
**Packet:** A1 in the [packet catalog](actor-monolith-work-frontier.md)
(closed). **Priority:** [the queue](../queue.md).

## The invariant

**One accepted restore supplies the destination, construction continuity,
subject restoration and item and occurrence restoration for the same
operation. A refused request changes none of them.** Preparation reads the
selected checkpoint; it never installs it into live resources in order to read
it. Completion means the room and every participating domain passed
verification and the host published the resulting baseline.

Session lifecycle owns the operation from request to terminal outcome. A shrine
owns interaction, healing and the capture request only. Occurrence and item
owners own their snapshot data and reducers. Runtime composition orders domain
application; it does not implement it.

## Current shape

All code is in `crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs`
unless stated otherwise.

| Value | Role |
| --- | --- |
| `OutstandingCheckpointRequest` | the owed request; a reset asked while another intent owns the slot is re-asked until it is admitted |
| `CheckpointOperationKey` | session ownership stamp + a sequence minted by `SessionCheckpointOperations` that advances only on admission; rollback state; not reset at a rebase; refuses overflow before the slot is taken |
| `AcceptedCheckpointRestore` | the accepted operation: its room intent, subject and the occurrence, minted, custody and owned-item snapshots pinned at admission; outlives its frame; retired when the slot gives up the intent |
| `CheckpointDomainApply` | a schedule only a commit executor runs; the three domain reducers live here |
| `CheckpointRestoreInputs` / `ItemCheckpointRestoreInputs` | inputs installed for the duration of the apply and removed on every path; a reducer without them does nothing |
| `SessionCheckpointOutcomes` | exactly one terminal outcome per key; `RestoreFailure` is a closed set |
| `SessionStartupResume` | startup state machine: `Routed(key)` becomes `Satisfied` only when that operation publishes its outcome |

- The lifecycle slot (`session/lifecycle_commit.rs`) is the only admission
  arbiter across checkpoint, door and replay requests. It stays with session,
  not in `shared_tangle`.
- Room loading derives both the prefetch cache key and the fresh plan's
  `OccurrenceContinuity` from the accepted operation, matched by intent, so a
  door recorded while a restore is outstanding is still prepared from live
  state. A cached plan carries its `PrefetchIdentity`.
- Both executors apply by key: the eager `commit_ready_room_transition_system`
  path and the confirmed `commit_confirmed_lifecycle` tail (after the spawn
  drain, before the rebase).
- A room rebuild has no destructive phase: `RoomConstructionPlan::replace_live_world`
  builds every root as a hidden candidate, verifies the projected roster and
  publishes or drops. Custody, domain ledgers and session scope are applied by
  the domain reducers.
- A same-room startup placement is a small rollback-registered simulation
  operation, not an accepted restore.
- New Game (`NewGameRequested`) uses the same commit with `FreshRunRestore`
  installed: each domain's fresh-run reducer runs and the pinned values become
  the checkpoint. It outranks a reset asked on the same tick.

## Request policy

| Request | Target | Completion |
| --- | --- | --- |
| Startup, no checkpoint | explicit no-op | startup satisfied; no rebuild |
| Startup, checkpoint in the current room, no reconstruction needed | place the primary body once through transit, zero velocity | complete when placement is applied |
| Startup, checkpoint in another room | subject-bearing transition to the saved room | complete on the matching committed outcome |
| Startup, saved destination missing | diagnostic and skip | a terminal skipped reason |
| Reset, valid checkpoint | reconstitute from the checkpoint, even in the current room | all installed domains restore in one commit |
| Reset, no or invalid checkpoint | current-room authored start | an explicit start-state snapshot |
| Imported save needing reconstruction | adopt as candidate checkpoint data, then the reset road | parsing a save is not a live restore |
| Bodyless world profile | the bodyless reconstitution intent, only when the profile selects it | never manufacture a body |
| New Game | the start room's authored spawn with the fresh baseline | as reset, with `FreshRunRestore` |

Pin the checkpoint and source-room preconditions on admission. A waiting request
has not acquired an earlier capture; an admitted operation never adopts a later
capture or a newly controlled body. Once the subject resolves, keep its `SimId`.
The primary-avatar restore policy and healing of every interacting body are
preserved.

## State machine and failure rules

| State / event | Required | Forbidden |
| --- | --- | --- |
| Waiting, subject or save incomplete | keep the request; retry | mark complete, consume it, choose another body |
| Waiting, slot owned by another intent | keep both; no restore writes | restore ledgers or items before admission |
| Admitted | store immutable snapshots and intent under the key | retarget a load by editing its payload |
| Admitted, speculative rollback | restore request and slot from rollback | treat host readiness as confirmed admission |
| Admitted, preparation pending | wait with fixed inputs | swap live ledgers to prepare the room |
| Admitted, preparation invalid | terminal failure once; discard candidate; keep live state | publish a partial room or retry every frame |
| Admitted, scope, content or subject invalidated | cancel the matching operation | clear a newer slot or substitute a subject |
| Commit authorized, transient precondition missing | retry before destructive application | partially restore a domain and return Retry |
| Apply started, reducer or recipe violates contract | block gameplay; report operation and failing domain | claim the old world is intact |
| Verified | one committed outcome; retire pending state | publish before domain restore or rebase before verification |

The session coordinator is the only writer of request progress and outcome. The
slot is the only admission writer. Domains are the only writers of their live
data. A preparation failure is terminalized at a commit boundary on both hosts;
`Update` only leaves a host-side note, because `PendingLifecycleCommit` is
rollback state. The key is adopted when the transaction record is built, so
early preflight failures carry it.

## Commit ordering

```text
collect/resume request in the simulation
    -> lifecycle admission + immutable pinned checkpoint
    -> loading/preparation using selected continuity
    -> eager authorization OR matching confirmed authorization
    -> revalidate scope, content, subject and prepared plan
    -> apply domain state and canonical room reconstruction
    -> flush construction work; reconcile restored relations
    -> verify room + domain postconditions
    -> publish readiness / install frame-zero baseline
    -> terminal outcome
```

Only the prefix through admission runs in speculative simulation. Make restored
occurrence and accounting values visible before construction consumers need
them. Restore custody against stable subject and item identities after
structural work, then verify. Keep the body carryover road; never spawn a second
primary body. The rollback host rebases only after restore and verification, or
its first restore undoes the checkpoint.

**Verification** compares the applied world with the snapshots the operation was
accepted with, not live baselines. It checks ledgers, the bag, custody (naming
the custodian and refusing a duplicate identity) and population completeness:
every occurrence the pinned ledger places in the rebuilt room is live. Body
placement, clocks and portals are deliberately not checked: placement has no
postcondition from the transit authority, and clocks and portals have no
accepted snapshot.

**Replay consequences run at the commit, not at the admission.** The admission
pins its replay in the operation (`AcceptedRestore::replay`) and writes no
`RoomReplayAdmitted`. The room of a restore is built from
`CommitFactsSource::AfterTheRestore`: the facts at the commit, with the
consequences that construction reads applied to copies by the same functions
(`prospective_commit_fates`: `retract_defeat_records`, and the timer
consequences `forget_room` and `keep_only_owners`). The consequences run in the
`RestoreConsequences` schedule, from `verify_and_publish` after its verdict
accepts the room, while the candidates are hidden and before the old room is
retired. Every replay reader (spawn return, boss retraction, timers,
`WorldTimeSchedule`, `ConsumedSinceCheckpoint`, gravity, portals, cut-rope
arenas, pending hits) reads through one parameter, `AdmittedReplays`
(`ambition_combat::events`): the messages in the simulation, the pinned replay
in that schedule. `ResetToCheckpoint` and `RoomReplayAdmitted` are registered by
the host's `CheckpointHorizonPlugin`, not by a domain offer.

**One terminal rule.** An operation whose intent leaves the slot with no outcome
gets `Cancelled { NotCommitted }` from `retire_accepted_checkpoint_restore`, the
one place that a refused publication (before the room is built, or at the
verdict), a subject that is gone or cannot transit, and a retraction all reach.
A reset request with two primary bodies gets `Cancelled { AmbiguousSubject }`
and is spent; a startup resume in that world ends as a checkpoint in an unknown
room ends, because it admitted no operation.

## Acceptance matrix

Every row has a witness that fails for that row's property.

| Row | Witness |
| --- | --- |
| Busy slot with different live and saved ledgers | `a_refused_reset_changes_no_domain_state_and_is_not_lost` (the acquired object stays live and held) |
| Busy slot released in the same session | same test, second half; `a_refused_slot_leaves_the_checkpoint_resume_retryable` |
| Two reset requests in one tick | `two_reset_requests_in_one_tick_become_one_operation` |
| Missing primary during construction | `a_resume_with_no_constructed_subject_stays_pending_until_the_body_exists` |
| Control or capture changes after admission | `an_admitted_operation_keeps_its_subject_and_its_snapshot_while_it_waits` |
| Same room, different occurrence outlook | `a_checkpoint_outlook_refuses_a_plan_prepared_without_one`; `every_prefetched_plan_carries_an_empty_occurrence_outlook` |
| No checkpoint or invalid destination | `a_checkpoint_from_another_room_leaves_the_body_where_it_spawned` |
| Save adoption plus startup | `canonical_reconstitution::a_save_with_a_checkpoint_and_an_occurrence_lands_both` |
| A cancelled restore changes nothing (body, boss, purse, defeats) | `a_cancelled_restore_changes_nothing` (preparation failure; refused publication; refused verdict); control: a committed restore changes each fact |
| The prospect is what construction reads | `breakable_respawn_across_rooms`, `a_restored_room_builds_the_boss_the_restore_takes_back_alive` |
| Two primary bodies | `a_checkpoint_restore_with_two_primary_bodies_is_refused_and_the_next_one_commits`; `a_resume_with_two_primary_bodies_ends_and_does_not_ask_for_ever` |
| Prepare failure or cancellation | `a_failed_preparation_ends_the_operation_once_and_does_not_retry_it`, `a_failed_preparation_is_ended_by_the_confirmed_host_too`, `a_failed_ordinary_crossing_is_left_alone`, `a_startup_resume_whose_operation_is_retracted_asks_again` |
| Failure after destructive apply | `a_restore_that_fails_verification_blocks_gameplay_and_publishes_one_failure` |
| Held, thrown and minted carried items | the death suite; `custody_verification_names_the_custodian_and_refuses_a_duplicate` |
| Restore then frame-zero rollback | the `rollback_lifecycle_reset` suite |
| Eager and confirmed hosts | `a_confirmed_death_restores_the_entitlement_bag_the_checkpoint_banked` and the eager suite |
| Checkpoint-only composition | `a_checkpoint_only_composition_resumes_without_the_item_domain` |
| Domain reducers only through inputs | `the_commit_applies_the_operation_it_was_opened_for_and_always_removes_its_inputs`; `domain_restoration_is_registered_in_the_commit_schedule_and_not_in_the_simulation` |
| Teardown and re-entry | `a_key_from_a_retired_session_matches_nothing_in_the_next_one` |
| A row about another room | `a_reset_restores_a_whereabouts_row_about_a_room_it_is_not_rebuilding` |

Integration homes: `game/ambition_app/tests/canonical_reconstitution.rs`,
`death_restores_the_checkpoint.rs`, `carried_item_crosses_rooms.rs`. A zero-test
filter is not a pass.

## Open work

- Widen post-apply verification: checkpoint replay consequences, deferred
  flushes and the final rollback baseline.
- A subject that is gone or cannot transit has a unit witness only. No
  production road removes the primary body or its `MotionModel`, cluster or
  `BodyCombat` while its session lives, so a composed arm would remove it by
  hand.
- The terminal outcome has no presentation consumer. When one is needed, publish
  a message at completion; do not poll the single-latest outcome resource.
- `prefetch_neighbor_room_preparation_system`
  (`game/ambition_app/src/app/world_flow/room_transition_assets.rs`) braids a
  construction-plan cache with neighbor asset residency, so headless
  compositions hold no prefetched plans. Split the plan half from the asset
  half in its own packet.

## Forbidden regressions

- Nothing writes checkpoint-resume progress, or any consequence of a lifecycle
  request, before the slot says yes.
- No domain reads the raw `ResetToCheckpoint` request.
- No ordering edge stands in for admission: consumers need an answer, not a
  differently ordered read.
- No type-erased snapshot map, save-every-component protocol or restore-callback
  registry. No pinned snapshot in a shared vocabulary type.
- No mixed mode where some domains use the accepted operation and others act on
  raw requests.
- A cached plan is promoted only for the outlook it was prepared with; do not
  fill in the prefetch outlook to raise the hit rate.
- Do not repair a cancellation by reversing mutations afterwards: that is a
  second reconstruction authority.
- Do not build a restore's room from the live save: the prospect is the commit
  facts with the replay's consequences applied to copies.
