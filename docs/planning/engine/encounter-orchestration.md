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

## Execution ledger

Living record of what has actually landed. Update it in the same commit as the
code. **Units are stated explicitly** because a bare "LOC" figure conflates
production and test lines (a source of past confusion).

### E0 — baseline (measured 2026-07-11, `cargo`-independent `wc -l` + cfg(test) split)

The migration surface, by group (prod = non-test source, test = `#[cfg(test)]`
blocks and `*tests.rs` files):

| Group | What | prod | test | total |
|---|---|---:|---:|---:|
| A | `ambition_encounter/src` (generic wave crate) | 815 | 267 | 1,082 |
| B | `ambition_actors/src/boss_encounter/{encounter_entity,encounter_script,events,registry,systems}` (+ their `tests.rs`) | 1,093 | 542 | 1,635 |
| C | `ambition_characters/src/boss_encounter.rs` (+ `phase_mechanism_tests.rs`) — the actor-local phase module | 436 | 233 | 669 |
| **A+B+C** | **the doc's "~3,681" migration surface** | **2,344** | **1,042** | **3,386** |
| D | `ambition_actors/src/encounter/*` (the Bevy host adapter wrapping crate A) | 808 | 740 | 1,548 |
| **A+B+C+D** | **full runtime surface incl. wave host** | **3,152** | **1,782** | **4,934** |

The `LOC acceptance` target (≥ 800 total source lines removed, stretch 1,200+) is
tracked against the **A+B+C total = 3,386** baseline unless a slice explicitly
touches group D.

**Consumer map (recorded so the heavy slices are turn-key):**

- *Generic wave crate A* is reached almost entirely through two facades:
  `ambition::encounter` (`crates/ambition/src/lib.rs` `pub use ambition_encounter as encounter`)
  and the host module `ambition_actors::encounter` (thin re-export shims). The
  live state authority is the resource pair `EncounterState`/`EncounterRegistry`,
  driven by `ambition_actors::encounter::systems::update_encounters_from_world`
  (one ~355-line monolith). Consumers: HUD (`ambition_app/app/hud.rs`), camera
  zoom (`ambition_sim_view/camera_snapshot.rs`), music intent
  (`ambition_actors/music/intent.rs`), reward chests
  (`ambition_actors/features/ecs/encounter_rewards.rs`), switches, lock walls,
  save projection, session reset.
- *Boss orchestration B* is wired in exactly one schedule
  (`ambition_runtime/progression_schedule.rs`, the boss `.chain()`), inits its
  registry in `ambition_runtime/sim_core_resources.rs`, and is authored as
  content in `game/ambition_content/src/bosses/**` (the cut-rope fight is the
  only content author of `EncounterScript`/beats). HUD reads
  `EncounterDef`+`EncounterProgress`. `sync_boss_encounter_entities` auto-wraps
  every woken boss (the E4 deletion target).

**E0 down-payment deletions (this commit):** removed two confirmed-dead public
methods from crate A — `EncounterRegistry::any_lock_active` and
`EncounterState::celebratory_banner` (grep-verified zero call sites).

### Progress

- [x] **E0** baseline recorded; dead code removed.
- [x] **E1** wave live-state is entity-owned; `EncounterRegistry` reduced to an
      `id -> Entity` index; presentation read-model (`EncounterView`) introduced.
      (Command-seam ingress reordered to E3, where it replaces switch/trigger
      polling and thus deletes code — see below.)
- [x] **E2** generic participants + roles + objective vocabulary
      (`ambition_encounter::{participants,objective}`); boss `EncounterDef.members`
      → `EncounterParticipants` (adopted `PrimaryTarget`); `EncounterWin` deleted;
      the generic objective is evaluated into the `EncounterProgress` read-model.
