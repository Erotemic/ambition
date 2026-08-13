# Authority Convergence — D73 closure campaign

**Status:** OPEN  
**Date:** 2026-08-13  
**Run:** armed 2026-08-13T13:10Z as the 72h run's first ordering authority (`.goal/queue-72h-2026-08-13.json`, deadline 2026-08-16T13:10Z).  
**Primary objective:** close D73 by deleting duplicate actor/body authority, making intrinsic character construction genuinely common, and deleting the remaining enemy-archetype authority.  
**Finish line:** D73 closure. Rollback-registration inversion and `tick_actor_brains` decomposition are successor campaigns, not part of this campaign's definition of done.

## Run progress (live updates)

⛔ **This table is the resumption point after a compact, and the goal guard's engine.** A phase flips to ✔ only when its own hard exit criteria are met AND its named deletion has happened — flipping a box without the deletion is the one dishonesty that breaks the run.

| Phase | Status | Started | Landed | Note |
|---|---|---|---|---|
| P0.1 confirm presentation effects before rollback escape | ✔ | 2026-08-13 | 2026-08-13 `89e0f493d` | the mechanism already existed: `external_effects` carries five families to the confirmed boundary. The shake became the sixth — `CameraShakeRequest` published by the sim, applied by `apply_camera_shake_requests`. Deleted: the `replaying_history` parameter guard, which saw the duplicate and not the phantom |
| P0.2 rollback-consistent boss phase edges | ✔ | 2026-08-13 | 2026-08-13 `ab1ab4d1d` | the authoritative edge ALREADY EXISTS: `ActorPhaseState::tick` returns `BossPhaseEvent::PhaseChanged`, and `update_boss_encounters` fans it to `publish_events` inline. `boss_phase_transition_feedback` re-derives that same edge from a non-rollback `Local<HashMap<String, Phase>>` — so on a resimulation the Local already holds the new phase, the diff is empty, and the shockwave `DamageBox` is LOST on the authoritative timeline. Fix: consume the authority, delete the Local |
| AC0 census + surface maintainer decisions | ⏳ | 2026-08-13 | | AC0.1 ✔ (below) · AC0.5 ✔ — Jon settled the whole casting/completeness backlog unprompted, see the decisions section. anchors confirmed live 2026-08-13: `ActorIntent` 29, `ActorCooldowns` 25, `adopt_character_intrinsics` 10, `CharacterRoster` 223, `ArchetypeSpec` 93, `ActorTuning` 57 |
| AC1 delete dead actor mirrors | ✔ | 2026-08-13 | 2026-08-13 `839b5887d` | **DELETED**, 24 files. 4 maintenance roads removed (construction, per-frame sync, reset/damage/aggression, rollback), 2 bundle fields, 2 schema rows (v25→v26), the boss's per-frame `CharacterAiMode` classification that existed only to fill the mirror, and the sync-only tests. ⚠ finding for AC3/AC6: `ActorStatus::ai_mode`'s only remaining reader is now the rollback snapshot |
| AC2 scheduler-perturbation determinism guard | ✔ | 2026-08-13 | 2026-08-13 | `app_it::scheduler_perturbation` — A/B over two GRAPHS (the desync canary compares one graph against itself and structurally cannot see this class). Three benign readers placed by PHASE only, execution COUNTED so a filtered-out probe cannot pass as a perturbation. Falsification kept as a second test: a real conflicting writer must make the digests differ. ⚠ green means *no implicit ordering was disturbed by this perturbation*, which is weaker than *the graph has none* — stated in the module docs rather than overclaimed |
| AC3 converge body/reaction authority | ▢ | | | `sync_actor_components_from_cluster` 14 refs |
| AC4 complete prepared character bodies | ▢ | | | |
| AC5 construction convergence, delete build-then-patch | ▢ | | | |
| AC6 delete archetype/roster/tuning authority | ▢ | | | `character_archetypes.ron` still present (16K) |
| AC7 final naming/docs + D73 closure + amplification probes | ▢ | | | |

## ⭐⭐ Maintainer decisions given 2026-08-13, during this campaign

Jon settled the campaign's whole casting/completeness backlog in one handoff,
unprompted, hours after AC0 began. **These are settled. Do not reopen them as
architectural questions.** Verbatim in
[`maintainer-decisions.md`](maintainer-decisions.md); the operative content here:

**The architectural invariant** — *"there is no separate 'can fight' character
property. A character can fight exactly to the extent that its body has
abilities/capabilities that can produce combat effects."* Body owns abilities and
moves; controller decides whether and when to use them; disposition owns who is
friendly; a ruleset may restrict verbs but never creates an ability the body
lacks. ⛔⛔ **do not introduce a `can_fight`, `combatant`, or
peaceful-vs-fighter taxonomy while completing bodies in AC4.**

**The five content rulings, each a deletion the campaign can now take:**

| Ruling | What it unblocks |
|---|---|
| the dive-drill and its anonymous `Target` may be DELETED — *"deletion over architecture for disposable AI-authored content"* | AC5's required-character-id work; ⛔ do not build a fixture/exception to preserve it, and re-home any engine behaviour only it exercised |
| previously unauthored health is TUNING — author it now: humanoid/NPC **4**, pirate **4**, heavy pirate **6**, Patent Clerk **6** | AC4 entirely; ⛔ *"do not retain fallback health or incomplete body definitions because we are waiting for balance decisions"* |
| **Carl Stargan does not fly**; he fights because his body authors abilities | one `REGISTERED_WITHOUT_A_BODY` entry; ⛔ do not infer flight from art, `body_kind`, NPC role or a legacy archetype |
| **skitters ARE Puppy Slug** — `SmallSkitter` and `under_town_skitter` author as `npc_puppy_slug` | AC6's `medium_striker` justification where that is its only remaining purpose |
| **`large_brute` becomes a real authored Goblin Brute character**, with a SEPARATE Python sprite generator target (may share helpers/vocabulary with the ordinary goblin) | the last uncast identifier that earns a character rather than a deletion; ⛔ no image-generation tooling — sprites stay code-generated |

⚠ **the distinction Jon drew himself, and it decides how these age**: the
invariants persist; the HP numbers are *"initial tuning choices, intentionally
easy to retune later"*. Changing a number later is not reopening a decision.

⛔ **his closing instruction**: *"do not invent replacement compatibility
machinery for rows whose only consumers disappear or become character-first."*
Recompute which `CharacterRoster` / `ArchetypeSpec` / `character_archetypes.ron`
rows remain justified after applying these — that recomputation is AC0 work and
its answer sizes AC6.

## Purpose

