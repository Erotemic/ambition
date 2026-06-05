//! The data seam between Ambition's live 24-item inventory and the reusable
//! `ambition_inventory_ui` 3D-cube OoT pause menu (#31).
//!
//! The game owns the item state (`crate::items`); this module builds the cube's
//! page MODELS from it via the lib's host-data seam (`ItemsOnlyPageSpec`, which is
//! deliberately renderer-agnostic — it can feed the Lunex cube, a Bevy-UI grid
//! fallback, or a test renderer). The cube RENDERER itself is the shared lib.
//!
//! This gives the "wire us up to use it" part: our `Item::ALL` (already 24 in OoT
//! grid order) → the cube's items page, with owned/equipped/selected reflected and
//! a host-defined [`CubeAction`] emitted back to the game.
//!
//! ## Items-page layout (matches `ambition_mock_demo`)
//!
//! The proven demo (`crates/ambition_mock_demo/src/app/models.rs`) does NOT render
//! each item's full description inside its grid cell (that overlapping mush is the
//! bug this file fixes). Instead it shows short item NAMES in a compact grid and
//! renders the *focused* item's wrapped description once, in a dedicated detail
//! panel beside the grid. We replicate that structure here:
//!
//! * the grid sits in the left/centre (panel rect [`GRID_RECT`]), each cell shows a
//!   short, wrapped item name and a one-word action hint (Equip/Use/...),
//! * the [`DETAIL_PANEL_RECT`] on the right shows the [`CubeFocus`]ed item's name +
//!   wrapped description (filled by `oot_cube_app::cube_sync_detail_panel`),
//! * the L/R page-turn buttons live in the *side margins* ([`EDGE_LEFT_RECT`] /
//!   [`EDGE_RIGHT_RECT`]) OUTSIDE the grid, exactly like the demo.

use ambition_inventory_ui::{
    InventoryItemNode, InventorySlotId, ItemsOnlyPageSpec, MenuColor, MenuControlKind,
    MenuPageModel, MenuRect, MenuTextAlign,
};

use crate::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use crate::persistence::settings::{AudioSettings, UserSettings};

/// Edge page-turn buttons flank the page in the side margins (NOT over the grid),
/// matching the demo's `add_edge_buttons` rects (`crates/ambition_mock_demo/src/
/// app/models.rs`). The game draws these as REAL controls and turns the lib's
/// decorative arrows off (`draw_nav_arrows = false`) so they aren't double-drawn.
const EDGE_LEFT_RECT: MenuRect = MenuRect { x: 1.8, y: 43.5, w: 7.5, h: 13.0 };
const EDGE_RIGHT_RECT: MenuRect = MenuRect { x: 90.7, y: 43.5, w: 7.5, h: 13.0 };

/// The item grid lives in the left/centre, clear of the side arrows. Matches the
/// demo's items-grid panel so the lib's auto-laid cells never reach the margins.
const GRID_RECT: MenuRect = MenuRect { x: 11.0, y: 19.0, w: 58.0, h: 55.0 };

/// The right-hand detail panel: shows the focused item's name + wrapped
/// description (the demo's "SELECTED" panel). One panel, not one-per-cell.
const DETAIL_PANEL_RECT: MenuRect = MenuRect { x: 70.5, y: 19.0, w: 18.2, h: 55.0 };

/// Description wrap width for the detail panel, matching the demo's
/// `DETAIL_WRAP_COLS`.
const DETAIL_WRAP_COLS: usize = 18;
/// Max description lines shown in the detail panel (demo's `DETAIL_VISIBLE_LINES`).
const DETAIL_VISIBLE_LINES: usize = 7;
/// Wrap width for an item NAME inside its grid cell, so long names like
/// "Puppy-Slug Gun" don't bleed across neighbouring cells (Text3d does not clip).
const LABEL_WRAP_COLS: usize = 10;
/// Max lines of a wrapped item name shown in a cell.
const LABEL_MAX_LINES: usize = 2;

impl CubePage {
    /// The neighbouring page when turning the ring left/right (wraps), matching
    /// [`CubePage::ALL`] order.
    pub fn neighbor(self, dir: isize) -> CubePage {
        let all = CubePage::ALL;
        let cur = all.iter().position(|p| *p == self).unwrap_or(0);
        let next = (cur as isize + dir).rem_euclid(all.len() as isize) as usize;
        all[next]
    }

