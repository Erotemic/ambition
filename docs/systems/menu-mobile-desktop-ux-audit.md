# Menu UX audit: mobile + desktop

Status: historical UX audit kept as risk context, not current system ownership documentation. The old pause/adventure/map menu files have been folded into the unified menu stack; use this document only for the remaining UX risks.

## Current menu stack

- `crates/ambition_input/src/menu.rs` defines semantic menu input.
- `crates/ambition_ui_nav/src/` contains shared list, pointer, and drag helpers.
- `crates/ambition_actors/src/menu/ir/` and `crates/ambition_actors/src/menu/map/` own sandbox menu data and map-tab behavior.
- `crates/ambition_menu/src/render/` owns reusable Bevy-UI/kaleidoscope renderers.
- `crates/ambition_app/src/menu/` owns app menu state, dispatch, pointer integration, and renderer wiring.
- `crates/ambition_app/src/host/mobile_input/menu_bridge.rs` folds touch/drag input into menu intent.

## Remaining UX risks

1. **Runtime layout profile.** Menu metrics should be selected from window size, input kind, safe area, and user scale rather than compile-time target checks.
2. **Shared metrics.** Tab bars, settings rows, dialog choices, inventory rows, and map rows should share a small `UiMetrics`/`UiProfile` vocabulary for touch targets, font size, gaps, panel padding, and scrollbar width.
3. **Touch activation.** Touch rows should distinguish press, drag, release-inside, and cancel-outside instead of treating every press as activation.
4. **Drag ownership.** List rows, scrollbars, sliders, tab bars, panel backgrounds, and fixed gameplay touch controls should explicitly own or reject drag gestures.
5. **Touch HUD versus menu touch input.** Hiding gameplay touch controls should not necessarily disable touch-driven menu scrolling/selection.
6. **Safe-area handling.** Mobile HUD and menu roots need an ECS-visible safe-area resource, even if it starts as manually configurable.

## Suggested next implementation seam

Add a runtime `UiProfile` + `UiMetrics` resource near the app/menu host layer, feed it from the primary window and active input kind, then migrate one menu backend at a time. Keep renderer behavior and UX-policy changes separate so regressions are easy to isolate.
