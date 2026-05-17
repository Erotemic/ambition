# Settings system

`crate::settings` is the home of all user-facing tunable values
(video, audio, controls, gameplay) plus the menu vocabulary that the
pause overlay renders.

## Module layout

```text
crates/ambition_sandbox/src/settings/
  mod.rs        — UserSettings resource, SettingsItem rows, dispatch.
  video.rs      — display mode / camera zoom / flashes / colorblind.
  audio.rs      — master / music / SFX volume + mute snapshot.
  controls.rs   — deadzones, trigger hysteresis, dash mode, etc.
  gameplay.rs   — difficulty, assist, damage multiplier, trace toggle.
```

All four submodule structs are `serde::{Serialize, Deserialize}` so a
future persistence pass can read/write them directly. Persistence
itself is **not yet wired**; today `UserSettings::default()` is what
runs at startup. See "Future work" below.

## The `UserSettings` resource

```rust
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub controls: ControlSettings,
    pub gameplay: GameplaySettings,
}
```

Inserted at sandbox startup in `init_sandbox_resources`. Mutated by
the pause menu (`crate::pause_menu::pause_menu_navigate`). Read by:

- `populate_control_frame_from_actions` — applies `controls.left_stick_deadzone`,
  `controls.trigger_press_threshold`, and `controls.trigger_release_threshold`
  before the simulation sees the player's input.
- The pause-menu renderer (`sync_pause_menu`) — to format row labels.
- (Future) audio backend, VFX flash systems, encounter spawning,
  difficulty-scaled damage.

## Adding a new setting

1. **Pick the right submodule** and add the field to its struct.
   Example: a new `aim_sensitivity: f32` belongs on
   `ControlSettings`.
2. **Add a `SettingsItem` variant** in `settings/mod.rs`. Naming:
   one CamelCase variant per row.
3. **Add the variant to `SettingsItem::rows_for(page)`** for the
   page that should display it.
4. **Implement `label(&UserSettings)`** — typically
   `format!("Sensitivity: {:.2}  < / >", settings.controls.aim_sensitivity)`.
5. **Implement an `apply_action` arm** that mutates the field on
   `Prev` / `Next` / `Confirm`. Heavy logic should live as a method
   on the struct (e.g. `ControlSettings::nudge_sensitivity`); the
   dispatcher should be a router.

The pause-menu renderer picks up new rows automatically — no UI
spawn changes needed unless the row needs a non-text widget.

## Adding a new page

1. Add a variant to `SettingsPage`.
2. Add a `title()` arm.
3. Add a row list in `SettingsItem::rows_for(...)` (rows + a
   trailing `Back`).
4. Add a top-page entry like `OpenFoo` plus an `apply_action` arm
   that returns `SettingsOutcome::OpenPage(SettingsPage::Foo)`.

## Architecture notes

- The pause menu is a renderer/controller; it never owns settings
  business logic. Mutation logic stays close to the field on its
  submodule struct.
- `SettingsAction { Prev, Next, Confirm }` is the only verb the menu
  emits. Toggles / cycles use `Next`/`Confirm`; numeric nudges use
  `Prev`/`Next`.
- The page stack lives on `PauseMenuState::stack`; `MenuBack`
  delegates `SettingsOutcome::PopPage` for clean back navigation.

## Persistence (future work)

Each settings struct already implements `Serialize`/`Deserialize`,
and `UserSettings::clamp_all` re-clamps everything after a load. A
`settings/persistence.rs` module can write JSON or RON to a known
path on edit and load on startup. Until that lands the user sees
defaults every run.

## Tests

`cargo test -p ambition_sandbox --lib settings::` exercises:

- per-submodule clamp / cycle / round-trip,
- `SettingsItem::rows_for` shape,
- `UserSettings` serde round trip,
- mute snapshot/restore,
- trigger jitter hysteresis,
- dead-zone rescaling.
