# `ambition_input` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_input** — Device -> engine-owned `ControlFrame` input adapter layer for the sandbox.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`actions`](src/actions.rs) | The `Platformer2dInputActionMonolith` leafwing action enum — the logical-input vocabulary the device-binding layer maps physical keys/sticks onto, before it is folded into the device-agnostic `ControlFrame`/`MenuInputFrame`. |
| [`active_input`](src/active_input.rs) | Most recent genuine input device per seat and across the machine. |
| [`bindings`](src/bindings.rs) | One authority for "which physical control is this action on". |
| [`channels`](src/channels.rs) | Mapping between local input sources and dense session control channels. |
| [`control`](src/control.rs) | Device adapters that build the engine-owned `ControlFrame` resource. |
| [`cues`](src/cues.rs) | Resolved UI cues — what the submit-functional controls DO right now, in the owning surface's own words. |
| [`glyphs`](src/glyphs.rs) | Device-conditional glyph rendering for a seat's bindings. |
| [`layout`](src/layout.rs) | Game/mode-specific gamepad binding profiles. |
| [`local_seats`](src/local_seats.rs) | Local gamepad ownership for participant seats. |
| [`menu`](src/menu.rs) | Menu-side input vocabulary: the device-agnostic `MenuInputFrame` / `MenuControlFrame` / `MenuInputState` resources and the `MenuDir` / `analog_to_dir` helpers. |
| [`motion_input`](src/motion_input.rs) | Motion-input gesture recognition: a rolling directional buffer, a generic ordered-subsequence matcher ([`MotionInputBuffer::detect_sequence`]), and an open, content-owned [`MotionTechniqueCatalog`] of named techniques. |
| [`participant`](src/participant.rs) | The persistent input participant — the person in front of a controller. |
| [`presets`](src/presets.rs) | Default binding presets: the selectable keyboard layouts (`PresetId` / `KeyboardPreset` / `MovementKeys` / `ActionKeys`) and the shared gamepad bindings that seed leafwing's input map for `Platformer2dInputActionMonolith`. |
| [`rebind`](src/rebind.rs) | Pure capture of physical input into persisted binding overrides. |
| [`seating`](src/seating.rs) | WHERE A SESSION'S LOCAL SEATS COME FROM, and whose answer that is. |
| [`semantic`](src/semantic.rs) | Open semantic action vocabulary between physical bindings and consumers. |
| [`settings`](src/settings.rs) | Controls / input settings. |
| [`sources`](src/sources.rs) | Physical input-source identity and participant assignment policy. |

_18 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
