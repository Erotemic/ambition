# Ambition input model

Ambition treats physical inputs as mappings onto semantic actions. The sandbox
uses named presets now, and the code path is intentionally preset/remap-friendly
rather than hard-coded to one keyboard layout.

`F9` cycles to the previous preset. `F10` cycles to the next preset.

## Current keyboard presets

| Preset | Movement | Jump | Attack / slash | Dash | Pogo | Other mapped placeholders |
|---|---|---|---|---|---|---|
| `ArrowsZxc` | Arrow keys | `Z` | `X` | `C` | Down + `X` | Secondary `A`, quick `E`, modifier `S`, utility `D`, map `Tab`, inventory `I` |
| `WasdJkl` | `WASD` | `Space` | `J` | `K` | Down + `J` | Secondary `L`, quick `I`, modifier `Left Shift`, utility `U`, map `Tab`, inventory `V` |
| `ArrowsQwer` | Arrow keys | `Q` | `E` | `W` | `R` or Down + `E` | Map `Tab`, inventory `I` |
| `WasdUipo` | `WASD` | `U` | `P` | `I` | `O` or Down + `P` | Map `Tab`, inventory `V` |

The `ArrowsZxc` preset is the default because it gives a compact, familiar
keyboard action-platformer baseline without baking any specific game's verbs
into Ambition's terminology.

## Canonical gamepad target

| Gamepad control | Semantic action | Current gameplay meaning |
|---|---|---|
| L-stick / D-pad | Movement | Move, aim dash, aim slash/pogo |
| A / Cross | Jump | Jump / confirm |
| X / Square | Primary attack | Slash; Down+Attack is pogo |
| RT / R2 | Dash | Dash |
| B / Circle | Secondary action | Placeholder |
| RB / R1 | Quick action | Placeholder |
| LT / L2 | Modifier action | Placeholder |
| Y / Triangle | Utility action | Placeholder |
| LB / L1 | Map | Placeholder |
| Back / Touchpad | Inventory/select | Inventory later; sandbox restart for now |
| Start / Options | Pause | Pause/freeze |

## Universal sandbox/system controls

| Input | Semantic control | Current behavior |
|---|---|---|
| `Escape` | Start | Pause/freeze |
| `Delete` / `Backspace` | Select / restart | Full sandbox restart, including enemies and transient effects |
| `F1` | Debug | Toggle overlay |
| `F2` | Slow motion | Toggle slow motion |
| `F9` | Preset previous | Cycle backward through presets |
| `F10` | Preset next | Cycle forward through presets |

## Implementation note

The current Bevy implementation lives in `crates/ambition_sandbox/src/main.rs` as
`KeyboardPreset`, `MovementKeys`, `ActionKeys`, and `ControlFrame`. The engine
still consumes a compact `InputState`, so key remapping can evolve without
coupling movement physics to physical devices.

## Current action semantics

Only movement, jump, attack, dash, pogo, pause, and restart affect gameplay in
this prototype. The other generic gamepad-style verbs are deliberately kept in
the preset structures and shown in the debug overlay so future engine work can
attach mechanics without changing the physical layout model.

The current pogo rule is:

- Most action-platformer layouts: hold Down and press Attack.
- Chirality test layouts: use the dedicated fourth action key, or hold Down and
  press Attack.
