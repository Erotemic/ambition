---
status: current
last_verified: 2026-07-18
---

# Persistence, settings, and progression

`ambition_persistence` owns saved-game, quest, and user-settings vocabulary plus
save I/O. Provider content owns named quests/rewards/dialogue/encounters; UI
crates edit settings through renderer-neutral models. Presentation does not own
persistent truth.

## Data categories

| Category | Examples | Owner |
|---|---|---|
| Durable world/player history | collected rewards, defeated/converted actors, switches, quest flags, seen cutscenes | save data and provider progression integration |
| User preferences | audio mix, controls/deadzones, gameplay options, video/display preferences | persistence settings model + host/UI adapters |
| Live session state | current entities, timers, temporary effects, active move playback | simulation; reconstructed/snapshotted through canonical seams |
| Derived UI/read models | menu rows, quest text, settings pages | settings/menu/sim-view presentation models |

Do not store frame events or derived UI state as progression flags. Do not use
settings persistence as a general simulation state bag.

## Stable IDs and migration

Persist provider-qualified stable IDs, not Bevy `Entity` handles or source-file
paths. Renaming a persisted flag, quest, item, encounter, or character ID is a
save-format migration even when the Rust type is unchanged.

Parsers may retain backward-compatible defaults/aliases when practical. New
fields need deterministic defaults and explicit reset behavior.

## Settings flow

```text
persisted settings
    -> ambition_persistence host/settings vocabulary
    -> settings/system menu IR
    -> host/domain adapters derive effective runtime values
```

Gameplay may read effective values but should not perform disk I/O or own the
settings UI. Platform path selection stays in persistence/host helpers; never
hard-code a user directory.

## Progression flow

Provider content registers named quest, encounter, dialogue, and reward data.
Reusable domain state machines emit typed completion/reward facts. Persistence
records only the durable result. On load/reset/restore, canonical construction
and reconciliation rebuild the live world from authored content plus saved
facts.

## Validation

```bash
./run_tests.sh -p ambition_persistence
./run_tests.sh -p ambition_settings_menu
./run_tests.sh -p ambition_encounter
./run_tests.sh -p ambition_content -k quest
./run_tests.sh -k save
./run_tests.sh -k settings
```
