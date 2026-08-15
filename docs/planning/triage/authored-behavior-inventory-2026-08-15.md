# M0 inventory — authored/semi-authored behavior systems (census of 2026-08-15)

**Role:** EVIDENCE for
[`../engine/authored-gameplay-logic-and-orchestration.md`](../engine/authored-gameplay-logic-and-orchestration.md).
The plan owns the decisions; this file owns the classification that produced
them. ⛔ do not treat a row here as authority — it is a measurement with a date.

⚠ **two corrections applied after the census, by re-measurement:**

1. ⛔ **the `Brain` cursor's `_` arm is NOT a rewind gap.** The census read six
   families (Patrol, MeleeBrute, Skirmisher, Sniper, ChargeCrash, Aerial) as
   failing to rewind their timers. `rollback_component_cursor` calls
   `rollback_component_with_clone::<T>`, so bevy_ggrs clone-snapshots the whole
   component and `SnapshotCursor` feeds only the **checksum**. Those families
   rewind correctly; what they lack is **desync-detection coverage**, and a new
   brain family silently inherits the same blindness.
2. ⚠ the gate portal's rollback waiver remains **unverified either way** — no
   rewind was shown to land mid-`Opening`. If it is a defect it is one on its own
   merits, not evidence for this program.

The census's own confidence notes are preserved at the end. They were accurate
about their own weakest claims, which is why both corrections were findable.

---

# M0 — Authored/semi-authored behavior inventory (Ambition @ HEAD `38b77c749`, 2026-08-15)

Read-only census for `docs/planning/engine/authored-gameplay-logic-and-orchestration.md` § M0.
All paths absolute. `.claude/worktrees/` and `target/` excluded from every search (stale copies).
No `cargo` was run. Every rollback/persistence claim is sourced to a **registration site** or is
explicitly marked inferred.

---

## Part 0 — Verdict up front

**Program falsifier #1 is NOT tripped for the two selected proof customers.** But the census found
**four structural incompatibilities** (§B), and they point at a sharper conclusion than "build one
substrate" or "don't":

> ⭐ **The gap is on the CONDITION side, not the effect side, and sequencing should NOT be unified.**
>
> - There is **no shared condition/predicate type anywhere in the workspace.** Every gate is a
>   bespoke hand-rolled read at its own call site. Six of the fourteen systems inventoried have no
>   condition vocabulary at all.
> - The effect side is far healthier: five-plus typed per-channel command buses already exist, and a
>   monolithic `GameplayEffect` enum **already existed and was already deleted** in favour of them
>   (`.../features/ecs/effect_bus.rs:6`). The god-enum failure has been met and rejected once.
> - Five independent ordered-beat executors exist and their execution models genuinely differ
>   (monotonic cursor · reversible timer machine · subroutine stack with interrupts · float timeline
>   with overlapping windows). **Unifying sequencing would require a branch naming the customer.**
>
> Therefore the substrate that is warranted is: **conditions + commands + prepared references +
> preparation + discovery.** Sequencing stays domain-owned.

---

## Part 1 — The inventory

### 1. `EncounterScript` — ordered beat timeline
`/home/joncrall/code/ambition/crates/ambition_encounter/src/timeline.rs:123`
(executor `/home/joncrall/code/ambition/crates/ambition_platformer2d_actor_monolith/src/boss_encounter/encounter_script.rs:39`)

| Axis | Finding |
|---|---|
| Condition | Closed 4-variant `EncounterTrigger { Gate(String), MemberDied(usize), AllMembersDead, Timer(f32) }` (`:39`). Pure `holds()`. |
| Effect | Closed 6-variant `EncounterEffect { ForceKill, Banner, SetMusic, CommandMoveTo, DropHazard }` (`:78`), yielded as *requests* the host executes. |
| Sequencing | Ordered list, monotonic cursor, **one beat max per tick**, no branch, inert past the end (`:151`). |
| Runtime state | `EncounterScript { beats, cursor: usize, elapsed: f32 }` — **Component**. |
| Reference | Member **index `usize`** into `EncounterParticipants` + raw `String` gate names. |
| Rollback / persist | ⛔ **NOT rollback state, and neither registered nor waived.** No `SnapshotState` impl anywhere (`crates/ambition_encounter/src/snapshot_impls.rs` covers `EncounterLifecycle`, `EncounterParticipants`, `EncounterWaves` — not this). Compensated by **despawning the encounter entity and rebuilding the script** on reset (`features/ecs/reset.rs:268`). Not saved. |
| Preparation | ⛔ **Not authored.** No serde derives on `EncounterTrigger`/`Effect`/`Beat`/`Script`. The one production script is a Rust literal (`game/ambition_content/src/bosses/cut_rope/mod.rs:321`). |
| Inspection | None. |

**One production customer in the entire workspace:** the cut-rope Smirking Behemoth fight.

### 2. `EncounterLifecycle` + `Objective` — the generic reducer
`/home/joncrall/code/ambition/crates/ambition_encounter/src/lifecycle.rs:79`, `/home/joncrall/code/ambition/crates/ambition_encounter/src/objective.rs:177`

Condition: ⭐ `Objective { AllWithRoleDefeated, AnyWithRoleDefeated, Survive(f32), ReceiveSignal(String), All(Vec), Any(Vec) }` — **the only composable boolean condition tree in the workspace**, with a deliberate refusal of any `Custom(String)` escape hatch. Ingress: closed `EncounterCommandKind { Start, Complete, Fail, Signal(String), Reset }` (`lifecycle.rs:245`). Sequencing: 5-phase machine `Inactive → Starting{remaining} → Active → Completed|Failed` + `Reset`. State: `{ phase, intro_seconds, elapsed_active, signals: BTreeSet<String> }` (`BTreeSet` chosen for canonical snapshot bytes). Rollback ✔ hand-written `SnapshotState` (`snapshot_impls.rs:317`,`:361`) registered at `rollback/domains/encounter.rs:54`,`:58`,`:62`,`:66`. Saved ✔ but projected to terminal-only `PersistedEncounterState { Untouched, Cleared, Failed }` — in-flight collapses to `Untouched` (`lifecycle.rs:115`).

⭐ **The latch for a multi-input condition is `signals: BTreeSet<String>`** — monotonic within an activation, cleared only on `Reset` (`lifecycle.rs:152`,`:175`,`:184`).

### 3. Wave director (`EncounterWaves` / `EncounterRun`)
`/home/joncrall/code/ambition/crates/ambition_encounter/src/waves.rs:42`

Condition implicit (runs while `Active`). Sequencing: timeline with per-mob `delay: f32` seconds. Reference: mob `character` is a **content-pack `PendingRef` against the `character` schema** (`content_schema.rs:51`) — the only prepared cross-schema reference in this inventory. Rollback ✔ `SnapshotResolve` (`snapshot_impls.rs:398`) registered at `domains/encounter.rs:66`. Preparation ✔ full `ContentSchemaHandler` with sorted diagnostics and `deny_unknown_fields` as contract. Inspection ✔ in the CLI registry.