    /// The page that physically sits to the viewer's LEFT — i.e. the one that
    /// rotates to the front when the cube turns LEFT. This is the same
    /// inside-the-cube convention as the demo's `page_on_viewer_left`
    /// (`from_index(index + 1)`): pressing the LEFT affordance rotates the ring
    /// left, which brings the RIGHT-neighbour (`+1` in ring order) to the front.
    pub fn on_viewer_left(self) -> CubePage {
        self.neighbor(1)
    }

    /// The page to the viewer's RIGHT (rotates to front when turning RIGHT).
    pub fn on_viewer_right(self) -> CubePage {
        self.neighbor(-1)
    }
}

/// Append the Left/Right edge page-turn buttons to a page model. Emitted as real
/// `Action` controls so Bevy picking + directional focus can dispatch them. The
/// LEFT button turns the cube left → brings the viewer-left page to front
/// ([`CubePage::on_viewer_left`]); mirrors the demo's `add_edge_buttons`.
fn add_edge_buttons(model: &mut MenuPageModel<CubePage, CubeAction>, page: CubePage, focus: CubeFocus) {
    model.control(
        EDGE_LEFT_RECT,
        MenuControlKind::Action,
        format!("<\n{}", page_label(page.on_viewer_left())),
        Some("turn cube left".to_string()),
        focus == CubeFocus::EdgeLeft,
        false,
        Some(CubeAction::ChangePage(page.on_viewer_left())),
    );
    model.control(
        EDGE_RIGHT_RECT,
        MenuControlKind::Action,
        format!(">\n{}", page_label(page.on_viewer_right())),
        Some("turn cube right".to_string()),
        focus == CubeFocus::EdgeRight,
        false,
        Some(CubeAction::ChangePage(page.on_viewer_right())),
    );
}

fn page_label(page: CubePage) -> &'static str {
    match page {
        CubePage::Items => "Items",
        CubePage::Map => "Map",
        CubePage::Quest => "Quest",
        CubePage::System => "System",
    }
}

/// The cube faces (pages). `Items` is wired live from our inventory; the rest
/// mirror OoT's subscreen tabs as host-data-driven placeholders for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CubePage {
    Items,
    Map,
    Quest,
    System,
}

impl CubePage {
    pub const ALL: [CubePage; 4] = [
        CubePage::Items,
        CubePage::Map,
        CubePage::Quest,
        CubePage::System,
    ];
}

/// The cursor's logical position on the items page: either an item slot or one of
/// the flanking edge (page-turn) buttons. This is the game-side equivalent of the
/// demo's `MockAction`-as-selection, and the unit of [`crate::oot_cube_app`]'s
/// directional navigation (`move_spatial`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CubeFocus {
    EdgeLeft,
    EdgeRight,
    Item(usize),
    /// A System-face option row (index into [`SystemOption::ALL`]).
    System(usize),
}

impl Default for CubeFocus {
    fn default() -> Self {
        CubeFocus::Item(0)
    }
}

impl CubeFocus {
    /// The item slot index this focus refers to, clamped into range. Edge buttons
    /// report slot 0 so callers always have a valid item to describe.
    pub fn item_index(self) -> usize {
        match self {
            CubeFocus::Item(idx) => idx.min(crate::items::ITEM_COUNT - 1),
            _ => 0,
        }
    }
}

/// Actions the cube emits back to the game (the host consumes these — the cube
/// never mutates item state itself, matching the existing `oot_menu` seam).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeAction {
    Equip(Item),
    Use(Item),
    ChangePage(CubePage),
    /// A toggle/cycle/close on the System face. The host applies it by mutating
    /// `UserSettings` (see `oot_cube_app::apply_system_option`), which the
    /// existing `save_settings_on_change` system then persists — no parallel
    /// persistence path. `SystemOption` is the unit selected by [`CubeFocus::System`].
    System(SystemOption),
}

