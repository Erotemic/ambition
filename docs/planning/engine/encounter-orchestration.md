# Unified encounter orchestration

**Priority:** P0 architecture refactor, immediately after the currently in-flight
test-organization work.

**Origin:** This design direction is Jon Crall's idea (2026-07-10): split the
encounter layer from bosses, compose encounters with ordinary enemies or other
world objects, and allow encounters with no boss at all. The unification and
migration details below develop that idea against the current code.

**Status:** DESIGN LOCKED; implementation not started.

## Thesis

An encounter is not a kind of actor and a boss is not a kind of encounter.

- An **actor** owns its body, controller, capabilities, health, actions, local
  phase/pattern state, and visual identity.
- An **encounter** is event-driven orchestration over actors, hazards, objectives,
  world gates, presentation requests, rewards, and persistence.
- A boss is an actor profile with unusually rich capabilities or local phase
  behavior. It may exist outside any encounter.
- An encounter may involve one boss, several bosses, ordinary enemies, no actors,
  or actors that already existed before the encounter began.

The common boss fight becomes composition, not a special constructor:

```text
spawn or adopt actor(s)
+ create encounter controller
+ assign participant roles
+ author objective(s)
+ author start/completion effects
```

This aligns with the repository's unified-actor rule: Player / Enemy / Boss / NPC
are data and controller choices, not separate engine paths.

## Why this is high priority

The repository currently carries two partially overlapping encounter systems:

1. `ambition_encounter`: a generic but wave-centric resource state machine with
   its own phase enum, registry, music request, reward path, and spawn commands.
2. `ambition_actors::boss_encounter`: actor-local boss phase machinery plus a
   second encounter entity, registry, event stream, music request, scripted beat
   VM, rewards, and automatic boss-to-encounter wrapping.

The code already states the right philosophy in places, but the runtime still
contains parallel authorities and boss-shaped assumptions. The result is harder
to compose, harder to snapshot, and larger than necessary.

Measured 2026-07-10:

- `crates/ambition_encounter/src`: 1,082 total source lines including tests.
- `crates/ambition_actors/src/boss_encounter`: 5,493 total source lines including
  tests, much of which is legitimately actor-local attack geometry, profiles,
  behavior, and sprites.
- The migration surface - generic encounter files, boss encounter/entity/script/
  event/registry/system files, and the actor phase module - is approximately
  3,681 total source lines including tests.

The final design must be net delete-heavy. Do not add a third abstraction over
both systems.

## The unified model

### 1. Actor-local state

Actors own only state that remains meaningful with no encounter present:

```text
Actor body
Controller (Human / Brain / RL)
Capabilities and action profile
Movement/body profile
Health/combat state
Optional local phase or pattern graph
Optional composite children / payloads
Visual profile
```

The current `BossPhaseState` is conceptually actor-local, not encounter state.
Its long-term name should reflect that (`ActorPhaseState` or `CombatPhaseState`),
and its phase identifiers should become authored data rather than a fixed
boss-only enum when the migration reaches that seam.

A phased actor outside an encounter must still wake, fight, transition, die, and
be possessable. It simply has no encounter HUD, arena lock, music ownership,
completion reward, or encounter persistence record.

### 2. One first-class encounter entity

The canonical live encounter is a Bevy entity. Pure reducer functions keep its
state headless-testable; a resource may index `EncounterId -> Entity`, but must
not duplicate live state.

Suggested components/data:

```text
EncounterId                    stable authored/runtime identity
EncounterLifecycle             Inactive / Starting / Active / Completed / Failed
EncounterDefinition            objectives, policies, presentation, persistence
EncounterTimeline              optional event-driven scripted beats
EncounterParticipants          stable member relations / roles
EncounterObjectiveState        derived progress and completion
EncounterOwned                 marks things spawned and lifetime-owned by it
```

Do not store raw `Entity` handles as the only durable participant identity.
Membership and snapshot state need stable `SimId`-level identity.

### 3. Participants are relations, not boss members

A participant relation records:

```text
encounter id
participant SimId
role
ownership policy (spawned / adopted)
lifetime policy
```

Roles are generic data, for example:

```text
PrimaryTarget
Elite
Minion
Hazard
Objective
Protected
Escort
Narrative
Rival
```

