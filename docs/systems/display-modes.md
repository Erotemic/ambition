# Display Modes and Window Scaling

Ambition starts in ordinary resizable **windowed** mode. The default logical
window size is `1600 x 900` (`config::WINDOW_W` / `config::WINDOW_H`), matching
the current authored 16:9 composition, but the user can resize the window.

## Display-mode ownership

Display mode is user-facing configuration, not a developer hotkey. Use the
pause menu's **Settings → Display Mode** row to cycle Windowed, Borderless, and
Fullscreen with Left/Right and Confirm.

The former F6/F7 window-mode shortcuts were removed. Those keys are now owned by
the canonical developer deck for FPS-overlay and portal-gun diagnostics. See
[`developer-hotkeys.md`](developer-hotkeys.md).

The mode-application logic remains in `settings::apply_display_mode`, so every
menu backend produces the same `WindowMode` mapping.

## Scaling policy

The current Bevy sandbox keeps the simulation in Ambition Engine world units and
uses Bevy's default orthographic 2D convention where one world unit is close to
one logical pixel. Larger windows therefore reveal more room instead of
stretching the game.

Camera clamping uses the active `Window` dimensions, not the startup defaults, so
resized, borderless, and fullscreen modes should all follow the player without
cutting off the room edges.

## Future options

If the game must always render at a fixed aspect ratio, the next step is camera
viewport letterboxing/pillarboxing. For now the sandbox accommodates the user's
requested window size because it is a movement lab and seeing more of a large
room is useful.