/// The selectable options on the System cube face. Each maps to a real
/// `UserSettings` field (or the menu itself, for `CloseMenu`). Kept as a flat
/// `Copy` enum so it rides inside [`CubeAction`] and the [`CubeFocus::System`]
/// cursor without allocation. Mutation + persistence is the pause menu's single
/// source of truth (`UserSettings` change detection → `save_settings_on_change`);
/// see `oot_cube_app::apply_system_option`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemOption {
    ToggleFps,
    ToggleDebugHud,
    ToggleQuestHud,
    ToggleTouchControls,
    ToggleMute,
    CycleMasterVolume,
    CycleMusicVolume,
    CycleSfxVolume,
    CycleCameraZoom,
    CloseMenu,
}

impl SystemOption {
    /// Every System option, in display order (top-to-bottom on the face). The
    /// length drives the System grid's row count and the cursor's clamp.
    pub const ALL: [SystemOption; 10] = [
        SystemOption::ToggleFps,
        SystemOption::ToggleDebugHud,
        SystemOption::ToggleQuestHud,
        SystemOption::ToggleTouchControls,
        SystemOption::ToggleMute,
        SystemOption::CycleMasterVolume,
        SystemOption::CycleMusicVolume,
        SystemOption::CycleSfxVolume,
        SystemOption::CycleCameraZoom,
        SystemOption::CloseMenu,
    ];

    /// The control LABEL for this option, reflecting the CURRENT settings state
    /// (e.g. "Show FPS: on", "Camera Zoom: combat 800x450"). Volume rows append
    /// `< >` to signal they cycle, matching the pause menu's slider affordance.
    pub fn label(self, settings: &UserSettings) -> String {
        match self {
            SystemOption::ToggleFps => {
                format!("Show FPS: {}", on_off(settings.video.show_fps))
            }
            SystemOption::ToggleDebugHud => {
                format!("Debug HUD: {}", on_off(settings.gameplay.debug_hud_visible))
            }
            SystemOption::ToggleQuestHud => {
                format!("Quest HUD: {}", on_off(settings.gameplay.quest_hud_visible))
            }
            SystemOption::ToggleTouchControls => format!(
                "Touch Controls: {}",
                on_off(settings.controls.touch_controls_visible)
            ),
            SystemOption::ToggleMute => format!(
                "Mute: {}",
                if settings.audio.muted { "muted" } else { "off" }
            ),
            SystemOption::CycleMasterVolume => format!(
                "Master Volume: {}%  < >",
                AudioSettings::percent(settings.audio.master_volume)
            ),
            SystemOption::CycleMusicVolume => format!(
                "Music Volume: {}%  < >",
                AudioSettings::percent(settings.audio.music_volume)
            ),
            SystemOption::CycleSfxVolume => format!(
                "SFX Volume: {}%  < >",
                AudioSettings::percent(settings.audio.sfx_volume)
            ),
            SystemOption::CycleCameraZoom => {
                format!("Camera Zoom: {}", settings.video.camera_zoom.label())
            }
            SystemOption::CloseMenu => "Close Menu".to_string(),
        }
    }

    /// One-line detail-panel description of what this option does. Shown in the
    /// System face's right-hand panel for the focused option.
    pub fn description(self) -> &'static str {
        match self {
            SystemOption::ToggleFps => "Toggle the on-screen frames-per-second counter.",
            SystemOption::ToggleDebugHud => "Toggle the debug HUD overlay (state, timers).",
            SystemOption::ToggleQuestHud => "Toggle the quest objective HUD panel.",
            SystemOption::ToggleTouchControls => {
                "Show or hide the on-screen touch control pads."
            }
            SystemOption::ToggleMute => "Mute or unmute all game audio.",
            SystemOption::CycleMasterVolume => "Step the master output volume up/down.",
            SystemOption::CycleMusicVolume => "Step the music volume up/down.",
            SystemOption::CycleSfxVolume => "Step the sound-effects volume up/down.",
            SystemOption::CycleCameraZoom => "Cycle the gameplay camera zoom preset.",
            SystemOption::CloseMenu => "Close this menu and return to the game.",
        }
    }
}

/// Shared "on"/"off" word for boolean rows, matching the pause menu's wording.
fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

