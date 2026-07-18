---
id: input-and-game-modes
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
implemented_by:
  - crates/ambition_input
  - crates/ambition_characters/src/action_scheme.rs
  - crates/ambition_sim_view/src/control_prompt.rs
  - crates/ambition_touch_input
  - crates/ambition_menu
  - crates/ambition_ui_nav
related_adrs:
  - docs/adr/0025-character-actions-input-ownership.md
related_docs:
  - docs/systems/input-control-and-ui.md
---

# Input and game modes

Input has three distinct responsibilities: translating physical devices into
semantic intent, assigning control authority to a subject, and resolving that
subject's current capabilities into actions and prompts.

## Canonical flow

```text
keyboard/gamepad/touch/RL/brain
    -> semantic source frame
    -> control authority / controlled subject
    -> actor-local control intent
    -> ActorActionScheme
    -> shared slot resolver
    -> movement / MovePlayback / interaction
    -> ControlPrompt for UI, touch, and dynamic labels
```

`ActorActionScheme` is derived from live authorities such as abilities, moveset,
and registered techniques. It is not an independent authored truth. The shared
resolver both gates gameplay and produces the meaning shown through
`ControlPrompt`; UI must not maintain its own capability logic.

## Invariants

- Gameplay reads semantic actions, never keyboard labels.
- Pressed/held/released edges are captured at the source; they are not recreated
  later from held state.
- Devices provide slots. Characters decide what those slots mean.
- A missing/unavailable action is stripped before simulation and hidden/disabled
  in presentation through the same resolved contract.
- Humans, brains, temporary possession, and RL ultimately drive the same
  actor-local action/body paths.
- Menu/dialogue/shell input is semantically separate from gameplay input even
  when the same physical button is used.
- Dialogue choice selection is one semantic cursor: keyboard arrows, D-pad,
  physical/touch sticks, mouse wheel, touch drag, pointer presses, and semantic
  Confirm all converge on `MenuControlFrame` / `DialogState`. A long-list
  presenter windows that cursor; it does not own a second scroll selection.
- Pointer press behavior comes from `MenuTapMode`, while genuine active-input
  tracking prevents a stationary mouse from stealing focus after a windowed UI
  rebuild. Direct touch upgrades the desktop guard default to select-then-confirm
  so a finger press can safely turn into a drag without activating a choice.
- App-wide modes use explicit state/scope vocabulary; per-entity authority stays
  on the controlled entity or its relationships.
- Touch controls are action-shaped and prompt-driven, not a fixed picture of a
  keyboard layout.
- `InputState::control_dt` carries real-time control responsiveness when needed;
  it does not create a second simulation clock or player-only body tick.

## Ownership guide

- `ambition_input`: device bindings, semantic gameplay/menu frames, presets,
  motion-input recognition, active-source policy.
- `ambition_characters`: actor-local control, action schemes, brains, perception.
- `ambition_sim_view`: `ControlPrompt` and other observation read models.
- `ambition_touch_input`: touch HUD and fold into the same semantic frames.
- `ambition_menu`, `ambition_settings_menu`, `ambition_ui_nav`: renderer-neutral
  menu models/settings IR and navigation helpers.
- shell/app/host crates: top-level routing, focus, pause/home/product policy.

## Validation

```bash
./run_tests.sh -p ambition_input
./run_tests.sh -p ambition_touch_input
./run_tests.sh -k action_scheme
./run_tests.sh -k control_prompt
./run_tests.sh -k possession
```

Manual checks remain necessary for touch layout, controller feel, and focus
transitions, but they supplement rather than replace shared-resolution tests.