⭐ **The best "authored → validated → prepared → typed runtime" pipeline of any sequencer here.**

### 4. Boss scripted patterns — ⭐ **the one family that already did this correctly**
`/home/joncrall/code/ambition/crates/ambition_characters/src/brain/boss_pattern/`

| Axis | Finding |
|---|---|
| Condition | Three closed enums, deliberately no scripting language. `SituationBucket { PlayerNear, PlayerFar, PlayerAbove, PlayerBehind, HpBelow(f32) }` (`mod.rs:263`) evaluated only over `BossPatternContext` (`control_flow.rs:44`). `InterruptTrigger { OnHitTaken{min_damage}, OnPhaseEnter{phase}, OnTimer{every_s} }` (`mod.rs:284`). |
| Effect | Closed `BossPatternStep { Telegraph, Strike, Rest, Select{table}, Stance{id} }` (`mod.rs:167`). Writes `BossAttackIntent`, which `features/ecs/bosses/tick.rs:155` turns into a `MovePlayback` — so boss attacks execute on the move timeline (§6). |
| Sequencing | ⛔⛔ **Richest in the tree**: per-phase timeline that **loops at end**; `Stance` is a **subroutine with a return stack**; `interrupts` **preempt the cursor**; `Select` is a **weighted random splice resolved once at timeline-resolution**, depth-limited to 4 (`control_flow.rs:99`). |
| Runtime state | `BossPatternState` (`mod.rs:724`) — an opaque struct nested inside the `Brain` component: `step_index`, `step_elapsed`, resolved `timeline`, `stance`, `stance_stack`, `interrupt_cooldowns/timers`, `last_hp`, **`rng_seed`**. |
| Reference | Raw `String` for stance ids, strike-geometry keys, technique keys, music tracks; phase is an enum. |
| Rollback | ✔ **Registration found** — `rollback_component_cursor::<Brain>` at `rollback/domains/characters.rs:51`, with `SnapshotCursor for Brain` (`ambition_characters/src/snapshot_impls.rs:339-384`) hand-encoding `step_index`, `step_elapsed`, the resolved timeline, the stance stack and both interrupt vectors. Phase side separately at `domains/combat.rs:132`,`:134`. |
| Preparation | ✔✔ Full schema family in `boss_pattern/content_schema.rs`: `boss_profiles`, `boss`, `boss_encounter` (aggregating 9 files), `boss_seed_library`, `boss_validator_bands`. Cross-refs resolved at compile. Real authored data: `game/ambition_content/assets/data/boss_profiles.ron`, `game/ambition_content/assets/data/boss_encounters/*.ron`. |
| Inspection | ✔✔ **Best-tooled system in the tree.** In the CLI registry (4 schemas), *plus* a design validator (`boss_pattern/validator.rs`) checking telegraph proportionality, answer coverage, punish windows and telegraph distinctness against data bands. Currently diagnostic/non-blocking, run by `game/ambition_content/tests/boss_fight_validator.rs`. |

Phase triggers are a separate guarded transition **graph**: `PhaseTrigger { when: PhaseTriggerCondition { HpBelow(f32), TimeInPhase(f32), External(String) }, from: Vec<Phase>, to: Phase, lock: f32 }` (`ambition_characters/src/boss_encounter.rs:95`,`:113`). ⚠ A legacy twin still ships beside it: hardcoded `phase1_to_transition_hp`/`phase2_to_enrage_hp` (`:36-46`).

### 5. Cutscene beats
`/home/joncrall/code/ambition/crates/ambition_cutscene/src/lib.rs` (992 lines, one file)

Condition: effectively none — a room-entry binding table (`:407`) plus one save-flag guard `seen_flag` (`platformer2d_actor_monolith/src/cutscene.rs:67`), which is a **genuine latch** (once seen, never replays). Effect: closed `CutsceneBeat { Wait, Dialogue, CameraPan, Fade, SetFlag, Banner }` (`:20`) — ⚠ `CameraPan` and `Fade` are documented unfinished **with no readers**. Sequencing: ordered list, monotonic `beat_index`, per-beat durations. State: `CutsceneRuntime { script, beat_index, elapsed, finished }` inside the **Resource** `ActiveCutscene` (`:313`) — ⛔ **singleton, exactly one cutscene may ever play**. Reference: raw `String` everywhere; camera target a bare `[f32;2]`.

Rollback: ✔ registered at `rollback/domains/cutscene.rs:32`,`:46`. ⛔⛔ **but the codec at `lib.rs:554` encodes the ENTIRE PROGRAM into every rollback frame**, including each `Dialogue` beat's speaker and body text. Immutable program and mutable cursor are fused in the wire format.

Preparation: typed **in Rust** at startup (`game/ambition_content/src/intro/cutscene.rs:37`); serde derives exist but **there is no loader and no file format**. Inspection: none.

Does **not** pause the sim: `GameMode::Cutscene` exists (`shared_tangle/src/schedule.rs:587`) but **has no setter anywhere**; suppression is per-seat input context only.

### 6. Move / action timelines (`MoveSpec` / `MovePlayback`)
`/home/joncrall/code/ambition/crates/ambition_entity_catalog/src/lib.rs:703`, `/home/joncrall/code/ambition/crates/ambition_combat/src/moveset/mod.rs:196`

⭐ **Indexed by SECONDS (`f32`), never by frame number** — `MoveWindow { start_s, end_s }` (`:603`), `MoveEvent { at_s }` (`:659`), `MoveSpec::duration_s` (`:708`); clock is accumulated float proper time (`moveset/mod.rs:576`). Nothing named `EventFrame` exists anywhere.

Condition: times + closed `WindowTag { Startup, Active, Recovery, Invuln, Armor, Cancelable{into, condition} }` (`:201`); the only other gates are `MoveGates.grounded: Option<bool>` and `CancelCondition { Always, OnHit, OnWhiff }`. Effect: `Active` windows spawn hitbox entities; `MoveEvent { at_s, kind: { Sfx, Vfx, Effect(EffectRef), Ranged } }` emitted as `MoveEventMessage`; `sustain_effect` fires **every tick** in-window. Sequencing: float timeline with **overlapping windows** and hitbox tracks. Reference: raw `String` throughout — move ids resolved by *linear scan* (`:1202`), `hit_targets` deduped by `Vec<String>`.

Rollback: ✔ **registration found** — `rollback_component_resolved::<MovePlayback>` at `rollback/domains/combat.rs:127` + `rollback_map_entities` at `:131`, checksum at `ambition_combat/src/snapshot_impls.rs:157`. ⚠ **`MovePlayback` clones the entire `MoveSpec` per playing entity** (`moveset/mod.rs:197`) — the same program/cursor fusion as the cutscene.

