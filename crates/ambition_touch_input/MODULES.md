# `ambition_touch_input` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_touch_input** — Mobile / touch presentation-input adapter for the Android demo path.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`bevy_plugin`](src/bevy_plugin.rs) | The Bevy wiring: the touch HUD's spawn/despawn lifecycle and the fold from joystick + virtual-button state into `ControlFrame`. |
| [`exclusion`](src/exclusion.rs) | Touch-control exclusion zones for menu drag gestures. |
| [`layout`](src/layout.rs) | Touch HUD layout: action button identity, fixed positions, and visible-circle hit testing. |
| [`menu_bridge`](src/menu_bridge.rs) | Bridge touch / mouse / joystick input into both the gameplay `ControlFrame` and the menu-side `MenuControlFrame`. |
| [`state`](src/state.rs) | Pure touch input state types and the `TouchInputState -> ControlFrame` fold helper. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
