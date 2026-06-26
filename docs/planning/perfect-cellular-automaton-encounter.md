# Perfect Cell-ular Automaton (PCA) encounter — design note & slice plan

Status: **in progress** (live progress window — Jon reads, can't ask).
Author model: Opus 4.8 (1M).

## Goal

An in-game encounter: the player meets the **Perfect Cell-ular Automaton** as a
talking NPC; the fight begins *only* if the player picks the "challenge"
dialogue option. In combat the PCA is driven by a smart, reactive fighter brain
that never cheats — it perceives only what a player could and acts only through
the same ability/ActionSet pipeline a player uses. The brain must be cleanly
swappable (hand-authored utility now, learned policy later).

Theme: the player is an AI seeking purpose; abilities are theorems; "every boss
is a failed objective function." The PCA is a cellular automaton; its ranged
tool is a **glider** (Conway Game-of-Life spaceship) — small diagonal CA forms
that travel in the characteristic glider gait, leaving a brief cell trail.

## What already exists (recon, 2026-06-26)

The brief's paths had drifted. Ground truth:

- **Universal brain seam = `Brain` enum → `ActorControlFrame`**, NOT
  `PlayerInputFrame`. `crates/ambition_characters/src/brain/mod.rs`:
  `Brain::tick(snapshot, &mut ActorControlFrame)` /
  `tick_with_actions(actions, snapshot, out)`. Module doc explicitly reserves
  `Remote`, `Scripted`, `RlPolicy` variants. **This is the RL-swappable
  interface the brief asks us to "define" — it already exists.** The brain
  emits abstract intent; per-actor `ActionSet` resolves it to concrete
  `ActorActionMessage`s (capability vs policy split). Determinism is pinned by
  tests; RNG seeds from a stable actor id.
- **The reactive fighter brain already exists**: `brain/smash/` — a 5-stage
  utility pipeline (observe → choose mode → choose action → difficulty filter →
  emit). `DifficultyProfile { reaction_delay_s, commit_probability, accuracy }`
  are the difficulty knobs. `SpecificAction` already covers Walk/Dash/
  MeleeAttack/RangedAttack/Dodge/Idle, capability-gated by `ActionSet`. Wired as
  `StateMachineCfg::Smash`.
- **Unified actor pipeline** is the spawn path to use (NOT the legacy boss FSM /
  `BossConfig` cluster). `EnemyActorSpawnPlan::hostile()` /
  `SpawnActorKind::Enemy { brain }` in
  `features/ecs/spawn_actors.rs`; ticked by unified `update_ecs_actors`.
- **PCA content already partly wired**: catalog entry `perfect_cellular_automaton`
  (`character_catalog.ron`), committed sprite sheet + manifest with clips
  idle/walk/crouch/jab/punch/block/jump/fly/special, an `Aerial` brain preset
  `cellular_automaton_raider`, `striker_swipe` action set. Body `Floating`.
- **Gaps confirmed**:
  - `CharacterAnim` has **no `Special` variant** → the PCA's `special` row is
    silently dropped at load (`character_sprites/anim/mod.rs::from_name`).
  - Smash brain has no Block / Fly-reposition / Blink-dodge / Special verbs.
  - `ObservationFrame` has no in-flight-projectile awareness (needed for
    reactive block/dodge).
  - No dialogue-choice → combat bridge. Dialogue is **Yarn**
    (`bevy_yarnspinner`, `assets/dialogue/sandbox/*.yarn`); NPC disposition
    flip peaceful→hostile exists (`ActorDisposition`), today only via strikes.
  - `ProjectileKind` is a **closed enum** (Fireball/Hadouken/HadoukenSuper). The
    glider must go through the data-driven enemy ranged/special path, not a new
    variant.

## Design decisions (resolved from project values, no user ask needed)

1. **Brain output = `ActorControlFrame` via `Brain`/Smash**, not `PlayerInputFrame`.
   The brief's `PlayerInputFrame` instruction is stale; `ActorControlFrame` is
   the frame-agnostic seam every actor (player/NPC/enemy/boss) already shares.
2. **Extend the Smash brain, don't fork a new one.** Add defensive/aerial/special
   verbs + projectile awareness to the existing proven pipeline. The "observe()
   / decide()" RL seam the brief wants = the existing observe→…→emit stages,
   each independently replaceable (already documented as such).
3. **Glider = data-driven special**, routed through `ActionSet.special` →
   `ActorActionMessage::Special` → a content-side effect consumer that spawns a
   CA-glider projectile. Avoids touching the closed `ProjectileKind` enum;
   keeps "another platformer adds a projectile by adding content" true.
4. **PCA is actor-like, on the unified pipeline.** Spawned as a peaceful NPC
   with dialogue; the challenge choice swaps `Brain` to the Smash fighter and
   flips disposition hostile + arms hostile volumes. No boss FSM, no
   `BossKinematics`.
5. **Engine vs content split**: generic machinery (Smash brain verbs, glider
   projectile *primitive*, `CharacterAnim::Special`, dialogue→provoke Yarn
   command) lives in core/engine; PCA stats/dialogue/placement/glider *tuning*
   live in `ambition_content`.

## Encounter state machine

`Dormant → Talking(dialogue) → {Combat | Disengage}`, `Combat → {Defeated | PlayerDefeated}`.
- Meeting: PCA is a peaceful NPC; `Interact` (E/F/RB or double-tap-Up) opens
  Yarn dialogue. Single-press Up must not trigger it (existing rule).
- Dialogue options: at minimum **"Challenge it"** (→ Combat) and a peaceful exit
  (→ Disengage). Optional lore lines (purpose / theorems / ethical-funding axis).
  Only the explicit challenge arms the fight.
- Transition: a Yarn command (`<<provoke_pca>>` / `<<start_encounter "...">>`)
  flips the PCA hostile, swaps in the Smash fighter brain, arms health + hurt/
  hitboxes + arena bounds.
- Resolution: PCA defeated = win; player defeated = loss; each resolves the
  encounter cleanly.

## Slice plan (small, compiling, committed)

- **S0** — this doc. ✅
- **S1** — Consume the PCA `special` clip end-to-end: add `CharacterAnim::Special`
  + `from_name("special")`, map it in `pick_enemy_anim` when a special verb is
  active. Make PCA a fightable combat actor on the unified pipeline with a
  **Smash fighter** brain preset + an action set (melee + ranged) — verify it
  spawns & animates. (No glider yet; uses existing ranged verb.)
- **S2** — Encounter SM + dialogue gate: peaceful NPC PCA with Yarn dialogue;
  `<<provoke>>`-style command flips it to the hostile Smash brain. Win/loss
  resolution. Bevy world-assert tests for the gate (only challenge arms combat).
- **S3** — LDtk placement of the PCA NPC encounter via `ambition_ldtk_tools`
  (add a spec / subcommand if missing; never hand-edit JSON). Roundtrip-check.
- **S4** — Glider projectile primitive (engine, data-driven) + PCA glider
  special (content). CA-cell visual + brief trail. Owner/seq stable-id ordering.
- **S5** — Extend Smash brain: Block (reactive, within reaction window), Blink
  dodge, Fly reposition, Special(glider) zoning verbs; enrich `ObservationFrame`
  with in-flight projectile awareness; surface difficulty knobs (reaction time,
  decision freq, aggression, risk, execution, max combo). Headless harness:
  brain vs dummy proves it reacts within latency, blocks reactively, respects
  cooldowns (i.e. doesn't cheat).
- **S6** — Finalize design note: observation/action interface, difficulty knobs,
  how to drop in a learned policy (the `Brain::RlPolicy` seam).

## Out of scope
Sprite/rig polish; training an RL policy (leave the seam); rebalancing player
abilities.

## Progress

- **S0** ✅ design note + slice plan committed.
- **S1a** ✅ consume PCA jab/punch/special clips: `CharacterAnim::Punch` +
  `Special`, `from_name` aliases, `pick_enemy_anim` routing via new
  `EnemyAnimState.attack_heavy`/`special_active` (false until S4/S5), non-looping
  marks. Tests green.
- **S1b** ✅ data-expose the Smash reactive fighter as `BrainPreset::Smash` +
  resolver mapping; author `cellular_automaton_fighter` catalog preset (MEDIUM
  difficulty, dash-to-close). Roster + resolver tests green.
- **S2** ✅ encounter gate. The generic `<<challenge>>` Yarn command +
  `ActorStimulus::Challenged` provoke the dialogue speaker unconditionally
  through the existing `provoke_actor_in_place` flip. `DialogState` now records
  the speaker entity. PCA dialogue authored in `symmetry.yarn`
  (`perfect_cellular_automaton`: CA/glider/objective-function theme,
  ethical-funding lore, a gating "Challenge it" + peaceful exit). Hostile
  archetype `cellular_automaton_fighter` (content RON, Smash, 18 HP boss);
  `hostile_brain_id_for_actor` PCA branch. Tests: challenge flips peaceful→
  hostile-Smash with zero strikes; passive un-challenged NPC ignores damage;
  PCA resolves to its boss archetype by id/name/dialogue.
- **S3** ✅ LDtk placement. PCA NpcSpawn in `symmetry_room` (Noether Chamber)
  at the kernel's down face — peaceful + stationary (catalog hostile Aerial
  brain is correctly rejected by `npc_brain_from_catalog` → stand_still).
  Authored via `entity add` (surgical) + mirrored in the room generator. All 72
  LDtk load/validate tests green.

- **S4a** ✅ fix the float→ground desync on provoke. `provoke_actor_in_place`
  overwrote tuning but not `surface.gravity_scale`, so the Floating PCA would
  freeze mid-air when hostile (aerial integrator reads `velocity_target`, which
  the grounded Smash brain never sets). Now provoke re-syncs gravity to the
  hostile archetype's locomotion mode; the PCA descends and fights via the
  tested grounded Smash path. Test pins it. **The encounter is now a fully
  functional reactive melee boss.**
- **S5 (reaction latency)** ✅ the never-cheats core. `reaction_delay_s` was
  inert; now `SmashState.obs_history` makes the brain perceive a lagged opponent
  (~150 ms for the PCA via MEDIUM). Headless tests prove it can't frame-perfectly
  counter. Difficulty knobs documented.
- **S6** ✅ design note `docs/design/pca-fighter-brain.md` (observation/action
  interface, difficulty knobs, reaction-latency guarantee, RL-policy seam).

**Deferred depth** (verbs whose `ActorControlFrame` bits already exist; tracked
in the design note):
- glider special (CA-themed diagonal projectile via the data-driven ranged path),
- reactive block (`shield_held` + an incoming-threat read in `ObservationFrame`),
- blink dodge (`blink_pressed`),
- fly / aerial-Smash (emit `velocity_target` + 2D pursuit so the PCA fights
  airborne instead of descending).

## How to try it
Walk into the symmetry_room (Noether Chamber), approach the Perfect Cell-ular
Automaton hovering at the kernel's down face, press Interact, and pick
"Challenge it" — it flips to a hostile reactive Smash fighter. "Leave it in
peace" exits without a fight. (Runtime-verify in the GUI; headless tests cover
the gate logic + LDtk load, but the in-world feel is unverified by me.)

## Wall-clock log
- S0 design note: started 2026-06-26.
- S1a + S1b: same session, 2026-06-26. Engine-side foundation (anim consumption
  + data-exposed fighter brain), all incremental builds <20s, tests green.
</content>
</invoke>
