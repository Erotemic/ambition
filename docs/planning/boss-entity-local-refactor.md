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

### Resolved decisions (Jon, 2026-06-22)
- **Phases are intrinsic-but-OPTIONAL DATA, not a mode.** A boss carries a (possibly empty) list of
  intrinsic health-gated phase triggers as DATA on the archetype/entity. Empty list → the boss just
  fights, no phase-up (works as a plain enemy). Non-empty → it phases up on its own, **with or
  without an encounter**. The encounter never *gates* phase-up; it only FRAMES/displays phases (HUD)
  and may ADD external (e.g. scripted) triggers. **Key requirement: the per-boss decision must be
  trivially changeable** — flipping a boss between "has phases" and "no phases" is editing its
  trigger DATA, never a code change. "Most bosses will have them; some won't" must be cheap.
- **"Cleared" is keyed to the ENCOUNTER (placement/instance), not the archetype.** Confirmed, no
  question. Reusing a boss archetype elsewhere is NOT pre-marked cleared. (Supersedes Stage 1a's
  archetype-keyed save; the per-encounter cleared key lands with the encounter entity in Stage B.)

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
> **The original Stages 2–6 below are SUPERSEDED by the refined model.** Execute the
> "Remaining work" list that follows instead. (Kept here only as the pre-refinement history.)
> - ~~Stage 2 — phase transitions as triggers~~ → folded into refined Stage R1
> - ~~Stage 3 — reactions as entity-event consumers~~ → refined Stage R2 (encounter ENTITY)
> - ~~Stage 4 — delete live registry map~~ → refined Stage R3
> - ~~Stage 5 — save persistence~~ → refined Stage R4 (now encounter-keyed)
> - ~~Stage 6 — spawn tweaks Z~~ → refined Stage R6

---

## ⇒ TASK FOR THE NEXT AGENT (refined remaining work) — START HERE

**Baseline already on `main` (anchors):** `92dd6f56` plan · `a3907567` Stage 1a (per-entity keying)
· `449868ad` Stage 1b (entity-local `BossStatus.encounter` copy) · `c1fb4ec9` design refinement.

**What is already true:** live encounter state is keyed per-entity AND copied onto
`BossStatus.encounter` each frame (additive, not yet read). The global `BossEncounterRegistry` still
holds the authoritative live map. The combat-feel tests (`boss_contact_iframes`) + the canary
`two_same_archetype_bosses_have_independent_encounter_state` + the 949 gameplay_core lib tests are
green — keep them green (the differential harness). Read the "DESIGN REFINEMENT" + "Resolved
decisions" + "Blast radius" sections above before starting; they are authoritative.

**Working rules:** each stage must COMPILE before committing (the only hard gate); commit each stage
as a checkpoint with a `boss:`-prefixed message; sign as the executing model + the Co-Authored-By
trailer; commit directly to `main`; stage explicit paths (never `git add -A`); watch disk
(`rm -rf ~/ambition-target/debug/incremental` is the safe pressure valve). Builds are ~10 min — batch
edits per stage, build once. `cargo`/test invocations use `-p ambition_app` (e.g.
`cargo test -p ambition_app --test boss_contact_iframes`).

- [x] **R1 — Split state + phases as intrinsic-OPTIONAL data (entity-local). LANDED.**
      Trim `BossStatus.encounter` to the ENTITY half: HP (already `BossStatus.health`) + phase state
      (current phase, `transition_lock: f32` tell/scream timer) + a **`Vec<PhaseTrigger>` of intrinsic
      triggers (DATA, may be empty)**. The encounter-only fields (per-phase music, thresholds-as-
      display, lock-walls, HUD) stop living on the entity — leave them on the profile/data catalog for
      now; they move to the encounter entity in R2. Build the phase-transition mechanism as its OWN
      entity-local component+system (parallel to hitstun, NOT shared per Jon): a trigger fires →
      enter `transition_lock` (invuln + emit a tell/"scream" event) → swap the brain's exposed phase.
      Triggers: `HpBelow(frac)`, `TimeInPhase(s)`, `External(gate: String)` (fired by message).
      **Empty trigger list ⇒ no phase-up, fights till `health==0`** (a boss reused as a plain enemy);
      a boss with triggers phases up on its own WITH OR WITHOUT an encounter. The decision is pure
      DATA and must be trivially flippable — no code change to add/remove a boss's phases. Drop the
      forced Intro invulnerability (make it an opt-in `TimeInPhase` trigger).
      - **What landed:** `PhaseTrigger { when: HpBelow|TimeInPhase|External, from: Vec<phase>, to,
        lock }` + `BossPhaseState { phase, phase_elapsed, transition_lock, triggers, start_phase }`
        in `ambition_characters::boss_encounter` (pure mechanism: `tick(dt, hp_fraction)` /
        `notify_external(gate)` / `wake()`, 8 unit tests). `PhaseTrigger::intrinsic_from_spec` derives
        a legacy spec's graph as DATA (intro opt-in, Phase1→Phase2 tell via `lock`, Phase2→Enrage).
        `BossStatus.encounter` is now `Option<BossPhaseState>` (the entity half), still mirror-only
        (`BossPhaseState::mirror_from`); the registry stays authoritative (flipped in R3). The
        entity-local Bevy system `tick_boss_phases` (`boss_encounter/phase_runtime.rs`) is built +
        tested as a real system but NOT yet registered — R3 wires it as the sole driver and bridges
        its `BossPhaseEvent`s to music/banner/VFX. Fixed the stale "not a registered encounter"
        comments in `tests/boss_contact_iframes.rs`. Green: 8 mechanism + 2 system + 67 boss_encounter
        lib tests + the 3 app boss tests (canary + both i-frame traces).