Preparation: typed and validated (`EntityCatalogDoc::validate`, `lib.rs:1431`; `presentation_problems`, `:786`), **not** in the content pack. ⚠ Exception: `EffectRef.params` is an opaque `ron::Value` hydrated **at fire time** (`ParamValue::hydrate`, `lib.rs:110`), which combined with `sustain_effect` is a per-tick `ron::Value` → struct deserialize on held moves.

⚠ **The authored-move seam exists and is unused:** `MovePrefabRegistry` (`ambition_combat/src/moveset/prefab_registry.rs:35`) maps `key + params → MoveSpec` and has **zero production callers**; `CharacterDefinition.moveset` is serde-capable with zero authored consumers; **no `.ron` file in the repo contains `windows:`**. Every `MoveSpec` is built in Rust.

### 7. Yarn dialogue
`/home/joncrall/code/ambition/crates/ambition_dialog/` + `/home/joncrall/code/ambition/game/ambition_content/src/yarn_vocabulary.rs`

Condition: ⭐ **the richest condition vocabulary in the repo, and reachable only from dialogue** — Yarn `<<if>>` over 3 published `$variables` (`bridge.rs:255`) plus 7 library functions `boss_cleared`, `flag`, `visit_count`, `quest_active`, `inventory_has`, `wallet_balance`, `can_afford` (`yarn_vocabulary.rs:462-522`), all pure reads of a per-frame snapshot `YarnStateMirrorData` (`bindings.rs:42`).

Effect: ⭐ **open registry, no central enum** — `add_command` on `runner.commands_mut()`, each name → a registered Bevy system. 15+ author-visible commands: `present_speaker`, `portrait_clip`, `set_flag`, `clear_flag`, `challenge`, `use_brain`, `restore_brain`, `give_item`, `buy_item`, `sell_item`, `spawn_chest`*, `play_sfx`, `spawn_fireworks`, `camera_zoom`*, `watch_cut_rope_video`, `reset_cut_rope_room`, `duel` (* = logged stub).

Sequencing: full branching state machine in third-party `bevy_yarnspinner` (nodes, 170 `<<jump>>`s, options). State: ⛔ opaque third-party `DialogueRunner` component holds the real program counter and variable storage. Reference: node names raw `String`; ⭐ participants cross into the sim as **`SimId`** (`yarn_vocabulary.rs:229`; a comment at `:233` records that passing raw `Entity` was a bug).

Rollback: ⛔ **no registration site for any dialog type**; namespace-waived (`rollback_coverage.rs:75`). The sim-side conversation authority *is* registered (`domains/actors.rs:196`,`:226`). Preparation: compiled up front (7 `.yarn` files `include_str!`-embedded) but **interpreted at runtime**, with per-invocation string resolution (`SfxId::new(&id_str)` hashes a string per `play_sfx`). Inspection: a text arity validator with a **hand-maintained 14-entry table** (`game/ambition_content/tests/dialogue_lint.rs`); no CLI.

⭐ **The Yarn→sim seam is the best-engineered effect path in the repo**: `NarrativeInputWriter` → `NarrativeInputLedger<M>` → `release_narrative_inputs` at the head of the sim schedule (`conversation/ledger.rs:195`,`:256`,`:327`) — instance-stamped, `next_tick`-stamped, edge-released, horizon-pruned.

### 8. Save flags — the de-facto world-fact store
`/home/joncrall/code/ambition/crates/ambition_persistence/src/save_data.rs:117` (`PersistedFlag`), `:217` (`flags`), `:395` (`flag`), `:403` (`set_flag`)

Global, flat, **raw `String` keys, no runtime registry**. Names built by `format!` at ~8 sites (`npc_{id}_talked`, `enemy_{id}_dead`, `switch_{id}_used`, `room_visited_{id}`, …). Write funnel: `SetFlagRequested` (`ambition_combat/src/events.rs:104`) → `apply_flag_effects` (`features/ecs/effect_bus.rs:16`), which also mirrors into `QuestAdvanceEvent::FlagSet`. ⚠ **Four sites bypass the bus** with direct `save.data_mut().set_flag`: `encounter/systems.rs:363`,`:364`; `menu/map/systems.rs:23`; `game/ambition_content/src/encounters.rs:139`; `game/ambition_content/src/quest.rs:266`.

Rollback ✔ `rollback_resource_clone::<AmbitionGameSave>(ENGINE, "resource.sandbox_save")` at `rollback/mod.rs:277`; `clear_message_on_rollback::<SetFlagRequested>` at `domains/combat.rs:275`,`:295`. Saved ✔. Latching: **not by construction** — `set_flag(id,false)` deletes the row and Yarn `<<clear_flag>>` is wired; in practice every producer writes `true`.

⚠ Validation exists but only partially: `authored_flag_ids(project)` (`game/ambition_content/src/content_validation.rs:623`) re-derives legal names from LDtk and errors when a *quest step* names an unknown flag (`:471`). **Nothing validates flags referenced from Rust or from Yarn.**

A second, parallel bool store sits beside it: `PersistedSwitch` (`save_data.rs:49`,`:211`,`:332`,`:340`).

### 9. Switches
`/home/joncrall/code/ambition/crates/ambition_encounter/src/registry.rs:61`, `/home/joncrall/code/ambition/crates/ambition_platformer2d_actor_monolith/src/encounter/switches.rs`

`SwitchActivation { id, action, target_encounter }` — three raw `String`s parsed once from the LDtk wire string `switch:<id>:<action>:<target>` (`registry.rs:74`). ⛔ **but `action` is re-string-matched inside the simulation**: `strip_prefix("SetGravity")` at `encounter/systems.rs:392` and `matches!(action, "ResetEncounter")` at `:405`, with a silent `continue` for anything unrecognised and a `_ => Down` fallback for a mistyped direction. Open at the author's keyboard, closed in code.

`SwitchOn(bool)` **latches on** in the interact system (`features/ecs/interact.rs:218` never sets false); the un-latch is the save mirror (`features/ecs/save_sync.rs:158`). Rollback ✔ registration sites at `domains/actors.rs:465` (`SwitchOn`, probed), `:470` (`SwitchFeature`), `:256` (`SwitchActivationQueue`), `:762` (`EncounterSwitchIndex` declared **derived**, not snapshotted).

`EncounterSwitchIndex::encounter_armed` (`switches.rs:30`) is a genuine N-input reduction — but it is a hardcoded `any(!on)` OR/NOR, not authorable, not composable.

### 10. ⭐ Noether "symmetry attunement" — the ONE shipped multi-input AND
`/home/joncrall/code/ambition/game/ambition_content/src/encounters.rs:75`

