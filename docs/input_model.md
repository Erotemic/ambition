# Ambition input model

Ambition treats physical inputs as mappings onto semantic actions. The current
sandbox exposes two keyboard presets and a canonical future gamepad map.

## Semantic face-button actions

| Semantic action | Gamepad label | Current gameplay meaning |
|---|---|---|
| `A` | South / A | Jump / confirm |
| `B` | East / B | Dash / cancel |
| `Y` | North / Y | Slash / attack |
| `X` | West / X | Dedicated downward/pogo slash / alternate attack |

## Keyboard presets

| Preset | Movement | A | B | Y | X | Switch |
|---|---|---|---|---|---|---|
| `ArrowsQwer` | Arrow keys | `Q` | `W` | `E` | `R` | `F9` |
| `WasdUipo` | `WASD` | `U` | `I` | `P` | `O` | `F10` |

Universal sandbox/system controls:

| Input | Semantic control | Current behavior |
|---|---|---|
| `Escape` | Start | Pause/freeze |
| `Delete` / `Backspace` | Select / reset | Reset to spawn |
| `F1` | Debug | Toggle overlay |
| `Tab` | Slow motion | Toggle slow motion |

## Reserved controls

Bumpers and triggers are not required by the current sandbox. They are reserved
as innocuous placeholders for future chord, stance, shoulder-swap, analog aim,
or math-mode modifiers.

## Implementation note

The current implementation lives in `crates/ambition_sandbox/src/main.rs` as
`KeyboardPreset`, `MovementKeys`, `FaceKeys`, and `ControlFrame`. The engine
still consumes a compact `InputState`, so key remapping can evolve without
coupling movement physics to physical devices.
