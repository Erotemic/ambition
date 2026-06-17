# 3D-cube OoT pause-menu integration plan (#31)

Wiring the real `ambition_inventory_ui` rotating 3D-cube inventory/pause menu into
the game, runtime-toggleable with the current Bevy-UI grid. Jon's directions
(2026-06-04): full push (cube + 24 items + nav); **runtime toggle + trait/data
seam**; use **`ambition_mock_demo`** as the example; **promote the cube into the
reusable lib**; add the submodule dep (submodule edits OK); **wire our real
inventory in** ("the demo is a demo, we need to wire us up to use it").

## Status

- ✅ **Dependency** — `ambition_inventory_ui` added to `ambition_gameplay_core` (path dep,
  `optional`, gated by `oot_inventory`); root `Cargo.toml` `exclude`s the submodule
  (it's its own workspace). Resolves + compiles clean: the game and submodule are
  both **bevy 0.18**, so it unifies with `bevy_lunex`. (`build(#31)` commits.)
- ✅ **Data seam** — `crate::oot_cube` builds the cube's page models from our live
  24-item inventory: `Item::ALL` (already 24 in OoT grid order) → `InventoryItemNode`
  → `ItemsOnlyPageSpec` → `MenuPageModel<CubePage, CubeAction>`, with owned /
  equipped / selected reflected and a host `CubeAction` (Equip/Use/ChangePage)
  emitted back. `ItemsOnlyPageSpec` is explicitly renderer-agnostic (the lib doc:
  "feed the Lunex cube, a Bevy-UI grid fallback, or a test renderer"), so this IS
  the trait/data seam. Unit-tested (24 slots, flags). Items face live; Map/Quest/
  System are host-data placeholders.
- ⏳ **Cube renderer** — not yet promoted (the bottleneck; see Next steps).

## Architecture (what's where)

- **Reusable lib** (`submodules/ambition_inventory_ui`, the root crate) owns the
  DATA + config: `MenuPageModel`/`MenuNode` (generic over Page/Action),
  `ActiveMenuPages<PageId, Action>`, `ItemsOnlyPageSpec`/`InventoryItemNode`,
  `MenuShellConfig`/`MenuCubeGeometry`/`MenuOpenCloseStyle::OotPageFold`,
  `AmbitionInventoryUiPlugin` (minimal: resources + messages only).
- **The 3D cube RENDERING lives in the DEMO** `crates/ambition_mock_demo` (NOT the
  lib): `app.rs` = the App (a `Camera3d` pause cam + a `UiRoot3d` ring via
  `UiLunexPlugins` + systems); `app/render.rs` = page model → Lunex faces
  (panels/text/controls + depth bands); `app/systems.rs` = `setup` (spawns the ring
  + `spawn_all_faces`), `rebuild_lunex_faces`, `animate_menu_ring`; `app/input.rs` =
  kbd/mouse/touch nav; `app/state.rs` = the menu state machine. The rendering is
  already generic over the page model (`MenuPageModel<MockPage, MockAction>`).

## Next steps (renderer promotion + game integration)

1. **Promote the cube renderer into the lib** as a generic `CubeMenuPlugin<PageId,
   Action>`. Move the generic parts of the demo's `app/{render,systems,input,state}.rs`
   into the lib (the demo `include!`s these — convert to real `mod`s). The plugin
   should, on open: spawn the pause `Camera3d` + `UiRoot3d` ring, `rebuild_lunex_faces`
   from `ActiveMenuPages<PageId, Action>`, `animate_menu_ring`, and emit
   selection/page actions; reuse `MenuCubeGeometry` + `MenuShellConfig`. Keep
   demo-only bits (MockPage/MockAction/MockDemo, FPS + dummy overlays) in the demo.
2. **Wire the renderer into the game** behind the runtime toggle: an
   `InventoryUiBackend { Grid, Cube }` resource (startup/settings). When the
   inventory opens with `Cube`, add the cube camera/ring and feed
   `ActiveMenuPages<CubePage, CubeAction>` from `oot_cube::build_inventory_pages(...)`
   (rebuild on `OwnedItems` change); map `CubeAction` back via the existing
   `item_pickup`/`OwnedItems` seam (same as `oot_menu`). **Integration care:** the
   cube `Camera3d` must layer over the 2D game view (RenderLayers + camera order),
   active only while paused; pause/input must route to the cube while open and not
   leak to gameplay (reuse `InventoryUiState.visible`).
3. **Navigation** — reuse the demo's input (D-pad/stick rotates the ring; A/B
   select/back) driving `MenuShell`/`ActiveMenuPages` selection.
4. **Polish** — item icons (our sprites → `InventoryItemNode::icon`), the
   equipment/detail side panel (demo `add_items_page` layout), the OoT page-fold
   open/close (`MenuShellConfig.open_close_style = OotPageFold`).

## Gotchas

- Submodule is its own workspace → keep it `exclude`d in the root `Cargo.toml`.
- `bevy_lunex` grows the ~10-min build; the dep is gated by `oot_inventory` so
  headless/RL stay lean.
- The demo's `app.rs` `include!`s its sub-files — promote them as real modules.