/// A short, cell-sized verb hint for an item, mirroring the demo's
/// `item_slot_detail` (e.g. "equip" / "use" / "key item"). Deliberately NOT the
/// full description — the description lives in the detail panel.
fn cell_hint(owned: &OwnedItems, equipped: Option<Item>, item: Item) -> &'static str {
    if !owned.has(item) {
        "--"
    } else if equipped == Some(item) {
        "equipped"
    } else if item.held_item_id().is_some() {
        "equip"
    } else {
        "use"
    }
}

/// Wrap + truncate an item name so it fits a grid cell. Long names like
/// "Puppy-Slug Gun" otherwise bleed across neighbouring cells.
fn cell_label(name: &str) -> String {
    let mut lines = wrap_text(name, LABEL_WRAP_COLS);
    lines.truncate(LABEL_MAX_LINES);
    lines.join("\n")
}

/// The items-face spec built from our live inventory. Kept separate from
/// [`build_items_page`] so it's unit-testable without the renderer.
///
/// Each cell carries a SHORT wrapped name + one-word action hint (matching the
/// demo's grid). The full description is NOT placed per-cell; it goes in the
/// detail panel (see [`build_items_page`]).
pub fn items_spec(
    owned: &OwnedItems,
    equipped: Option<Item>,
    focus: CubeFocus,
) -> ItemsOnlyPageSpec<CubePage, CubeAction> {
    let selected = match focus {
        CubeFocus::Item(idx) => Item::from_index(idx),
        _ => None,
    };
    let mut spec = ItemsOnlyPageSpec::new(CubePage::Items, "ITEMS")
        .with_grid(ITEM_GRID_ROWS, ITEM_GRID_COLS)
        .with_grid_rect(GRID_RECT);
    spec.selected_slot = selected.map(|i| InventorySlotId(i.index()));
    spec.cells = Item::ALL
        .into_iter()
        .map(|item| {
            let owns = owned.has(item);
            let mut node = if owns {
                InventoryItemNode::new(item.index(), cell_label(item.display_name()))
            } else {
                InventoryItemNode::unowned(item.index(), cell_label(item.display_name()))
            };
            // Short per-cell hint only (NOT the full description — that mush was the
            // bug). The description is rendered once, in the detail panel.
            node = node
                .detail(cell_hint(owned, equipped, item))
                .equipped(equipped == Some(item));
            if owns {
                // Held-item weapons/abilities equip; everything else "uses".
                let (action, label) = if item.held_item_id().is_some() {
                    (CubeAction::Equip(item), "Equip")
                } else {
                    (CubeAction::Use(item), "Use")
                };
                node = node.action(action).action_label(label);
            }
            node
        })
        .collect();
    spec
}

/// The Items cube face, built from our live inventory. Lays out the grid, the
/// right-hand detail panel for the [`CubeFocus`]ed item, and the flanking
/// page-turn buttons — matching the demo's `add_items_page` structure.
pub fn build_items_page(
    owned: &OwnedItems,
    equipped: Option<Item>,
    focus: CubeFocus,
) -> MenuPageModel<CubePage, CubeAction> {
    let mut model = items_spec(owned, equipped, focus).into_page_model();
    add_detail_panel(&mut model, owned, equipped, focus);
    add_edge_buttons(&mut model, CubePage::Items, focus);
    model
}

/// Render the single right-hand detail panel for the focused item: its name and
/// wrapped description. This is the demo's "SELECTED" panel — exactly ONE detail
/// region for the whole page, not one per cell.
fn add_detail_panel(
    model: &mut MenuPageModel<CubePage, CubeAction>,
    owned: &OwnedItems,
    equipped: Option<Item>,
    focus: CubeFocus,
) {
    model.panel(
        DETAIL_PANEL_RECT,
        MenuColor::rgba(0.035, 0.046, 0.105, 0.96),
        None,
    );
    model.text(
        79.6,
        23.0,
        2.6,
        "SELECTED",
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );
    let item = Item::from_index(focus.item_index()).unwrap_or(Item::ALL[0]);
    for (line_idx, line) in detail_lines(owned, equipped, item).into_iter().enumerate() {
        model.text(
            79.6,
            28.5 + line_idx as f32 * 3.2,
            1.55,
            line,
            MenuTextAlign::Center,
            MenuColor::rgba(0.88, 0.94, 1.0, 0.96),
        );
    }
}

