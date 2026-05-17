# Input and control frame

Ambition normalizes physical input into gameplay-facing actions and control frames before simulation code consumes it. This keeps keyboard, controller, and mobile/touch sources from leaking device-specific policy into movement and combat systems.

## Current shape

```text
Physical devices / host adapters
  keyboard, gamepad, mobile touch
        ↓
crates/ambition_sandbox/src/input/
  actions, control frame, menu navigation, presets, tests
        ↓
Gameplay systems
  player movement, attacks, dash, blink, menus, pause, inventory, map
```

Important paths:

- `crates/ambition_sandbox/src/input/` — action vocabulary, control-frame normalization, keyboard/controller presets, menu input.
- `crates/ambition_sandbox/src/app/input_systems.rs` — Bevy systems that collect and apply input.
- `crates/ambition_sandbox/src/host/mobile_input/` — touch layout and touch-to-control bridge.
- `crates/ambition_sandbox/src/persistence/settings/controls.rs` — persisted control settings, deadzones, trigger thresholds, dash-repeat policy, controller profile defaults.

## Rules

- Gameplay should read action/control-frame state, not raw device state.
- Device calibration belongs in input settings and host adapters.
- Hysteresis belongs at the input edge where analog triggers become discrete actions.
- Menu navigation uses the shared input vocabulary but has its own repeat/selection semantics.
- Mobile/touch must bridge into the same gameplay actions so mechanics stay testable.

## Validation anchors

```bash
cargo test -p ambition_sandbox input
cargo test -p ambition_sandbox mobile_input
```

Related docs: `docs/concepts/input-and-game-modes.md`, `docs/systems/ui-navigation-and-pause.md`, `docs/systems/settings-and-persistence.md`, `docs/systems/mobile-touch-controls.md`.
