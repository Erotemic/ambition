# Unified encounter orchestration

**Priority:** active architecture convergence.

**Status:** **E8–E12 LANDED (2026-07-16).** HEAD has ONE encounter lifecycle
authority (`EncounterLifecycle` + the pure reducer), one generic command
ingress (`EncounterCommand`), objective-driven completion,
ownership/policy-driven cleanup, a snapshot-registered authority with stable
participant relations, and generic consumers over lifecycle + staging policy.
Remaining: the non-boss acceptance customer (E13).

The detailed E0–E7 execution report is preserved at
[`docs/archive/reviews/planning-history-2026-07-11/encounter-orchestration-e0-e7-report.md`](../../archive/reviews/planning-history-2026-07-11/encounter-orchestration-e0-e7-report.md).
That report is historical evidence, not the current completion grade.

## Thesis

An encounter is orchestration, not an actor type.

- Actors own bodies, controllers, capabilities, health, actions, local phase
  state, and visual identity.
- Encounters coordinate participants, objectives, timeline effects, locks,
  presentation requests, rewards, cleanup, and persistence.
- A boss can exist and fight outside an encounter.
- An encounter can contain bosses, ordinary enemies, hazards, protected actors,
  or no actors at all.

A conventional boss fight should be composition:

```text
actors/features
+ participant relations
+ lifecycle
+ objective
+ optional timeline
+ transition effects
```

## What HEAD actually has

### Landed foundation

- `BossPhaseState` was correctly renamed to actor-local `ActorPhaseState`.
- `ambition_encounter` provides shared:
  - `EncounterParticipants`, roles, and `Ownership`;
  - `Objective` and the pure `objective_met` predicate;
  - `EncounterScript`, triggers, gates, beats, and effects;
  - one prioritized `EncounterMusicRequest` vocabulary.
- Wave live state moved from a resource-owned map onto encounter entities.
- Wave mobs use `EncounterParticipants` instead of a parallel `alive_ids` list.
- Boss wrappers use the shared participant and objective vocabulary.
- A boss with `no_encounter` can remain an ordinary actor with no encounter HUD.

### One lifecycle authority (E8/E9, landed 2026-07-16)

Every encounter entity — wave arena AND boss wrap — carries the same generic
authority set:

```text
Encounter                      (identity)
EncounterLifecycle             (phase / elapsed_active / signals — THE authority)
EncounterParticipants          (relations)
EncounterObjective             (win / optional fail)
optional EncounterCleanupPolicy (E10)
optional EncounterWaves        (wave policy: spawn cadence only)
optional EncounterDef/Progress (boss HUD projection)
optional EncounterScript       (timeline)
```

`ambition_encounter::lifecycle` owns the pure reducer
(`EncounterLifecycle::reduce`) and its one ECS registration
(`reduce_encounter_lifecycles`, in the public `EncounterLifecycleSet` the
runtime positions in Progression). The old wave state machine
(`EncounterState`) is deleted; `EncounterWaves` keeps only spawn cadence and
publishes `WAVES_EXHAUSTED_SIGNAL` through the command ingress — wave
completion is the generic objective `All([ReceiveSignal(waves_exhausted),
AllWithRoleDefeated(Minion)])`. The boss wrap starts its lifecycle through the
same ingress and completes through `AllWithRoleDefeated(PrimaryTarget)`.

## Missing semantics

### 1. Generic command ingress — ✅ LANDED (E8)

`EncounterCommand { encounter, kind: Start | Complete | Fail | Signal(key) |
Reset }` is the one ingress; the reducer is its only consumer. The wave
trigger adapter, the death/area-exit/re-arm paths, and the boss auto-wrap all
emit commands — no adapter writes the phase. (`AddParticipant` /
`RemoveParticipant` stayed component-level: participant bookkeeping belongs to
the adapter that owns the members; a cross-crate mutation command has no
consumer yet.)

### 2. Objective-driven lifecycle — ✅ LANDED (E9)

