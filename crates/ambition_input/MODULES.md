# `ambition_input` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_input** — Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actions`](src/actions.rs) | The `Platformer2dInputActionMonolith` leafwing action enum — the logical-input vocabulary the device-binding layer maps physical keys/sticks onto, before it is folded into the device-agnostic `ControlFrame`/`MenuInputFrame`. |
| [`active_input`](src/active_input.rs) | Which input device each SEAT most recently produced GENUINE input with. |
| [`bindings`](src/bindings.rs) | **One authority for "which physical control is this action on".** |
| [`channels`](src/channels.rs) | **A local input SOURCE is not a control CHANNEL, and one integer meant both.** |
| [`control`](src/control.rs) | Device adapters that build the engine-owned `ControlFrame` resource. |
| [`cues`](src/cues.rs) | Resolved UI cues — what the submit-functional controls DO right now, in the owning surface's own words. |
| [`glyphs`](src/glyphs.rs) | Device-conditional glyph rendering for a seat's bindings. |
| [`local_seats`](src/local_seats.rs) | **Which physical device drives which local seat.** (C4 slice 5) |
| [`menu`](src/menu.rs) | Menu-side input vocabulary: the device-agnostic `MenuInputFrame` / `MenuControlFrame` / `MenuInputState` resources and the `MenuDir` / `analog_to_dir` helpers. |
| [`motion_input`](src/motion_input.rs) | Motion-input gesture recognition: a rolling directional buffer, a generic ordered-subsequence matcher ([`MotionInputBuffer::detect_sequence`]), and an **open, content-owned** [`MotionTechniqueCatalog`] of named techniques. |
| [`participant`](src/participant.rs) | The persistent input participant — the person in front of a controller. |
| [`presets`](src/presets.rs) | Default binding presets: the selectable keyboard layouts (`PresetId` / `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad bindings that seed leafwing's input map for `Platformer2dInputActionMonolith`. |
| [`rebind`](src/rebind.rs) | **Turning "the player pressed a thing" into a persisted binding override.** |
| [`semantic`](src/semantic.rs) | **Semantic actions: the open vocabulary between a device and a consumer.** |
| [`settings`](src/settings.rs) | Controls / input settings. |
| [`sources`](src/sources.rs) | **Which physical input SOURCE a participant owns.** |

_16 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
