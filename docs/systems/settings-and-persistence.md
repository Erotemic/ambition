# Settings and persistence

Settings and save data are runtime state with disk persistence. They are sandbox responsibilities, not engine mechanics.

## Current paths

```text
crates/ambition_sandbox/src/persistence/save.rs
  game/save flags, player/world progression state

crates/ambition_sandbox/src/persistence/settings/
  model.rs           aggregate settings model
  audio.rs           volume/mute policy
  controls.rs        deadzones, trigger thresholds, profile defaults
  gameplay.rs        difficulty and gameplay tuning
  video.rs           display/window/video preferences
  persistence.rs     load/save plumbing
  platform_paths.rs  platform-specific storage location selection
```

The older `crates/ambition_sandbox/src/settings/` path is retired. Do not add new code or docs that point there.

## Runtime policy

- Settings are grouped by domain: audio, controls, gameplay, and video.
- UI screens edit settings through the settings model, not through ad-hoc resources.
- Persistence owns load/save paths and platform storage rules.
- Gameplay code may read settings-derived effective values, but should not own persistence.
- Save flags and progression state belong with save data; presentation-only preferences belong with settings.

## Agent rules

- When adding a setting, update the model, defaults, persistence, UI row/page, and tests together.
- Use platform-path helpers instead of hard-coded user directories.
- Keep backwards-compatible parsing for existing local settings when practical.
- Mention reset/default behavior in the UI docstring or tests.

## Validation anchors

```bash
cargo test -p ambition_sandbox settings
cargo test -p ambition_sandbox save
cargo test -p ambition_sandbox pause_menu
```

Related docs: `docs/systems/input-and-control-frame.md`, `docs/systems/ui-navigation-and-pause.md`, `docs/concepts/platform-targets.md`.
