# `ambition_touch_input` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_touch_input** — Mobile / touch presentation-input adapter for the Android demo path.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bevy_plugin`](src/bevy_plugin.rs) | The Bevy wiring: the touch HUD's spawn/despawn lifecycle and the collect step that turns joystick + virtual-button UI state into the virtual device's `MobileTouchState`. |
| [`exclusion`](src/exclusion.rs) | Touch-control exclusion zones for menu drag gestures. |
| [`layout`](src/layout.rs) | Touch HUD layout: action button identity, fixed positions, and visible-circle hit testing. |
| [`menu_bridge`](src/menu_bridge.rs) | The touch pointer-GESTURE lane and the touch active-input marker. |
| [`state`](src/state.rs) | Pure touch input state types — the raw virtual-device state the Bevy collect systems fill and the leafwing input kinds (`crate::virtual_device`) publish through the participant's bindings. |
| [`virtual_device`](src/virtual_device.rs) | The touch overlay as a VIRTUAL DEVICE: leafwing input kinds computed from [`MobileTouchState`], so touch resolves through the participant's `InputMap` bindings and the active input context exactly like a keyboard or gamepad — never as a second system writing gameplay/menu resources directly. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