`Objective::All([ReceiveSignal("gravity_down"), ReceiveSignal("gravity_left"), ReceiveSignal("gravity_up"), ReceiveSignal("gravity_right")])` — **four switches, all of which must be activated, in any order, across time.** Switch ids `kernel_switch_down/left/up/right` (`encounters.rs:41`) are authored in `sandbox.ldtk`; the bespoke adapter `drive_symmetry_attunement` (`:88`) maps `SwitchActivated.activation.id` → a stable signal key; payout at `celebrate_symmetry_attunement` (`:121`). The accumulator is `EncounterLifecycle.signals: BTreeSet<String>`.

⭐⭐ **This is the single most important content finding in the census.** The doc's motivating example is *"when two switches are active, power a lift"*. That mechanism exists in shipped Ambition content **exactly once**, and to get a composable AND it had to be **dressed up as an encounter** — a room puzzle wearing an encounter costume, with a hand-written adapter to translate switch presses into encounter signals. That is the cost of having no world-level condition vocabulary, measured in real content.

### 11. Intro flag chains + flag-gated lock walls
`/home/joncrall/code/ambition/game/ambition_content/src/intro/route_state.rs` (215 lines + tests)

`INTRO_FLAG_CHAINS: &[(&str,&str)]` (`:32`, 5 rows) — "trigger set AND target unset ⇒ emit `SetFlagRequested`", 14 lines of Rust (`:63-76`), idempotent by the `!data.flag(target)` guard. Includes a 2-hop chain (`switch_gate_official_report_used → alice_route_note_reported → private_routes_compromised`) resolved **one hop per frame**.

`INTRO_FLAG_GATED_LOCK_WALLS: &[(&str,&str)]` (`:89`, 2 rows) — while the flag is clear, `compute_intro_flag_gated_lock_walls` (`:106`) walks **every LDtk level's entity list** for `LockWall` entities whose `id` field matches, and `sync_intro_flag_gated_lock_walls` (`:176`) pushes `intro_lock:<id>` solids into the per-frame overlay, behind a hand-written `Local<IntroLockWallCache>` with a three-input invalidation rule that exists purely because the uncached version measured ~1.8% of a headless boss profile.

⚠ **Registered in `Update`, not the sim schedule** (`intro/plugin.rs:99-107`) while its consumer `apply_flag_effects` runs in the sim `GameplayEffects` set — so chained flags land a frame later and **outside the rollback-ordered sim**.
⚠ **Four of the five chain targets are dead vocabulary**: nothing reads `map_private_marks_unlocked`, `route_memory_received`, `map_basic_unlocked`, or `private_routes_compromised`. Only `bob_field_survey_received` (a *trigger*) gates anything.

### 12. Gate portal phase machine
`/home/joncrall/code/ambition/crates/ambition_platformer2d_world/src/rooms/gate_portal.rs:22`

Condition: one boolean read **from the save**, `save.data().switch(&config.switch_id)` (`world/rooms/systems.rs:84`). Effect: `allows_traversal()` refuses a room transition (`world/rooms/systems.rs:219`). Sequencing: ⛔⛔ **reversible, interruptible machine with progress-preserving reversal** — `Opening{elapsed} → Closing{elapsed'}` where `elapsed' = CLOSING * (1 - elapsed/OPENING)` (`:145`,`:161`), symmetric in both directions, cycling with no terminal state. State: `GatePortalConfig.phase` inside the Resource `GatePortalRegistry` (`:81`), a `HashMap<String, _>`. Reference: raw `String` zone id, switch id, and **LDtk display names** for sprites.

⛔⛔ **Rollback: WAIVED, and the waiver looks wrong.** `("::gate_portal::GatePortalRegistry", "authored gate portals")` at `game/ambition_app/tests/rollback_coverage.rs:1150`. The registry is not authored data — it carries live phase+timer state, ticked on `world_time.scaled_dt` (not `sim_dt`), that decides whether a room transition may fire. This is exactly the program doc's warning: *"an interpreter's program counter that decides whether a gate is open is world state wearing a disguise."*

✅ **SETTLED 2026-08-15 — real defect, fixed.** `tick_portal_phases_system` is registered into `app.sim_schedule()` (`runtime/src/progression_schedule.rs:123`), which is `bevy_ggrs::GgrsSchedule` under the shipped host (`runtime/src/lib.rs:206`), so the timer advances on speculative frames. The switch it integrates is in `AmbitionGameSave`, **which is rollback-registered** (`runtime/src/rollback/mod.rs:277`) — so a rewind restored the input and not the integral, leaving `elapsed` permanently ahead by the depth of each rollback taken inside the ~38-tick opening window, and peers promoting `Opening → On` on different frames disagree about a room transition (`world/rooms/systems.rs:219`). The live phase moved to a new rollback-registered resource `GatePortalPhases` (`resource.gate_portal_phases`); `GatePortalConfig.phase` is **deleted**, so the surviving `GatePortalRegistry` waiver is now a true statement. ⚠ the registry deliberately stays UNregistered: it is populated from `Update` behind a one-shot flag that does not rewind, so snapshotting it would let a rewind erase authored portals nothing refills. Schema `v30 → v31`.

Populated **only by hardcoded content Rust** (`game/ambition_content/src/intro/plugin.rs:174`) — exactly one portal exists. An author cannot write "this door needs flag X" in LDtk.

### 13. Moving platforms / kinematic paths
`/home/joncrall/code/ambition/crates/ambition_platformer2d_world/src/platforms/mod.rs:227`

⛔⛔ **It cannot be gated at all.** `advance_moving_platforms` is an unconditional `for platform in platforms.0.iter_mut() { platform.update(sim_dt); }` (`platformer2d_actor_monolith/src/avatar/body_integration.rs:318`). `MovingPlatformState`/`MovingPlatformSpec` have **no** `enabled`/`active`/`gate`/`trigger` field, and a previous per-entity gate was **deliberately deleted** as duplicate authority. The only external influence is the global clock.

No effect model (self-motion only). Sequencing: closed `MovingPlatformMotion { Sweep, Path, Loop }` with `Once|Loop|PingPong`. Rollback ✔ `rollback_resource_canonical::<MovingPlatformSet>` at `rollback/mod.rs:268`. Preparation ✔ `AuthoredPlatformMotion::classify()` refuses ambiguous field combos at conversion. ⚠ `KinematicPath.start_offset_seconds` has **no reader** — a dead authored field. ⚠ **A second, independent stepper of the same `KinematicPath` exists** (`ambition_combat/src/path_motion.rs`), whose own doc admits the two "are the same algorithm twice and must not drift"; hazards carry an `enabled: bool` with **no runtime writer**.

### 14. Loading zones · Interaction · Quests (abbreviated)