The reducer evaluates fail-before-win objectives over participants,
`elapsed_active`, and received `signals` (a `BTreeSet`, determinism contract)
every Active tick. Waves complete through the signal+minions objective; the
boss wrap through `AllWithRoleDefeated(PrimaryTarget)`. Survive / ReceiveSignal
/ protected-death-fails all have headless exit tests against the live reducer.

### 3. Ownership-driven cleanup — ✅ LANDED (E10)

`apply_encounter_cleanup` reacts to end events (Completed/Failed/Reset) and
consults each participant's `Ownership` plus the optional
`EncounterCleanupPolicy { spawned: DespawnOnEnd | Keep }`. Adopted participants
are never touched; spawned participants despawn (records leave the relation
list) unless the policy keeps them. The wave-specific mob marker scan
(`despawn_encounter_mobs`) is deleted — all three former call sites (death,
completion, re-arm) converge on the one ownership-driven adapter. Deliberate
behavior change: abandoning an arena (area exit) now also despawns its spawned
mobs — the pre-E10 lingering was accidental, not policy.

### 4. Snapshot-stable identity — ✅ LANDED (E11)

Every encounter authority carries `SimId::encounter(id)` (its own namespace —
a boss WRAP and the boss BODY share the raw id but are two roster rows). The
registry registers `EncounterLifecycle` + `EncounterParticipants` as plain
state and `EncounterWaves` as a RESOLVED codec (the blob stores the live run;
the authored spec resolves from the surviving component). Participant `Entity`
handles are never serialized: the durable identity is the id string, and the
adapters re-resolve — including healing a restore-nulled cache by id
(`update_encounter_progress` / the wave liveness refresh), and the boss wrap's
coverage check matches by id as well as entity so a restore never double-wraps.
`EncounterProgress` is declared derived; `Encounter` / `EncounterObjective` /
`EncounterDef` are reviewed authored-config debt
(`known_component_debt.txt`). The command/event channels are registered (a
pending Start replayed after a restore would double-apply).

### 5. Consumer convergence — ✅ LANDED (E12)

Generic consumers now read the LIFECYCLE plus authored STAGING policy
components (`EncounterLockWall` / `EncounterCameraZoom` / `EncounterTrack`,
installed from the spec at populate): the lock-wall contributor, the
`EncounterView` camera read-model, and the base-tier music request no longer
name `EncounterWaves` — any encounter kind stages alike. The HUD's encounter
status line reads the generic lifecycle (wave text is optional flavor); the
member-HP line stays keyed on `EncounterDef` (that component IS the
HUD-binding policy). Deliberately NOT converged, with rationale:

- **Boss persistence + quest events** stay on the boss phase machine
  (`save.bosses`, keyed by placement, written at death-OUTRO completion).
  The outro gating is actor-local death presentation — moving the write to
  the lifecycle's `Completed` (HP-zero) would shift quest/banter sequencing
  earlier by the outro length, a blind behavioral change with no architectural
  win. The wave save projection (`save.encounters`) rides the generic
  lifecycle already.
- **Reward chests** keep two adapters (trigger-floor chest from `spec.reward`
  vs boss-anchor chest from the profile): both react to their completion
  facts, but anchor derivation and payload resolution are genuinely different
  authored policies, not a lifecycle fork. A shared reward-intent channel is
  warranted when a third reward shape lands.

## Target model

Use one first-class encounter entity with a pure, headless-testable reducer. The
exact component split may evolve, but there must be one lifecycle/read model:

```text
EncounterId / stable SimId relation
EncounterLifecycle
EncounterDefinition or policy
EncounterParticipants
EncounterObjectiveState
optional EncounterTimeline
received signals / elapsed time
```

Participant relations need:

```text
stable participant identity
role
spawned/adopted ownership
explicit cleanup/lifetime policy
live resolution as a cache, not the durable identity
```

Transitions emit neutral intent; adapters perform concrete effects:

```text
spawn participant recipe
lock/unlock exit
request/release music
request/release camera framing
show/hide HUD
emit content signal
apply encounter-scoped modifier
grant reward
cleanup according to ownership policy
```

Do not add `Custom(String)` behavior interpreted by the engine. Content publishes
stable signals; generic objectives consume them.

