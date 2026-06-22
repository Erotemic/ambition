# Boss encounter → entity-local refactor

**Status:** in progress (started 2026-06-22)
**Owner:** executing model (Opus 4.8), with Jon
**Goal in one line:** "Spawn boss X (with tweaks Z) at position Y and it just works" — no
global encounter registration, correct for gauntlets / multiple bosses at once, with
phases as a trigger-driven property of the entity (its own mechanism, parallel to hitstun).

This doc is the recoverable source of truth for the refactor. If context is lost, read this
top-to-bottom, check `git log` for the stage commits, then resume at the first unchecked stage.

---

## Why (the smell)

A boss today is split across three places and the **entity is the least authoritative**:

- **`BossEncounterRegistry`** (a global `Resource`, `boss_encounter/registry.rs`) owns the live
  truth: `encounters: BTreeMap<encounter_id_string, BossEncounterState>`. `BossEncounterState`
  (`ambition_characters/src/boss_encounter.rs`) holds **HP, phase, the phase state machine,
  stagger, timers**, and emits music/cutscene/reward events.
- **`BossStatus.health` / `BossStatus.encounter_phase` on the entity are one-way MIRRORS**,
  overwritten from the registry every frame by `update_boss_encounters`.
- **The brain** (`BossPattern`) just reads the phase to choose patterns/movement.

Damage flow: player slash → `HitEvent` → `record_boss_damage(registry, runtime_id_string)`
(`boss_encounter/damage.rs`) → looks up the encounter via `runtime_ids: encounter_id → boss_id`
→ mutates registry state → mirrors HP back onto the entity.

### The core defect: live state is keyed by `encounter_id` (the archetype string), not by entity

- **Two of the same boss collide.** Spawn two mockingbirds → both `link_runtime("mockingbird", …)`
  and both mirror from the single `encounters["mockingbird"]` state → shared HP pool + shared
  phase. **Gauntlets / multi-boss rooms are broken by construction.**
- `active_phase()` returns "the one boss mid-fight" → music, lock-walls, HUD all assume a single
  global active encounter.
- Spawning a working boss depends on `update_boss_encounters` reaching into the global map and
  wiring it by string (`systems.rs` lazy register/link/wake). Invisible coupling.
- Phase is a monolithic 8-variant enum (`Dormant→Intro→Phase1→Transition→Phase2→Stagger→Enrage→
  Death`) imposed on every boss, with baked-in invulnerable beats — not a per-boss/brain property.

> NOTE: an earlier claim that "a programmatically-spawned boss isn't a registered encounter" was
> imprecise. `update_boss_encounters` DOES auto-register/link/wake any boss in the room by its
> `behavior.id`. The reasons a fresh spawned boss looked inert in the i-frame test were (a) the 2s
> `Intro` invulnerability and (b) a room-edge transition resetting it away — both orthogonal to
> this refactor. Fix the stale comment in `tests/boss_contact_iframes.rs` during Stage 1.

---

## Decisions (locked with Jon)

1. **Full refactor**, not a partial: entity-local HP + entity-local phase state + pluggable phase
   triggers + music/cutscene/rewards become entity-event consumers + delete the live registry map.
2. **Phase transitions are their OWN parallel mechanism**, not literally the hitstun/recoil code.
   They may *resemble* the "event → brief locked beat → exposed controls change" shape, but live in
   their own component/system so combat-feel and boss-phase code stay decoupled.

---

## DESIGN REFINEMENT (2026-06-22, with Jon) — encounter as a first-class OPTIONAL entity

The original target below said "everything the registry owned moves onto the boss entity." That's
half right. Jon's HUD / reuse / Smirking-Behemoth discussion sharpened it into a **three-layer
model**, where the encounter is NOT deleted — it is *promoted* to a first-class, OPTIONAL entity:

1. **Archetype (data)** — the reusable creature (a boss is an enemy with more HP/attacks). Reuse a
   boss as a normal enemy = spawn the archetype with NO encounter wrapped around it.
2. **Entity instance** — a specific spawned creature. Owns: HP, phase STATE + behavior, intrinsic
   phase triggers, and **per-instance payload** (e.g. a swallowed NPC), plus generic capabilities.
