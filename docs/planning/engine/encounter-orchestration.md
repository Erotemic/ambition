# Unified encounter orchestration

**Priority:** active architecture convergence.

**Status:** **PARTIAL.** Shared vocabulary landed, but HEAD does not yet have one
encounter lifecycle, one component schema, one command ingress, or one snapshot
representation.

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

### Two live schemas remain

**Wave encounter entity**

```text
Encounter
EncounterState
EncounterParticipants
```

The wave reducer in `crates/ambition_encounter/src/state.rs` owns trigger polling,
intro/wave timing, failure, clearing, lock state, and persistence projection.

**Boss encounter wrapper**

```text
EncounterDef
EncounterParticipants
EncounterObjective
EncounterProgress
optional EncounterScript
```

`crates/ambition_actors/src/boss_encounter/encounter_entity.rs` auto-wraps active
bosses and projects actor-local boss state into a HUD/progress model.

Shared component names do not make these one lifecycle authority.

## Missing semantics

### 1. Generic command ingress

There is no generic encounter command/message seam for:

```text
Start
Fail
Complete
Signal
AddParticipant
RemoveParticipant
```

Wave activation still polls a player/trigger intersection. Boss wrappers still
appear from boss-specific state. Content cannot start or signal an encounter
through one generic API.

### 2. Objective-driven lifecycle

`objective_met` is a useful pure predicate. It is not yet the lifecycle reducer.

- Wave encounters do not carry `EncounterObjective`.
- The boss progress projection passes zero elapsed time and no signals.
- `Objective::Survive` and `Objective::ReceiveSignal` therefore have no complete
  live encounter path.
- `EncounterObjective.fail` has no generic lifecycle consumer.

A pure predicate test is not proof of a no-actor signal/timer encounter.

### 3. Ownership-driven cleanup

`Ownership::{Spawned, Adopted}` is stored in participant records, but runtime
cleanup does not branch on it. Wave cleanup scans wave-specific mob markers;
boss wrappers disappear independently of the ownership field. There is also no
explicit lifetime policy beyond the ownership enum.

### 4. Snapshot-stable identity

Encounter participant durable identity is currently a `String` plus an optional
ECS `Entity`, not a snapshot-registered stable relation. The snapshot registry
does not register the wave or boss encounter schemas. Entity-local storage alone
is not a snapshot representation.

### 5. Consumer convergence

Camera/music have shared read-model work, but HUD, locks, rewards, and persistence
still consume wave- or boss-specific state. The architecture is not complete
until consumers can depend on generic intent where their semantics are generic.

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

### E8 — canonical lifecycle and command seam

Add a generic reducer and ingress for `Start`, `Fail`, `Complete`, and `Signal`.
Adapt the wave trigger and boss auto-wrap paths to commands rather than letting
them remain lifecycle owners.

**Exit:** a reducer test drives inactive → active → completed/failed without boss
or wave-specific code.

### E9 — objective integration

Store elapsed time and received signals on the generic authority. Evaluate win
and fail objectives as part of the lifecycle reducer. Adapt ordinary wave
completion to the same objective path.

**Exit:** headless tests prove:

- all minions defeated;
- survive timer with no actors;
- receive signal with no actors;
- protected participant death causes failure.

### E10 — ownership and lifetime semantics

Make cleanup explicitly consult ownership/lifetime policy. Adopted participants
must survive encounter retirement unless a separately authored policy says
otherwise; spawned participants must follow the selected cleanup rule.

**Exit:** poison tests fail if adopted actors are despawned or spawned-owned
actors leak under a cleanup policy.

### E11 — stable identity and snapshot registration

Replace string-only durable relations with the repository's stable simulation
identity model, register the encounter authority needed by rollback, and prove
entity handles can be re-resolved after restore.

**Exit:** snapshot/restore of an active encounter preserves lifecycle, objective
progress, signals, and participant relations.

### E12 — generic consumer convergence

Move HUD, locks, rewards, persistence, camera, and music to generic encounter
intent where appropriate. Keep actor-local phase presentation actor-local.

**Exit:** no generic consumer needs to ask whether an encounter is a boss or a
wave in order to perform the same semantic action.

### E13 — non-boss acceptance customer

Use the generic path for a race, puzzle, defense, or timed section. Sanic's race
or a signal-driven no-actor puzzle is suitable.

**Exit:** the customer adds content/rules without adding another lifecycle,
objective evaluator, cleanup path, or presentation authority.

## Acceptance ledger

| Criterion | HEAD grade |
|---|---|
| Boss actor works outside an encounter | **YES** |
| Ordinary-enemy wave exists without boss machinery | **YES** |
| No-actor signal/timer encounter uses live generic lifecycle | **NO** |
| Spawned/adopted cleanup is controlled by ownership policy | **NO** |
| Actor-local phase remains independent | **YES** |
| One lifecycle/objective/timeline authority | **NO** |
| Generic presentation/reward/persistence intent | **PARTIAL** |
| One snapshot representation with stable participant relations | **NO** |
| First non-boss customer proves reuse | **NO** |

The architecture is complete only when the missing criteria are demonstrated by
code and tests at HEAD. Line-count reduction is useful evidence of deleted
parallel authority, but it is not a substitute for the semantic criteria above.
