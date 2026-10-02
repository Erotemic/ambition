# Display modes and window scaling

Ambition starts in an ordinary resizable **windowed** mode. The default logical
window size is `1600 x 900` (`ambition_platformer2d_core::config::WINDOW_W` /
`WINDOW_H`), which matches the authored 16:9 composition.

## Display-mode setting

Display mode is user-facing configuration, not a developer hotkey. The pause
menu's **Settings → Display Mode** row cycles Windowed, Borderless and
Fullscreen.

- The vocabulary and the cycle order are `DisplayModeKind`,
  `next_display_mode` and `prev_display_mode` in
  `crates/ambition_persistence/src/host/windowing.rs`.
- The menu row is applied by `apply_settings_option` in
  `crates/ambition_settings_menu/src/settings/apply.rs`. It writes
  `settings.video.display_mode` (`SerializableDisplayMode`).
- `DisplayModeState` is a resource the app inserts and the HUD reads for its
  label. The facade re-exports it.

⚠ No system in `crates/` or `game/` writes a Bevy `WindowMode`. The setting is
stored and shown, but it does not change the window. Verify with
`rg -n "WindowMode" crates game` before you rely on it.

## Scaling policy

The simulation uses Ambition Engine world units and Bevy's default orthographic
2D convention, where one world unit is close to one logical pixel. A larger window
reveals more of the room; it does not stretch the game. Camera clamping uses the
active `Window` dimensions, not the startup defaults.
