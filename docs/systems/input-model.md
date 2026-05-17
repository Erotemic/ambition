# Ambition input model

Input is semantic and platform-aware. Physical keyboard, controller, and touch inputs should fold into shared gameplay/menu actions instead of separate gameplay paths.

## Current shape

- `SandboxAction` is the Bevy/Leafwing action vocabulary.
- `ControlFrame` is the simulation seam used by engine/headless/test paths.
- Touch input folds into the same control frame where possible.
- Menu navigation has separate actions so pause/settings/dialogue do not consume gameplay buttons accidentally.
- Platform-specific input should preserve controller and touch semantics.

## Current actions of note

- Movement, jump, dash, blink, attack, projectile, pogo, interact.
- Quick action currently carries shield input into engine `shield_held`.
- Projectile action supports Fireball and motion-input Hadouken upgrade.
- Menu actions are separate from gameplay actions.

## Common failure modes

- Updating keyboard bindings without controller/touch equivalents.
- Reading raw keys inside gameplay systems instead of semantic actions/control frames.
- Letting gameplay consume menu actions while paused/dialogue/cutscene modes are active.
- Adding platform-specific behavior that bypasses the shared simulation seam.

See `docs/concepts/platform-targets.md` and `docs/systems/mobile-touch-controls.md`.
