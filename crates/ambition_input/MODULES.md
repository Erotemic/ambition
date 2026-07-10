# `ambition_input` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_input** — Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actions`](src/actions.rs) | The `SandboxAction` leafwing action enum — the logical-input vocabulary the device-binding layer maps physical keys/sticks onto, before it is folded into the device-agnostic `ControlFrame`/`MenuInputFrame`. |
| [`active_input`](src/active_input.rs) | Which input source is CURRENTLY active — the last one to produce GENUINE input. |
| [`control`](src/control.rs) | Device adapters that build the engine-owned `ControlFrame` resource. |
| [`menu`](src/menu.rs) | Menu-side input vocabulary: the device-agnostic `MenuInputFrame` / `MenuControlFrame` / `MenuInputState` resources and the `MenuDir` / `analog_to_dir` helpers. |
| [`motion_input`](src/motion_input.rs) | Quarter-circle / half-circle motion-input recognition. |
| [`presets`](src/presets.rs) | Default binding presets: the selectable keyboard layouts (`PresetId` / `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad map (`GAMEPAD_MAP`) that seed leafwing's input map for `SandboxAction`. |
| [`settings`](src/settings.rs) | Controls / input settings. |

_7 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
