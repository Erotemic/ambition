# `ambition_touch_input` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_touch_input** — Touch input adapter and on-screen controls.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bevy_plugin`](src/bevy_plugin.rs) | Bevy wiring for touch input: the touch HUD's spawn and visibility lifecycle, and the collect step that turns joystick and button UI state into the virtual device's `MobileTouchState`. |
| [`layout`](src/layout.rs) | Layout values are bound to the visible circle, not to the square `Node` bounds. |
| [`menu_bridge`](src/menu_bridge.rs) | The touch pointer-gesture lane and the touch active-input marker. |
| [`placement`](src/placement.rs) | The touch HUD's resolved on-screen placement. |
| [`state`](src/state.rs) | Pure touch input state types — the raw virtual-device state the Bevy collect systems fill and the leafwing input kinds (`crate::virtual_device`) publish through the participant's bindings. |
| [`virtual_device`](src/virtual_device.rs) | The touch overlay as a virtual device: leafwing input kinds computed from [`MobileTouchState`], so touch resolves through the participant's `InputMap` bindings and the active input context like a keyboard or gamepad, never as a second system that writes gameplay or menu resources. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
