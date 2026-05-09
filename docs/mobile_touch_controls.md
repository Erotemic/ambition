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

- **Settings-menu wiring for `TouchControlsVisible`**: the toggle
  is a public resource but no UI exposes it yet. F2-style hotkeys
  are intentionally avoided per Jon's "move all dev hotkeys into
  the settings menu" rule.
- **Android target build verified**: the plugin compiles for desktop
  but the `cargo apk` / `cargo ndk` pipeline isn't wired in this
  repo yet. Real-device touch-multitouch testing also depends on
  this.

## What's done

- ✅ Pure helper + 10 unit tests (deadzone, edge semantics, all
  buttons, threshold-pressed)
- ✅ Bevy plugin behind `mobile_touch` feature
- ✅ Move + Aim joysticks spawned on Startup with procedural circle
  textures (Knob + outline)
- ✅ 6 action buttons (Jump/Atk/Dash/Blink/E/Proj) + 2 menu buttons
  (Pause/Reset) with text labels
- ✅ Bottom-right cluster + bezel layout (192x128 + 216x152 backdrop)
- ✅ TouchControlsVisible runtime toggle
- ✅ Activity-gated ControlFrame write (empty touch state doesn't
  stomp keyboard input)
- ✅ Mouse-drag works on desktop because `virtual_joystick` and
  Bevy `Interaction` route mouse + touch through the same path
- ✅ Edge-derivation in `read_joystick_messages` (move_y crossings
  populate `move_y_just_crossed_up` / `move_y_just_crossed_down`
  so held Down doesn't keep firing the down_pressed edge)

## Limitations

- Mouse single-pointer can't test simultaneous two-thumb gestures
  (drag-Move + tap-Jump at the same time). For real touch
  multitouch testing, build for Android and run on a phone /
  emulator.
- Mouse-click on the joystick: `virtual_joystick` produces messages
  on `mouse_buttons.just_pressed(MouseButton::Left)` AND on drag.
  A bare click without drag will start a "press" but the resulting
  axis is zero (knob at center). To actually fire ControlFrame
  values you have to drag the knob away from center.

## Keyboard + touch interaction (per Jon's intent)

Implemented via the merge-fold in `fold_to_control_frame`:

- **Movement axis**: mutually exclusive. If the touch stick is past
  its deadzone, touch wins; otherwise the keyboard axis passes
  through unchanged. This is "disable the touch dpad when I'm
  using the keyboard arrows, and disable the keyboard arrows when
  I'm using the touch dpad."
- **Action buttons** (Jump / Attack / Dash / Blink / Interact /
  Projectile / Reset / Start): OR-merge. A held touch button OR
  a held keyboard button counts as held. Edge flags merge similarly.
  This is "the held/release buttons for actions I think should be
  independent."
- **Aim**: same mutually-exclusive shape as movement (touch wins
  past deadzone, otherwise keyboard).

The activity gate (`touch_state_is_active`) keeps a neutral touch
state from stomping keyboard input.

## TODO row

See `TODO.md` → "Android demo touch controls via `virtual_joystick`
+ ControlFrame bridge". Tracks the remaining work.


## Menu/dialog touch policy

During menu-like modes, mobile touch should feed semantic menu intent rather
than gameplay movement. The on-screen joystick may act like directional menu
navigation, drag gestures should accumulate into scroll steps, and mouse input
may proxy touch on desktop for testing. Gameplay `ControlFrame` movement should
be suppressed while dialog/pause/menu state owns the input.

Shared list/windowing, pointer-row activation, and drag-scroll accumulation now
live in `crate::ui_nav` (`crates/ambition_sandbox/src/ui_nav/`). Pause menus,
dialog choices, and the mobile touch bridge should reuse those helpers instead
of reimplementing scroll sign conventions or visible-window math locally.

The idle touch HUD should remain visible enough to teach the controls, but less
visually dominant until the user actively touches or drags a control.
