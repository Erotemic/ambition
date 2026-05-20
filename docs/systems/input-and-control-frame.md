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
- `crates/ambition_sandbox/src/player/components.rs::PlayerInputFrame` — per-player input snapshot component (OVERNIGHT-TODO #17.5). The global `Res<ControlFrame>` resource still represents the local primary player's input; `sync_local_player_input_frame` mirrors it onto this component each tick so simulation systems can move toward reading input from a `Query<&PlayerInputFrame>` rather than the global resource. Future remote / co-op players carry their own `PlayerInputFrame` populated by a network adapter or a second input device, bypassing the global resource entirely.

## Per-player input migration status

Today's readers of player input come in two flavors:

- **In-phase `Res<ControlFrame>` readers** — `apply_player_reset_input_system`, `input_timer_system`, and `interaction_input_system` run inside the `PlayerInput` schedule set and mutate or consume the resource mid-phase. They stay on the resource because the sync system runs at the tail of the same set; reading from the component there would observe the previous frame's snapshot. These also include the writers (`apply_player_reset_input_system` clears `reset_pressed` so the engine path doesn't re-trigger).
- **Out-of-phase `&PlayerInputFrame` readers** — `update_projectiles` (Combat), `sandbox_update` (PlayerSimulation), `attack_advance_system` (Combat), `record_frame_system` (Trace). These run after the sync system has mirrored the resource onto the component, so they see the current tick's input. New gameplay systems should follow this pattern.

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