- **Loading zones** (`platformer2d_world/src/rooms/loading_zone.rs:10`): closed `LoadingZoneActivation { EdgeExit, Door, Walk }` + one `wants_interact` bool. **No flag/key/item test exists in the zone**; the only conditional gate is the out-of-band gate portal. ⭐ Strongest LDtk validator in the repo (`platformer2d_ldtk/src/lib.rs:211-266`). ⚠ activation parse has a `_ => Door` catch-all.
- **Interaction** (`crates/ambition_interaction/src/lib.rs`): ⚠ `Interactable.requires_facing` and `.enabled` **both have zero consumers**; `InteractionKind::Door{target}` is constructed and **never matched**. No sequencing, first-hit-wins.
- **Quests** (`crates/ambition_persistence/src/quest/mod.rs:32`,`:63`): a genuine event-condition matcher — `QuestAdvanceEvent { NpcTalked, ItemCollected, BossDefeated, EncounterCleared, FlagSet, RoomEntered }` with `QuestStepCondition::matches`. Strictly linear, **no conjunction, no disjunction, no failure** (its own module doc says so). Rollback ✔ `rollback/mod.rs:281`. Saved ✔.

### Checked and ruled out
No `WorldFlag`/`GameFlag`/`StoryFlag` resource or enum. No `Powered`/wire/circuit/pressure-plate mechanic. `Item::GateKey` exists in the catalog but **nothing gates on it** — there is no lock-and-key mechanic. `MoveGates`/`ActionGate` in the demos are per-move grounded/airborne predicates, unrelated to world state. The intro dialog redirect table referenced by `route_state.rs:94-99` **no longer exists** (stale comment; `assets/data/dialogue/registry.ron` is not on disk).

---

## Part A — Genuine similarities

**A1 — "Ordered program with a monotonic cursor over a closed effect enum, stepped by a small fixed
fact set."** Members: `EncounterScript`, `CutsceneRuntime`, `Quest` step chain, wave director.
Shared: cursor is a `usize` that only increases; a step never re-fires; `elapsed: f32` resets on
advance; effects are a closed enum yielded as data; no branch, no loop, no early exit.

**A2 — ⭐ Everything is `f32` SECONDS.** Every timer in every system inventoried — beat elapsed,
cutscene beat duration, move windows, wave delays, portal phase, boss step duration, `Survive(secs)`
— is float seconds on a scaled sim clock. **There is no integer-tick timeline anywhere.** This is
the strongest positive result in the census: the time axis is already uniform, so a shared prepared
representation does not have to reconcile two time domains.

**A3 — "Latch by writing a string-keyed fact into `AmbitionGameSaveData`, re-derive everything else
per frame."** Members: intro chains, intro lock walls, gate portal, switch persistence, cutscene
`seen_flag`, Yarn `flag()`, quest `FlagSet`. ⭐ Seven independent systems already agree on this
store. It is the most reusable thing in the census.

**A4 — "A typed per-channel message applied by a focused system."** `SetFlagRequested`,
`QuestAdvanceRequested`, `SwitchActivated`, `EncounterCommand`, `EncounterGate`,
`GameplaySfxRequested`. ⭐ Precedent: `effect_bus.rs:6` records that a single monolithic
`GameplayEffect` enum **already existed and was already deleted** for exactly this.

**What is NOT a similarity:** "they all have conditions." They do not. Platforms, loading zones,
interaction, the wave director, cutscenes and quests have either no condition vocabulary or a single
hardcoded bool. The four real condition models (`Objective`, `EncounterTrigger`, Yarn `<<if>>`,
`SituationBucket`) share nothing structurally.

---

## Part B — Genuine incompatibilities  ⟵ **THE FALSIFIER, answered honestly**

Four found. I did not manufacture them and I did not suppress them.

### B1 — ⛔⛔ Monotonic cursor vs. reversible timer machine
`EncounterScript` (§1) and `GatePortalPhase` (§12) cannot share one sequencing representation.
`advance` only ever does `cursor += 1; elapsed = 0.0`; there is no transition out of a beat except
forward and no way to re-enter one. `tick_gate_portal_phase` reverses **and maps the timer
backwards** to preserve visual progress, symmetrically in both directions, in a cycle with no
terminal state. An ordered-beat-with-cursor form cannot express that without a branch naming the
customer; a general state machine can express the beat list only by making "inert past the last
beat" and "one beat max per tick" special cases.

### B2 — ⛔⛔ Subroutine stack + interrupts + seeded randomness vs. flat cursor
`BossPatternState` (§4) has a **stance return stack**, **interrupts that preempt the cursor**, a
**weighted-random `Select` with an `rng_seed` in the snapshot**, and a timeline that **loops at the
end**. `EncounterScript` has none of those and terminates. These are different execution machines,
not two configurations of one. ⭐ And the boss family is the **incumbent**: it already has the
schema, the compile-time cross-ref resolution, the design validator and the rollback-encoded cursor.
Any "unify the sequencers" move would be asking the finished system to adopt the unfinished one's
shape.

### B3 — ⛔⛔ Three shipped answers to "is a program counter rollback state?"

| System | Answer | Evidence |
|---|---|---|
| Cutscene (§5), `MovePlayback` (§6) | **Register the cursor AND the whole immutable program**, re-encoded every frame | `domains/cutscene.rs:32` + codec `ambition_cutscene/src/lib.rs:554`; `domains/combat.rs:127` + `moveset/mod.rs:197` |
| `EncounterScript` (§1) | **Register nothing** — despawn the entity and rebuild from Rust | no `SnapshotState`, no waiver; `features/ecs/reset.rs:268` |
| Gate portal (§12) | **Waive as "authored"** — while it holds live state gating a room transition | `rollback_coverage.rs:1150`; gating at `world/rooms/systems.rs:219` |

⭐ Boss patterns (§4) are the fourth and *correct* answer: encode the cursor and the *resolved*
timeline, not the source program. Any shared representation must pick one, and picking one changes
the behavior of at least two existing systems. This maps to program falsifier #5 — it does **not**
trip it (the fix is the doctrine already written: immutable program out of band, cursor/timers in
rollback state) but it means **M4 cannot be deferred behind M2/M3**, because two of the four shipped
answers are wrong and one is a live latent desync.

### B4 — ⛔ Three occurrence models
`ActiveCutscene` is a **Resource holding `Option<_>`** — the engine can play exactly one cutscene,
ever. `EncounterScript` is a **Component** — N concurrent. `GatePortalRegistry` is a **Resource
holding a `HashMap<String, _>`** — N occurrences keyed by string rather than entity, and therefore
invisible to every entity-scoped rollback sweep, which is precisely how B3's bad waiver went
unnoticed. A shared substrate must impose one model, and doing so is a rewrite of the cutscene
singleton, not a wrapper around it.

### Does falsifier #1 trip?

**No — for the two customers selected in §C**, which share a condition/effect/reference/preparation
representation cleanly and need no sequencing at all in one case and a flat beat list in the other.

