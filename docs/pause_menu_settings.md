# Pause-menu settings

The sandbox's pause overlay (`crate::pause_menu`) has two pages.

## Top page

Resume / Settings / Music / Inventory / Quit. Up/Down navigate, Confirm
(Jump action) activates, Left/Right cycles values on rows that support
it (Music, Display Mode).

## Settings page

Currently exposes:

| Row          | Behavior                                            |
| ------------ | --------------------------------------------------- |
| Display Mode | Cycles Windowed → Borderless → Fullscreen with Left/Right. Confirm advances to the next mode. |
| Back         | Returns to the top page.                            |

The page is reached via the **Settings** entry on the top page.

## Adding a new settings row

The intent is to make this page the home for any user-facing toggle —
volume, gamma, controls, accessibility, etc. — so they have one
discoverable location and the harness gets used as more options
appear.

Steps:

1. Add a variant to `SettingsItem` in `crates/ambition_sandbox/src/pause_menu.rs`.
2. Append it to `SettingsItem::ALL` in the order you want users to
   navigate.
3. Implement `SettingsItem::label` so the row text shows the current
   value (e.g. `format!("Master Volume: {value}%  < / >")`).
4. Handle the row in `handle_settings_input` — typically Left/Right
   cycle, Confirm advances. Mutate the relevant resource directly.
5. Adjust `spawn_pause_menu` only if the new row needs custom spawning
   beyond the existing `for item in SettingsItem::ALL` loop. The loop
   already creates one Text entity per item.

Settings that mutate window state should call
`pause_menu::apply_display_mode` (or the equivalent helper for the new
resource) so the same logic runs whether the user reached it via the
menu or via a developer hotkey.

## Developer hotkeys

The display-mode hotkeys in `crate::windowing::window_mode_hotkeys`
remain as a developer convenience:

- `F6` — windowed
- `F7` — borderless fullscreen

`F8` is reserved for the gameplay trace recorder
(`docs/gameplay_trace_recorder.md`); exclusive fullscreen is reachable
only through the menu now.

The hotkeys delegate to `pause_menu::apply_display_mode` so the menu
and the keystrokes never drift out of sync.

## Tests

`crates/ambition_sandbox/src/pause_menu.rs` covers:

- `PauseMenuState::default` lands on the top page,
- `enter_page` resets `selected` to zero,
- `next_display_mode` / `prev_display_mode` cycle correctly,
- `SettingsItem::DisplayMode.label` includes the current mode label,
- `PauseMenuItem::ALL` includes `Settings`.

Run with `cargo test -p ambition_sandbox pause_menu::`.