/// The wrapped detail-panel lines for an item: its name, a blank, the wrapped
/// description, and a one-line status. Mirrors the demo's `detail_lines`.
fn detail_lines(owned: &OwnedItems, equipped: Option<Item>, item: Item) -> Vec<String> {
    let mut lines = Vec::new();
    lines.extend(wrap_text(item.display_name(), DETAIL_WRAP_COLS));
    lines.push(String::new());
    lines.extend(wrap_text(item.description(), DETAIL_WRAP_COLS));
    // Cap the description so it stays inside the fixed panel (Text3d does not clip
    // to its parent rect — the demo wraps + caps for the same reason).
    lines.truncate(DETAIL_VISIBLE_LINES);
    lines.push(String::new());
    let status = if !owned.has(item) {
        "not owned".to_string()
    } else if equipped == Some(item) {
        "equipped".to_string()
    } else if item.held_item_id().is_some() {
        "Activate to equip".to_string()
    } else {
        "Activate to use".to_string()
    };
    lines.extend(wrap_text(&status, DETAIL_WRAP_COLS));
    lines
}

/// Build every cube face from our inventory (Items live, the rest placeholders
/// until their host data is wired). The renderer consumes these via
/// `ActiveMenuPages<CubePage, CubeAction>`. `focus` highlights the items page's
/// cursor and selects the detail-panel item.
pub fn build_inventory_pages(
    owned: &OwnedItems,
    equipped: Option<Item>,
    focus: CubeFocus,
    settings: &UserSettings,
) -> Vec<MenuPageModel<CubePage, CubeAction>> {
    vec![
        build_items_page(owned, equipped, focus),
        placeholder_page(
            CubePage::Map,
            "MAP",
            "Area map: discovered rooms, anchors, portal markers (host data TODO).",
        ),
        placeholder_page(
            CubePage::Quest,
            "QUEST",
            "Quest status + key items from save data (host data TODO).",
        ),
        build_system_page(settings, focus),
    ]
}

/// Index of the focused System option, clamped into range. Non-System focuses
/// (e.g. a stale items cursor) default to the first option so callers always
/// have a valid row to describe in the detail panel.
fn system_focus_index(focus: CubeFocus) -> usize {
    match focus {
        CubeFocus::System(idx) => idx.min(SystemOption::ALL.len() - 1),
        _ => 0,
    }
}

/// The System cube face: one control per real option in [`SystemOption::ALL`],
/// each labelled with its CURRENT settings state, plus a right-hand detail panel
/// describing the focused option and the generic L/R edge buttons (so rotation
/// still works). Mirrors the items page's grid + detail-panel + edge-button
/// structure (`add_detail_panel` / `add_edge_buttons`) for visual consistency.
pub fn build_system_page(
    settings: &UserSettings,
    focus: CubeFocus,
) -> MenuPageModel<CubePage, CubeAction> {
    let mut model =
        MenuPageModel::new(CubePage::System, "SYSTEM", MenuColor::rgba(0.03, 0.04, 0.10, 0.96));
    model.text(
        40.0,
        13.0,
        3.4,
        "SYSTEM",
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );

    let focused = system_focus_index(focus);
    let options = SystemOption::ALL;
    // Lay the option rows out as a single vertical column inside the same
    // left/centre band the items grid uses (clear of the side arrows). Evenly
    // spaced between the title and the bottom margin.
    let count = options.len() as f32;
    let top = 21.0;
    let bottom = 74.0;
    let step = (bottom - top) / count;
    let row_h = (step * 0.78).min(5.0);
    for (idx, option) in options.into_iter().enumerate() {
        let y = top + idx as f32 * step;
        model.control(
            MenuRect {
                x: GRID_RECT.x,
                y,
                w: GRID_RECT.w,
                h: row_h,
            },
            MenuControlKind::OptionToggle,
            option.label(settings),
            Some(option.description().to_string()),
            idx == focused,
            false,
            Some(CubeAction::System(option)),
        );
    }

    add_system_detail_panel(&mut model, settings, focused);
    add_edge_buttons(&mut model, CubePage::System, focus);
    model
}

