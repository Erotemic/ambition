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

## 2. Target architecture (engine crate + game content)

The three layers below split cleanly into a REUSABLE ENGINE (the renamed
`ambition_menu` crate) and GAME CONTENT (`ambition_sandbox::menu`). The seam is
**empirically validated**, not speculative: we will have TWO real presentations
(cube + bevy_ui) of ONE `MenuPageModel`, which is exactly the "two landed use
cases" bar for generalizing. Discipline: generalize ONLY what the two
presentations + our one game actually exercise — no custom-control-kind trait, no
pluggable-category system, no second-game speculative knobs.

```
ENGINE  →  crate `ambition_menu`  (rename of `ambition_inventory_ui`; ships MenuPlugin)
  PRIMITIVES   MenuPageModel<PageId, Action>, MenuNode, MenuControlKind,
               MenuVisualState, MenuFocusKey, scrollbar/text/dynamic-text.   (generic)
  RENDERERS    kaleidoscope (bevy_lunex cube)  +  bevy_ui (flat/tabbed).      ← two real consumers
               Swapped by InventoryUiBackend. Each renders ANY MenuPageModel.
  INTERACTION  cursor / focus / tab-switch / nav over a MenuPageModel; routes a
               selected control's Action back to the game's dispatch callback.  (generic)
  SETTINGS-IR FRAMEWORK  SettingsOption / SettingsOptionKind (Toggle/Cycle/Slider/
               Action) + the "render a settings tree" machinery + the SystemMenu
               TREE shape.  (the game-agnostic SHAPE of a settings menu)
  CONTENT SEAM the game provides: {PageId/Action types, page builders → MenuPageModel,
               a dispatch callback, a settings provider (its option list + apply fn)}.
               Optionally a few BASIC option helpers ship here (volume slider,
               display-mode cycle) a game MAY reuse.

CONTENT  →  `ambition_sandbox::menu`  (Ambition's concrete menu; plugs into the engine)
  • MenuPage { Inventory, System, Map, Quest } + MenuPageAction vocabulary.
  • page builders -> MenuPageModel<MenuPage, MenuPageAction> (items / system / map / quest).
  • dispatch_menu_action — the game's action handler (equip/use/apply-setting/quit/…).
  • Ambition's settings IR: SettingsOptionId + apply_settings_option + the concrete
    SystemMenuModel entries (Radio/Video/Audio/Controls/Gameplay/Language/Reset*/Quit).
  • item-grid content; reads UserSettings; the plugin wiring that hands all the
    above to the engine's MenuPlugin.
```

A new game would depend on `ambition_menu`, get both renderers + nav + the
settings framework for free, and supply its OWN `PageId`/`Action` + settings
options (its own IR) — reusing whatever basic option helpers we ship. Content
stays content; presentation is reusable. That is the north star.

Key realization that makes this safe NOW: the page model + the renderers are
ALREADY generic (the cube proves it); adding the bevy_ui renderer as a SECOND
consumer of the same model *validates* the engine/content seam rather than
guessing at it.