⭐ **But B1 and B2 are a real result and must be recorded as one:** *sequencing must not be
unified.* The program doc's open question — *"Is sequencing expressed as an explicit state machine,
an ordered program with a cursor, or a behavior-tree-shaped evaluation?"* — is answered by this
census as **"at least three of those already exist, all justified, and the choice belongs to the
domain."** The substrate that survives contact with this tree is: **conditions, commands, prepared
references, preparation, and discovery — with sequencing left domain-owned.**

### What I looked for and did NOT find
No condition model here is genuinely inexpressible in a prepared predicate tree. Yarn's `<<if>>` is
the richest, but its 7 functions are all pure reads of the save/inventory snapshot and compose into
`Objective`-shaped trees trivially.

---

## Part C — The two proof customers

### Customer A (sequenced) — the cut-rope Smirking Behemoth encounter

**Files:** `/home/joncrall/code/ambition/game/ambition_content/src/bosses/cut_rope/` (943 lines:
`mod.rs` 375, `arena.rs` 404, `victory.rs` 164); host executor
`/home/joncrall/code/ambition/crates/ambition_platformer2d_actor_monolith/src/boss_encounter/encounter_script.rs` (231).

**Cost in bespoke Rust today.** The fight's entire authored content is a Rust literal at
`mod.rs:321`, assembled by a 68-line Bevy system `setup_cut_rope_encounter` (`mod.rs:281-348`) that
must: query encounters `Without<EncounterScript>` for idempotence, **poll `room_set.active_props()`
every frame** until the authored anvil prop loads, filter participants by `is_cut_rope_boss`, and
compute `align_tolerance` from the boss's combat width. None of that is fight design; all of it is
plumbing to get four numbers and two gate names into a `Vec`.

**What the authored source must express.** Two beats. Trigger `Gate(name)`; effects `CommandMoveTo`,
`DropHazard`, `ForceKill`. A **reference to a room prop by identity** (the anvil) whose `pos`/`size`
are read at prepare time rather than authored as literals. A member reference (today `usize` index 0
— should become a participant *role* or a prepared reference). Two gate names that must round-trip
(`"cut_rope_impact"` is written by `FallingHazard` at `encounter_script.rs:224` and read by beat 2).

**DELETED.**
- `setup_cut_rope_encounter` in full — ~68 lines, including the per-frame prop-polling loop and the
  idempotence query.
- The content crate's coupling to `BossConfig` / `EncounterParticipants` query types via
  `is_cut_rope_boss`.
- `ROPE_LURE_SPEED`, `ROPE_ALIGNMENT_TOLERANCE`, `ANVIL_GRAVITY`, `ANVIL_TERMINAL_SPEED`,
  `ANVIL_KIND` as Rust consts — they become authored fields.
- ⭐ **The strongest deletion is a class, not a line count**: the "despawn the encounter entity so
  the script rebuilds itself" arm at `features/ecs/reset.rs:268` exists *only* because the script is
  Rust-constructed and unrollbackable. A prepared immutable program plus a registered cursor deletes
  it and closes B3's second row.

⚠ Honest caveat: this is a **small** customer — two beats. It proves the pipeline, not scale.

### Customer B (ordinary world mechanism) — the intro flag chains + flag-gated lock walls

**File:** `/home/joncrall/code/ambition/game/ambition_content/src/intro/route_state.rs` (215 lines +
tests), registered at `/home/joncrall/code/ambition/game/ambition_content/src/intro/plugin.rs:106`,`:116`.

**Cost in bespoke Rust today.** Two `const` string-pair tables and two Bevy systems. One polls the
save every frame emitting `SetFlagRequested` for `(trigger set, target unset)`. The other re-walks
**every LDtk level's entity list** matching `LockWall` `id` field strings against a table, pushing
solids into the per-frame overlay, behind a hand-written cache whose three-input invalidation rule
exists purely for a measured ~1.8% profile cost.

**What the authored source must express.** Condition: `world_fact(name)` set/unset, and the
conjunction of two. Effect: `set_world_fact(name)`, `enable_solid(ref)` / `disable_solid(ref)`.
Reference: a **prepared reference to an LDtk `LockWall`** (native `EntityRef`/iid) replacing the
`id`-field string scan. Latch: none needed — the save *is* the latch and the rule is a pure
idempotent re-derive.

**DELETED.**
- `INTRO_FLAG_CHAINS` + `emit_intro_flag_chains` — the whole "when flag X, set flag Y" mechanism
  (~40 lines) replaced by 5 authored rows.
- `INTRO_FLAG_GATED_LOCK_WALLS` + `compute_intro_flag_gated_lock_walls` +
  `sync_intro_flag_gated_lock_walls` (~110 lines).
- ⭐ **`IntroLockWallCache` and its entire invalidation rule (~35 lines) vanish**, because a prepared
  rule resolves its `LockWall` reference **once at preparation** instead of scanning LDtk levels on
  every cache miss. That is a deletion the rule layer *causes*, and a measured performance win.
- ⭐ The `Update`-vs-sim-schedule defect (`intro/plugin.rs:99`) is deleted by construction: a
  prepared rule runs where the substrate runs.
- The `playtest-handoff.md` cross-reference the module doc admits is the only way to audit the chain.

**Why it qualifies.** Real shipped intro content; lives in a **content crate** (exactly the code the
program wants authors to stop writing); materially different from Customer A (no sequencing, no
timers, no participants, no occurrence state); and it exercises the two things Customer A does not —
**world-fact conditions** and **a prepared reference into LDtk**.

⚠ **Honest weakness, stated because it affects the plan:** four of the five chain targets are dead
vocabulary with no readers. The *live* deletion is the lock-wall half plus the cache. ⭐ This is
itself an argument for the layer — an authored rule file makes "this fact has no consumer" a
queryable fact (M6 reverse references), which is impossible today.

### Runners-up considered and rejected
- **Noether symmetry attunement (§10)** — ⭐ the strongest *evidence* in the census and the best M3
  stretch target, but rejected as Customer B because it is implemented as an encounter, so it fails
  the doc's "materially different from an encounter" requirement. Its existence is the argument, not
  the proof.
- **Gate portal (§12)** — rejected as a first proof because B1 would force the sequencing question
  open on day one. ⭐ Best **second-wave** customer for exactly that reason. Its rollback waiver
  should be fixed independently of this program.
- **Switch action dispatch (§9)** — deleting the in-sim `strip_prefix("SetGravity")` match would be
  a pure win, but it is an *effect* migration with no condition side, so it under-tests the substrate.
- **Moving platform gating (§13)** — rejected: **there is nothing to delete.** A gated lift is
  net-new capability, which is program falsifier #2. It is the first thing to build *after* the
  substrate proves out, not the proof.
- **Boss patterns (§4)** — ⛔ **not a customer at all.** See §F.

---

## Part D — What already exists that a rule layer should CONSUME