Ambition's character-template work has reached a useful transition point. The replacement model is already the dominant path for several important contexts, while the remaining difficulty is concentrated in older actor-state and construction seams. Small changes still amplify because the same gameplay fact can participate in several representations, family-specific synchronization paths, rollback registrations, and construction roads.

This campaign reduces that **change amplification** in dependency order.

The operating rule is:

> **Each slice removes authority or representation that the next slice would otherwise have to understand.**

This is the final engineering campaign for D73. It does not create a new architecture beside the character-template model. It makes the replacement architecture complete enough that the old actor/archetype machinery can be deleted.

The desired end state is straightforward:

```text
reusable CharacterDefinition
        ↓
prepared character authority
        ↓
one intrinsic body construction path
        ↓
context supplies controller / disposition / placement / session facts
        ↓
body owns its health, geometry, locomotion, capabilities, combat repertoire,
and held inventory
```

A character used as an NPC, hostile actor, encounter participant, runtime summon, or match fighter is the **same body definition in different context**. Controller changes and provocation change policy/relationship; they do not reconstruct the body.

## Read with

Read these before beginning work, then inspect current HEAD before trusting any status claim in this document:

- [`character-template-architecture-2026-08-10.md`](character-template-architecture-2026-08-10.md) — D73's authoritative destination and maintainer decisions.
- [`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md) — existing transactional construction architecture. Extend it rather than introducing another construction framework.
- [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md) — current ownership/dependency findings and later decomposition context.
- [`triage/bevy-system-parameter-architecture.md`](triage/bevy-system-parameter-architecture.md) — semantic `SystemParam` guidance.
- [`maintainer-decisions.md`](maintainer-decisions.md) and [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) — decisions already made and questions genuinely requiring Jon.
- [`status.md`](status.md) and [`tracks.md`](tracks.md) — current source-backed state and executable queue.
- [ADR 0023](../adr/0023-same-build-determinism.md) — deterministic simulation.
- [ADR 0026](../adr/0026-immutable-prepared-content-and-exact-session-identity.md) and [ADR 0027](../adr/0027-ggrs-is-the-sole-rollback-authority.md) — content/rollback authority.
- [ADR 0031](../adr/0031-public-facade-is-the-compatibility-boundary.md) and [ADR 0032](../adr/0032-authoring-is-declarative.md) — API and authoring direction.

## Current evidence anchors

These are **evidence anchors, not permanent counts**. Recompute them against HEAD before acting.

At the 2026-08-13 review snapshot:

- `ActorIntent` and `ActorCooldowns` still had many construction/sync/reset/rollback references but no substantive production consumer found by review.
- `BodyCombat` still mixed independently authoritative reaction history with fields reconstructed as read-model projections.
- `sync_actor_components_from_cluster` still contained the architectural shape this campaign targets: preserve selected `BodyCombat` fields, rebuild the component, restore the preserved fields.
- `ActorClusterSeed::new_character_in` was already used by multiple character-first construction roads.
- `CharacterSpawnPlan` had a much smaller production surface than `new_character_in`; it is a useful resolution seam, not automatically the universal plan type.
- `adopt_character_intrinsics` remained a live build-legacy-body-then-patch seam.
- `CharacterRoster`, `ArchetypeSpec`, and the authored archetype file remained the concentrated legacy ontology.
- the surviving casting exceptions included names such as `small_lurker`, `large_brute`, and `SmallSkitter`; they are content decisions, not justification for a permanent generic fallback ontology.
- `shake_camera_on_landed_hits` still crossed from speculative rollback simulation into non-rollback camera presentation without confirmed-frame quarantine.
- `boss_phase_transition_feedback` still inferred a simulation transition using non-rollback `Local` history while also producing gameplay consequences.

If HEAD contradicts one of these anchors, update this plan/status instead of recreating already-finished work.

---

# Architectural doctrine

## 1. History has one authoritative owner

A fact whose previous value can affect future simulation has one authoritative representation.

Examples include:

- health;
- hitstop/hitstun/recoil/landing-lag history;
- cooldown history;
- brain-internal history;
- perception memory;
- persistent combat-slot hysteresis;
- inventory/held-item state.

An authoritative fact may be rollback-registered when rollback must restore its history. It is not duplicated into another mutable structure merely because another subsystem wants a convenient read model.

## 2. Observations are projections

A fact deterministically reconstructible from authoritative state for the current step is a projection.

Examples include:

- current liveness view derived from health;
- current melee-swing observation derived from authoritative attack/move state;
- crowding and nearest-neighbor results;
- visibility results;
- future `BrainInputFrame`-style observations;
- presentation snapshots.

A projection is disposable. Simulation must not depend on a stale copy when it can read the authority.

## 3. Character is body authority; controller and placement are context

The body owns:

- geometry;
- locomotion and intrinsic movement capability;
- health/body durability;
- abilities/capabilities;
- combat repertoire;
- held inventory;
- gameplay character identity.

Controller/brain owns policy and intent.

Placement/session owns contextual facts such as disposition, team, encounter membership, respawn/session policy, explicit placement overrides, and controller assignment.

A ruleset may restrict a body's available capabilities. It does not manufacture capabilities the body does not have.

## 4. One body, one construction path

The campaign converges every body-complete character onto one intrinsic construction path. Upstream authoring/resolution roads may differ when their semantics differ; the **body construction authority does not**.

`CharacterSpawnPlan` is retained when it truthfully models a caller's resolution problem. It is not expanded into a universal god-plan merely so every path can be drawn through the same box.

## 5. Deletion is the proof of convergence

A new abstraction counts as progress when it removes or makes unreachable an old representation, branch, dependency, sync path, or compatibility path in the same slice or names the exact deletion gate.

This campaign prefers:

```text
old authority removed
        ↓
compiler exposes surviving consumers
        ↓
move only the facts that still have real consumers
```

over faithfully transplanting every field of a transitional structure before discovering which fields were obsolete.

## 6. Control concepts remain semantically distinct

Use existing concepts precisely:

```text
currently controlled body      → ControlledSubject
human/AI/script/replay driver  → control authority / Brain / input source
match participant              → MatchSeat / participant/slot identity
view/camera focus              → ViewSubject / presentation focus
all bodies                     → body query
original/home avatar           → PrimaryPlayer only when the code truly means it
```

Body-generic gameplay does not acquire a new `PlayerEntity` or `PrimaryPlayer` dependency merely to locate whichever body happens to matter.

---

# Non-goals

This campaign does **not** perform:

- broad actor-monolith carving for line-count reduction;
- rollback-registration dependency inversion;
- full `tick_actor_brains` world→observation→decision decomposition;
- wholesale `shared_tangle` splitting;
- broad public-facade cleanup;
- a flag-day `PrimaryPlayer` rename/removal;
- source-policy cleanup as a standalone project;
- cosmetic splitting of `spawn_actors.rs`;
- creation of a second character/body/construction/capability registry.

It also does not create new umbrella concepts such as `BodyCombat2`, `ActorContext`, `SimulationContext`, `ControlContext`, or a generic actor factory merely to make existing code easier to move.

Rollback inversion and brain-decision decomposition are valuable successor campaigns. They start from the simpler authoritative vocabulary produced here.

---

# Review contract for every slice

Every implementing commit/slice must answer these questions in its review summary:

1. **Owner:** Which domain owns every moved/new fact?
2. **Category:** Is it authoritative history, a derived simulation projection, or presentation state?
3. **Rollback:** If rollback-registered, what future behavior would be wrong if its history were lost?
4. **Deletion payoff:** Which old representation, branch, sync site, dependency, or compatibility path disappeared?
5. **No shadow architecture:** Is the old mechanism still equally usable? If so, state the named deletion gate; otherwise the slice is incomplete.
6. **Scheduling:** What semantic phase/data dependency establishes required ordering?
7. **Control semantics:** Did touched body-generic gameplay avoid adding a home-avatar/player marker dependency?
8. **Enforcement:** Can visibility, Cargo dependencies, types, or a behavioral test enforce the property instead of a new source-text scanner?

A transitional adapter is acceptable only when the same plan/commit names what deletes it.

---

# Campaign topology

The primary line is sequential:

```text
P0   close finite correctness hazards that would contaminate the campaign
 ↓
AC0  minimal current-HEAD census + surface maintainer decisions immediately
 ↓
AC1  delete dead actor mirrors
 ↓
AC2  establish scheduler-perturbation determinism guard
 ↓
AC3  converge body/reaction authority
 ↓
AC4  complete prepared character bodies
 ↓
AC5  converge construction and delete build-then-patch
 ↓
AC6  delete remaining archetype/roster/tuning authority
 ↓
AC7  final naming/docs + D73 closure + change-amplification probes
```

Each phase should make the next phase cheaper.

---

# P0 — Close finite correctness hazards before measuring architecture

**Status: OPEN until HEAD disproves the named evidence.**

P0 is intentionally small. It closes bugs that can falsify the campaign's behavioral measurements. It is not an invitation to clear unrelated planning backlog.

## P0.1 Confirm presentation effects before they escape rollback

### Current evidence

Inspect:

- `shake_camera_on_landed_hits`;
- `ambition_platformer2d_runtime::external_effects`;
- `SimulationReplayState`;
- the current combat simulation schedule;
- the existing effect-quarantine tests.

The previously reviewed shape was:

```text
predicted simulation frame
    ↓
strong hit observed
    ↓
CameraShakeState mutated immediately
    ↓
remote input later invalidates that predicted hit
    ↓
rollback cannot undo presentation already emitted
```

Checking only `replaying_history == false` is insufficient: the first speculative execution is not a replay, but it is still unconfirmed.

### Required end state

An irreversible/non-rollback presentation effect caused by simulation is released only from the existing confirmed/external-effect boundary, or another existing mechanism that proves equivalent semantics.

The simulation may publish an effect request/fact. Presentation applies it after confirmation. Non-rollback/no-GGRS execution may release immediately through the same semantic path.

### Required falsifier

A behavioral test must establish all three cases:

```text
predicted strong hit
    → camera effect is not externally released yet

prediction is abandoned/corrected to no hit
    → the camera effect is never released

strong hit reaches confirmation
    → exactly one camera effect is released
```

The test must not prove only that replayed history is suppressed.

### P0.1 exit

P0.1 is DONE when:

- speculative hit feedback cannot escape before confirmation in rollback play;
- abandoned prediction produces no camera kick;
- confirmed hit produces one kick;
- ordinary non-rollback behavior still produces feedback;
- no home-avatar/`PrimaryPlayer` requirement is reintroduced.

## P0.2 Make boss phase-transition edges rollback-consistent

### Current evidence

Inspect `boss_phase_transition_feedback` and its registration in the simulation/progression schedule.

The previously reviewed implementation inferred phase edges by comparing current rollback state to a `Local<HashMap<..., BossEncounterPhase>>`. `Local` history is not restored with simulation rollback. If that comparison also emits gameplay state such as a `DamageBox`, a restored old phase can be mistaken for a new transition and produce gameplay on the corrected timeline.

### Required end state

Gameplay-significant transition edges come from rollback-consistent authority.

Acceptable shapes include:

- store the necessary previous-phase history in rollback state owned by the boss/encounter domain; or
- derive a transition event from the authoritative state change at the point where that change is committed, with deterministic rollback semantics.

Camera/SFX/VFX consequences then cross the appropriate presentation/effect boundary rather than defining the transition themselves.

### Required falsifier

Exercise at least one rollback/resimulation transition case in which stale non-rollback local history would previously have manufactured a false transition. Prove:

- corrected history produces no extra gameplay shockwave/damage effect;
- a real transition produces the effect exactly once on the authoritative timeline.

### P0.2 exit

P0.2 is DONE when gameplay phase-transition consequences no longer depend on non-rollback `Local` history.

## P0 exit

P0 is DONE when P0.1 and P0.2 are green or HEAD evidence shows the defects have already been replaced by stronger equivalent mechanisms.

Do not repair D107's `BodyCombat.attacking` mirror separately in P0; AC3 removes that class of mirror. Do not create a special D108 architecture in P0; AC3 owns reaction-timer lifecycle convergence.

---

# AC0 — Minimal baseline and early maintainer-decision surfacing

**Status: OPEN after P0.**

AC0 gathers only evidence that changes execution order. It is not a workspace census project.

## AC0.1 Recompute dead-mirror consumers

For each of:

```text
ActorIntent
ActorCooldowns
```

classify every production reference as one of:

```text
real read/consumer
write/synchronization
construction/default
reset/damage/aggression maintenance
snapshot/rollback registration
re-export/type plumbing
debug/test-only
```

Record the real production readers in this plan or the active run ledger.

If there are zero real consumers, AC1 deletes the component.

### ✔ AC0.1 RESULT, recomputed against HEAD 2026-08-13 (54 references, **0 real readers**)

⛔ **both components are write-only.** Every reference is one of: the definition,
a snapshot impl, a rollback registration, a bundle field, a construction site, a
per-frame write-sync, a reset, a re-export, or a test. Nothing in production ever
reads the value back.

| Category | `ActorIntent` | `ActorCooldowns` | Sites |
|---|---:|---:|---|
| definition + accessors | 4 | 2 | `ambition_combat::components::actors` |
| snapshot impl + rollback registration | 3 | 2 | `snapshot_impls.rs`, `rollback/domains/combat.rs` (`actor.intent`, `actor.cooldowns`) |
| bundle field + construction | 6 | 6 | `actor_bundles.rs`, `actors/conversion.rs`, `bosses/sync.rs` |
| per-frame write-sync | 6 | 6 | `actors/update.rs` (incl. `sync_actor_components_from_cluster`), `bosses/sync.rs` |
| reset / damage / aggression / save maintenance | 4 | 4 | `reset.rs`, `damage/mod.rs`, `aggression.rs`, `save_sync.rs` |
| re-export / import plumbing | 3 | 3 | `features/mod.rs` |
| test-only | 2 | 3 | `spawn/tests.rs` |
| **real production read** | **0** | **0** | — |

⭐ **the doc comments claim consumers that do not exist.** `ActorIntent`'s says it
exists *"so rendering and HUD systems can branch on actor state"*; no rendering or
HUD system mentions it. Its `is_dangerous()` accessor is called from exactly one
place — its own body — and a comment in `features/enemies/integration.rs:96`
already recorded the finding: *"`is_dangerous()` has no gameplay caller."*
`ActorCooldowns`' two fields are read only by `spawn/tests.rs`; every other
`attack_cooldown`/`respawn_timer` in the tree belongs to a different type
(`enemy.status`, the boss behaviour config).

⚠ **neither is exported by the public facade**, so ADR 0031 raises no external
consumer. Deleting them removes two rows from
`game/ambition_app/tests/rollback_schema_baseline.txt` (`actor.intent`,
`actor.cooldowns`) and bumps the schema version — which Jon's 2026-08-08 ruling
makes a non-event: the wire format is unstable by policy.

If a real consumer exists, identify the actual authority it needs. Migrate that consumer first; preserving the mirror is not the default answer.

## AC0.2 Build the `BodyCombat` authority inventory

For every current `BodyCombat` field, record:

```text
field
production readers
production writers
history affects future simulation? yes/no
current authority
intended authority
derived/presentation? yes/no
rollback requirement and why
reset sites
decay sites
family-specific carry/sync sites
```

Explicitly locate any:

```text
save selected fields
rebuild BodyCombat
restore selected fields
```

pattern and any player/actor/boss parallel timer maintenance.

This matrix is working evidence and may be deleted/archive-reduced when AC3 closes.

## AC0.3 Recompute character completeness

Record:

- prepared definitions with complete intrinsic body blueprints;
- registered/prepared characters still relying on body assistance or legacy construction facts;
- every production caller of `adopt_character_intrinsics` or renamed equivalent;
- which incomplete bodies can be transcribed behavior-preservingly from current shipped behavior;
- which genuinely require a maintainer/content decision.

Do not author new capabilities/health/moves solely to drive the incomplete count to zero.

## AC0.4 Recompute construction roads

For each production construction context, record two separate questions:

```text
How is character identity / prepared content resolved?
How are intrinsic body facts materialized?
```

At minimum inspect:

- NPC;
- authored hostile/enemy;
- encounter;
- runtime/programmatic summon;
- match/Smash;
- any boss/special path that claims to construct an ordinary character body.

Record use of:

- `PreparedCharacterDefinition` / body blueprint;
- `CharacterSpawnPlan`;
- `ActorClusterSeed::new_character_in` or current canonical constructor;
- legacy `CharacterRoster` / `ArchetypeSpec` construction;
- build-legacy-body → `adopt_character_intrinsics` patching.

The invariant is common **intrinsic construction**, not mandatory use of one plan struct.

## AC0.5 Surface maintainer decisions immediately

Cross-reference `maintainer-decisions.md`, `awaiting-maintainer-decision.md`, and D73's decision ledger.

For every remaining body/casting question:

- if Jon already decided it, treat that as input and implement when its phase arrives;
- if genuinely unresolved, put it on the existing maintainer-decision surface now with the agent's best recommendation and concrete consequences;
- do not ask again for a decision already recorded;
- continue AC1–AC3 while unresolved product decisions are pending.

Current historical examples include `small_lurker`, `large_brute`, and `SmallSkitter`; recompute rather than assuming that exact set still survives.

## AC0 exit

AC0 is DONE when the active ledger contains:

- real consumers for `ActorIntent` and `ActorCooldowns`;
- the `BodyCombat` field-authority matrix;
- the body-completeness/incomplete population split;
- the construction-road table;
- every genuinely blocking maintainer decision surfaced early.

Do not add a permanent scanner merely to freeze these counts.

---

# AC1 — Delete dead actor mirrors

**Status: OPEN; entry gate is AC0's consumer census.**

## Goal

Remove `ActorIntent` and `ActorCooldowns` if HEAD confirms they are maintained representations with no real production consumer.

This happens before adding the scheduler guard so later tests protect the simpler graph rather than memorializing dead synchronization.

## Steps

For each confirmed-dead component:

1. Delete the component/type.
2. Delete spawn/bundle initialization.
3. Delete actor and boss per-frame synchronization into it.
4. Delete reset/damage/aggression maintenance performed only to keep the mirror current.
5. Delete snapshot implementations and rollback registration.
6. Delete re-exports/imports/docs whose only subject is the deleted read model.
7. Delete tests that only assert synchronization of the mirror.
8. Preserve or rewrite behavioral tests around the actual authoritative state.

If a real consumer is discovered, migrate it to the real authority before deleting the mirror. If the consumer exposes genuinely missing history, place the smallest authoritative fact in its owning domain rather than renaming the mirror.

## Required measurements

Record before/after counts for:

- mirror synchronization sites removed;
- reset/damage maintenance sites removed;
- rollback schema entries intentionally removed.

These are diagnostic measurements, not new ratchets.

## AC1 exit

AC1 is DONE when either:

```text
ActorIntent    has no production definition/use
ActorCooldowns has no production definition/use
```

or AC0 demonstrated a genuine authority that requires one of those concepts and the plan has been updated to reflect the revised owner.

Also require:

- no differently named replacement mirror;
- rollback schema/version procedure followed for intentional state removal;
- focused actor/boss/damage/reset tests green;
- `cargo check -p ambition_app` green.

---

# AC2 — Establish scheduler-perturbation determinism guard

**Status: OPEN after AC1.**

## Why here

The guard is valuable before AC3 and AC5, where body state and construction dataflow change materially. It is cheaper after AC1 because dead synchronization nodes have already left the graph.

## Required test

Reuse an existing deterministic harness/desync-canary/state digest if practical. Do not build a second simulation framework.

Run the same deterministic scenario twice with identical inputs:

```text
A. ordinary engine graph
B. same graph plus unrelated benign systems inserted into the relevant
   simulation phases
```

The perturbation systems must:

- actually execute;
- read real simulation state or a harmless resource so their graph nodes are real;
- perform no simulation mutation;
- avoid explicit ordering against the writer being probed.

Compare an existing deterministic trace/hash when available. Otherwise compare a selected authoritative-state digest sufficient to expose the known scheduler-topology defect class.

The test should prove that **adding an unrelated reader does not alter deterministic behavior**.

## Falsification requirement

Demonstrate that the test is capable of failing. Acceptable evidence includes a temporary poison that introduces a real conflicting writer/order change or reproduces a known historical ordering defect. The poison is removed before merge.

The final test must not merely compare two variants that happen to skip the perturbation system.

## If AC2 is red before refactoring

Treat a pre-existing divergence as a real scheduling defect.

Repair the semantic dependency by:

- explicit simulation phase/set ordering;
- dataflow dependency;
- or ownership correction.

Do not fix the test by ordering the benign leaf system relative to unrelated writers.

## AC2 exit

AC2 is DONE when:

- both graph variants execute;
- deterministic authoritative output matches;
- the test has a demonstrated failure mode;
- no new source-text ordering policy was added;
- `cargo check -p ambition_app` is green.

This guard remains green for every subsequent AC phase.

---

# AC3 — Converge body/reaction authority

**Status: OPEN after AC2.**

This is the highest-leverage state-model slice.

## Goal

Eliminate the mixed state/read-model architecture around `BodyCombat` and parallel actor-family maintenance.

The campaign does **not** require the symbol `BodyCombat` to disappear. It requires the surviving type, if any, to have one coherent authority.

## AC3.1 Classify fields by actual authority

Use AC0's field matrix and migrate fields in this order unless HEAD evidence suggests a cheaper dependency order.

### A. Liveness mirror

If `BodyCombat.alive` mirrors `BodyHealth`, migrate consumers to `BodyHealth` and delete the mirror.

Simulation never needs a second mutable liveness answer.

### B. Attack-state mirror

If `BodyCombat.attacking` mirrors melee/move state, migrate consumers to the authoritative attack/move signal and delete the mirror.

Do not repair D107 by broadening the mirror to `MovePlayback::is_some()`. Ranged/special playback is not automatically a melee swing. Use the existing semantic melee classifier/body-melee authority.

Required poison cases when relevant:

```text
melee swing      → attacking/melee observation true
ranged playback  → false for melee-attacking observation
special playback → false unless its authored semantics explicitly include melee
```

### C. Attack timeline/read-model fields

Fields such as attack windup/active time belong with the authoritative attack/move runtime or as one-way projections for presentation. Remove them from independently mutable reaction history when appropriate.

### D. Authored/configuration facts

Fields such as training/practice behavior belong to authored capability/configuration authority unless they are genuinely runtime history. Migrate consumers accordingly.

### E. Reaction history

History-bearing reaction state such as:

- damage invulnerability;
- hitstun;
- recoil lock;
- hitstop;
- landing lag;
- other timers whose prior value changes future control/integration;

must have one body-level owner and one semantic lifecycle.

Player, autonomous actor, and boss roads must not each maintain parallel timer carry/reset/decay lists for the same body semantics.

### F. Presentation-only temporal state

Classify presentation state explicitly. Do not move a temporal presentation fact out of rollback merely because gameplay does not read it if losing its history would make rollback presentation incorrect.

The rule is:

> rollback participation follows temporal correctness requirements, not the label "gameplay" versus "presentation".

If presentation state belongs behind confirmed effects instead, migrate it only when the existing presentation/rollback boundary supports that semantic correctly.

## AC3.2 Eliminate save → rebuild → restore

The following shape must disappear:

```text
save authoritative fields from a component
reconstruct the same component as a read model
restore the saved authoritative fields
```

In particular, inspect `sync_actor_components_from_cluster` and boss/player equivalents.

A surviving sync system may publish coherent derived views. It may not rebuild a structure that also owns independent history.

## AC3.3 Unify reaction lifecycle

For every surviving reaction timer, identify:

- writers/events that start or extend it;
- the single semantic decay path;
- reset semantics;
- rollback registration;
- integration/control consumers.

The D108 bug shape is the falsifier: adding a new reaction timer must not require remembering a second actor-family carry list plus a third reset list.

`BodyCombat::reset()` or its final replacement must reset every timer whose semantics say a body reset clears it. Landing lag should not survive reset merely because a destructure/list forgot the field.

## AC3.4 Decide the surviving type after the field migration

Only after mirrors/configuration/presentation fields have moved:

- delete `BodyCombat` if nothing coherent remains; or
- keep it temporarily if it now clearly represents one authority, such as body reaction history; or
- rename once if the final surviving semantics make another name materially clearer.

Do not perform a rename-only workspace churn before authority convergence.

## AC3 hard exit criteria

AC3 is DONE only when all are true:

- `ActorIntent` and `ActorCooldowns` remain gone;
- `BodyCombat.alive` or equivalent duplicate liveness mirror is gone;
- `BodyCombat.attacking` or equivalent duplicate melee/attack mirror is gone;
- no save → rebuild → restore pattern remains for the surviving reaction-state component;
- actor/boss/player paths do not maintain separate semantic copies of the same reaction timer lifecycle;
- liveness-critical gameplay reads `BodyHealth` directly;
- attack-state gameplay reads the authoritative melee/move state directly;
- each surviving history-bearing reaction field has one documented owner, reset rule, decay rule, and rollback justification;
- scheduler perturbation remains green;
- focused hit-reaction, landing-lag, attack, death/revive, controlled-body, autonomous-actor, and boss tests are green;
- `cargo check -p ambition_app` is green.

### Change-amplification falsifier

Before closing AC3, answer:

> If a new reaction timer analogous to landing lag were added now, which production files would need edits?

The answer must no longer include parallel player/actor/boss mirror carry lists or a save/rebuild/restore list.

---

# AC4 — Complete prepared character bodies before widening construction

**Status: OPEN after AC3.**

## Why before NPC convergence

A body-complete NPC already has a route to common character construction in the recent architecture. The dangerous population is the set of registered/prepared characters whose body facts still fall through legacy/catalog/archetype assistance.

Changing the fallback/trunk before shrinking that population repeats the failure mode that previously changed large numbers of NPCs at once.

AC4 therefore makes prepared definitions complete first.

## AC4.1 Recompute body-incomplete definitions

From AC0's inventory, divide incomplete definitions into:

```text
A. behavior-preserving transcription possible
B. existing maintainer decision already specifies intended body
C. genuinely unresolved product/content choice
```

Category A should move immediately.

Category B consumes the recorded decision without asking again.

Category C stays on the existing maintainer-decision surface and does not justify inventing an arbitrary body simply to close the migration.

## AC4.2 Complete behavior-preserving bodies

For Category A, author the character's intrinsic body facts directly from current shipped behavior.

Relevant facts may include:

- health/durability;
- geometry/body size/collision body;
- locomotion/free-flight baseline;
- intrinsic capabilities;
- action/moveset repertoire;
- contact damage/body combat characteristics;
- weight or other intrinsic physics facts.

Preserve context ownership:

- peaceful/hostile disposition is not intrinsic body identity;
- encounter/team/session policy is not intrinsic body identity;
- controller policy is not intrinsic body identity;
- respawn/session facts remain with their current contextual owner unless D73 already settled otherwise.

## AC4.3 Prove completeness through production preparation

A test should verify that each migrated definition prepares a complete body through the actual registration/preparation seam, rather than constructing a test-only finalized object that production cannot create.

Use the gated test-support path only for pure unit-level preparation behavior; production-composition claims go through production composition.

## AC4.4 Keep incomplete product choices explicit

Where the content decision remains open, keep the temporary state visible through the existing maintainer-decision mechanism. Do not encode that ambiguity as permanent generic archetype semantics.

## AC4 exit

AC4 is DONE when:

- every body-incomplete definition that can be migrated behavior-preservingly has been migrated;
- every already-decided body choice is implemented;
- every genuinely unresolved choice is explicitly BLOCKED on the existing maintainer-decision surface;
- the ordinary prepared-character population no longer relies on body assistance merely because the authoring file was never filled in;
- focused preparation/body-blueprint tests are green;
- scheduler perturbation remains green;
- `cargo check -p ambition_app` is green.

AC4 does not require inventing answers for unresolved content decisions.

---

# AC5 — Construction convergence and deletion of build-then-patch

**Status: OPEN after AC4 has reduced body assistance enough that the common road is the ordinary case.**

AC5 combines what would otherwise become two transitional phases: common NPC construction and deletion of `adopt_character_intrinsics`.

## Required end state

For a body-complete character:

```text
NPC
authored enemy/hostile
encounter participant
runtime/programmatic summon
match/Smash fighter
```

all materialize intrinsic body facts from the same prepared character/body-blueprint authority and canonical intrinsic constructor.

Upstream resolution/context objects may differ when they solve different authoring problems.

## AC5.1 Treat `CharacterSpawnPlan` as a resolution tool, not a mandatory universal object

Inspect whether NPC resolution naturally fits `CharacterSpawnPlan`.

Reuse it when doing so makes its semantics more general and truthful.

Do **not** add NPC-only display/dialogue/session/controller fields to `CharacterSpawnPlan` solely to force every road through one struct.

The hard invariant is one intrinsic body construction path, not one universal planning record.

## AC5.2 Route body-complete NPCs through canonical intrinsic construction

The NPC road should obtain intrinsic facts from `PreparedCharacterDefinition` / body blueprint and reach `ActorClusterSeed::new_character_in` or the current canonical replacement.

NPC-specific context is applied as context:

- disposition;
- dialogue/ambient behavior;
- placement/session facts;
- controller/autonomous policy selection;
- NPC-specific interaction composition.

It does not re-decide character health, geometry, locomotion, capabilities, or combat repertoire.

## AC5.3 Delete build-legacy-body → patch-character-intrinsics

Inventory all production callers of `adopt_character_intrinsics` or renamed equivalents.

For each caller:

1. resolve prepared character authority before materializing the body;
2. construct the intrinsic body directly;
3. supply contextual values through their owning spawn/session/controller inputs;
4. delete precedence logic between legacy body values and character patches.

Delete `adopt_character_intrinsics` in this same phase once its production population reaches zero.

A phase that routes NPCs through a new path but leaves the body-patch architecture indefinitely available is not complete.

## AC5.4 Finish D102 through explicit caller/provider-owned temporary intent

Unknown authored body identifiers have two semantically distinct cases:

```text
ordinary unresolved identifier
    → construction/preparation error

known content decision explicitly still open
    → owning caller/provider deliberately chooses a temporary generic body
      with a reason/decision reference
```

The temporary choice belongs with the content/caller that owns the unresolved decision.

Do not improve the current exception mechanism by building another global or provider-name registry that AC6 will delete immediately afterward.

Required properties:

- reusable engine code does not contain a central list of Ambition creature names;
- another provider using the same string does not inherit Ambition's exception;
- a typo not explicitly acknowledged cannot become a generic body;
- the temporary body is chosen deliberately at the owning content seam;
- resolving the product decision deletes a local temporary choice.

If HEAD already has a provider-local exception registry, prefer deleting/bypassing it in favor of explicit caller intent when that shortens the route to AC6.

## AC5.5 Cross-context equivalence test

Choose at least one character used in two or more contexts and prove intrinsic facts agree after construction:

- body health/max health;
- geometry/collision body;
- locomotion baseline;
- intrinsic capabilities;
- relevant combat/moveset facts.

Also prove contextual facts may legitimately differ:

- disposition;
- controller/brain;
- encounter/seat/participant context;
- dialogue/interaction context.

## AC5 hard exit criteria

AC5 is DONE only when:

- body-complete NPCs use the canonical intrinsic character constructor;
- all ordinary body-complete character contexts derive intrinsic facts from the same prepared authority;
- production `adopt_character_intrinsics` or equivalent build-then-patch path is gone;
- no ordinary constructor needs `CharacterRoster`/`ArchetypeSpec` to reconstruct character intrinsic facts;
- ordinary unresolved identifiers fail rather than silently choosing a generic body;
- temporary casting ambiguity is explicit at its owning content/caller seam;
- no central creature-name waiver list remains in reusable engine code;
- cross-context intrinsic-body equivalence is behaviorally tested;
- scheduler perturbation remains green;
- focused NPC/enemy/encounter/summon/match tests are green;
- `cargo check -p ambition_app` is green.

---

# AC6 — Delete the legacy enemy-archetype authority

**Status: OPEN after AC5.**

This is the deletion payoff of D73.

## Strategy: delete roots, classify compiler survivors

Do not faithfully migrate every field of every transitional structure before deletion.

Once no ordinary character construction road needs the legacy ontology:

1. remove the root roster/archetype production path;
2. compile/test;
3. classify the surviving users the compiler exposes;
4. move only facts that still have a genuine owner/consumer.

This minimizes transplanting dead architecture.

## AC6.1 Delete roster/archetype roots

Recompute exact names against HEAD. Expected deletion candidates include the current equivalents of:

```text
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
ArchetypeSpec
character_archetypes.ron
legacy enemy-roster assembly/schema
spec_for_brain-style fallback lookup
open-casting exception registry made obsolete by AC5
```

Preserve unrelated concepts that merely share a file.

A literal `combatant` character/policy/body may survive only if some content intentionally names it through the final architecture. It is not a magic error/fallback row.

## AC6.2 Let `ActorTuning` collapse by ownership

After removing roster producers/consumers, inspect only surviving `ActorTuning` uses.

Classify each surviving field as:

```text
reusable character fact
controller policy
spawn/session/ruleset context
runtime body history/state
presentation/editorial metadata
obsolete/dead
```

Move it to an **existing** owner where possible.

Create a replacement type only when surviving fields form one coherent concept with multiple real consumers. Do not create `ActorTuning2` as a bag of leftovers.

Delete fields that lost their only consumer with the archetype road.

## AC6.3 Prepared bodies no longer delegate back to legacy gameplay registries

A `PreparedCharacterDefinition` / body blueprint must contain or directly reference the final intrinsic body authority required by construction.

Delete remaining scaffolding whose absence means “ask the old host/archetype system at runtime,” including current equivalents identified by D73 such as incomplete host-code kit/body delegation.

Do not conflate presentation asset lookup with gameplay body authority. Presentation identifiers may survive temporarily when they are genuinely presentation-only; final vocabulary cleanup belongs to AC7.

## AC6.4 Remove obsolete tests, then retain behavioral contracts

Delete tests whose only purpose was:

- fallback row existence;
- legacy roster lookup behavior;
- precedence between archetype construction and character patching;
- provider fallback concepts that production no longer consumes.

Retain/rewrite tests that prove final behavior:

- explicit bad identifiers fail;
- character intrinsic facts are context-independent;
- temporary casting intent, if any remains blocked on a maintainer decision, is local and explicit;
- provocation/controller changes preserve body facts;
- multi-provider composition does not cross-resolve unrelated content.

## AC6 hard exit criteria

AC6 is DONE when production code/content no longer requires the legacy enemy-archetype authority.

Expected production-absence checks, recomputed for current names:

```text
CharacterRoster              → gone as gameplay authority
CharacterRosterFragment      → gone as gameplay authority
ArchetypeSpec                → gone as character/enemy body authority
character_archetypes.ron      → gone from ordinary runtime content
spec_for_brain fallback road  → gone
adopt_character_intrinsics    → already gone from AC5
```

Also require:

- `ActorTuning` is gone or reduced/renamed to narrowly coherent surviving owner concepts;
- prepared character/body authority suffices for intrinsic construction;
- NPC/enemy/encounter/summon/match roads agree on intrinsic body facts;
- provocation/control changes do not reconstruct the body;
- relevant Ambition, Mary-O, Sanic, and Smash tests are green;
- scheduler perturbation remains green;
- `cargo check -p ambition_app` is green.

Do not add a new permanent grep ratchet solely to memorialize deleted type names. Prefer compilation, visibility, dependency structure, and behavioral tests.

---

# AC7 — Final naming, documentation, D73 closure, and amplification measurement

**Status: OPEN after AC6.**

AC7 is the campaign closeout. It is intentionally after structural deletion so naming/docs describe the final architecture rather than another transition.

## AC7.1 Final semantic naming

Review surviving transitional names.

Examples:

- if `BodyCombat` now means only reaction history, decide whether its current name is still truthful;
- presentation identity fields such as `sprite_character_id` should be named so they cannot be mistaken for gameplay character authority;
- stale comments calling `combatant` a fallback must disappear if fallback semantics are gone;
- stale comments describing NPC construction as a separate body road must reflect the common constructor.

Rename only when it materially improves semantic truth. Avoid churn for names that are already accurate.

## AC7.2 Remove stale transition documentation

Update:

- `character-template-architecture-2026-08-10.md`;
- `status.md`;
- `tracks.md`;
- active run ledger;
- maintainer-decision entries consumed by the campaign;
- this plan.

Archive/reduce migration-only matrices and detailed execution narration according to `docs/planning/README.md` once they no longer direct open work.

## AC7.3 Change-amplification probe A — add a hypothetical reaction timer

Without implementing the feature, identify the production edit sites required for a new history-bearing timer analogous to hitstun/landing lag.

Target shape:

```text
one body-level authoritative owner
specific event writers
one semantic reset/decay lifecycle
rollback declaration if its history must rewind
consumers
```

The list must not contain parallel player/actor/boss mirror carry lists or a save→rebuild→restore list.

Record the before/after edit-site count from AC0/AC3.

## AC7.4 Probe B — move control authority to another body

Trace what must change when a different body receives control.

Target:

```text
control subject / participant authority changes
body-generic damage, movement, abilities, portals, pickups, combat do not
```

If body-generic systems require `PrimaryPlayer` merely to locate the relevant body, record/fix the inappropriate dependency before closure.

Do not launch a broad identity migration during AC7.

## AC7.5 Probe C — use one character in several contexts

Trace the work required to use one reusable character as:

```text
NPC
hostile/enemy or encounter body
match/Smash fighter
```

Target:

```text
one CharacterDefinition/preparation path
one intrinsic body construction path
different contextual/controller composition only
```

No central archetype row or second body registry should be required.

## AC7 hard exit criteria

AC7 and the campaign are DONE when:

1. D73's own definition of done is green against HEAD.
2. Dead actor mirrors removed in AC1 remain absent.
3. No mixed authoritative/read-model component is reconstructed every frame while selected fields are preserved by hand.
4. A new reaction-history fact has one obvious body-level lifecycle rather than family-specific maintenance lists.
5. Scheduler perturbation produces identical deterministic authoritative output.
6. A body-complete character uses one intrinsic construction authority across NPC/enemy/encounter/summon/match contexts.
7. `adopt_character_intrinsics` / build-then-patch construction is gone.
8. The legacy `CharacterRoster`/`ArchetypeSpec` enemy-body authority is gone.
9. Ordinary unresolved body identifiers are errors; temporary casting ambiguity is explicit at the owning content seam rather than encoded as global fallback ontology.
10. The three change-amplification probes require materially fewer independent authorities and central edit sites than the AC0 baseline.
11. No compatibility adapter created during this campaign remains without a real external consumer and explicit deletion condition.
12. Current planning/status documents describe the final architecture rather than the migration.

At that point this campaign stops. Do not continue into the successor campaigns merely because context remains.

---

# Opportunistic rules during AC0–AC7

These rules apply only to code already touched by the critical path. They are not separate work queues.

## Control identity

For every touched system, state which semantic it needs:

```text
controlled body
participant/seat
controller/input source
view subject
all bodies
original/home avatar
```

Prefer the existing representation of that semantic. New `PlayerEntity`/`PrimaryPlayer` uses in body-generic gameplay require explicit justification in review.

Do not rename every `PrimaryPlayer` use merely to modernize vocabulary; reduce inappropriate uses until its legitimate semantic remainder is clear.

## `shared_tangle`

When a touched lower/coherent domain imports `ambition_platformer2d_shared_tangle`, inspect the exact symbols used.

A carve is justified when it removes a demonstrably bad dependency edge for one coherent concept, following the `ambition_binding` precedent.

Prefer an existing natural owner. Create a new crate only when the concept has coherent semantics, multiple real consumers, and no suitable existing home.

Do not launch a broad `shared_tangle` LOC-reduction campaign from this document.

## Public facade

New consumer-facing work should use semantic facade surfaces. Avoid adding new implementation-shaped compatibility exports merely to make a migration convenient.

Facade cleanup is not a D73 gate unless the critical-path deletion makes an obsolete export trivially removable.

## Source policy checks

Prefer structural enforcement:

- Cargo dependency direction;
- private visibility;
- types;
- behavioral regression tests;
- existing architecture tests where they still express a subtle property.

Do not add a new source-text scanner for a property the new architecture can make impossible.

---

# Validation strategy

Use focused validation while iterating and the appropriate integration gate at the end of each slice.

Re-discover exact test names against HEAD rather than assuming this list stays current.

Useful current anchors include:

```text
cargo check -p ambition_app
cargo test -p ambition_app --test app_it -- rollback_schema_baseline
cargo test -p ambition_app --test desync_canary
cargo test -p ambition_app --test rollback_provoked_actor
cargo test -p ambition_app --test rollback_exit_oracle
```

Also run the narrow library/domain tests for every touched crate and the relevant character/construction tests for Ambition, Mary-O, Sanic, and Smash when their paths are touched.

For rollback-related P0 work, assert that the test actually exercises rollback/confirmation rather than inferring it from a harness setting. Use the existing execution counters/oracles where appropriate.

For authored character/schema changes, inspect Rust and authored RON/content. Rust call graphs alone are not sufficient evidence that an authored identifier or row is unused.

For pure organizational changes, do not update baselines merely to make a supposedly behavior-neutral change pass.

Do not use formatting commands or diff commands as validation gates. Validation is behavior, compilation, schema/architecture assertions, and production-shaped tests.

---

# Campaign measurements

Record these at AC0 and again at the relevant exit phase. They are diagnostic, not permanent ratchets.

| Measure | Baseline | Target |
|---|---:|---:|
| real production readers of `ActorIntent` | recompute | 0 |
| real production readers of `ActorCooldowns` | recompute | 0 |
| duplicate liveness/attack mirrors in body reaction state | recompute | 0 |
| save→rebuild→restore body-state sync sites | recompute | 0 |
| distinct actor-family reaction-timer maintenance lists | recompute | 1 semantic lifecycle per timer |
| body-complete character contexts bypassing canonical intrinsic construction | recompute | 0 |
| production `adopt_character_intrinsics` calls | recompute | 0 |
| legacy roster/archetype body-authority types | recompute | 0 |
| ordinary unknown-id → generic-body fallback roads | recompute | 0 |
| scheduler-perturbation deterministic divergence | establish AC2 | 0 |

The north-star measurement is not actor-monolith LOC. It is the number of authorities and central edit sites required by AC7's three change-amplification probes.

---

# Stop and redirect conditions

Stop the current slice and re-evaluate before merging when any of these occur:

- a new long-lived abstraction is added while the old representation/path remains equally usable and no deletion gate is named;
- a proposed D107/D108 fix adds more fields/lists to a mirror AC3 is supposed to delete;
- NPC convergence creates another body builder instead of using the canonical character-body authority;
- `CharacterSpawnPlan` starts accumulating unrelated context solely to force every path through one struct;
- D102 cleanup proposes another central exception registry instead of explicit owning-caller intent;
- a character receives invented health/capabilities merely to make a migration count reach zero;
- a new reaction/observation projection is rollback-registered without explaining what history would be lost;
- presentation becomes a simulation input;
- scheduler correctness is restored by ordering unrelated leaf systems rather than expressing the semantic dependency;
- body-generic gameplay gains a home-avatar/player marker dependency where `ControlledSubject`, participant authority, or a body query already represents the need;
- a new source-text policy test is proposed for a property that can be enforced structurally;
- work begins on rollback-registration inversion or full `tick_actor_brains` decomposition before AC7 closure.

Prefer a smaller reversible deletion slice over completing a diagram at the cost of another transition architecture.

---

# Successor campaigns — deliberately out of scope

These are recorded so the current agent understands the direction without beginning them early.

## Successor A — Decision pipeline decomposition

After AC7, remeasure `tick_actor_brains` from the simplified state model.

Expected direction:

```text
body/world maintenance
        ↓
complete observation/perception
        ↓
combat coordination
        ↓
derived brain input
        ↓
Brain::tick
        ↓
ActorControl
```

The goal is AI as a consumer of simulation facts rather than an orchestrator of the whole simulation. A small parameter count is a consequence, not the objective.

Do not preserve this document's old parameter inventory as requirements; AC changes the vocabulary first.

## Successor B — Rollback declaration ownership inversion

After AC has stabilized authoritative rollback types/schema churn, prototype the dependency inversion on the smallest leaf domains whose only runtime edge is rollback registration.

Expected architecture:

```text
domain owns "which of my facts rewind"
        ↓
backend-neutral generic registrar vocabulary keeps T generic
        ↓
runtime-owned GGRS adapter implements the mechanism
        ↓
composition selects participating capabilities
```

The proof should first remove real runtime→leaf dependency edges while keeping schema behavior/fingerprint stable. Do not migrate actor/combat rollback declarations until the small proof demonstrates the Cargo graph and composition join cleanly.

These successor campaigns are intentionally **not** AC7 gates.

---

# Final operating instruction to the implementing agent

Work in dependency order.

Before each slice, inspect HEAD and disprove stale planning claims with named symbols/tests. Then make the smallest change that removes an authority the next slice would otherwise have to understand.

The preferred sequence is:

```text
correctness hazards
→ dead mirrors
→ determinism guard
→ body/reaction authority
→ complete character bodies
→ common construction + delete patch path
→ delete archetype ontology
→ final naming/docs and D73 closure
```

Do not repair transitional mirrors that the next phase is intended to delete. Do not migrate every field of a legacy structure before deleting its root and seeing which fields still have consumers. Do not invent content decisions to make architecture metrics look clean.

The campaign succeeds when future feature work becomes obvious:

> **Domains own facts and history. Character owns the intrinsic body. Context owns control and placement. Derived views are projections. One body is constructed one way.**