Naming/move note: `dispatch_kaleidoscope_action` → `dispatch_menu_action` (game
side, in `sandbox::menu`). The crate `ambition_inventory_ui` is RENAMED
`ambition_menu` and becomes the engine (primitives + BOTH renderers + nav +
settings framework + MenuPlugin). `MenuPage`/`MenuFocus`/`MenuPageAction` stay as
the game's types (already neutral from the earlier rename).

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
- **Map/Quest:** PAUSE-MENU tabs only (100% a pause feature — NOT shown in-game).
  Placeholder pages (reuse the cube's `placeholder_page` content) — present as
  tabs in both backends so the tab set matches; show a "coming soon" body. No
  gameplay wiring (deferred; they're not well-formed yet). A future in-world
  **mini-map / quest HUD** is a SEPARATE presentation/HUD concern (off by
  default), explicitly NOT part of this menu and not built here — don't conflate
  the pause Map/Quest tabs with any in-game overlay.
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

Plus the crate `ambition_inventory_ui` — which is ALSO mis-placed/mis-named: it
already holds a GENERIC menu model + the cube renderer (it is the menu *engine*,
not "inventory ui"). Four bevy_ui menu surfaces + a cube + a content model + a
buried IR + a mis-named engine crate.

### Proposed target — ENGINE crate `ambition_menu` + game content `sandbox::menu`
We have enough information to find the clean crate/plugin seam NOW (not "module
now, crate later"), because the bevy_ui renderer becomes a SECOND real consumer of
the same `MenuPageModel` — two presentations empirically validate the
engine/content boundary (§2). So:

**ENGINE — rename the crate `ambition_inventory_ui` → `ambition_menu`.** It owns
the game-agnostic engine and ships a `MenuPlugin`:
```
ambition_menu/ (crate)
  src/lib.rs        — MenuPlugin + the CONTENT-SEAM contract (the trait/config a game
                      implements: page builders, dispatch callback, settings provider).
  src/model.rs      — MenuPageModel<PageId, Action>, MenuNode, MenuControlKind,
                      MenuVisualState, MenuFocusKey, scrollbar/text/dynamic-text.   (already here)
  src/render/
    kaleidoscope.rs — bevy_lunex 3D-cube renderer.                                  (already here, was kaleidoscope.rs)
    bevy_ui.rs      — NEW flat/tabbed bevy_ui renderer (built in Phase C, IN the crate).
  src/nav.rs        — generic cursor / focus / tab-switch over a MenuPageModel;
                      routes a selected control's Action to the game's dispatch callback.
  src/settings_ir.rs— SETTINGS-IR FRAMEWORK: SettingsOption / SettingsOptionKind +
                      the SystemMenu TREE shape + the "render a settings tree"
                      machinery + a few BASIC option helpers a game MAY reuse.   (generic SHAPE only)
  src/backend.rs    — InventoryUiBackend (which renderer is active) + the A/B seam.
```

**CONTENT — `ambition_sandbox::menu`** (Ambition's concrete menu; plugs into the
engine via the content-seam contract):
```
crate::menu/         lib.rs gains ONE `mod menu;`; this is the ONLY menu code in the game crate
  mod.rs            — the plugin WIRING: install ambition_menu::MenuPlugin with Ambition's
                      content (page builders + dispatch + settings provider); the open/close
                      routing owner (Esc/Start/inventory); the `\` backend-toggle input.
  model.rs          — MenuPage{Inventory,System,Map,Quest}, MenuFocus, MenuPageAction +
                      the page builders (items/system/map/quest).                   [was menu_model.rs]
  dispatch.rs       — dispatch_menu_action — Ambition's action handler.             [from lunex app]
  ir.rs (or ir/)    — Ambition's CONCRETE settings: SettingsOptionId +
                      apply_settings_option + settings_menu_model + the concrete
                      SystemMenuModel entries (Radio/Video/.../Quit). Built ON the
                      engine's SettingsOption/Kind framework types.                  [was persistence/settings/{menu,system_menu}.rs]
  map.rs            — the Map tab content.                                           [folds in map_menu/]
```
(Game-specific overlay coordination — `opened_from_pause`, GameMode integration —
lives in `menu/mod.rs`; the *generic* cursor/focus state lives in the engine's
`nav`.)

Stays put (NOT moved):
- `crate::inventory/` — gameplay item DATA + effects (`OwnedItems`, item model,
  use-a-potion). Content/gameplay, not menu chrome. (Only `InventoryUiState`'s
  generic overlay bits inform the engine cursor; its game coordination is in
  `menu/mod.rs`.)
- `persistence/settings/{model.rs (UserSettings), audio.rs, video.rs, gameplay.rs,
  persistence.rs, platform_paths.rs}` — raw settings DATA + disk persistence.
  `menu::ir` reads/writes `UserSettings`; the IR is the menu's VIEW of it.

GENERIC-vs-CONCRETE split rule for the settings IR: the TYPES that describe the
shape of any settings menu (`SettingsOption`, `SettingsOptionKind`, the SystemMenu
tree node types) move to the engine (`ambition_menu::settings_ir`); Ambition's
CONCRETE option ids, value labels, `apply_settings_option`, and the concrete
SystemMenu entry roster stay in `sandbox::menu::ir`.

### Old → new map
| Old | New |
|---|---|
| crate `ambition_inventory_ui` | crate `ambition_menu` (rename; becomes the engine) |
| `ambition_inventory_ui/src/kaleidoscope.rs` | `ambition_menu/src/render/kaleidoscope.rs` |
| (NEW bevy_ui renderer) | `ambition_menu/src/render/bevy_ui.rs` |
| generic `SettingsOption`/`Kind` + SystemMenu tree TYPES | `ambition_menu/src/settings_ir.rs` |
| `InventoryUiBackend` (in lunex app) | `ambition_menu/src/backend.rs` |
| `menu_model.rs` | `sandbox::menu/model.rs` |
| `dispatch_kaleidoscope_action` | `sandbox::menu/dispatch.rs::dispatch_menu_action` |
| `persistence/settings/menu.rs` (concrete) | `sandbox::menu/ir.rs` (concrete; types → engine) |
| `persistence/settings/system_menu.rs` (concrete) | `sandbox::menu/ir.rs` (concrete; node types → engine) |
| `bevy_ui_grid_menu/` | DELETED → the engine's `bevy_ui.rs` renders the shared model |
| `pause_menu/` | DELETED → absorbed (System tab renders the IR) |
| `inventory.rs` (adventure menu) | DELETED → absorbed into the tabbed bevy_ui renderer |
| `lunex_kaleidoscope_app.rs` | DELETED → cube renderer is the engine; its game glue → `sandbox::menu/mod.rs` |
| `map_menu/` | `sandbox::menu/map.rs` |

Net: ~7 top-level game menu modules collapse to ONE `mod menu;`, and the renderers
+ framework live in the reusable `ambition_menu` crate.

### Judgment calls — ALL RESOLVED (2026-06-07, confirmed by Jon)
1. **IR home** → ✅ generic SHAPE types → `ambition_menu::settings_ir`; Ambition's
   concrete options/apply/entries → `sandbox::menu::ir`. `persistence/settings/`
   keeps raw `UserSettings` + disk.
2. **`map_menu/`** → ✅ **`sandbox::menu/map.rs`** (the Map tab).
3. **`InventoryUiState`** → ✅ generic cursor in the engine; game overlay
   coordination in `sandbox::menu/mod.rs`.
4. **Crate vs module — SUPERSEDED** → ✅ **find the crate/plugin seam NOW.** The
   engine is the renamed `ambition_menu` crate (primitives + BOTH renderers + nav +
   settings framework + MenuPlugin); `sandbox::menu` is content-only. Justified
   because two real presentations validate the seam (§2). Discipline: no
   speculative generalization beyond what the two renderers + one game exercise.

### Sequencing (fold into the §7 phases)
Behavior-neutral `git mv`/rename commits ride WITH the refactor, separate from
logic commits so regressions bisect cleanly:
- **Phase 0 (new):** rename crate `ambition_inventory_ui` → `ambition_menu`
  (Cargo.toml package name + the `dep:` in `ambition_sandbox` + all `use`/path refs;
  `git mv kaleidoscope.rs render/kaleidoscope.rs`). Pure rename, green build.
- **Phase A:** move the generic settings TYPES into `ambition_menu::settings_ir`;
  move Ambition's concrete settings into `sandbox::menu::ir`; add Quit to the IR.
- **Phase B:** create `sandbox::menu` (`mod.rs`+`model.rs`+`dispatch.rs`), move
  `menu_model.rs` → `menu/model.rs`, `dispatch_kaleidoscope_action` →
  `menu::dispatch`. Move `InventoryUiBackend` into the engine.
- **Phase C:** build the bevy_ui flat/tabbed renderer IN `ambition_menu::render::
  bevy_ui` (the second real consumer that validates the seam).
- **Phase D:** delete `pause_menu/`, `inventory.rs` adventure menu, `bevy_ui_grid_
  menu/`, and `lunex_kaleidoscope_app.rs` (its renderer is now the engine; its game
  glue moved to `sandbox::menu/mod.rs`); fold `map_menu/` → `menu/map.rs`.
- **Phase E:** input parity + the cube back/select-fire fix (§6).
