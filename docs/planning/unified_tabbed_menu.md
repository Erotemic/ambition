# Unified Tabbed Menu — design & execution plan

**Status:** APPROVED FOR AUTONOMOUS EXECUTION (planned 2026-06-07).
**Goal:** one menu *content model* (tabs + shared settings IR) rendered by two
interchangeable *presentations* — the bevy_ui "grid" backend and the 3D
"kaleidoscope" backend — A/B-swappable with `\`. This kills the
inventory-pauses-separately-from-settings split, makes the IR the single source
of truth, and cleanly separates content from presentation (part of the larger
content-vs-core refactor).

This doc is written to be executed in ONE autonomous pass with **no questions**.
Every open decision is resolved below; if a step is blocked, prefer the stated
fallback and record it, don't stop.

---

## 1. The value (why this is a big cleanup, not marginal)

- **One source of truth.** All menu content comes from backend-agnostic models:
  the page set (`MenuPage`: Inventory/System/Map/Quest), the page builders in
  `menu_model.rs` (`build_items_page`/`build_system_page`/`placeholder_page`), and
  the settings IR (`SystemMenuModel`/`SettingsMenuModel` +
  `apply_settings_option`). Adding/changing a setting or a page updates BOTH
  presentations automatically.
- **Content vs presentation separated.** The cube and the grid become two
  renderers of the same `MenuPageModel` feeding the same action dispatcher. The
  `\` toggle is a pure presentation swap.
- **No more split menus.** Today: a "Paused" list (Resume/Settings/Radio/
  Inventory/Reset/Quit) *and* a separate grid inventory *and* a separate cube.
  After: ONE menu with bumper-switchable tabs, presented as either flat bevy_ui
  tabs or cube faces.

---

## 2. Target architecture (three layers)

```
CONTENT (backend-agnostic, the source of truth)
  • MenuPage { Inventory, System, Map, Quest }              (menu_model.rs)
  • page builders -> MenuPageModel<MenuPage, MenuPageAction>(menu_model.rs)
  • SystemMenuModel / SettingsMenuModel / apply_settings_option (persistence/settings)
  • MenuPageAction vocabulary (Equip/Use/ChangePage/System*/OpenSystemEntry/…)

INTERACTION (shared)
  • the action DISPATCHER: dispatch_menu_action(MenuPageAction, …)  ← shared by both backends
  • a cursor over the active page's controls (focus / select / back / tab-switch)

PRESENTATION (per-backend, swapped by InventoryUiBackend)
  • Kaleidoscope: 3D cube faces                (lunex_kaleidoscope_app.rs + ambition_inventory_ui)
  • Grid (NEW):  flat bevy_ui tab bar + list   (replaces bevy_ui_grid_menu + the pause_menu top page/settings)
