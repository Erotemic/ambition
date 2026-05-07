# Leafwing Input Manager port

The sandbox now uses `leafwing-input-manager` for the Bevy-facing input layer.
The movement engine still receives a compact `InputState`; Leafwing is used to
collect keyboard and gamepad inputs into semantic `SandboxAction` values before
that engine input frame is built.

## Why this boundary

- `ambition_engine` remains backend-neutral and testable.
- `ambition_sandbox` owns Bevy plugins, physical key/gamepad bindings, preset
  cycling, debug hotkeys, HUD text, and other presentation concerns.
- Future rebinding can replace the player's `InputMap<SandboxAction>` without
  rewriting movement, combat, doors, blink, fly, or room transition code.

## Current action model

`SandboxAction::Move` is a Leafwing dual-axis action. It is bound to the active
keyboard preset's virtual D-pad, the gamepad D-pad, and the left stick. The
individual `MoveUp`, `MoveDown`, `MoveLeft`, and `MoveRight` actions intentionally
share those keyboard direction keys so the sandbox can still detect gestures like
`just_pressed Up` and `just_pressed Down`.

The Bevy adapter converts Leafwing's conventional `+Y = up` movement vector into
Ambition's current screen-space simulation convention where `+Y = down`.

## Preset cycling

`F9` and `F10` still cycle the four built-in keyboard presets. Cycling now swaps
the player entity's `InputMap<SandboxAction>` and calls `ActionState::reset_all()`
so held keys from the previous preset do not stick across a layout change.

## Gamepad status

The first useful gamepad mapping is wired now:

| Control | Action |
|---|---|
| Left stick / D-pad | Move / aim |
| South / A / Cross | Jump |
| West / X / Square | Attack |
| RightTrigger2 / RT / R2 | Dash |
| East / B / Circle | Blink / secondary |
| North / Y / Triangle | Utility / fly toggle |
| Start | Pause |
| Select / Back | Reset, and inventory placeholder |

The next refinement should be explicit gamepad assignment for local multiplayer
or multiple connected controllers. Leafwing supports assigning a specific gamepad
to an `InputMap`, but this sandbox currently accepts input from any connected
gamepad.

## Next steps

1. Move the preset definitions to a serializable config format, probably RON or
   TOML.
2. Add an input-rebinding menu that mutates the player `InputMap` in place.
3. Add tests that feed synthetic `ActionState<SandboxAction>` values into the
   same `ControlFrame` conversion path used by gameplay.
4. Consider splitting debug/system actions (`F1`, `F2`, `F9`, `F10`) into their
   own Leafwing action set if the sandbox debug shell grows further.