/// The System face's right-hand detail panel: the focused option's label (its
/// current state) plus a wrapped description. Reuses the items panel's rect +
/// styling so the two faces read identically.
fn add_system_detail_panel(
    model: &mut MenuPageModel<CubePage, CubeAction>,
    settings: &UserSettings,
    focused: usize,
) {
    model.panel(
        DETAIL_PANEL_RECT,
        MenuColor::rgba(0.035, 0.046, 0.105, 0.96),
        None,
    );
    model.text(
        79.6,
        23.0,
        2.6,
        "OPTION",
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );
    let option = SystemOption::ALL[focused.min(SystemOption::ALL.len() - 1)];
    let mut lines = wrap_text(&option.label(settings), DETAIL_WRAP_COLS);
    lines.push(String::new());
    lines.extend(wrap_text(option.description(), DETAIL_WRAP_COLS));
    lines.truncate(DETAIL_VISIBLE_LINES + 2);
    for (line_idx, line) in lines.into_iter().enumerate() {
        model.text(
            79.6,
            28.5 + line_idx as f32 * 3.2,
            1.55,
            line,
            MenuTextAlign::Center,
            MenuColor::rgba(0.88, 0.94, 1.0, 0.96),
        );
    }
}

fn placeholder_page(page: CubePage, title: &str, body: &str) -> MenuPageModel<CubePage, CubeAction> {
    let mut model = MenuPageModel::new(page, title, MenuColor::rgba(0.03, 0.04, 0.10, 0.96));
    model.text(
        50.0,
        38.0,
        4.0,
        title,
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );
    model.text(
        50.0,
        54.0,
        1.7,
        body,
        MenuTextAlign::Center,
        MenuColor::rgba(0.85, 0.92, 1.0, 0.9),
    );
    // Placeholder pages keep the edge buttons too (no item focus → CubeFocus::Item(0)
    // is harmless; only the items page reads it for highlighting).
    add_edge_buttons(&mut model, page, CubeFocus::Item(0));
    model
}

/// Greedy word-wrap to `width` columns, hyphenating words longer than the column.
/// Ported from the demo's `wrap_text` so wrapping matches exactly.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(4);
    let mut out = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        for raw_word in paragraph.split_whitespace() {
            for word in split_long_word(raw_word, width) {
                let needs_space = !line.is_empty();
                let next_len = line.chars().count() + word.chars().count() + usize::from(needs_space);
                if next_len > width && !line.is_empty() {
                    out.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(&word);
            }
        }
        out.push(line);
    }
    out
}

