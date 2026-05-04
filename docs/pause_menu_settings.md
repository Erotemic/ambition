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

## Architecture

The settings page splits into two modules:

- [`crate::settings`](../crates/ambition_sandbox/src/settings.rs) owns
  the vocabulary (`SettingsItem`, `SettingsAction`, `SettingsView`,
  `SettingsOutcome`) and the per-row mutation logic
  (`handle_action`, `apply_display_mode`).
- [`crate::pause_menu`](../crates/ambition_sandbox/src/pause_menu.rs) is
  the renderer/controller: it spawns the UI nodes, decodes
  `ActionState` into a compact `NavInput`, dispatches to
  `settings::handle_action`, and reads the page page back into UI text.

The pause menu does not own any setting's business logic. Adding a new
setting touches `settings.rs` only; the renderer picks it up
automatically through `SettingsItem::ALL`.

## Adding a new settings row

The intent is to make this page the home for any user-facing toggle —
volume, gamma, controls, accessibility, etc. — so they have one
discoverable location and the harness gets used as more options
appear.

Steps (all in `crates/ambition_sandbox/src/settings.rs`):

1. Add a variant to `SettingsItem`.
2. Append it to `SettingsItem::ALL` in the desired display order.
3. Implement `SettingsItem::label` so the row text shows the current
   value (e.g. `format!("Master Volume: {value}%  < / >")`). Add a
   matching field to `SettingsView` if the value lives in a resource
   the renderer doesn't already inspect.
4. Add a match arm in `handle_action` to mutate the relevant resource on
   `Prev`/`Next`/`Confirm`. Setting-specific mutation logic should live
   close to the resource it owns (audio volume → `audio.rs`, controls →
   `input.rs`); `handle_action` only routes.
5. The pause menu's `spawn_pause_menu` already loops over
   `SettingsItem::ALL` and spawns one Text entity per item — no
   renderer change needed unless the row needs a non-standard widget.

Settings that mutate window state should call
`settings::apply_display_mode` (or the equivalent helper for the new
resource) so the same logic runs whether the user reached it via the
menu or via a developer hotkey. Display-mode mutation lives in the
settings module; the F6/F7 hotkeys delegate there.

## Developer hotkeys

The display-mode hotkeys in `crate::windowing::window_mode_hotkeys`
remain as a developer convenience:

- `F6` — windowed
- `F7` — borderless fullscreen

`F8` is reserved for the gameplay trace recorder
(`docs/gameplay_trace_recorder.md`); exclusive fullscreen is reachable
only through the menu now.

The hotkeys delegate to `settings::apply_display_mode` so the menu
and the keystrokes never drift out of sync.

## Audio-off compatibility

The navigation system compiles and runs with `--no-default-features
--features input` (no audio). The Music row still appears on the top
page so menu indices stay stable, but its label collapses to
`"Music: <audio disabled>"` and selecting it is a no-op.

## Tests

`crates/ambition_sandbox/src/settings.rs` covers:

- `SettingsItem::ALL` lists known rows,
- `next_display_mode` / `prev_display_mode` cycle correctly (forward
  three times returns to the start),
- `SettingsItem::DisplayMode.label` includes the current mode label,
- `SettingsItem::Back.label` is the static "Back" string.

`crates/ambition_sandbox/src/pause_menu.rs` covers:

- `PauseMenuState::default` lands on the top page,
- `enter_page` resets `selected` to zero,
- `PauseMenuItem::ALL` includes `Settings`,
- `MenuSettingsItem` re-export resolves.

Run with `cargo test -p ambition_sandbox settings::` and
`cargo test -p ambition_sandbox pause_menu::`.
