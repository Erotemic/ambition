# Input and control frame

Ambition normalizes physical input into gameplay-facing actions and control frames before simulation code consumes it. Keyboard, controller, and mobile/touch sources should not leak device-specific policy into movement and combat systems.

## Current shape

```text
Physical devices / host adapters
  keyboard, gamepad, mobile touch
        ↓
ambition_input vocabulary
  actions, control frame, menu navigation, presets
        ↓
Bevy integration / host bridges
        ↓
player brain, movement, attacks, dash, blink, menus, map, settings
```

Important paths:

- `crates/ambition_input/src/` — action vocabulary, control-frame normalization, keyboard/controller presets, menu input, and input tests.
- `crates/ambition_gameplay_core/src/app/input_systems.rs` — Bevy systems that collect and apply local input to the sandbox simulation.
- `crates/ambition_app/src/host/mobile_input/` — touch layout and touch-to-control/menu bridge.
- `crates/ambition_gameplay_core/src/persistence/settings/controls.rs` — persisted control settings, deadzones, trigger thresholds, dash-repeat policy, controller profile defaults.
- `crates/ambition_gameplay_core/src/player/components.rs::PlayerInputFrame` — per-player input snapshot. The global `Res<ControlFrame>` is still the local primary-player source; `sync_local_player_input_frame` mirrors it onto the component each tick so actor systems can read per-entity input.

## Reader rules

- In-phase input systems that mutate or consume the primary `ControlFrame` may keep reading the resource.
- Player brain ticking, player simulation, attack lifecycle, and trace systems should prefer `PlayerInputFrame` or the brain/action seam after sync.
- New actor verbs should usually become `ActorActionMessage` / `ActionRequest` paths rather than direct input reads.
- Direct `PlayerInputFrame` reads are appropriate only for explicitly player-local policy.

## General rules

- Gameplay should read action/control-frame state, not raw device state.
- Device calibration belongs in input settings and host adapters.
- Hysteresis belongs where analog triggers become discrete actions.
- Menu navigation uses the shared input vocabulary but has its own repeat/selection semantics.
- Mobile/touch must bridge into the same gameplay actions so mechanics stay testable.

## Validation anchors

```bash
cargo test -p ambition_input
cargo test -p ambition_gameplay_core input
cargo test -p ambition_app mobile_input
```

Related docs: `docs/concepts/input-and-game-modes.md`, `docs/systems/ui-navigation-and-pause.md`, `docs/systems/settings-and-persistence.md`, `docs/systems/mobile-touch-controls.md`.