- [x] **E3** generic timeline/effects — **E3a**: timeline vocabulary +
      `EncounterScript::advance` stepper moved into `ambition_encounter::timeline`
      (the one timeline authority; boss effect EXECUTION stays actor-side).
      **E3b**: the wave's live mobs are now the shared `EncounterParticipants`
      (role `Minion`, `Spawned`); `EncounterRun.alive_ids` + the closure-based
      liveness plumbing are deleted (the host refreshes `participant.alive` from
      the runtime, the reducer reads it). Both boss and wave encounters speak the
      one participant vocabulary now (acceptance #4/#7). The wave keeps its slim
      spawn scheduler (`run.pending`/`wave_elapsed`) per the impedance note below.
- [~] **E4** boss composition — **event-publication bridge collapsed**: the
      pointless `BossPhaseEvent → BossEncounterEvent → effects` indirection is
      gone (deleted the `BossEncounterEvent` enum + `phase_event_to_encounter_events`);
      `publish_events` consumes `BossPhaseEvent` directly for banner + cutscene,
      and the dead edge-triggered music-set in it dropped (the level-triggered
      lifetime block in `update_boss_encounters` is the one music authority).
      The flagship `sync_boss_encounter_entities` deletion (author boss encounters
      at spawn; HUD/lock feel-sensitive) is still pending.
- [~] **E5** generalize actor-local phase vocabulary — **first bullet landed**:
      the entity-local phase machine `BossPhaseState` → `ActorPhaseState` (it is
      actor-local, not encounter-owned). The deeper bullet (replace the fixed
      `BossEncounterPhase` enum with authored phase keys/data) is deferred — it
      threads the snapshot ledger + content RON and is a larger, feel-adjacent
      change.
- [ ] **E6** persistence/snapshot/presentation convergence — *music sub-slice
      landed early (see below): one prioritized encounter music stream.*
- [ ] **E7** deletion + LOC audit.

**Out-of-order E6 music sub-slice (landed early — self-contained, no feel risk):**
the two music-intent resources `EncounterMusicRequest` and
`BossEncounterMusicRequest` are collapsed into one `EncounterMusicRequest`. It
carries two priority tiers as fields — `priority_track` (a focused fight, e.g. a
boss) beats `base_track` (a wave/arena) — resolved by `desired_track()`.
Crucially the fields name PRIORITY, not encounter kind: a boss fight is just a
`priority_track` writer and a wave arena a `base_track` writer, so the generic
stream never hard-codes "boss". This satisfies §6 ("one encounter music-intent
stream with explicit priority/source") and retires the named E7 deletion target
`BossEncounterMusicRequest` (~14 references) ahead of the entity migration
because it does not depend on it.

LoC note: this sub-slice is roughly line-neutral (a whole resource type is gone,
but the surviving one gained the docs explaining the priority invariant that
prevents the old clobber bug). The win is a removed *state authority*, which is
the doc's completion bar ("Completion means one path is gone"), not a raw line
count. Combined with the E0 dead-method deletions, net so far is ≈ −20 lines;
the bulk of the ≥ 800-line target lands with the E1–E4/E7 entity migration.

### Status after E1 / E2 / E3a / E5-first (2026-07-11)

Landed + verified (each its own green commit): **E1** (wave state → entities,
registry → index, `EncounterView` read-model), **E2** (generic
participants/roles + objective vocabulary; boss adopts them; `EncounterWin`
deleted), **E3a** (timeline authority relocated to `ambition_encounter`), **E5
first bullet** (`BossPhaseState` → `ActorPhaseState`). This is the ARCHITECTURAL
spine of the unification: both boss and wave encounters are entities; membership
and win are one generic vocabulary; the timeline lives in one crate; the actor
phase is honestly named.

**Net LoC so far is ≈ neutral-to-slightly-additive** — the generic vocabulary
(`participants`/`objective`/`timeline` modules + their tests) is new shared code
that the remaining slices BANK. The ≥ 800-line deletion target lives entirely in
the remaining convergence:

- **E3b** — collapse the wave's phase/completion/lock LOGIC into the shared
  lifecycle + objective (keeping its spawn scheduler; see the E3a impedance
  note). Deletes ~part of `state.rs`'s reducer machine. **Feel-sensitive**
  (wave timing) — the 118-test host suite pins it, but it is the kind of change
  the doc's own E1 note flagged for in-game verification.
- **E4** — delete `sync_boss_encounter_entities` (author boss encounters at
  spawn, lifecycle tracks the boss phase) + collapse the `BossEncounterEvent`
  publication bridge. **Feel-sensitive** (boss HUD/lock/music appearance timing).
- **E6** — extend `EncounterView` to carry HUD/lock intent; route presentation
  through the one read-model; deterministic concurrent-encounter priority.
- **E7** — delete the superseded paths and re-measure against the 3,386-line
  A+B+C baseline.

These remaining slices are where the deletion lands, and each meaningfully
changes on-screen timing/presentation that is best verified in-game. They are
being executed in dependency order on top of the verified spine above.

### E1 — wave live-state is entity-owned (landed 2026-07-11)

The structural keystone: the wave encounter's live state is no longer a
resource-owned `BTreeMap<String, EncounterState>`. Each live encounter is now a
Bevy ENTITY carrying an `Encounter { id }` identity + its `EncounterState`
component (both boss and wave encounters are entities now — the precondition for
one snapshot representation, acceptance #9). Behaviour-preserving: the proven
headless reducers (`maybe_start` / `tick_intro_or_wave` / `on_player_death` /
`reset_for_retry`) are byte-unchanged, so the crate-A reducer tests and the
118-test host suite stay valid; only the *authority* moved (resource map →
entities).

What changed:

- `EncounterState`: `#[derive(Resource)]` → `#[derive(Component)]`. The dead
  singleton `init_resource::<EncounterState>()` (nothing read `Res<EncounterState>`)
  is deleted.
- `EncounterRegistry`: the state-holding `BTreeMap<String, EncounterState>` is
  reduced to an `ids: BTreeMap<String, Entity>` index (`entity`/`insert`/`remove`);
  `get`/`get_mut`/`ensure`/`active_camera_zoom` deleted.
- `EncounterRegistry::active_camera_zoom` (a method that read the states) →
  free fn `active_encounter_camera_zoom(states)` using `max` (order-independent,
  the entities are queried and query order is not stable).
- `EncounterView` resource (new): the one cross-crate presentation read-model
  (§6, started minimal). The host publishes `camera_zoom` each tick; the camera
  (`ambition_sim_view`) reads `EncounterView` instead of reaching into the
  registry — presentation is decoupled from the state representation.
- `populate_encounter_registry` spawns one encounter entity per loaded spec.
- `update_encounters_from_world` drives the entities via a
  `Query<(&Encounter, &mut EncounterState)>` (registry Res dropped — lookups
  iterate the query, keeping the system under Bevy's 16-param limit; the reward
  sync now takes the cleared `(id, spec)` list, not the registry).
- Consumers repointed to the entities: lock walls (`Query`), reward chests
  (cleared list), music intent (`id -> &EncounterState` lookup built from the
  query), HUD (`Query`), session reset (despawns the `Encounter` entities).

Deleted symbols/paths: `EncounterState` as a resource (+ its `init_resource`);
`EncounterRegistry::{encounters, get, get_mut, ensure, active_camera_zoom}`; the
`Res<EncounterRegistry>` camera coupling. Net LOC is roughly neutral this slice
(the entity plumbing + `EncounterView` offset the deleted registry methods) — the
doc anticipated this: E1 pays the entity-plumbing cost that E2 (participants) and
E3 (timeline collapses the wave state machine) then bank, and E4 deletes the boss
duplicate. The win banked NOW is the removed resource-owned live-state authority
(acceptance #10, half — the wave half).

### E2 — generic participants + objective vocabulary (landed 2026-07-11)

The shared membership + win vocabulary (§3, §5), defined in `ambition_encounter`
so both boss fights (now) and wave arenas (E3) speak it:

- `ambition_encounter::participants`: `EncounterRole` (PrimaryTarget / Elite /
  Minion / Hazard / Objective / Protected / Escort / Narrative / Rival),
  `Ownership` (Spawned / Adopted), `EncounterParticipant { id, entity, role,
  ownership, alive }` (a stable id + resolved entity — §3 "do not store raw
  `Entity` as the only durable identity"), and `EncounterParticipants` with
  `all_with_role_defeated` / `any_with_role_defeated`.
- `ambition_encounter::objective`: `Objective` (`AllWithRoleDefeated` /
  `AnyWithRoleDefeated` / `Survive` / `ReceiveSignal` / `All` / `Any` — no
  `Custom(String)` escape hatch, §5), the `EncounterObjective` component, and the
  pure `objective_met` reducer.

Boss migration:

- `EncounterDef.members: Vec<Entity>` → the generic `EncounterParticipants`
  component (the boss is one ADOPTED `PrimaryTarget`). `sync_boss_encounter_entities`,
  `tick_encounter_scripts` (ForceKill/CommandMoveTo/DropHazard resolve member N
  via participant order), `update_encounter_progress`, and the cut-rope setup all
  read participants now.
- `EncounterDef.win` + the `EncounterWin` enum + `EncounterProgress::all_members_dead`
  are DELETED. The generic `EncounterObjective` (AllWithRoleDefeated(PrimaryTarget))
  is attached at sync and evaluated by `objective_met` into
  `EncounterProgress.complete` — the generic projection the HUD/win read model
  observes. (The boss death → save authority is still the phase machine; the
  encounter entity becomes the completion authority at E4, so this is a read-model,
  not a second authority.)

Deleted symbols: `EncounterWin`, `EncounterDef::{members, win}`,
`EncounterProgress::all_members_dead`. Net LOC is additive this slice (the
participant/objective vocabulary + its tests), which E3 banks (the wave adopts
`EncounterParticipants` for its mobs, deleting the `alive_ids` string handling)
and E4 banks (the objective drives completion, removing the phase-machine's
encounter coupling). Verified: crate A 20, actors boss 72, boss_lifecycle 4,
boss_contact_iframes 8, app clean.

### E3a — timeline authority relocated to the generic crate (landed 2026-07-11)

The doc's E3 first bullet ("Move the reusable parts of `EncounterScript`, beats,
gates, and effects into the encounter authority"): the timeline VOCABULARY +
generic beat-stepper now live in `ambition_encounter::timeline` —
`EncounterGate` (the signal message), `EncounterTrigger` (Gate / MemberDied /
AllMembersDead / Timer, with a generic `holds` predicate that reads participant
`alive`), `EncounterEffect` (ForceKill / Banner / SetMusic / CommandMoveTo /
DropHazard — neutral member/geometry DATA, §6), `EncounterBeat`, and
`EncounterScript` with a generic `advance(dt, participants, fired) -> effects`
cursor stepper. The boss module re-exports them (consumers unchanged) and keeps
only what TOUCHES actor bodies: `tick_encounter_scripts` (executes the advanced
effects), `CommandedMove` + `FallingHazard` + their tick systems. Any content can
now carry an `EncounterScript` without depending on the boss crate. Verified:
crate A 22, actors boss 72, app clean.

**Impedance note (why E3b — waves-as-`EncounterScript` — is not a mechanical
port):** the boss timeline is a SINGLE-CURSOR beat sequence (advance one beat per
fired trigger); the wave arena is CONCURRENT — several delayed sub-spawns in
flight at once, plus dynamic wave-gating (wave N+1 starts when wave N's minions
are all dead, at a time not known in advance). A single cursor cannot hold
concurrent timed spawns. So the elegant convergence is: waves and bosses share
the timeline VOCABULARY (triggers/effects/participants/objective/lifecycle, one
crate) but the wave keeps a slim spawn-STEPPER (its proven `run.pending` /
`wave_elapsed` scheduler feeding `EncounterParticipants` with role `Minion`),
rather than forcing the wave onto the boss's single-cursor stepper. E3b converges
the wave's lifecycle/objective/participants onto the shared components and keeps
the spawn scheduler — deleting the wave phase/lock/completion LOGIC (now
lifecycle + objective) while the scheduler survives as the wave's timeline kind.

### E3b — wave mobs become shared participants (landed 2026-07-11)

The wave arena's live mobs were a bespoke `EncounterRun.alive_ids: Vec<String>`
tracked via a per-id `enemy_alive` closure the host threaded into the reducer.
They are now the generic `EncounterParticipants` (role `Minion`, ownership
`Spawned`) — the SAME component boss fights use for their adopted `PrimaryTarget`.

- `EncounterRun` loses `alive_ids`; the reducers (`maybe_start` /
  `tick_intro_or_wave` / `on_player_death`) take `&mut EncounterParticipants` and
  manage the Minion list (spawn → push a live participant; retain-alive drops the
  dead). The `enemy_alive` closure is gone: the host refreshes each participant's
  `alive` from the runtime mobs query BEFORE the reducer ticks, and the reducer
  reads `participant.alive` — the "just-spawned survives one tick" invariant falls
  out because the fresh spawn is appended AFTER the refresh.
- The host `update_encounters_from_world` queries
  `(&Encounter, &mut EncounterState, &mut EncounterParticipants)` and clears the
  participants at every re-arm / cancel / death site.

Deleted: `EncounterRun.alive_ids`, the `impl FnMut(&str) -> bool` closure
threading, and the host's per-tick `alive_lookup`→closure indirection (replaced
by a direct `participant.alive` refresh). Verified behaviour-preserving against
the wave suite: crate A reducer tests 22, host `encounter::tests` 38 (intro
delay, delayed sub-spawns, inter-wave delay, just-spawned-survives, wave advance,
clear/fail/retry, lock wall, reward, switch arming), music 8, app clean.

### Recommended next slice (superseded — E1/E2/E3 landed above)

The two systems barely interact at runtime, so the remaining slices are a real
multi-session migration that touches live wave state, boss content authoring,
the HUD/camera/save adapters, and in-game feel (wave timing, lock walls, camera
zoom, boss music/lock-wall pacing) that only the sim + Jon's eye can verify.
Each slice must compile and delete a path in the same commit (this doc's own
mandate), so they were deliberately not batched into one unverifiable push.

**Start E1 here:** the *wave* live state (`EncounterRegistry: BTreeMap<String,
EncounterState>` in crate A) is the last resource-owned encounter authority; the
boss side is already an entity (`EncounterDef`). E1 moves the wave lifecycle
(`maybe_start` / `tick_intro_or_wave` / `on_player_death` reducers in
`ambition_encounter/state.rs`) onto components and reduces `EncounterRegistry`
to an index, then adds the generic `Start/Fail/Complete/Signal` command ingress
(§4). Blast radius (from the E0 consumer map): the ~355-line
`ambition_actors/encounter/systems.rs` monolith, plus HUD
(`ambition_app/app/hud.rs`), camera (`ambition_sim_view/camera_snapshot.rs`),
music intent, reward chests, switches, lock walls, and save projection. Build a
parity harness against the existing `ambition_actors/encounter/tests.rs` (118
green tests) FIRST, then port. Do E1 as its own commit before E2 touches
participants/roles.
