# UI navigation and pause

The pause/menu layer is a gameplay mode boundary. Pausing should stop ordinary gameplay simulation, route input to UI, and keep settings/inventory/map flows from mutating player motion accidentally.

## Current shape

```text
GameMode
  Playing
  Paused / menu-facing modes
        ↓
Input menu bridge
        ↓
Pause menu, inventory, map menu, settings pages
```

Important paths:

- `crates/ambition_sandbox/src/runtime/game_mode.rs` — coarse game mode and gameplay gating helpers.
- `crates/ambition_sandbox/src/pause_menu/` — pause menu model, input, pointer interaction, UI, tests.
- `crates/ambition_sandbox/src/inventory/` — inventory model/input/pointer/UI.
- `crates/ambition_sandbox/src/map_menu/` — map menu model/input/pointer/UI.
- `crates/ambition_sandbox/src/ui_nav/` — shared UI navigation vocabulary.
- `crates/ambition_sandbox/src/input/menu.rs` — menu-facing input interpretation.

## Rules

- Gameplay systems should be gated by `GameMode` or a named schedule set when pause must stop them.
- UI screens should consume menu actions rather than raw device input.
- Settings changes should mutate `SettingsState` and persist through the settings persistence layer.
- Pointer/touch support should route through each menu's pointer module instead of special-casing the visual tree.
- Do not put UI layout policy in `engine_core` or reusable mechanics modules.

## Validation anchors

```bash
cargo test -p ambition_sandbox pause_menu
cargo test -p ambition_sandbox inventory
cargo test -p ambition_sandbox map_menu
cargo test -p ambition_sandbox ui_nav
```

Related docs: `docs/concepts/input-and-game-modes.md`, `docs/systems/input-and-control-frame.md`, `docs/systems/settings-and-persistence.md`.
