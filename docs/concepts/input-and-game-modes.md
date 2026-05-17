---
id: input-and-game-modes
aliases:
  - Leafwing controls
  - ControlFrame
  - semantic input
  - pause mode
  - menu navigation
  - touch controls
implemented_by:
  - crates/ambition_sandbox/src/input.rs
  - crates/ambition_sandbox/src/pause_menu.rs
  - crates/ambition_sandbox/src/map_menu.rs
  - crates/ambition_sandbox/src/time.rs
related_docs:
  - docs/input_model.md
  - docs/menu_navigation.md
  - docs/mobile_touch_controls.md
  - docs/game_mode_pause.md
related_memory:
  - dev/journals/lessons_learned.md
  - dev/benchmark-candidates/ui-nav-refactor-questions.md
  - dev/benchmark-candidates/ui-nav-test-questions.md
last_verified: 2026-05-17
---

# Input and game modes

## Definition

Input and game modes cover semantic actions, control presets, menu/dialogue/cutscene/pause gating, touch/controller adapters, and `ControlFrame`-style normalized input state.

## Core invariants

- Gameplay systems should read semantic actions, not keyboard labels.
- Edge signals cannot be reconstructed from held state after the fact.
- Menu controls and gameplay controls have different semantics and should not silently share assumptions.
- App-wide modes use Bevy `States`; per-entity behavior should move toward state-machine vocabulary gradually.
- Touch UI should present action-shaped affordances, not keyboard-shaped labels.

## Edit protocol

1. Identify whether the change affects gameplay, menus, dialogue, cutscenes, or touch/mobile adapters.
2. Preserve semantic action names across input devices.
3. Search dev memory for edge-vs-held, menu, touch, Bevy UI, and mutable-query traps.
4. Add tests or manual validation notes for input-mode transitions.

## Validation

```bash
cargo test -p ambition_sandbox --lib input
cargo test -p ambition_sandbox --lib pause_menu
cargo run -p ambition_sandbox --bin headless
```

Manual playtesting is still important for feel and touch layout.
