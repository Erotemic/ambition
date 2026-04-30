# Display Modes and Window Scaling

Ambition starts in ordinary resizable **windowed** mode. The default logical
window size is `1600 x 900`, matching the current authored 16:9 composition,
but the user can resize the window.

## Runtime hotkeys

- `F6`: windowed
- `F7`: borderless fullscreen on the current monitor
- `F8`: exclusive fullscreen using the monitor's current video mode

These are sandbox/debug hotkeys for now. Later, they should be exposed through a
settings menu and persisted to a user config file.

## Scaling policy

The current Bevy sandbox keeps the simulation in Ambition Engine world units and
uses Bevy's default orthographic 2D convention where one world unit is close to
one logical pixel. Larger windows therefore reveal more room instead of
stretching the game.

Camera clamping uses the active `Window` dimensions, not the startup defaults, so
resized, borderless, and fullscreen modes should all follow the player without
cutting off the room edges.

## Future options

The authored target aspect ratio is documented in `config::TARGET_ASPECT`. If we
later decide the game must always render at a fixed aspect ratio, the next step
would be to add camera viewport letterboxing/pillarboxing. For now we favor
accommodating the user's requested window size because this sandbox is a
movement lab and seeing more of a large room is useful.
