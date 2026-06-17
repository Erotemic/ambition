# UI navigation and pause

The menu layer is a gameplay-mode boundary. Pausing should stop ordinary gameplay simulation, route input to menu intent, and keep settings/inventory/map flows from mutating player motion accidentally.

## Current shape

```text
GameMode
  Playing
  Paused / menu-facing modes
        ↓
ambition_input menu vocabulary + app host bridge
        ↓
ambition_gameplay_core menu IR/map + ambition_menu renderers + app menu stack
```

Important paths:

- `crates/ambition_gameplay_core/src/runtime/game_mode.rs` — coarse game mode and gameplay gating helpers.
- `crates/ambition_input/src/menu.rs` — menu-facing input vocabulary.
- `crates/ambition_gameplay_core/src/ui_nav/` — shared list, pointer, and drag helpers.
- `crates/ambition_gameplay_core/src/menu/ir/` — reusable menu item/page IR.
- `crates/ambition_gameplay_core/src/menu/map/` — map-tab model, input, pointer, systems, and UI helpers.
- `crates/ambition_menu/src/render/` — reusable Bevy-UI and kaleidoscope render backends.
- `crates/ambition_app/src/menu/` — app-hosted menu state, dispatch, pointer tests, and renderer integration.

## Rules

- Gameplay systems should be gated by `GameMode` or a named schedule set when pause must stop them.
- UI screens should consume menu actions rather than raw device input.
- Settings changes should mutate `SettingsState` and persist through the settings persistence layer.
- Pointer/touch support should route through shared menu/pointer seams instead of special-casing visual trees.
- Do not put UI layout policy in `engine_core` or reusable mechanics modules.

## Validation anchors

```bash
cargo test -p ambition_input menu
cargo test -p ambition_gameplay_core menu
cargo test -p ambition_gameplay_core ui_nav
cargo test -p ambition_menu
cargo test -p ambition_app --tests --features "bevy_ui_menu kaleidoscope_menu input"
```

Related docs: `docs/concepts/input-and-game-modes.md`, `docs/systems/input-and-control-frame.md`, `docs/systems/settings-and-persistence.md`.
