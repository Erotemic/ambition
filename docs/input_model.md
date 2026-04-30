# Ambition input model

Ambition treats physical inputs as mappings onto semantic actions. The sandbox
uses named presets now, and the code path is intentionally preset/remap-friendly
rather than hard-coded to one keyboard layout.

`F9` cycles to the previous preset. `F10` cycles to the next preset.

## Current keyboard presets

| Preset | Movement | Jump | Attack / slash | Dash | Pogo | Other mapped placeholders |
|---|---|---|---|---|---|---|
| `HollowKnight` | Arrow keys | `Z` | `X` | `C` | Down + `X` | Focus `A`, quick cast `E`, super dash `S`, dream nail `D`, map `Tab`, inventory `I` |
| `WasdJkl` | `WASD` | `Space` | `J` | `K` | Down + `J` | Focus `L`, quick cast `I`, super dash `Left Shift`, dream nail `U`, map `Tab`, inventory `V` |
| `ArrowsQwer` | Arrow keys | `Q` | `E` | `W` | `R` or Down + `E` | Map `Tab`, inventory `I` |
| `WasdUipo` | `WASD` | `U` | `P` | `I` | `O` or Down + `P` | Map `Tab`, inventory `V` |

The Hollow Knight preset is the default because it gives a known-good baseline
for keyboard platformer muscle memory.

## Canonical gamepad target

| Gamepad control | Semantic action | Current gameplay meaning |
|---|---|---|
| L-stick / D-pad | Movement | Move, aim dash, aim slash/pogo |
| A / Cross | Jump | Jump / confirm |
| X / Square | Attack / nail | Slash; Down+Attack is pogo |
| RT / R2 | Dash | Dash |
| B / Circle | Focus / cast | Placeholder |
| RB / R1 | Quick cast | Placeholder |
| LT / L2 | Super dash | Placeholder |
| Y / Triangle | Dream nail | Placeholder |
| LB / L1 | Quick map | Placeholder |
| Back / Touchpad | Inventory/select | Inventory later; sandbox reset for now |
| Start / Options | Pause | Pause/freeze |

## Universal sandbox/system controls

| Input | Semantic control | Current behavior |
|---|---|---|
| `Escape` | Start | Pause/freeze |
| `Delete` / `Backspace` | Select / reset | Reset to spawn |
| `F1` | Debug | Toggle overlay |
| `F2` | Slow motion | Toggle slow motion |
| `F9` | Preset previous | Cycle backward through presets |
| `F10` | Preset next | Cycle forward through presets |

## Implementation note

The current implementation lives in `crates/ambition_sandbox/src/main.rs` as
`KeyboardPreset`, `MovementKeys`, `ActionKeys`, and `ControlFrame`. The engine
still consumes a compact `InputState`, so key remapping can evolve without
coupling movement physics to physical devices.

## Current action semantics

Only movement, jump, attack, dash, pogo, pause, and reset affect gameplay in
this prototype. The other Hollow Knight / gamepad-style verbs are deliberately
kept in the preset structures and shown in the debug overlay so future engine
work can attach mechanics without changing the physical layout model.

The current pogo rule is:

- Hollow Knight style: hold Down and press Attack.
- Chirality test layouts: use the dedicated fourth action key, or hold Down and
  press Attack.