- [x] **R2 — Promote the encounter to a first-class OPTIONAL entity; migrate READERS. LANDED.**
      Introduce an `Encounter` entity with an `EncounterDef`: member entity refs + a **progress model
      DERIVED from member state** (single boss = its HP/phase; wave = adds remaining; etc.) + music +
      lock-walls config + win/lose condition + optional `EncounterScript` (see shape above) + optional
      HUD binding. A boss spawned with NO encounter just exists (no HUD/lock-walls/progress required —
      headless/RL-fine). Migrate readers onto the right layer: HUD reads encounter progress;
      `sync_boss_encounter_phase` reads the entity phase; music + lock-walls become per-encounter-
      entity properties instead of the global `active_phase()`. Keep the global live map alive in
      parallel still (deleted in R3) so this stage is reader-only and stays green.
      Ordering gotcha: a reader switched to the entity copy must run AFTER the mirror or see a
      one-frame-stale phase — order it explicitly.
      - **What landed:** new `boss_encounter/encounter_entity.rs` — `EncounterDef { placement_id,
        members, hud, win: EncounterWin::AllMembersDead }` + `EncounterProgress { members:
        Vec<MemberProgress{name,phase,hp,max_hp}> }` (a first-class entity, optional by construction).
        `sync_boss_encounter_entities` wraps each WOKEN boss in a single-boss HUD-bound encounter (R6
        adds the spawn-seam opt-out for an encounter-LESS boss); `update_encounter_progress` derives
        progress from member `BossStatus.health` + the entity-local `BossPhaseState.phase` copy and
        retires an encounter whose members all left the world. Both scheduled right after
        `update_boss_encounters` in the Progression chain. **Readers migrated:** HUD `boss_line` now
        reads `EncounterProgress` (one line per HUD-bound member) instead of `registry.active_phase()`;
        `sync_boss_encounter_phase` reads `BossStatus.encounter.phase` (entity copy) instead of the
        registry — both old/new sources are written by `update_boss_encounters`, so no new staleness
        (R3 must reorder this after `tick_boss_phases` once that becomes the writer). The global map
        stays authoritative. `BossEncounterRegistry::active_phase()` is now unused (deleted with the
        map in R3). Green: 3 encounter unit tests + 1 app integration test
        (`woken_boss_is_wrapped_by_an_encounter_entity_with_live_progress`) + 954 gameplay_core lib
        tests + 4 app boss tests.
      - **Scoping note:** music + lock-walls for bosses are NOT registry/`active_phase`-coupled today
        (boss music flows through `BossEncounterMusicRequest` events; boss lock-walls don't read
        `active_phase`). They are an *authority* concern, so they fold into R3 alongside the writer
        flip rather than being a separate reader migration here.