| Need | What exists | Path |
|---|---|---|
| **World-fact store** | `AmbitionGameSaveData` — `flags`, `switches`, `encounters`, `bosses`, `quests`, `dialog_visits`, `items`, `wallet`, `checkpoint`; `CURRENT_SAVE_VERSION = 3` with stepwise `migrate()` and a `FromTheFuture` write refusal. Rollback-registered at `rollback/mod.rs:277`. ⚠ Global and flat; **nothing is room-scoped**; keys are unvalidated `String`. | `crates/ambition_persistence/src/save_data.rs:205`,`:475` |
| **Composable condition tree** | `Objective` + pure `objective_met`; ⭐ deliberately **no `Custom(String)` escape hatch** | `crates/ambition_encounter/src/objective.rs:177` |
| **Semantic commands (flags)** | `SetFlagRequested` → `apply_flag_effects`, mirroring into `QuestAdvanceEvent::FlagSet`. ⭐ Precedent: the monolithic `GameplayEffect` enum was already deleted for this | `crates/ambition_platformer2d_actor_monolith/src/features/ecs/effect_bus.rs:16` |
| **Semantic commands (encounters)** | `EncounterCommand { encounter: String, kind }` — sole ingress; the reducer is the only writer of `phase` | `crates/ambition_encounter/src/lifecycle.rs:260` |
| **Out-of-sim decider → deterministic sim command** | `NarrativeInputWriter` → `NarrativeInputLedger<M>` → `release_narrative_inputs`: instance-stamped, `next_tick`-stamped, edge-released, horizon-pruned | `crates/ambition_platformer2d_actor_monolith/src/conversation/ledger.rs:195`,`:256`,`:327` |
| **Occurrence identity** | `SimId(String)` — `#[require(SimIdCounter)]`, counter lives **on the spawner** so it is snapshot state and restore never double-mints; wall-clock and `Entity`-index ids forbidden by contract; segments percent-escaped | `crates/ambition_platformer2d_shared_tangle/src/sim_id.rs:71`,`:206` |
| **Definition identity** | `ContentId { namespace, schema, name }`; also `PlacementId`, `CharacterId`. ⚠ **Not connected to `SimId`** — see §G | `crates/ambition_content_pack/src/identity.rs:67` |
| **Prepared cross-schema references** | `PendingRef { schema, name, noun, declared_by, field, local }` → `ResolvedContentRef<T>`; matched as `<namespace>:<schema>/<name>` with **no dependency edge between crates**; unresolved = compile error | `crates/ambition_content_pack/src/refs.rs:96`,`:36`,`:71` |
| **Preparation / validation pass** | `ContentSchemaHandler::check` → diagnostics → `out.lower(v)` **only when clean**; 7 ordered `CompileStage`s with `stopped_before()`; `UnknownField` is an error; `define`d rows feed a platform-stable `ContentFingerprint` | `crates/ambition_content_pack/src/lib.rs:79`, `diagnostic.rs:48`, `prepared.rs:85` |
| **Transactional entity spawn + relation wiring** | `ConstructionRequest → ConstructionPlan<D> → ConstructionReceipt::entity(&SimId)`; the plan is "immutable once prepared: every fallible decision is already made", canonically ordered by `SimId`. Dangling authored refs fail the whole room (`MountLinkNamesNobody`) | `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs:487`,`:786`,`:768` |
| **Read sim state without touching components** | `ambition_sim_view` facts resources — plain data, rebuilt per tick, no `Entity`/`Handle` borrows | `crates/ambition_sim_view/src/facts.rs` |
| **Explanation ("why did this not fire?")** | `ambition_causal` — `CausalLog::explain(tick, &SubjectKey)`, `SubjectKey::Sim(String)` carries a `SimId`, `Execution::{Original, Resimulated}` labels replays. ⭐ **`FactDetail::kind` is an open `&'static str`, explicitly so a capability publishes kinds without editing a central enum** | `crates/ambition_causal/src/lib.rs`, `log.rs:234` |
| **Content CLI** | `ambition_content <pack-dir> [--fingerprint] [--list-schemas] …`; runs the *identical* `compile` the app runs, deliberately | `crates/ambition_content_cli/src/lib.rs:130`,`:185` |
| **LDtk validation** | Strongest authored validator in the repo (edge adjacency, half-a-target, orphan landing pads) | `crates/ambition_platformer2d_ldtk/src/lib.rs:211-266` |

⛔ **`ambition_causal` is observer-only by contract** — *"the simulation must never read a fact"* — so
it is the right substrate for M5 explanation and **not** an event channel a rule *condition* may
subscribe to. Note it already satisfies program falsifier #4: it explains a tick without replay.

**What does NOT exist and must be built:** a condition/fact-query vocabulary shared across domains;
a discovery index over conditions and commands; any notion of a rule *occurrence*; a `SimId → Entity`
lookup outside one commit's `ConstructionReceipt`; a `ContentId ↔ SimId` bridge; room-scoped
persistence; typed LDtk diagnostics (`LdtkValidationReport` is `Vec<String>`).

---

## Part E — Control-flow backends already in tree

**`bonsai-bt` is NOT present.** Verified against `/home/joncrall/code/ambition/Cargo.lock` — no
`bonsai`, `seldom_state`, `statig`, `big-brain`, `beet`, or any generic behavior-tree / state-machine
crate.

The only third-party control-flow engine is **`bevy_yarnspinner` / `yarnspinner*`**
(`Cargo.lock:2976`,`:9252-9309`) — a branching dialogue VM with an opaque program counter,
**explicitly waived from rollback** (`rollback_coverage.rs:75`). ⛔ Not a candidate substrate: its
state is opaque and unrollbackable by construction.

**No first-party generic sequencer exists.** Five hand-rolled ordered-beat executors, each
domain-private, none reused: `EncounterScript::advance`, `CutsceneRuntime` tick, `Quest::try_advance`,
`EncounterWaves::tick_active`, `BossPatternState` tick — plus `tick_gate_portal_phase` and **two
independent copies of the same kinematic path stepper**.

⚠ **One thing that reads like a god-registry and must be examined before M1 claims success:**
`ambition_content_cli::default_registry()` (`crates/ambition_content_cli/src/lib.rs:20-58`) is a
hand-edited central function listing all schemas. Ownership is properly distributed (each schema is a
function on its own domain crate) but *composition* is central, so strictly read, adding a schema
**does** require editing a central function. ⭐ I judge this **not** a falsifier-#3 trip — it is a
composition root (its own doc calls it "the same act as installing it in an app"), per-binary rather
than per-engine, and holds no authority. But M1 must state that distinction explicitly rather than
let it pass unexamined. Rollback registration has the same shape: 289 call sites, ~65% concentrated
in `crates/ambition_platformer2d_runtime/src/rollback/domains/`, but per-plugin registration works
and is enforced by a startup guard (`crates/ambition_platformer2d/src/rollback.rs:245`).