fn split_long_word(word: &str, width: usize) -> Vec<String> {
    if word.chars().count() <= width {
        return vec![word.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let chunk_width = width.saturating_sub(1).max(3);
    for ch in word.chars() {
        if current.chars().count() >= chunk_width {
            current.push('-');
            out.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_face_wires_all_24_slots_from_our_catalog() {
        let owned = OwnedItems::default();
        let spec = items_spec(&owned, None, CubeFocus::default());
        assert_eq!(
            spec.cells.len(),
            crate::items::ITEM_COUNT,
            "the cube's items face has one cell per inventory slot (24)"
        );
        // Slots are in grid order; labels are wrapped from our catalog.
        for (idx, cell) in spec.cells.iter().enumerate() {
            assert_eq!(cell.slot.0, idx);
            assert_eq!(cell.label, cell_label(Item::ALL[idx].display_name()));
        }
    }

    #[test]
    fn owned_and_equipped_flags_reflect_inventory_state() {
        let mut owned = OwnedItems::default();
        owned.grant(Item::Blink, 1);
        let spec = items_spec(&owned, Some(Item::Blink), CubeFocus::default());
        let blink = &spec.cells[Item::Blink.index()];
        assert!(blink.owned, "granted item reads owned");
        assert!(blink.equipped, "equipped item reads equipped");
        assert!(blink.action.is_some(), "owned item has an action");
        // An un-granted item is unowned + actionless.
        let unowned = spec.cells.iter().find(|c| !c.owned).expect("some unowned");
        assert!(unowned.action.is_none());
    }

    #[test]
    fn cell_labels_wrap_and_stay_short() {
        // Long names must wrap to <= LABEL_MAX_LINES lines, each <= LABEL_WRAP_COLS
        // chars, so they never bleed across neighbouring cells.
        let label = cell_label("Puppy-Slug Gun");
        let lines: Vec<&str> = label.split('\n').collect();
        assert!(lines.len() <= LABEL_MAX_LINES, "label wraps to few lines: {label:?}");
        for line in lines {
            assert!(line.chars().count() <= LABEL_WRAP_COLS, "line fits the cell: {line:?}");
        }
    }

    #[test]
    fn items_page_has_one_detail_panel_not_per_cell_descriptions() {
        // Regression for the "24 overlapping descriptions" mush: NO grid cell may
        // carry the full item description as its detail text.
        let owned = OwnedItems::default();
        let page = build_items_page(&owned, None, CubeFocus::Item(Item::Blink.index()));
        for node in &page.nodes {
            if let ambition_inventory_ui::MenuNode::Control { detail: Some(d), kind, .. } = node {
                if *kind == MenuControlKind::Item {
                    assert!(
                        !d.contains(Item::Blink.description()),
                        "grid cell must not render the full description: {d:?}"
                    );
                }
            }
        }
        // The detail panel DOES render the focused item's description text.
        let has_desc = page.nodes.iter().any(|n| matches!(
            n,
            ambition_inventory_ui::MenuNode::Text { text, .. } if Item::Blink.description().contains(text.as_str()) && !text.is_empty()
        ));
        assert!(has_desc, "detail panel renders the focused item's description");
    }

    #[test]
    fn system_page_has_one_control_per_option_with_state_labels() {
        let mut settings = UserSettings::default();
        settings.video.show_fps = true;
        let focus = CubeFocus::System(0);
        let page = build_system_page(&settings, focus);
        // One actionable System control per option (edge buttons are also actions
        // but carry ChangePage, not System).
        let system_controls = page
            .nodes
            .iter()
            .filter(|n| matches!(
                n,
                ambition_inventory_ui::MenuNode::Control { action: Some(CubeAction::System(_)), .. }
            ))
            .count();
        assert_eq!(system_controls, SystemOption::ALL.len());

        // The Show FPS row's label reflects the ON state we set above.
        let has_on = page.nodes.iter().any(|n| matches!(
            n,
            ambition_inventory_ui::MenuNode::Control { action: Some(CubeAction::System(SystemOption::ToggleFps)), label, .. }
                if label == "Show FPS: on"
        ));
        assert!(has_on, "Show FPS row reflects the current ON state");

        // Edge buttons are present so rotation still works.
        let has_edges = page.nodes.iter().any(|n| matches!(
            n,
            ambition_inventory_ui::MenuNode::Control { action: Some(CubeAction::ChangePage(_)), .. }
        ));
        assert!(has_edges, "System page keeps the L/R edge buttons");
    }

    #[test]
    fn system_option_label_tracks_settings_changes() {
        let mut settings = UserSettings::default();
        let off = SystemOption::ToggleQuestHud.label(&settings);
        settings.gameplay.quest_hud_visible = !settings.gameplay.quest_hud_visible;
        let on = SystemOption::ToggleQuestHud.label(&settings);
        assert_ne!(off, on, "toggling the setting changes the row label");
    }

    #[test]
    fn viewer_left_button_turns_to_the_right_neighbor() {
        // Pressing LEFT rotates the cube left, bringing the +1 ring neighbour to
        // front (matches the demo's page_on_viewer_left = index + 1).
        assert_eq!(CubePage::Items.on_viewer_left(), CubePage::Map);
        assert_eq!(CubePage::Items.on_viewer_right(), CubePage::System);
        let owned = OwnedItems::default();
        let page = build_items_page(&owned, None, CubeFocus::default());
        let left = page.nodes.iter().find_map(|n| match n {
            ambition_inventory_ui::MenuNode::Control { action: Some(CubeAction::ChangePage(p)), rect, .. }
                if rect.x < 10.0 => Some(*p),
            _ => None,
        });
        assert_eq!(left, Some(CubePage::Map), "left edge button turns to viewer-left page");
    }
}