- [x] **R3 — Flip WRITERS + delete the global live map (the big-bang). LANDED.**
      Player damage (`apply_boss_hit`) mutates the entity HP/phase directly (drop string routing).
      Cut-rope environmental kill becomes an `EncounterScript` `ForceKill(member)` effect. The
      encounter win-condition observes members. `BossStatus.encounter` (the entity copy) becomes the
      source of truth; DELETE `BossEncounterRegistry.encounters` + `runtime_ids` (registry = read-only
      `profiles`/`specs_loaded` catalog only); remove/repoint `record_boss_damage`/`force_boss_death`.
      All mutators+readers move together here — half-migration is overwritten by the mirror.
      - **R3 surface map (scouted 2026-06-22, NOT yet executed).** The atomic edit set:
        1. **Register `tick_boss_phases`** (built in R1, `boss_encounter/phase_runtime.rs`) as the
           phase driver: add a `wake()` step (alive && Dormant ⇒ wake) + bridge the returned
           `BossPhaseEvent`s to the existing `publish_events` consumers (music/banner/cutscene).
           Order `sync_boss_encounter_phase` (reader) AFTER it.
        2. **`features/ecs/damage/boss_hit.rs::apply_boss_hit`** already holds the boss `BossMut`.
           Replace the `record_boss_damage(registry,…)` branch with direct entity mutation: swallow if
           `boss.status.encounter.boss_invulnerable()` (make that method also honor
           `phase.boss_invulnerable()` so Intro/Transition stay invuln), else `health.damage(amount)`;
           on kill set `encounter.phase = Death`. Drop the `boss_registry/music/cutscene` params.
        3. **New entity death-resolution system** (replaces the death half of `update_boss_encounters`):
           on a boss reaching `Death` (or `alive==false` first frame), do save `set_boss(Cleared)` +
           `QuestAdvanceEvent::BossDefeated` + victory banner + music restore. (R4 rekeys the save to
           the encounter placement.)
        4. **`update_boss_encounters` guts out**: delete register/wake/tick/mirror/music-lifetime/death.
           What may remain (or move): behavior-profile application, max_hp seeding, save-Cleared skip,
           reward-chest sync. Then **delete `BossEncounterRegistry.{encounters,runtime_ids}` + `ensure`/
           `link_runtime`/`get`/`active_phase`**; keep `profiles`/`specs_loaded`. Delete `damage.rs`
           (`record_boss_damage`/`force_boss_death`) + its tests.
        5. **`features/ecs/encounter_rewards.rs`** iterates `registry.encounters` + `runtime_ids` for
           reward-chest placement on defeat — repoint to the boss entities / save-Cleared.
        6. **`boss_encounter/systems.rs::boss_phase_transition_feedback`** iterates `registry.encounters`
           — read boss entity phases instead.
        7. **cut-rope content** (`ambition_content/src/bosses/cut_rope/`): `arena.rs` ALREADY kills the
           entity directly (`alive=false; health.current=0`) — just drop the extra
           `force_boss_death(registry,…)` call + set `phase = Death`. `victory.rs` reads
           `encounters.get(CUT_ROPE_BOSS_ID)` and `mod.rs::reset_cut_rope_boss_attempt` uses
           `runtime_ids` + `encounters.get_mut().reset_for_retry()` — repoint both to the boss entity.
           (Full `EncounterScript`/`Contains`/`ReleaseOnDeath` is R5; R3 only de-registries the kill.)
        8. **Tests to rewrite onto the entity contract**: the canary
           `two_same_archetype_bosses_have_independent_encounter_state` + the R2 integration test (read
           per-entity `BossStatus`/`EncounterProgress`, not `reg.encounters`); `damage.rs` unit tests
           (gone with the file → re-pin as entity damage tests); `systems.rs` `phase_feedback_tests`;
           `encounter_rewards.rs` tests.
      - **Verification gap (why this is a deliberate checkpoint boundary):** save-cleared persistence,
        reward-chest spawning, cut-rope victory + in-place replay, and adaptive-music restore have NO
        headless test coverage and can't be observed in this environment. Land R3 either (a) with new
        headless tests pinning each of those, or (b) blind in its own commit with an explicit
        "verify in-game" note (per Jon's blind-fix rule). Do NOT ship it silently as "done".
      - **R3 EXECUTED (2026-06-23).** `BossStatus.health` + `BossStatus.encounter` are the source of
        truth; the global `BossEncounterRegistry.{encounters,runtime_ids}` map + `ensure`/`link_runtime`/
        `get`/`active_phase` + `boss_encounter::damage` (`record_boss_damage`/`force_boss_death`) +
        `phase_runtime.rs`/`tick_boss_phases` are DELETED (registry = read-only `profiles` catalog).
        `apply_boss_hit` → `apply_entity_boss_damage` (swallow if `BossPhaseState::boss_invulnerable`,
        else `health.damage`, on kill `phase.kill()`). `update_boss_encounters` rewritten entity-based:
        per boss it SEEDS state from the profile catalog once, applies save-Cleared, wakes, ticks the
        `BossPhaseState`, bridges `BossPhaseEvent`→`publish_events`, resolves death (outro elapsed →
        save Cleared + quest) and keeps the music lifetime + reward-chest sync. `BossPhaseState` gained
        `boss_invulnerable` (delegates to the phase enum), death-outro timing (tick advances
        `phase_elapsed` during Death), and `kill()`. `boss_phase_transition_feedback`, the boss HUD,
        `sync_boss_encounter_phase`, reward placement (`encounter_rewards`, archetype-keyed anchors),
        and cut-rope (arena drops `force_boss_death`→`phase.kill()`; victory + replay read the save, not
        the map) all read/write the entity. Tests rewritten onto the entity contract (canary,
        `entity_damage_tests`, `phase_feedback_tests`, `mockingbird_profile_registers_in_the_catalog`,
        `boss_lifecycle` kill helper). **Green:** boss_lifecycle (2) + boss_contact_iframes (4) + 951
        gameplay_core lib + 63 boss_encounter lib. **IN-GAME VERIFY (headless-blind):** real boss fight
        feel, reward-chest spawn position, cut-rope anvil→victory NPC + in-place replay, adaptive-music
        crossfade. The old `BossEncounterState` machine survives only as roster spec-validation tests.
      - **NET LANDED (2026-06-22, pre-R3, "test-first" per Jon):** new
        `crates/ambition_app/tests/boss_lifecycle.rs` pins the generic death CONTRACT —
        `boss_music_plays_during_the_fight` + `defeated_boss_is_recorded_cleared_drops_reward_and_clears_music`
        (save Cleared + reward chest + music restore). Writing it immediately surfaced a **pre-existing
        regression**: Stage 1a re-keyed `encounters` to the runtime id but the music-lifetime `.get()`
        in `update_boss_encounters` still keyed by archetype id → boss music was set on wake then cleared
        the same frame (silently broken since 1a). Fixed (look up by runtime id) + logged in
        `dev/journals/lessons_learned.md`. The kill helper (`force_kill_boss`) is the only
        authority-coupled part — **R3 repoints it to the entity** and the assertions must stay green.
        Cut-rope victory NPC + in-place replay stay an in-game verification item (headless-hard; R5
        rewrites cut-rope), but cut-rope's death consequences ride the same generic path this net pins.
- [ ] **R4 — Save persistence keyed to the ENCOUNTER placement (not archetype).**
      `cleared: HashSet<encounter_placement_id>` written on encounter win. Supersedes Stage 1a's
      archetype-keyed save — reusing a boss archetype elsewhere is NOT pre-marked cleared.
- [ ] **R5 — Smirking Behemoth via the generic pieces** (see "decomposition" above). Add a generic
      `Contains(npc) + ReleaseOnDeath` instance-payload component (freed at the entity's position on
      death — NOT scripted). Express the cut-rope fight as an `EncounterScript`
      (rope-cut → `CommandMoveTo` behemoth under boulder + `DropHazard` → on `HazardImpact`
      `ForceKill` → death auto-frees the NPC). Keep `environmental_kill_only` (generic immune flag).
      Delete the bespoke `ambition_content::bosses::cut_rope` registry plumbing it replaces.
- [ ] **R6 — Spawn seam tweaks Z.** `SpawnActorRequest::Boss { overrides }` + `spawn_boss_at(...,
      overrides)` applies hp / size / phase-trigger overrides. Tests: spawn two DIFFERENT bosses both
      fightable; spawn a boss with NO encounter (plain tough enemy, no HUD); spawn a boss with
      overridden/empty phase triggers (proves phases are trivially-flippable data).

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
  consumers of `active_phase()`; they assume one global active boss. Under the refined model these
  become **encounter-entity** properties (R2/R3): music + lock-walls belong to the ENCOUNTER, so a
  room with no active encounter plays room music and a gauntlet's encounter owns its own music/walls.
  No more global "the one active boss" assumption.
- `BossEncounterState` currently bundles entity concerns (HP, phase) with encounter concerns (per-
  phase music, thresholds-as-display, HUD). R1 splits it; don't move the whole blob to the entity (as
  Stage 1b temporarily did) — that copy is trimmed to the entity half in R1 and deleted-as-mirror in R3.
- Disk on the dev VM is tight (was 100% full; freed the 32G `debug/incremental` cache). Watch space
  across the many heavy rebuilds; `rm -rf target/debug/incremental` is the safe pressure valve.
