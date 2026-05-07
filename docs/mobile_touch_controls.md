# Mobile / Android touch controls

Status (2026-05-07): foundation landed. The `mobile_touch` feature
flag is default-enabled; the `mobile_input` module exposes a pure
helper + a Bevy plugin behind the flag that wires `virtual_joystick`
to the engine's `ControlFrame`. **Touch buttons (Jump/Attack/Dash/...)
are not yet authored as on-screen UI — only the two analog sticks are
wired today.** RL agents and tests can still use the pure helper to
construct touch state from any code path.

## Goal

A sideloadable Pixel-class Android demo where the sandbox is playable
with on-screen joysticks + buttons. Polished mobile UX is **not** a
goal; the bar is "the Ambition sandbox loop runs on a phone". The
existing keyboard + gamepad pipeline stays canonical for desktop.

## Architecture

```text
+---------------------+         +-----------------------------+
| virtual_joystick    |  Move   | mobile_input::bevy_plugin   |
| stick UI (Bevy ECS) +-------->|  read_joystick_messages     |
+---------------------+  Aim    |    (writes MobileTouchState)|
                                |  fold_to_control_frame      |
+---------------------+ buttons |    (writes ControlFrame)    |
| Bevy UI buttons     +-------->|                             |
| (Jump/Attack/Dash...)         +--------------+-------------+
+---------------------+                        |
                                               v
                                +------------------------------+
                                | crate::input::ControlFrame  |
                                | (the canonical sim seam)     |
                                +------------------------------+
                                               |
                                               v
                                       sandbox simulation
```

## Pure helper

`mobile_input::fold_touch_into_control_frame(state, move_dz, aim_dz)`
takes a `TouchInputState` snapshot and returns a `ControlFrame`.
No Bevy / virtual_joystick deps; fully unit-tested. Use it from:

- The Bevy plugin (production path)
- RL agents that want to drive the sim from a touch-shaped input
  source (tests, fuzz harnesses)
- Any future input integration that produces stick + button data

## Bevy plugin (`mobile_input::bevy_plugin`)

Behind the `mobile_touch` feature. To enable:

```bash
cargo run -p ambition_sandbox --features mobile_touch
```

Default features include `mobile_touch`, so a plain `cargo run`
already picks it up.

The plugin:

- Adds `VirtualJoystickPlugin<MobileStick>` (where `MobileStick` is
  `{Move, Aim}`).
- Inserts a `MobileTouchState(TouchInputState)` resource.
- Registers an Update system chain:
  1. `read_joystick_messages`: reads `VirtualJoystickMessage<MobileStick>`
     and updates `MobileTouchState` (with the +Y-down sign flip).
  2. `fold_to_control_frame`: calls
     `fold_touch_into_control_frame(state, 0.05, 0.10)` and writes
     `ControlFrame`.

## Disabling for stripped builds

Mobile + RL features can both be disabled for distribution / console
ports / minimal builds:

```bash
cargo build -p ambition_sandbox --no-default-features --features visible
```

This skips `virtual_joystick` (mobile_touch) and the `crate::rl`
module (rl). Headless / RL binaries (rl_random_walker / rl_smoke /
trace_replay / headless) all `required-features = ["rl"]` so they
naturally drop out of a no-rl build.

## What's not done yet

- **Touch buttons**: Jump / Attack / Dash / Blink / Interact /
  Projectile / Start / Reset. Need a small Bevy UI layout that spawns
  a row of buttons + an Interaction-driven system that updates
  `TouchInputState.<button>` per frame. The pure helper already
  consumes them; only the UI authoring is missing.
- **On-screen stick spawn**: the plugin loads `VirtualJoystickPlugin`
  but doesn't spawn the actual stick UI nodes. Existing
  `virtual_joystick` examples use `create_joystick(...)` with custom
  art (Knob.png / arrows). The Bevy startup system to do this is a
  follow-up.
- **Android target build verified**: the plugin compiles for desktop
  but the `cargo apk` / `cargo ndk` pipeline isn't wired in this
  repo yet.

## TODO row

See `TODO.md` → "Android demo touch controls via `virtual_joystick`
+ ControlFrame bridge". Tracks the remaining work.
