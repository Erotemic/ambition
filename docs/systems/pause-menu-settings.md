# Pause-menu settings

The sandbox's pause overlay (`crate::pause_menu`) renders a stack of
pages backed by `crate::settings::UserSettings`. The pause menu is a
renderer/controller; per-setting business logic lives in the matching
submodule (`settings::audio`, `settings::controls`,
`settings::gameplay`, `settings::video`).

See `docs/systems/settings-system.md` for the architecture diagram and
extension recipe.

## Pages

```text
Pause overlay
├── Top: Resume / Settings / Music / Inventory / Quit
└── Settings (top page)
    ├── Video > (display mode, camera zoom, flashes, colorblind)
    ├── Audio > (master / music / SFX volume, mute)
    ├── Controls > (presets, deadzones, trigger threshold/hysteresis,
    │              D-pad nav, invert aim Y, dash mode, reset filter)
    └── Gameplay > (difficulty, assist, player damage multiplier,
                    flashes, trace auto-dump)
```

The page stack is small (`PauseMenuState::stack`) — each `Settings`
sub-page pushes onto the stack so `MenuBack` pops cleanly. From the
top page, `MenuBack` resumes the game.

## Inputs

The menu reads only the `Menu*` actions on
`crate::input::SandboxAction`, never gameplay actions. Bindings:

| Action               | Keyboard          | Gamepad          |
| -------------------- | ----------------- | ---------------- |
| `MenuNavigateUp`     | `Up` / `W`        | D-pad Up         |
| `MenuNavigateDown`   | `Down` / `S`      | D-pad Down       |
| `MenuNavigateLeft`   | `Left` / `A`      | D-pad Left       |
| `MenuNavigateRight`  | `Right` / `D`     | D-pad Right      |
| `MenuStick`          | —                 | Left stick       |
| `MenuSelect`         | `Enter` / `Space` / preset Jump key | South face button |
| `MenuBack`           | `Esc` / `Backspace` | East face button |
| `Start`              | `Esc`             | `Start`          |

`Enter` is a real binding on `MenuSelect` rather than a hardcoded
check inside the settings page — the same code path handles
controller confirm.

D-pad navigation can be disabled in `Settings → Controls → D-Pad
Menu Nav`. Analog stick navigation honors the configured deadzone
plus the configurable initial-delay / repeat-interval timers
(`menu_repeat_initial_delay` / `menu_repeat_interval`), so holding
left-stick Down does not blast through rows.

## Adding a setting

1. Pick the right submodule (`audio` / `controls` / `gameplay` /
   `video`) and add a field to its struct.
2. Add a `SettingsItem` variant in `settings/mod.rs`.
3. Add the variant to the matching `SettingsItem::rows_for(page)`
   list.
4. Add a `label(&UserSettings)` arm and an `apply_action` arm in
   `settings/mod.rs`. Mutation logic stays close to the field
   (`AudioSettings::nudge_master` etc.) so the dispatcher is mostly
   a router.

The pause-menu renderer (`sync_pause_menu`) loops the active page's
`rows_for`, fills pre-spawned UI text slots, and highlights the
selected row. No renderer changes are needed for new rows.

## Developer hotkeys

Display-mode hotkeys delegate to `settings::apply_display_mode` so the
menu and the keystrokes never drift out of sync:

- `F6` — windowed
- `F7` — borderless fullscreen

`F8` is reserved for the gameplay trace recorder
(`docs/systems/gameplay-trace-recorder.md`).

## Audio-off compatibility

The pause overlay still compiles and runs with `--no-default-features
--features input` (no audio). The Music row stays visible (so menu
indices stay stable) but its label collapses to
`"Music: <audio disabled>"` and selecting it is a no-op. Audio
settings rows still render but applying them is a no-op when the
audio backend is not present.

## Tests

- `crates/ambition_sandbox/src/settings/mod.rs` — page row lists,
  display-mode cycling, label formatting, `UserSettings` serde
  round-trip.
- `crates/ambition_sandbox/src/settings/audio.rs` — clamp / mute
  round-trip / percent format / effective volume composition.
- `crates/ambition_sandbox/src/settings/controls.rs` — deadzone /
  trigger jitter hysteresis / dash mode cycling.
- `crates/ambition_sandbox/src/settings/gameplay.rs` — difficulty
  multipliers / clamp / assist toggle.
- `crates/ambition_sandbox/src/pause_menu.rs` — page-stack push/pop
  + default state.
- `crates/ambition_sandbox/src/input.rs` — analog deadzone, menu
  repeat, cardinal edge passthrough, MenuSelect/MenuBack edges.

Run with `cargo test -p ambition_sandbox --lib settings::` or
`cargo test -p ambition_sandbox --lib pause_menu::` or
`cargo test -p ambition_sandbox --lib input::`.


## Radio and compact mobile layout

The pause menu now treats Radio as a pageable settings-style page rather than a
single toggle row. It should show selected-position context (`Radio — i/N`) and
use compact/windowed rows so 16:9 desktop, Steam Deck, and mobile screens all
communicate that more tracks exist. Drag, wheel, joystick repeat, and keyboard
navigation should all flow through `MenuControlFrame` instead of page-local raw
input reads.

Dialog choices should use the same menu-like interaction feel: bounded panels,
vertically centered row text where space allows, and windowed touch-friendly
choice lists when content would otherwise run off-screen.
