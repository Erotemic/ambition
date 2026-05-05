# Save game and user settings persistence

Two parallel files, both RON, both under the OS-conventional data
directory:

```
$XDG_DATA_HOME/ambition/                 (Linux: ~/.local/share/ambition/)
~/Library/Application Support/ambition/  (macOS)
%APPDATA%\ambition\                      (Windows)
  settings.ron        # crate::settings::UserSettings
  sandbox_save.ron    # ambition_engine::SandboxSaveData
```

Override the root with `AMBITION_DATA_DIR=/tmp/foo` for tests or
isolated sandbox sessions.

## settings.ron — `UserSettings`

User-global, not per-save. The pause menu mutates it; the
`save_settings_on_change` Bevy system writes when
`Res::is_changed()` fires (so we don't write every frame). The
`load_settings_at_startup` system reads at startup and replaces the
default-inserted resource.

Wire format is the literal `serde::Serialize` of `UserSettings`,
re-clamped on load so a hand-edit that puts `master_volume = 5.0`
doesn't escape to the audio backend.

I/O is non-fatal: missing file → defaults; corrupt file → defaults +
warning. Writes use temp-file-plus-rename so a crash mid-write
cannot corrupt the live file.

## sandbox_save.ron — `SandboxSaveData`

Sandbox-specific (not a campaign slot). Today carries:

- `encounters: Vec<PersistedEncounter>` — `id` + `state`
  (`Cleared` / `Failed`; `Untouched` removes the entry to keep the
  file compact).
- `switches: Vec<PersistedSwitch>` — `id` + `on`.
- `version: u32` — `CURRENT_SAVE_VERSION` (1 today). Missing
  versions reload as `CURRENT_SAVE_VERSION` so old files keep
  loading.

The encounter system pushes mob-lab clear / fail / reset state into
this file; the switch interaction toggles the matching `switches`
entry. `autosave_sandbox_save` writes when the resource is changed.

Load / save / corruption semantics mirror `settings.ron` exactly.

## Headless / RL drivers

Both load + autosave systems are registered in
`add_presentation_plugins`. The headless binary
(`cargo run --bin headless`) and RL drivers do **not** call that
helper, so they never read or write user files. This keeps test
runs hermetic by default.

## Adding a new persisted field

Engine-side data shapes (`SandboxSaveData`, `UserSettings`)
intentionally live separately from the I/O code. To add a field:

1. Add the field to the struct (`UserSettings` for user-global
   knobs, `SandboxSaveData` for sandbox-state knobs) with `#[serde(default)]`
   so old saves keep loading.
2. Bump `CURRENT_SAVE_VERSION` if the schema is no longer
   forward-compatible (a future patch can add migration steps in
   `load_save`).
3. Wire the system that mutates the field — change-detection on
   the resource handles autosave automatically.

## Tests

`cargo test -p ambition_engine --lib save::` (6 tests) covers the
data-shape round-trip + helpers.

`cargo test -p ambition_sandbox --lib settings::persistence` (4 tests)
covers settings I/O: missing file → defaults, round-trip, corruption
recovery, clamp on load.

`cargo test -p ambition_sandbox --lib save::` (3 tests) covers
sandbox-save I/O: missing file, round-trip, corruption recovery.