3. **Encounter (optional entity, references its members)** — owns ORCHESTRATION: a progress model +
   **HUD binding** (optional; headless-fine), lock-walls, win/lose conditions, music, and a
   **scripted event timeline**. Reusable structure: single-boss, add-wave, gauntlet, cut-rope puzzle.

Consequences for this plan:
- **Split `BossEncounterState`**: HP + phase-state half → the entity (Stage 1b over-moved the WHOLE
  blob; trim it). Thresholds-as-progress + music + lock-walls + HUD-binding + script → the encounter.
- The HUD is a *view bound to encounter progress* ("phase 2/3", "adds remaining" = encounter reading
  its members). No encounter → no HUD, no lock-walls, nothing required; the creature is just an enemy.
- The global `BossEncounterRegistry.encounters` map becomes a SET OF ENCOUNTER ENTITIES (each an
  entity with an `EncounterDef` + member refs), not a string-keyed resource the boss is a puppet of.

### Smirking Behemoth decomposition (the tricky case) — three generic pieces
- **Unkillable outside its encounter** = the existing generic `environmental_kill_only` flag.
- **Swallowed NPC** = INSTANCE payload, not encounter/archetype: a generic `Contains(npc)` +
  `ReleaseOnDeath` component. THIS entity frees ITS payload on death; a different instance has none.
- **Scripted move→crush→release** = an encounter SCRIPT. The release is NOT scripted — it falls out
  of the generic on-death capability. So the cut-rope script is only: rope-cut → move behemoth under
  boulder + drop boulder → on impact ForceKill → on death set victory music / reward / dialogue.

### Encounter-script shape (proposal)
An encounter entity carries an `EncounterScript`: ordered **beats** `{ when: Trigger, then: [Effect] }`
advancing as triggers fire.
- Triggers (observe world/members): `RopeCut`, `MemberAtPosition`, `HazardImpact`, `MemberDied`,
  `AllMembersDead`, `Timer`, `PlayerEntered`.
- Effects (command members/world): `CommandMoveTo`, `DropHazard`, `ForceKill`, `SetLockWalls`,
  `SetMusic`, `GrantReward`, (`ReleasePayload` only if ever wanted decoupled from death).
- Same `condition → effect-on-entity` shape as the phase-transition triggers — they can SHARE a
  trigger/effect vocabulary while staying separate mechanisms (a phase transition is a tiny built-in
  beat the entity owns intrinsically; the encounter owns the bespoke ones). Don't unify yet.

### Open forks (NOT yet decided)
- **Phase-up without an encounter?** Lean: phase state + intrinsic HP-triggers live on the entity, so
  a boss reused with no encounter still enrages at low HP (the encounter only FRAMES/displays phases
  and can ADD external triggers), with an opt-out knob. Confirm vs "no encounter = phase 1 only".