## Ordered patches

These are the encounter track's executable subtasks; E8–E12 are **DONE**
(2026-07-16); E13 remains open.

### E8 — canonical lifecycle and command seam — ✅ DONE (commit 25c12870a)

**Exit met:** `commands_drive_inactive_to_active_to_completed_or_failed`
(`ambition_encounter::lifecycle::tests`) drives the reducer through both
terminal paths with no boss or wave code in sight.

### E9 — objective integration — ✅ DONE (commit 25c12870a)

**Exit met:** the four headless exit tests live in
`ambition_encounter::lifecycle::tests` against the live reducer:
`all_minions_defeated_completes_the_objective`,
`survive_timer_completes_with_no_actors`,
`receive_signal_completes_with_no_actors`,
`protected_participant_death_fails_the_encounter` (fail evaluates before win).
Wave completion converges on the same path (`waves::tests`).

### E10 — ownership and lifetime semantics — ✅ DONE

**Exit met:** `encounter::tests::cleanup` runs the real reducer + cleanup
adapter chained: `end_despawns_spawned_participants_and_never_adopted_ones`
fails if an adopted actor is despawned OR a spawned-owned actor leaks;
`reset_applies_the_same_ownership_rule`;
`keep_policy_leaves_spawned_participants_in_the_world` proves the authored
policy is consulted, not just the enum.

### E11 — stable identity and snapshot registration — ✅ DONE

**Exit met:** `ambition_runtime::snapshot::tests::restore_preserves_an_active_encounter`
takes a mid-fight authority (Active phase, elapsed time, two signals, a
dead-but-retained spawned relation + an adopted one, a mid-wave run), wrecks
it, restores it, and asserts every field — with `entity: None` proving handles
are re-resolved, never serialized. The desync canary's restore-replay oracle
caught (and now pins) the real bug this surfaced: a restored wrap whose
participant caches were nulled read its boss as dead and replayed into a
different future until resolution healed by id.

### E12 — generic consumer convergence — ✅ DONE

**Exit met:** locks, camera, and base music derive from the lifecycle +
staging components (compile-enforced: the consumers no longer name
`EncounterWaves`); `a_non_wave_encounter_stages_the_same_lock_and_zoom` pins
it behaviorally. Actor-local phase presentation (boss adaptive music, phase
feedback, member HP rows, death-outro-gated persistence) stays actor-local —
see the §5 rationale for what deliberately did not move.

### E13 — non-boss acceptance customer

Use the generic path for a race, puzzle, defense, or timed section. Sanic's race
or a signal-driven no-actor puzzle is suitable.

**Exit:** the customer adds content/rules without adding another lifecycle,
objective evaluator, cleanup path, or presentation authority.

## Acceptance ledger

Grades are acceptance-criterion satisfaction (SATISFIED / UNSATISFIED / PARTIAL);
every UNSATISFIED or PARTIAL criterion maps to an OPEN patch below.

| Criterion | HEAD grade | Open subtask |
|---|---|---|
| Boss actor works outside an encounter | **SATISFIED** | — |
| Ordinary-enemy wave exists without boss machinery | **SATISFIED** | — |
| No-actor signal/timer encounter uses live generic lifecycle | **SATISFIED** (engine bar: live reducer + exit tests; the shipping customer is E13) | — |
| Spawned/adopted cleanup is controlled by ownership policy | **SATISFIED** | — |
| Actor-local phase remains independent | **SATISFIED** | — |
| One lifecycle/objective/timeline authority | **SATISFIED** | — |
| Generic presentation/reward/persistence intent | **SATISFIED** (staging generic; boss outro persistence + reward anchors recorded as actor-local/authored policy) | — |
| One snapshot representation with stable participant relations | **SATISFIED** | — |
| First non-boss customer proves reuse | **UNSATISFIED** | E13 |

The architecture is complete only when the missing criteria are demonstrated by
code and tests at HEAD. Line-count reduction is useful evidence of deleted
parallel authority, but it is not a substitute for the semantic criteria above.