The encounter may spawn a participant from a content-owned recipe or adopt an
already-authored actor/feature. Adopted actors are not automatically despawned
when the encounter ends. Spawned actors follow the encounter's explicit cleanup
policy.

### 4. Activation is command/event driven

The encounter engine must not hard-code every possible game trigger. Content and
world adapters translate authored events into a small command seam:

```text
Start(encounter_id)
Fail(encounter_id, reason)
Complete(encounter_id)
Signal(encounter_id, key, payload)
AddParticipant(...)
RemoveParticipant(...)
```

Examples of adapters that may emit `Start` or `Signal`:

- LDtk region entered;
- switch activated;
- dialogue or quest event completed;
- timer expired;
- previous encounter completed;
- race checkpoint crossed;
- content-specific script event.

This keeps the engine generic while allowing game events to dictate encounters.

### 5. Objectives are generic predicates

The common objective vocabulary should cover:

```text
AllMembersWithRoleDefeated(role)
AnyMemberWithRoleDefeated(role)
Protect(member)
Survive(duration)
Reach(member, zone)
ReceiveSignal(key)
All([...])
Any([...])
```

A conventional boss fight uses `AllMembersWithRoleDefeated(PrimaryTarget)`. A
wave arena uses `AllMembersWithRoleDefeated(Minion)` plus spawn timeline beats. A
race uses checkpoint signals. A puzzle can have no actors and complete entirely
from signals.

Do not add a `Custom(String)` escape hatch that forces the generic runtime to
interpret game names. If content needs a new fact, content publishes a typed or
stable-key signal and the generic objective consumes that signal.

### 6. Effects and presentation are requests

Encounter transitions and timeline beats emit neutral requests:

```text
spawn participant recipe
lock/unlock authored exits
request/release music cue
request/release camera framing
show/hide encounter HUD
emit content signal
apply encounter-scoped actor modifier
grant reward
cleanup owned participants
```

The encounter runtime owns intent and ordering; host/presentation/content
adapters own concrete Bevy spawning, audio, camera, UI, and named content.

There must be one encounter music-intent stream with explicit priority/source,
not separate `EncounterMusicRequest` and `BossEncounterMusicRequest` resources.

### 7. Actor phase and encounter phase are independent

```text
Actor phase:
  this actor changes attacks or capabilities at 50% health

Encounter phase:
  doors lock, adds spawn, camera changes, objective changes
```

An actor may publish local phase events. An encounter may react to them, but the
encounter does not own the actor's combat phase machine. Likewise, the actor does
not directly own music, arena locks, rewards, or encounter completion.

## Canonical examples

### Boss outside an encounter

```text
actor profile: giant_gnu
local actor phase graph: enabled
encounter membership: none
```

It behaves as a difficult actor with no boss HUD or arena framing.

### Conventional boss fight

```text
trigger: enter arena
participants:
  existing giant_gnu -> PrimaryTarget (adopted)
objective:
  all PrimaryTarget defeated
start effects:
  lock exits, request boss cue, enable encounter HUD
completion effects:
  unlock exits, release cue, grant reward, persist cleared
```

### Multi-boss encounter

Two actors with the same profile have independent actor-local phase state and two
participant relations. The encounter objective observes both stable identities.

### Wave arena

The existing generic encounter waves become timeline beats that spawn ordinary
actor recipes with role `Minion`. No boss code participates.

### No-boss encounter

A Sanic race, chase, timed survival section, switch puzzle, escort, or defense
sequence uses signals/timers/objectives and may never create a boss-capable actor.

## Migration plan

Every slice replaces and deletes an old path in the same commit. Compatibility
facades are forbidden: pre-release, zero external dependents.

### E0 - baseline and executable contracts

- Record production/test LOC for the 3,681-line migration surface.
- Add headless contract scenarios for:
  - boss-capable actor with no encounter;
  - encounter with ordinary enemies only;
  - encounter with no actors;
  - encounter adopting an existing actor;
  - encounter spawning and owning participants;
  - two simultaneous encounters;
  - one encounter containing two same-profile actors.
- These are behavior contracts, not frozen implementation tests.

### E1 - canonical encounter entity and command seam

- Make the first-class encounter entity the sole live-state authority.
- Reduce `EncounterRegistry` to an index if an index remains necessary.
- Add stable encounter identity and a generic command/signal ingress.
- Migrate the current wave encounter lifecycle reducer onto components without
  changing wave behavior.