```

Key realization from the codebase: the **page model and the dispatcher are
already backend-agnostic** (they live in `menu_model.rs` / the settings IR and only
*happen* to be consumed by the cube today). The redesign makes the bevy_ui grid a
**second consumer** of the same models, and deletes the bespoke grid/pause-menu
content code.

Naming note: `dispatch_kaleidoscope_action` should be renamed
`dispatch_menu_action` and moved to a backend-neutral location (a new
`crate::menu` module, or `menu_model.rs` next to the page builders) so both
backends call it. Keep `MenuPage`/`MenuFocus`/`MenuPageAction` (already neutral
from the earlier rename).

---

## 3. The unified menu — behavior spec (resolved decisions)

- **Tabs (in order):** `Inventory, System, Map, Quest` — identical set + order in
  both backends (the cube already has these as `MenuPage::ALL`).
- **Tab switch:** Left/Right bumper (`MenuControlFrame.page_left`/`page_right`)
  cycles tabs with wraparound, in BOTH backends. (The cube currently turns pages
  via edge buttons + spatial nav; ALSO honor the bumpers explicitly so the two
  backends share the input contract — see §6.) Pointer: clicking a tab in the
  grid switches to it; the cube keeps its edge buttons.
- **Default tab on open:** `Inventory`. The menu REMEMBERS the last-viewed tab
  across opens (store on the shared cursor/overlay state). Entry source (Esc vs
  inventory key) does NOT change the tab — there is one menu.
- **Open/close:** Esc/Start or the inventory key opens the unified menu (Paused).
  - **Resume is REMOVED.** `Back` (Esc co-fire / B / Backspace) at a tab's top
    level closes the menu → `GameMode::Playing` (respecting `opened_from_pause`).
  - Inside a System drill, `Back` pops the drill one level; at the System tab's
    top level, `Back` closes the menu (same as any tab).
- **Map/Quest:** placeholder pages (reuse the cube's `placeholder_page` content)
  — present as tabs in both backends so the tab set matches; show a "coming soon"
  body. No gameplay wiring (deferred; they're not well-formed yet).
- **Inventory tab:** the 6×4 item grid (the existing `build_items_page` content)
  rendered in bevy_ui for the grid backend; equip/use via the shared dispatcher.
- **System tab:** renders `SystemMenuModel` (the IR) — the SAME entries the cube
  shows: Radio, Video, Audio, Controls, Gameplay, Language, Reset All Settings,
  Quit to Desktop, [Developer, Reset Sandbox — dev builds]. Drill in/out with
  select/back. This REPLACES the old pause-menu Settings/Radio sub-pages.

---

## 4. What moves / what is deleted

### Move INTO the System IR
- **Quit to Desktop.** Add `SystemMenuAction::Quit` + a `SystemMenuEntryId::Quit`
  (always present, immediate action, no drill screen — like Reset All Settings).
  Its dispatch writes `AppExit::Success`. Place it after Reset All Settings.
  Current home to remove: `pause_menu/model.rs` `PauseMenuItem::Quit` +
  `pause_menu/input.rs:294` handler.

### Remove entirely
- **Resume** (`PauseMenuItem::Resume`) — replaced by `Back`.
- The whole **pause-menu top page** (`PauseMenuItem`: Resume/Settings/Radio/
  Inventory/Reset Sandbox/Quit) — replaced by the tab bar. The menu opens
  directly to a tab, not to this list.
- The pause-menu's **own settings rendering** (`SettingsItem`-driven pages in
  `pause_menu/{model,ui,input,pointer}.rs`) — replaced by the System tab
  rendering the IR. Keep the `SettingsItem -> SettingsOptionId` parity mapping +
  parity tests ONLY if still needed as a bridge; otherwise delete `SettingsItem`
  once nothing renders it. (Confirm: after the grid System tab renders the IR,
  `SettingsItem` should have no remaining renderer → delete it and its tests, OR
  keep a thin shim if some non-menu code reads it. Grep first.)
- The old **grid inventory** (`bevy_ui_grid_menu/**`) bespoke content/state — replaced by
  the bevy_ui renderer of `build_items_page`. (The rendering *widgets* may be
  reused, but the content/nav must come from the shared model.)

### Already in the IR (verify, don't re-add)
- Reset Sandbox, Reset All Settings (dev-gating as today).
- KeyboardPreset, ResetControlFiltering (added in stage 3b, commit `3a6b414c` —
  the `model.rs` "TODO: not in IR" comment is STALE; verify and delete the
  comment).

---

## 5. IR completeness — the "did we miss anything" checklist (PRODUCE + VERIFY)

Before deleting the old pause-menu settings, **emit a recorded diff** (a markdown
table in this doc's sibling `unified_tabbed_menu_settings_diff.md`, generated
during execution) of EVERY old-grid settings affordance vs IR coverage, so
nothing is silently lost. Known deltas to resolve (from the architecture map):

| Old grid affordance | In IR? | Action |
|---|---|---|
| Video basics (DisplayMode, Camera*, Flashes, Colorblind, ShowFps) | ✅ | render from IR |
| Shaders (19 sliders) | ✅ (flat under Video) | render from IR |
| Audio (Master/Music/SFX/Mute) | ✅ | render from IR |
| Controls (ControllerProfile, deadzones, triggers, DpadMenuNav, InvertAimY, DashInputMode, TouchControls, MenuTapMode) | ✅ | render from IR |
| KeyboardPreset, ResetControlFiltering | ✅ (3b) | render from IR; delete stale TODO |
| Gameplay (Difficulty, Assist, PlayerDamage, DebugHud, QuestHud, TraceAutoDump) | ✅ | render from IR |
| **GameplayFlashes** ("Flashes (gameplay)" — 2nd surface on `video.flashes`) | ⚠️ IR exposes `Flashes` once | DECISION: drop the duplicate surface; the single `Flashes` option in Video is canonical. Record it. |
| **Developer page** (DebugOverlay, SlowMotion, Inspector, WorldInspector, OverviewCamera, DebugViewMode, DebugArtMode, ShowHitboxes, FillDebugBoxes, MicroGrid, CameraFrame, PlayerBodyProfile, MovementProfile, LdtkAutoApply) | ⚠️ cube renders a Developer screen via `DevToggleId` | VERIFY the IR Developer screen lists EVERY one of these dev toggles. If any is missing, ADD it to the IR's Developer screen so the grid (and cube) show it. This is the highest-risk port — enumerate both lists and diff explicitly. |
| Radio | ✅ | render from IR |
| Reset All Settings, Reset Sandbox, Quit | ✅ (Quit added in §4) | render from IR |
| Page-opener rows (OpenVideo/OpenShaders/…), Back | n/a | replaced by drill/tab nav |

**Rule:** if the diff finds an affordance with no IR home, EXTEND the IR (don't
silently drop) — except `GameplayFlashes`, which is an intentional de-dup
(recorded above).

---

## 6. Input model + the back/select-fire bug

- **Shared MenuControlFrame contract** both backends honor:
  - `page_left`/`page_right` (bumpers) → switch tab (wraparound).
  - `up/down/left/right` → move the focus cursor within the active tab.
  - `select` → dispatch the focused control's `MenuPageAction`.
  - `back` → pop System drill, else close the menu.
  - `start`/Esc → open (when closed) / close (when open).
  - `inventory` key → open to Inventory tab (when closed) / close (when open).
- **Cube: honor bumpers explicitly.** Today the cube turns pages only via
  spatial nav onto edge buttons. Add explicit `page_left`/`page_right` handling in
  `kaleidoscope_focus_nav` so bumpers turn the cube pages (parity with the grid).
- **BUG to fix — back/select don't fire in the cube from joystick/touch.** User:
  "I can move the joystick but it doesn't cause inputs to fire" (re: Back).
  Investigate: `kaleidoscope_focus_nav`/`kaleidoscope_menu_open_routing` read
  `menu.back`/`menu.select`, but the on-screen touch BUTTONS / joystick-sourced
  back+select may not populate `MenuControlFrame.back`/`.select` while the cube is
  open (the touch fold may only set directional nav, not the action buttons).
  Trace `fold_to_menu_control_frame` + `populate_menu_control_frame_from_actions`
  and ensure the touch action buttons (and any on-screen Back) set
  `menu.select`/`menu.back` while the cube overlay is visible. Add a headless test
  that a touch-sourced back closes the cube. (Chunk-3 fixed the joystick *knob*
  animation + directional nav; this is specifically the ACTION buttons.)

---

## 7. Execution phases (each builds + tests + commits independently)

**Phase A — IR completeness + recorded diff (no UI changes).**
1. Add `Quit to Desktop` to the System IR (`SystemMenuAction::Quit` +
   `SystemMenuEntryId::Quit`, always-present action; dispatch → `AppExit`).
2. Verify KeyboardPreset/ResetControlFiltering in IR; delete the stale TODO.
3. Enumerate the pause-menu Developer rows vs the IR Developer screen; add any
   missing dev toggle to the IR so they match. 
4. Write `docs/planning/unified_tabbed_menu_settings_diff.md` — the full old-vs-IR
   table with every affordance resolved (extended or intentionally dropped).
5. Tests: System IR surfaces Quit + every dev toggle; dispatch of Quit writes
   AppExit; existing parity tests still green.
   Gate: `--lib settings`, `--lib kaleidoscope`, `--lib`.

**Phase B — Backend-neutral interaction core.**
1. Rename `dispatch_kaleidoscope_action` → `dispatch_menu_action`; move it (and
   the small pure helpers it needs) to a backend-neutral module
   (`crate::menu::dispatch` or keep in `menu_model.rs`), so a bevy_ui renderer can
   call it. No behavior change; cube still uses it.
2. Extract a shared cursor/nav contract if clean (focus over a `MenuPageModel`'s
   controls; tab index; drill state). If the cube's spatial nav is too entangled
   to share cleanly, DON'T force it — the grid can have its own linear nav that
   calls the shared dispatcher. Record the choice.
   Gate: full `--lib` + the menu integration tests.

**Phase C — bevy_ui tabbed renderer (the NEW grid backend).**
1. New module (replace `bevy_ui_grid_menu`): a bevy_ui overlay with a top TAB BAR
   (Inventory/System/Map/Quest, active highlighted) + the active page's body.
2. Render the active `MenuPage`'s `MenuPageModel` as bevy_ui: Items → 6×4 grid;
   System → the IR rows (drill in/out) with a scrollbar for long lists; Map/Quest
   → placeholder body.
3. Nav: bumpers switch tabs; up/down/left/right move the focus cursor; select →
   `dispatch_menu_action`; back → drill-pop/close. Reuse the shared cursor where
   Phase B made it shareable.
4. Gate it under `backend == Grid` (the existing `InventoryUiBackend` toggle).
5. Tests: opening shows the Inventory tab; bumper switches to System; a System
   settings select dispatches `apply_settings_option`; Quit dispatches AppExit;
   back closes; the grid and cube produce the SAME `MenuPageModel` per tab
   (cross-backend parity test on the content model).

**Phase D — delete the old + wire the swap.**
1. Delete the pause-menu top page (`PauseMenuItem` list) and the `SettingsItem`
   settings rendering (after confirming nothing else renders them). Delete the old
   `bevy_ui_grid_menu` content/state (keep reusable widgets if any). Remove Resume.
2. Ensure Esc/Start/inventory-key routing opens the unified menu in BOTH backends
   (the grid analog of `kaleidoscope_menu_open_routing`); one open/close owner.
3. Update `architecture_boundaries` + any tests referencing deleted types.
   Gate: full `--lib` + `--test movement_axis --test replay_fixture_regression
   --test scripted_gameplay --test architecture_boundaries`.

**Phase E — input parity + the back/select-fire fix (§6).**
1. Cube honors `page_left`/`page_right` bumpers explicitly.
2. Fix touch/joystick back+select not firing in the cube; headless test.
3. Final cross-backend parity sweep: same tabs, same System entries, same
   dispatch, `\` swaps presentation only.

---

## 8. Parity tests (the safety net for the whole refactor)

- **Content parity:** for each `MenuPage`, the `MenuPageModel` the grid renders ==
  the one the cube renders (same controls/actions) for a fixed state.
- **System parity:** the grid System tab and the cube System face list the same
  entries and, on a settings select, both call `apply_settings_option(id, dir)`
  with the same id — i.e. one source of truth.
- **Dispatch parity:** equip/use/quit/reset/change-tab produce identical effects
  regardless of backend.
- **No-drift exhaustiveness:** every `SettingsOptionId`/`SystemMenuEntryId` is
  surfaced by both backends (or explicitly excluded with a reason).

Keep `replay_fixture_regression` + `scripted_gameplay` green throughout (never
regenerate fixtures).

---

## 9. Deferred (explicitly NOT in this pass)
- Shaders applied to the menu camera (user: "some shaders should apply to the
  menu, but maybe not all — defer").
- Map/Quest real content (stubs only).
- A formal `MenuBackend` trait (the enum + shared models is the chosen seam).

---

## 10. Directory reorganization — consolidate ALL menu code under `crate::menu`

### The problem (agent-navigability)
`ambition_sandbox/src/lib.rs` is a flat list of ~60 top-level modules, and the
menu code is scattered across SEVEN of them with no shared home:
- `menu_model.rs` — backend-agnostic content/page model (just renamed).
- `lunex_kaleidoscope_app.rs` — the 3D cube backend host (one ~3.7k-line file).
- `bevy_ui_grid_menu/` — the "Grid" bevy_ui inventory (just renamed from oot_menu).
- `pause_menu/` — the Paused list + the `SettingsItem` settings UI.
- `inventory.rs` — a THIRD bevy_ui menu (the legacy Items/Map/Quests "adventure
  menu", feature-swapped with the grid).
- `map_menu/` — a standalone Map view.
- `persistence/settings/{menu.rs, system_menu.rs}` — the backend-agnostic IR,
  buried inside the persistence layer.

Plus the external renderer crate `ambition_inventory_ui` (the bevy_lunex cube
renderer). Four bevy_ui menu surfaces + a cube + a content model + a buried IR.

### Proposed target — one `crate::menu/` subtree (proto-boundary, future crate)
Following the plugin-refactor pattern (carve a stable same-crate boundary FIRST,
extract a real `ambition_menu` crate LATER), consolidate everything menu under one
module so an agent finds all of it in one place and the layers are explicit:

```
crate::menu/                       MenuPlugin owns the whole menu; lib.rs gains ONE `mod menu;`
  mod.rs            — MenuPlugin, InventoryUiBackend (the A/B seam), kaleidoscope_backend_active
                      run-condition, the single open/close routing owner (Esc/Start/inventory).
  model.rs          — MenuPage, MenuFocus, MenuPageAction, the page builders.        [was menu_model.rs]
  dispatch.rs       — dispatch_menu_action — the shared action handler.              [from lunex_kaleidoscope_app.rs]
  cursor.rs         — shared menu/overlay state: visibility, active tab, focus,
                      drill stack, opened_from_pause.                                 [absorbs InventoryUiState]
  input.rs          — menu-side MenuControlFrame population/consumption glue.
  ir/               — the backend-agnostic settings + menu-tree IR (single source of truth)
    mod.rs
    settings.rs     — SettingsMenuModel / SettingsOption / SettingsOptionId /
                      apply_settings_option / settings_menu_model.                    [was persistence/settings/menu.rs]
    system.rs       — SystemMenuModel / SystemMenuEntryId / SystemMenuAction.         [was persistence/settings/system_menu.rs]
  backends/
    grid/           — the unified bevy_ui flat/tabbed presentation.                   [replaces bevy_ui_grid_menu/ +
      mod.rs                                                                            pause_menu/ + inventory.rs adventure menu]
      ui.rs         — tab bar + active-page body.
      input.rs      — grid nav + bumper tab-switch.
      state.rs      — grid-only view state (scroll/selection).
    kaleidoscope/   — the 3D cube presentation host, SPLIT out of the 3.7k-line file. [was lunex_kaleidoscope_app.rs]
      mod.rs        — KaleidoscopeBackendPlugin, the cube open/close gate + wiring.
      nav.rs        — kaleidoscope_focus_nav / system_focus_nav.
      pointer.rs    — pointer press/move/release observers.
      render_sync.rs— fade / scrim / focus-visuals / scrollbar / republish.
      cursor.rs     — KaleidoscopeCursor (cube-spatial cursor) if not fully shared.
  map.rs            — the Map tab/view content.                                       [folds in map_menu/]
```

Stays put (NOT moved):
- `ambition_inventory_ui` crate — the external bevy_lunex CUBE RENDERER; the
  kaleidoscope backend host uses it. (It's already a clean crate boundary.)
- `crate::inventory/` — the gameplay item DATA + effects (`OwnedItems`, item
  model, use-a-potion effects). That's content/gameplay, not menu chrome. Only
  `InventoryUiState` (overlay state) moves out, into `menu::cursor`.
- `persistence/settings/{model.rs (UserSettings), audio.rs, video.rs,
  gameplay.rs, persistence.rs, platform_paths.rs}` — the raw settings DATA + disk
  persistence. `menu::ir::settings` depends on `UserSettings` (reads/writes it);
  the IR is the menu's VIEW of that data and belongs with the menu.

### Old → new path map (for the move commit)
| Old | New |
|---|---|
| `menu_model.rs` | `menu/model.rs` |
| `lunex_kaleidoscope_app.rs` | `menu/backends/kaleidoscope/` (split into files) |
| `bevy_ui_grid_menu/` | `menu/backends/grid/` (rebuilt into the tabbed view) |
| `pause_menu/` | DELETED → absorbed into `menu/backends/grid/` + `menu/ir/` |
| `inventory.rs` (adventure menu) | DELETED → absorbed into `menu/backends/grid/` |
| `map_menu/` | `menu/map.rs` (Map tab) |
| `persistence/settings/menu.rs` | `menu/ir/settings.rs` |
| `persistence/settings/system_menu.rs` | `menu/ir/system.rs` |
| `InventoryUiState` (in `inventory.rs`) | `menu::cursor` |
| `dispatch_kaleidoscope_action` (in lunex app) | `menu::dispatch::dispatch_menu_action` |

Net effect on `lib.rs`: ~7 top-level menu modules collapse to ONE `mod menu;`.

### Judgment calls — ALL RESOLVED (2026-06-07, confirmed by Jon)
1. **IR home** → ✅ **`menu/ir/`** (menu owns its content model). `persistence/
   settings/` keeps the raw `UserSettings` + disk layer; `menu/ir/settings.rs`
   depends on it.
2. **`map_menu/`** → ✅ **`menu/map.rs`** (it's a menu surface → the Map tab).
3. **`InventoryUiState`** → ✅ **`menu::cursor`** (menu overlay state, read only by
   menu code).
4. **Crate vs module now** → ✅ **`crate::menu` MODULE now** (proto-boundary);
   extract an `ambition_menu` crate LATER once decoupled (plugin-refactor cadence).

### Sequencing (fold into the §7 phases)
The big moves ride WITH the refactor, not as a separate churn pass:
- **Phase A** also moves the IR: `persistence/settings/{menu,system_menu}.rs` →
  `menu/ir/{settings,system}.rs` (pure `git mv` + path updates; behavior-neutral).
- **Phase B** creates `menu/mod.rs` + `menu/dispatch.rs`, moves `menu_model.rs` →
  `menu/model.rs`, `InventoryUiState` → `menu/cursor.rs`.
- **Phase C** builds `menu/backends/grid/` (the new tabbed view).
- **Phase D** deletes `pause_menu/` + `inventory.rs` adventure menu + old grid
  content; splits `lunex_kaleidoscope_app.rs` into `menu/backends/kaleidoscope/`;
  folds `map_menu/` → `menu/map.rs`.
- Keep each move a behavior-neutral `git mv` + path-update commit, separate from
  the logic commits, so a regression bisects cleanly.