---

## Part F — What changed my mind

1. **`EncounterScript` is not authored content, and I expected it to be.** The crate calls it "the
   ONE timeline authority" and `spec.rs` sits beside it full of serde RON types. But
   `EncounterTrigger`/`Effect`/`Beat`/`Script` carry **no serde derives at all**, the authored
   `EncounterSpec` has **no script field**, and the only production script in the workspace is a Rust
   literal. ⭐ This *strengthens* Customer A: the vocabulary is already designed and validated by one
   real fight, and simply has no front door.

2. ⭐⭐ **Boss patterns looked like a customer and are emphatically not one — they are the template.**
   The program doc lists "boss phase triggers and boss attack/pattern timelines" as likely customers.
   They are the **one family that already did the whole job**: typed schema family in the content
   pack, compile-time cross-reference resolution, a *design* validator with data-driven bands, and a
   rollback-encoded cursor that correctly snapshots the **resolved** timeline rather than the source
   program. Migrating them would be asking the finished system to adopt an unfinished one's shape.
   **They should be left alone and copied.** This is the single biggest correction the census makes
   to the program doc's own customer list.

3. **The moving platform is the doc's headline example and is disqualified as a proof.** "When two
   switches are active, power a lift" has nothing to migrate: platforms have no gate, no `enabled`
   field, and a previous per-entity gate was *deliberately deleted*. Migrating it would be pure
   addition — program falsifier #2. It is a post-substrate feature, not a proof.

4. ⭐ **The Noether symmetry-attunement puzzle reframed the whole census.** I went looking for a
   multi-input condition in world content and expected to find none. It exists, ships, and is a real
   four-switch AND — but only because it was **dressed up as an encounter**, with a hand-written
   adapter translating switch presses into encounter signals so it could borrow `Objective::All`.
   That is the cost of having no world-level condition vocabulary, measured in shipped content, and
   it is better evidence for the program than any amount of "several partial implementations".

5. **The gate portal's rollback waiver is, I believe, a live defect and not part of this program.**
   I went in looking for a customer and came out with a bug. ⭐ It should be fixed on its own merits
   and **should not be used as motivation for the rule layer** — attributing a plain accounting error
   to an architectural gap would misdirect the plan.

6. **I stopped believing "several partial implementations of condition→effect" is the story.** It is
   true on the **effect** side — five-plus independent closed effect enums and command buses. It is
   **false on the condition side**: there is essentially one composable model (`Objective`), one
   third-party one (Yarn `<<if>>`), one boss-local one (`SituationBucket`), and otherwise nothing.
   The gap is narrower and deeper than the doc's framing suggests, and it changes what M1 should
   prototype first: **the condition / fact-query side, not the command side.**

7. **The authored-move seam already exists and nobody uses it.** `MovePrefabRegistry` (key + params
   → `MoveSpec`) has **zero production callers**, `CharacterDefinition.moveset` has zero authored
   consumers, and **no `.ron` in the repo contains `windows:`**. A seam existing is not a seam being
   adopted — worth remembering before building another one.

---

## Part G — What I could NOT verify (lowest confidence first)

1. ⛔ **Whether `EncounterScript` is a real rollback hole or merely outside every sweep's population —
   I could not run the test.** It is neither registered nor waived (verified by grepping all
   `SnapshotState` impls and the `WAIVED` table). The sweep is explicitly population-driven and its
   boss-arena fixture boots `"mockingbird_arena"` (`rollback_coverage.rs:539`), **not** the cut-rope
   room, so the instrument has most likely never had an opinion about it. "Unregistered" is
   **verified**; "and therefore a desync risk" is **unverified** — the despawn-and-rebuild reset arm
   may cover it fully in practice.
2. ⛔ **Whether the gate-portal waiver actually causes a desync.** I verified the state is mutable,
   ticked per-frame on `scaled_dt`, waived, and gates a transition. I did **not** verify that a
   rollback ever occurs while a portal is mid-`Opening` in shipped content. "The waiver looks wrong"
   is well-evidenced; "this desyncs" is not.
3. ✔ **§F.2 (boss patterns are the template, not a customer) — RE-VERIFIED BY HAND, promoted out of
   this list.** I read `BossPatternStep` (`mod.rs:166`) and confirmed: `#[derive(serde::Deserialize)]`
   (so it genuinely is authored), `Select { table: Vec<WeightedArm> }` rolled once at resolution,
   `Stance { id }` with `stance_stack: Vec<StanceReturn>` documented as *"a stack, not a slot, so a
   stance may enter another stance without losing the way home"*, `InterruptRule { on, cooldown_s,
   stance }`, `SituationBucket` (5 arms), and `BossPatternState.timeline` holding the **resolved**
   step list. I also confirmed the authored data: nine files in
   `game/ambition_content/assets/data/boss_encounters/` and `boss_profiles.ron`, whose header reads
   *"To re-tune a fight: edit the row, restart the sandbox. No Rust changes needed."*
   ⚠ The one part still second-hand is the `SnapshotCursor for Brain` encoding at
   `ambition_characters/src/snapshot_impls.rs:339-384` — I did not open it.
4. ⚠ **Non-boss brain snapshot coverage.** `SnapshotCursor for Brain` reportedly encodes only
   BossPattern, Smash and Fighter state, leaving `Patrol`/`MeleeBrute`/`Skirmisher`/`Sniper`/
   `ChargeCrash`/`Aerial` internal state to a default arm. If true that is a separate latent
   rollback gap. **I did not confirm it and it is outside this census's scope** — flagging only.
5. ⚠ **`ActiveCutscene`'s registration site** (`rollback/domains/cutscene.rs:32`) is second-hand; I
   independently verified only the codec at `ambition_cutscene/src/lib.rs:554`.
6. ⚠ **Intro flag-chain rollback participation is INFERRED.** Its two systems hold no state of their
   own, so I inferred semantics from `AmbitionGameSave`'s registration at `rollback/mod.rs:277`. I
   did not read that registration's snapshot impl to confirm `flags` is inside the encoded bytes.
   ⭐ Separately, the `Update`-schedule finding (`intro/plugin.rs:99-107`) means it is **outside the
   rollback-ordered sim regardless**, which I did verify.
7. ⚠ **I did not verify the cut-rope fight currently works.** Customer A's value assumes it is live
   shipped content. The Rust is present and `game/ambition_app/tests/boss_lifecycle.rs` references it,
   but I ran no tests.
8. ⚠ **Deletion line counts in §C come from `wc -l` and reading function spans, not from producing
   the diff.** Treat "~68 lines" / "~110 lines" as ±25%.
9. ⚠ **I did not read the `.yarn` files or `boss_profiles.ron` myself**, so claims about what content
   actually authors (170 `<<jump>>`s, nine encounter files) are second-hand counts.