- Delete the old resource-owned live state as soon as parity is proven.

### E2 - generic participants and objectives

- Replace `EncounterDef.members: Vec<Entity>` and wave `alive_ids` with one stable
  participant model.
- Add participant roles and spawned/adopted ownership policy.
- Implement generic objective reduction.
- Migrate `AllMembersDead` and wave completion onto the objective vocabulary.
- Delete boss-specific progress/member structs once HUD/read models consume the
  generic progress projection.

### E3 - generic timeline/effects

- Move the reusable parts of `EncounterScript`, beats, gates, and effects into
  the encounter authority.
- Express wave spawning as timeline actions rather than a special parallel
  state machine.
- Keep content-specific recipe resolution outside the generic crate.
- Remove duplicate spawn/reward/event plumbing.

### E4 - boss composition migration

- Stop auto-creating encounters from active bosses. Delete
  `sync_boss_encounter_entities`.
- Author boss fights as encounter definitions that spawn or adopt actors.
- Route boss HUD, lock walls, music, rewards, cutscenes, and persistence through
  the generic encounter entity.
- Keep boss attack geometry, actor behavior profiles, sprites, and local phase
  state actor-owned.
- Delete `BossEncounterRegistry`, boss-specific encounter event publication, and
  the second music request.

### E5 - generalize actor-local phase vocabulary

- Rename the actor-local phase state away from encounter ownership.
- Replace fixed boss-only phase identity with authored phase keys/data where the
  current behavior requires it.
- Preserve the existing trigger grammar (HP, time, external signal) through the
  generic actor phase graph.
- The encounter may subscribe to phase signals but never becomes phase authority.

### E6 - persistence, snapshot, and presentation convergence

- Snapshot one encounter authority and stable participant relations.
- Persist encounter placement/result separately from actor archetype or actor
  death policy.
- Publish one encounter presentation read model for HUD/camera/audio.
- Ensure concurrent encounters resolve presentation intent deterministically by
  explicit priority, not update order.

### E7 - deletion and LOC audit

Delete the superseded paths and re-measure.

Required deletion targets include, subject to symbol movement:

```text
sync_boss_encounter_entities
BossEncounterMusicRequest
BossEncounterRegistry
parallel boss-only encounter events/progress structs
resource-owned duplicate encounter live state
wave-only spawn/completion path once timeline/objectives replace it
```

## LOC acceptance

The implementation is not complete merely because both paths call a shared
helper. Completion means one path is gone.

- Combined migration-surface LOC must be lower than the 3,681-line baseline.
- Target: at least 800 total source lines removed from that surface, with a
  stretch target of 1,200+.
- No new facade crate or compatibility adapter may count as progress.
- Generic tests may grow where they replace duplicated boss/wave tests, but total
  production LOC must fall.
- Report deleted symbols and files, not only moved lines.

If a slice adds more code than it deletes, it must explain which later slice is
already able to delete the temporary increase. No slice may add a third runtime
state authority.

## Acceptance criteria

The refactor is complete when all are true:

1. A boss-capable actor works outside an encounter.
2. A generic encounter can use ordinary enemies and no boss machinery.
3. A generic encounter can complete with no actors at all.
4. Encounters can spawn or adopt participants with explicit cleanup ownership.
5. Two simultaneous encounters and two same-profile actors remain independent.
6. Actor-local phases work with or without encounter membership.
7. Boss fights, waves, races/chases, puzzles, and scripted set pieces use one
   encounter lifecycle/objective/timeline authority.
8. HUD, camera, music, locks, rewards, and persistence consume generic encounter
   intent/read models.
9. Snapshot/restore has one encounter state representation.
10. The old boss-specific encounter wrapper and parallel wave state authority are
    deleted.
11. The migration surface is materially smaller than the measured baseline.

## Sanic as a first customer

Sanic should eventually prove two non-boss encounter shapes:

- an end-of-act race/chase or timed objective driven by checkpoints/signals;
- the act-3 mini-boss authored as an ordinary actor profile composed into a
  generic encounter.

Do not block the immediate Sanic input/character-presentation recovery on this
refactor. The recovery plan is [`../demos/sanic-recovery.md`](../demos/sanic-recovery.md).
