# Display Modes and Window Scaling

Ambition starts in ordinary resizable **windowed** mode. The default logical
window size is `1600 x 900` (`config::WINDOW_W` / `config::WINDOW_H`), matching
the current authored 16:9 composition, but the user can resize the window.

## Runtime hotkeys (developer convenience)

- `F6`: windowed
- `F7`: borderless fullscreen on the current monitor

`F8` is **not** a display-mode hotkey; it triggers the gameplay trace
recorder dump (`crate::trace::handle_trace_hotkey`). Exclusive fullscreen
was previously bound to F8 but the binding was removed because exclusive
mode is rarely useful during sandbox development. To reach exclusive
fullscreen, use the pause menu's Settings → Display Mode row, which
cycles Windowed / Borderless / Fullscreen with Left/Right and Confirm.

The hotkeys remain as a dev shortcut so contributors can flip between
windowed and borderless without going through the menu while iterating.
The actual mode-application logic lives in `settings::apply_display_mode`
so the menu and hotkeys stay in lock-step.

## Scaling policy

The current Bevy sandbox keeps the simulation in Ambition Engine world units and
uses Bevy's default orthographic 2D convention where one world unit is close to
one logical pixel. Larger windows therefore reveal more room instead of
stretching the game.

Camera clamping uses the active `Window` dimensions, not the startup defaults, so
resized, borderless, and fullscreen modes should all follow the player without
cutting off the room edges.

## Future options

If we later decide the game must always render at a fixed aspect ratio, the
next step would be to add camera viewport letterboxing/pillarboxing. For now
we favor accommodating the user's requested window size because this sandbox
is a movement lab and seeing more of a large room is useful.
