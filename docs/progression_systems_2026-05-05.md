# Progression systems landing — 2026-05-05

This pass adds the cross-cutting state machinery that the next wave of
sandbox content will plug into. Everything below builds on the existing
encounter / mob lab pipeline and persists through the standard
`SandboxSave` autosave path.

## What landed

### Save schema (extensible)
`ambition_engine::save::SandboxSaveData` (now version 2) gains:
- `bosses: Vec<PersistedBossDefeat>` — terminal state per boss id
  (`Cleared` / `Failed` / `Untouched`).
- `quests: Vec<PersistedQuest>` — per-quest progression + step.
- `flags: Vec<PersistedFlag>` — generic named on/off flags.
- `reset_all()` — debug "wipe gameplay state" entry point.

Backwards compatible: every collection is `#[serde(default)]`, so v1
saves load with empty new fields. Tested in
`save::tests::deserialize_v1_save_loads_with_empty_new_collections`.

### Quest system
`ambition_engine::quest` defines the data shape; sandbox owns the runtime.
- `QuestSpec { id, title, summary, steps }` and `QuestStepSpec` declare
  authored content.
- `QuestStepCondition` matches against `QuestAdvanceEvent` payloads:
  `NpcTalked`, `ItemCollected`, `BossDefeated`, `EncounterCleared`,
  `FlagSet`, `RoomEntered`.
- Sandbox `QuestRegistry` resource holds live quests, drains a pending-
  events queue each frame, and writes progress back to `SandboxSave`.
- Two starter quests ship: **First Steps** (talk to a hub NPC → clear
  mob lab → defeat the gradient sentinel) and **Test the Memory**
  (toggle the persistence test switch).
- HUD line surfaces all active + completed quests.

Quest events fire from gameplay automatically:
- NPC dialogue → `NpcTalked(id)` + `met_any_hub_npc` flag.
- Encounter clear → `EncounterCleared(id)`.
- Boss death → `BossDefeated(id)`.
- Switch press → `FlagSet("test_switch_toggled")` + `switch_<id>_used`.
- Room entry → `RoomEntered(id)`.
- Cutscene `SetFlag` beat → `FlagSet(id)`.

### Cutscene system
`ambition_engine::cutscene::CutsceneScript` is a list of beats:
`Wait` / `Dialogue` / `CameraPan` / `Fade` / `SetFlag` / `Banner`.
- `CutsceneRuntime` self-times beats, waits for player dismissal on
  Dialogue, and emits `CutsceneEvent::FlagWritten` so the sandbox can
  persist + propagate flag changes.
- `ActiveCutscene` resource carries the live runtime; player input is
  suppressed and the interact key dismisses dialogue while it's
  playing.
- `RoomCutsceneBindings` auto-fires a cutscene on first entry into a
  matched room (`central_hub_main` → `test_intro`, `basement_boss` →
  `boss_intro_gradient_sentinel`). `seen_flag` guards replays.

### Boss phase state machine
`ambition_engine::boss_encounter::BossEncounterState` runs the phase
graph:
**Dormant → Intro → Phase1 → Transition → Phase2 → Enrage → Death**
plus a Stagger sub-state that punishes high burst damage. Plot
thresholds (HP fractions) take precedence over stagger so transitions
never get skipped by a big hit. Music swap requests fire on every
phase change; the sandbox routes them through the existing
`EncounterMusicRequest` resource.

End-to-end coverage in
`boss_encounter::tests::full_encounter_progression_intro_to_death`.

Sandbox-side `BossEncounterRegistry` lazily registers a default
`BossEncounterSpec` for any `BossRuntime` in the active room (matching
by runtime id) so existing LDtk-authored bosses (the basement
"clockwork warden") boot into the phase machine without LDtk surgery.

### Hostile NPCs
NPCs accumulate `strikes` when hit by the player slash hitbox. After
`NPC_HOSTILE_STRIKE_THRESHOLD = 3` strikes they flip to hostile,
disabling dialogue and switching to a striker-style contact-damage
loop. Hostility writes `npc_<id>_hostile` to the save flag set and
re-applies on every room load via `apply_save_to_features`.

### Mob lab polish
- HUD encounter line shows `[id] WAVE n/m :: <wave label> :: k left`
  with a countdown bar during the intro.
- "ARENA CLEAR — <id>" banner fires for 3 s on encounter completion.

## Deferred to a follow-up

These were in scope but not landed because the risk/coverage was
worse than the value delivered tonight:

- **Ledge grab + ledge climb.** Movement.rs is 2461 lines and the
  state machine is dense; I won't land a half-tested edit there.
  Engine seam to add: `LedgeProbe` in `player_state.rs` (returns
  `Option<LedgeContact { facing, anchor }>` from a sweep against
  Solid blocks at chest height), an `Ability::ledge_grab` flag, and
  a movement state branch that snaps the player to the ledge AABB and
  releases on Up + Jump.
- **Swim mechanic + water room.** Adding `BlockKind::Water` cascades
  through ~10 match sites; safer to gate it behind a `RoomObjectKind::
  WaterVolume` (no collision change) plus a `Player::in_water`
  modifier in `update_player_simulation`. The new variant needs a
  `_ => {}` arm added in 4 sandbox match sites first.
- **Map menu + minimap.** Pure UI work; needs a sprite-atlas of
  drawn rooms or a wireframe rendering of `RoomSet::rooms`. Sketch:
  `MapMenuState` toggled by `M`, draws each room as a rect colored by
  visit state from a new `room_visited_<id>` flag set on
  `RoomEntered`. Minimap is the same data clipped to a screen-corner
  panel.
- **Basement door size + overlap fix.** LDtk content edit, not code.
  Author the fix in LDtk directly so the existing repair pipeline
  keeps it consistent.
- **Dedicated boss room (separate from `basement_boss`).** Reusing
  the existing room is fine for the phase-machine demo; a
  purpose-built arena with intro corridor + lock walls is content
  authoring, not engine work.
- **Adaptive music tracks.** The mechanism works (`MusicRequested`
  events route through `apply_encounter_music`); dedicated boss
  tracks need authoring in the existing RON arrangement format.

## Files of note

| Module                                            | Purpose                                       |
| ------------------------------------------------- | --------------------------------------------- |
| `crates/ambition_engine/src/save.rs`              | v2 schema, bosses/quests/flags collections   |
| `crates/ambition_engine/src/quest.rs`             | Quest data + state machine                   |
| `crates/ambition_engine/src/cutscene.rs`          | Cutscene script + runtime                    |
| `crates/ambition_engine/src/boss_encounter.rs`    | Boss phase state machine                     |
| `crates/ambition_sandbox/src/quest.rs`            | Quest registry + Bevy systems                |
| `crates/ambition_sandbox/src/cutscene.rs`         | Cutscene library + room-entry triggers       |
| `crates/ambition_sandbox/src/boss_encounter.rs`   | Boss runtime ↔ engine state bridge           |
| `crates/ambition_sandbox/src/features.rs`         | NPC hostility, FeatureEventBus                |