- **"Cleared" keyed to the encounter PLACEMENT, not the archetype.** Today save is archetype-keyed
  (Stage 1a kept that). Reuse story implies per-encounter-instance ("the cut-rope encounter is
  cleared"), so reusing the archetype elsewhere isn't pre-marked dead. Changes Stage 5.

---

## Target architecture (original — read together with the refinement above)

**Everything the registry owned becomes entity-local components + entity-emitted messages.**

### Components on the boss entity
- `BossStatus.health` — already there; becomes the SOURCE OF TRUTH. Death = `health.current == 0`.
- `BossEncounterState` — promoted from "value in a global map" to a **`Component`** on the entity.
  Keeps its existing pure logic (`apply_player_damage`, `tick`, thresholds, stagger). Its `phase`
  field is the authority; `BossStatus.encounter_phase` becomes a trivial read (or is removed).
- Phase-transition triggers as data on the entity (see "phase model" below).

### Reactions become message consumers (the existing pattern: `HitEvent`/`VfxMessage`/`EffectRequest`)
- The entity emits `BossPhaseChanged { entity, from, to }`, `BossDefeated { entity, encounter_id }`,
  `BossMusicRequested { entity, track }` (or reuse `BossEncounterEvent` carrying the entity).
- Music / cutscene / banner / reward / lock-wall systems subscribe per-entity instead of reading
  `registry.active_phase()`. Because events carry the entity, **multiple bosses never collide;
  correctness is emergent from per-entity state.**

### Registry shrinks to a read-only content catalog
- Keep `profiles: BTreeMap<id, BossProfile>` (authored thresholds/music/reward DATA) + `specs_loaded`.
- **Delete** `encounters` (live state) and `runtime_ids` (string routing). Spawn looks up the data
  catalog, applies tweaks Z, stamps components. No registration, no linking, no wake-by-global-map.

### Spawn
- `spawn_boss_at(id, name, pos, brain, overrides Z)`: resolve profile data → apply Z → spawn entity
  with `BossStatus{health}`, `BossEncounterState`, `Brain`, transition triggers. Done.
- The `SpawnActorRequest::Boss` seam (already built) gains optional `overrides` for Z.

### Persistence (the one thing the global gave us)
- "This boss is cleared" is a SAVE concern, not live combat. Keep a `HashSet<encounter_id>` of
  cleared bosses in save state, written when `BossDefeated` fires. Live fight is fully entity-local.

---

## Phase model (trigger-driven, its own mechanism)

Reframe phases the way Jon described — a transition is something that *happens to* the entity
(like getting hit triggers hitstun), gated by an external/internal trigger:

- `BossEncounterState` keeps `phase` + `phase_elapsed` + a new **`transition_lock: f32`** (the brief
  invulnerable "tell/scream" beat — its own field/timer, NOT the player's recoil_lock).
- **Triggers are pluggable data** (extend the existing threshold model):
  - `HpBelow(frac)` — the common case (already encoded as `phase1_to_transition_hp` etc.).
  - `TimeInPhase(s)` — intro tell (already encoded as `intro_seconds` etc.).
  - `External(gate: String)` — fired by a message (room switch, "all adds dead", cutscene cue).
    **This is the gauntlet/scripted hook.**
- **Default boss = no triggers → fights one phase until `health == 0`.** No forced Intro
  invulnerability unless the boss opts in. (Today every boss is forced through Intro; make it opt-in.)
- On a trigger firing: enter `transition_lock` (invulnerable + emit a "scream"/tell event), then on
  lock expiry swap the brain's exposed phase (patterns/movement/available actions). The brain owns
  *what each phase exposes*; this mechanism owns *when/how we move between them*.

Keep the existing named `BossEncounterPhase` enum as the phase VOCABULARY for now (existing bosses
are authored against it). Generalizing to arbitrary N phases can be a follow-up; the `External`
trigger + entity-local state already unlock gauntlets without that.

---

## Blast radius (files that touch the registry today)

Machinery (`ambition_gameplay_core`):
- `boss_encounter/registry.rs` — the resource (shrink to profile catalog).
- `boss_encounter/systems.rs` — `update_boss_encounters` register/wake/tick → per-entity.
- `boss_encounter/damage.rs` — `record_boss_damage` / `force_boss_death` → operate on a component.
- `boss_encounter/mod.rs` — re-exports.
- `features/ecs/damage/boss_hit.rs` + `damage/mod.rs` — boss-hit applies damage (string-routed today).
- `features/ecs/bosses/{mod,tick}.rs` — boss tick reads/syncs phase.
- `features/ecs/encounter_rewards.rs` — reward on defeat.
- `encounter/{registry,systems,lock_walls}.rs` — the *other* encounter layer (lock walls, music) reads `active_phase()`.
- `session/reset/mod.rs` — reset / retry (`reset_for_retry`).
- `persistence/save_data.rs` — save cleared bosses.
- `combat/boss_clusters.rs` + `features/bosses.rs` — spawn/profile resolution.

Characters (`ambition_characters`):
- `boss_encounter.rs` — `BossEncounterState` state machine (make it a `Component`).
- `brain/boss_pattern/mod.rs` — phase enum + `pattern_for`/`movement_for_phase`.

App (`ambition_app`):
- `app/hud.rs`, `app/sim_systems.rs`, `app/feedback.rs` — HUD/feedback read the active encounter.

Content (`ambition_content`):
- `bosses/cut_rope/{mod,arena,victory}.rs` — environmental boss uses `force_boss_death` + registry.
- `bosses/mod.rs` — boss data install.

---

## Staged migration (each stage must COMPILE; commit as a checkpoint; then keep moving)

- [ ] **Stage 0 — Safety net.** This doc. Plus a canary test
      `two_bosses_take_independent_damage` (spawn two same-archetype bosses, damage one, assert the
      other is untouched). Fails/awkward today (shared state); passes after Stage 1. Keep the
      existing `boss_contact_iframes` + full boss-encounter test suite green throughout.
- [x] **Stage 1a — Per-entity KEYING (the correctness win, landed first as the low-risk step).**
      The live `encounters` map is now keyed by the boss's UNIQUE runtime id (`config.id`), not the
      shared archetype `encounter_id`. Two same-archetype bosses get independent HP/phase/death.
      `record_boss_damage`/`force_boss_death` look up by runtime id directly; `update_boss_encounters`
      registers/wakes/ticks/mirrors per-entity; `sync_boss_encounter_phase` reads by `config.id`;
      cut-rope + rewards still route through the kept `runtime_ids` (archetype→runtime) link.
      Canary: `two_same_archetype_bosses_have_independent_encounter_state`. The map is STILL a global
      resource — Stage 1b moves it onto the entity.
- [x] **Stage 1b — State onto the entity (ADDITIVE foundation).** `BossStatus` now carries
      `encounter: Option<BossEncounterState>`, mirrored from the per-entity registry state every
      frame by `update_boss_encounters`. Purely additive (populated, not yet read), so zero behavior
      change — but the live encounter state now lives ON the entity, which is what readers (Stage 3)
      and then writers (Stage 4) migrate onto.
      - NOTE on sequencing: a full "component IS the source of truth, drop the registry" flip is a
        BIG-BANG, not incremental — every MUTATOR (player damage in `apply_boss_hit`, the cut-rope
        environmental `force_death`, retry-reset) and every READER (`sync_boss_encounter_phase`,
        HUD `active_phase`, music, lock-walls) must move together, or a registry-derived mirror
        overwrites whatever a half-migrated mutator wrote. So the authority flip is consolidated into
        Stage 4 (below); Stage 3 first migrates the READERS off the registry onto the entity copy.
      - Reader-migration ordering gotcha: `update_boss_encounters` (progression schedule) and
        `sync_boss_encounter_phase` (features schedule) have no explicit order, so a reader that
        switches to the entity copy may see a one-frame-stale phase unless ordered after the mirror.
- [ ] **Stage 2 — Phase transitions as triggers + transition-lock beat.** Add `transition_lock` +
      the trigger model (`HpBelow`/`TimeInPhase`/`External`). Default = fight-til-death (no Intro
      invuln unless opted in). Emit a "scream"/tell event on transition.
- [ ] **Stage 3 — Reactions become entity-event consumers.** Music / cutscene / banner / rewards /
      lock-walls subscribe to per-entity `BossPhaseChanged` / `BossDefeated` messages instead of
      `registry.active_phase()`.
- [ ] **Stage 4 — Delete the live registry map.** Remove `encounters` + `runtime_ids`; registry is a
      read-only data catalog. Update all remaining call sites.
- [ ] **Stage 5 — Save persistence.** `BossDefeated` → `cleared: HashSet<encounter_id>` in save.
- [ ] **Stage 6 — Spawn seam tweaks Z.** `SpawnActorRequest::Boss { overrides }` + `spawn_boss_at`
      applies hp/size/threshold overrides. Add a "spawn two different bosses, both fightable" test.

---

## Safety net / how we verify

- Differential harness = the existing test suite stays green + new per-entity tests:
  - `two_bosses_take_independent_damage` (Stage 0/1).
  - `spawned_boss_skips_to_phase1_and_dies_from_player_damage` (entity-local death).
  - `two_different_bosses_both_fightable` (Stage 6).
- Replay/behavior may legitimately change (per ADR-style bold-refactor rule); the gate is "it
  compiles + the boss tests encode the new contract." Commit each compiling stage.

## Risks / open questions

- The `encounter/` layer (lock walls, music gate) and the `cut_rope` content boss are the gnarliest
  consumers of `active_phase()`; they assume one global active boss. Per-entity may change their UX
  (e.g. which boss owns the music) — decide policy in Stage 3 (proposal: most-recently-aggroed boss
  owns music; lock-walls keyed per arena, not global).
- Disk on the dev VM is tight (was 100% full; freed the 32G `debug/incremental` cache). Watch space
  across the many heavy rebuilds; `rm -rf target/debug/incremental` is the safe pressure valve.
