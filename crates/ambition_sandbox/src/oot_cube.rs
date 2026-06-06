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
use crate::persistence::settings::{
    DevSnapshot, RadioSnapshot, SettingsOption, SettingsOptionId, SettingsOptionKind,
    SystemMenuAction, SystemMenuEntryId, SystemMenuModel, SystemMenuTarget, SystemOptionId,
    UserSettings,
};

/// Edge page-turn buttons flank the page in the side margins (NOT over the grid),
/// matching the demo's `add_edge_buttons` rects (`crates/ambition_mock_demo/src/
/// app/models.rs`). The game draws these as REAL controls and turns the lib's
/// decorative arrows off (`draw_nav_arrows = false`) so they aren't double-drawn.
const EDGE_LEFT_RECT: MenuRect = MenuRect {
    x: 1.8,
    y: 43.5,
    w: 7.5,
    h: 13.0,
};
const EDGE_RIGHT_RECT: MenuRect = MenuRect {
    x: 90.7,
    y: 43.5,
    w: 7.5,
    h: 13.0,
};

/// The item grid lives in the left/centre, clear of the side arrows. Matches the
/// demo's items-grid panel so the lib's auto-laid cells never reach the margins.
const GRID_RECT: MenuRect = MenuRect {
    x: 11.0,
    y: 19.0,
    w: 58.0,
    h: 55.0,
};

/// The right-hand detail panel: shows the focused item's name + wrapped
/// description (the demo's "SELECTED" panel). One panel, not one-per-cell.
const DETAIL_PANEL_RECT: MenuRect = MenuRect {
    x: 70.5,
    y: 19.0,
    w: 18.2,
    h: 55.0,
};

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
fn add_edge_buttons(
    model: &mut MenuPageModel<CubePage, CubeAction>,
    page: CubePage,
    focus: CubeFocus,
) {
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
    /// A settings toggle/cycle/slider on a drilled-in System SETTINGS screen. The
    /// host applies it by mutating `UserSettings` through the shared settings IR
    /// (`apply_settings_option`), which the existing `save_settings_on_change`
    /// system then persists — no parallel persistence path.
    System(SettingsOptionId),
    /// A non-settings System screen option (radio station / locale / dev toggle).
    /// Applied host-side against the matching live resource.
    SystemOption(SystemOptionId),
    /// An immediate, screen-less System action (Reset Sandbox).
    SystemAction(SystemMenuAction),
    /// Drill INTO a top-level System entry (show its screen rows). Handled
    /// host-side by setting the cube's drill-down state
    /// (`oot_cube_app::CubeSystemNav`).
    OpenSystemEntry(SystemMenuEntryId),
    /// Drill OUT of an open System entry (back to the top-level list).
    CloseSystemEntry,
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
    radio: &RadioSnapshot,
    dev: &DevSnapshot,
    open_entry: Option<SystemMenuEntryId>,
) -> Vec<MenuPageModel<CubePage, CubeAction>> {
    vec![
        build_items_page(owned, equipped, focus),
        placeholder_page(
            CubePage::Map,
            "MAP",
            "Area map: discovered rooms, anchors, portal markers (host data TODO).",
            focus,
        ),
        placeholder_page(
            CubePage::Quest,
            "QUEST",
            "Quest status + key items from save data (host data TODO).",
            focus,
        ),
        build_system_page(settings, radio, dev, focus, open_entry),
    ]
}

/// Index of the focused System row, clamped to the live row count. Non-System
/// focuses (e.g. a stale items cursor) default to the first row so callers always
/// have a valid row to describe in the detail panel.
fn system_focus_index(focus: CubeFocus, row_count: usize) -> usize {
    let max = row_count.saturating_sub(1);
    match focus {
        CubeFocus::System(idx) => idx.min(max),
        _ => 0,
    }
}

/// One System-face row. The cube's `CubeFocus::System` cursor is an index into the
/// currently-displayed row list: the top-level [`SystemMenuEntry`] list when no
/// entry is open, or the open entry's screen rows + a Back row otherwise. Carries
/// the shared SYSTEM-menu IR ids so the cube (and later the pause menu) draw from
/// one source of truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemRow {
    /// A top-level entry row (drill in / fire its action on select).
    Entry(SystemMenuEntryId),
    /// A settings option row inside an open settings screen (apply on select).
    Setting(SettingsOptionId),
    /// A non-settings screen option (radio / locale / dev toggle).
    Option(SystemOptionId),
    /// The Back row inside an open entry (drill out on select).
    Back,
}

/// The ordered rows shown on the System face for the given drill-down state and
/// live model. When no entry is open this is the top-level entry list; when an
/// entry is open it is that entry's screen rows followed by a Back row (immediate
/// Action entries have no screen, so they are never the open entry).
pub fn system_rows(model: &SystemMenuModel, open_entry: Option<SystemMenuEntryId>) -> Vec<SystemRow> {
    match open_entry.and_then(|id| model.entry(id)) {
        None => model
            .entries
            .iter()
            .map(|e| SystemRow::Entry(e.id))
            .collect(),
        Some(entry) => {
            let mut rows: Vec<SystemRow> = match &entry.target {
                SystemMenuTarget::Settings(options) => {
                    options.iter().map(|o| SystemRow::Setting(o.id)).collect()
                }
                SystemMenuTarget::Radio(rows) => rows
                    .iter()
                    .map(|r| SystemRow::Option(SystemOptionId::Radio(r.index)))
                    .collect(),
                SystemMenuTarget::Language(rows) => rows
                    .iter()
                    .map(|r| SystemRow::Option(SystemOptionId::Locale(r.id)))
                    .collect(),
                SystemMenuTarget::Developer(rows) => rows
                    .iter()
                    .map(|r| SystemRow::Option(SystemOptionId::Dev(r.id)))
                    .collect(),
                // An Action entry never opens a screen; defensively empty.
                SystemMenuTarget::Action(_) => Vec::new(),
            };
            rows.push(SystemRow::Back);
            rows
        }
    }
}

/// Look up a settings option's live IR entry by id from the live model.
fn setting_entry(model: &SystemMenuModel, id: SettingsOptionId) -> Option<SettingsOption> {
    model.entries.iter().find_map(|e| match &e.target {
        SystemMenuTarget::Settings(options) => options.iter().find(|o| o.id == id).cloned(),
        _ => None,
    })
}

/// The display label for a non-settings screen option (radio / locale / dev).
fn system_option_label(model: &SystemMenuModel, opt: SystemOptionId) -> String {
    match opt {
        SystemOptionId::Radio(index) => model
            .entry(SystemMenuEntryId::Radio)
            .and_then(|e| match &e.target {
                SystemMenuTarget::Radio(rows) => rows.iter().find(|r| r.index == index),
                _ => None,
            })
            .map(|r| {
                let marker = if r.active { "▶ " } else { "  " };
                format!("{marker}{}", r.label)
            })
            .unwrap_or_else(|| "Station".to_string()),
        SystemOptionId::Locale(id) => model
            .entry(SystemMenuEntryId::Language)
            .and_then(|e| match &e.target {
                SystemMenuTarget::Language(rows) => rows.iter().find(|r| r.id == id),
                _ => None,
            })
            .map(|r| {
                let marker = if r.active {
                    "▶ "
                } else if r.available {
                    "  "
                } else {
                    "× "
                };
                format!("{marker}{}", r.label)
            })
            .unwrap_or_else(|| "Locale".to_string()),
        SystemOptionId::Dev(id) => model
            .entry(SystemMenuEntryId::Developer)
            .and_then(|e| match &e.target {
                SystemMenuTarget::Developer(rows) => rows.iter().find(|r| r.id == id),
                _ => None,
            })
            .map(|r| format!("{}: {}  < >", r.label, r.value_label))
            .unwrap_or_else(|| id.label().to_string()),
    }
}

/// The description for a non-settings screen option.
fn system_option_description(opt: SystemOptionId) -> String {
    match opt {
        SystemOptionId::Radio(_) => {
            "Play this station now (the menu stays open to audition).".to_string()
        }
        SystemOptionId::Locale(id) => {
            if id.is_available() {
                "Active interface language.".to_string()
            } else {
                "Not available yet (real localization is a later pass).".to_string()
            }
        }
        SystemOptionId::Dev(id) => id.description().to_string(),
    }
}

/// The row's display label. Top-level entries append `>` (drill) except the
/// immediate Action entry; Back shows `< Back`.
fn system_row_label(model: &SystemMenuModel, row: SystemRow) -> String {
    match row {
        SystemRow::Entry(id) => match model.entry(id).map(|e| &e.target) {
            Some(SystemMenuTarget::Action(_)) => id.label().to_string(),
            _ => format!("{} >", id.label()),
        },
        SystemRow::Back => "< Back".to_string(),
        SystemRow::Setting(id) => match setting_entry(model, id) {
            Some(entry) => match entry.kind {
                SettingsOptionKind::Action => entry.label,
                _ => format!("{}: {}  < >", entry.label, entry.value_label),
            },
            None => "—".to_string(),
        },
        SystemRow::Option(opt) => system_option_label(model, opt),
    }
}

/// The detail-panel description for a row.
fn system_row_description(model: &SystemMenuModel, row: SystemRow) -> String {
    match row {
        SystemRow::Entry(id) => id.description().to_string(),
        SystemRow::Back => "Return to the SYSTEM list.".to_string(),
        SystemRow::Setting(id) => setting_entry(model, id)
            .map(|e| e.description)
            .unwrap_or_default(),
        SystemRow::Option(opt) => system_option_description(opt),
    }
}

/// The action a System row dispatches on select.
fn system_row_action(model: &SystemMenuModel, row: SystemRow) -> Option<CubeAction> {
    match row {
        SystemRow::Entry(id) => match model.entry(id).map(|e| &e.target) {
            Some(SystemMenuTarget::Action(action)) => Some(CubeAction::SystemAction(*action)),
            _ => Some(CubeAction::OpenSystemEntry(id)),
        },
        SystemRow::Setting(o) => Some(CubeAction::System(o)),
        SystemRow::Option(o) => Some(CubeAction::SystemOption(o)),
        SystemRow::Back => Some(CubeAction::CloseSystemEntry),
    }
}

// ---- System-face touch layout ------------------------------------------------
//
// The System face uses its OWN layout (distinct from the items grid): bigger
// fonts, taller touch-sized control rects, a CENTERED + ENLARGED option column
// (the description no longer steals the right third), and the focused option's
// description in a BOTTOM panel. This both improves finger/mouse pick targets
// and helps the perspective raycast land on the rows (thin rows read as dead).

/// Centered, widened option column. Spans most of the face width (the items
/// page's right-hand detail panel room is reclaimed here since the System
/// description now lives on the bottom).
const SYSTEM_LIST_RECT: MenuRect = MenuRect {
    x: 16.0,
    y: 20.0,
    w: 68.0,
    h: 52.0,
};
/// Bottom description panel for the focused option (replaces the right-side
/// panel the items page uses).
const SYSTEM_DESC_RECT: MenuRect = MenuRect {
    x: 12.0,
    y: 75.0,
    w: 76.0,
    h: 17.0,
};
/// Title font for the System face (larger than the old 3.4).
const SYSTEM_TITLE_SIZE: f32 = 4.2;
/// Row label font (larger than the items grid's ~1.5; touch-readable).
const SYSTEM_ROW_FONT: f32 = 2.3;
/// Max touch-sized row height; the actual height clamps to this so a sparse
/// category does not produce absurdly tall rows.
const SYSTEM_ROW_MAX_H: f32 = 8.5;
/// Description wrap width inside the wider bottom panel.
const SYSTEM_DESC_WRAP_COLS: usize = 60;

/// The System cube face. When no category is open it shows the CATEGORY list
/// (Video / Audio / Controls / Gameplay + Close Menu), mirroring the Bevy-UI pause
/// menu's settings page stack; drilling INTO a category lists that category's
/// option rows plus a Back row. Rows are drawn from the shared settings IR
/// (`settings_menu_model`), with a touch-friendly layout: large fonts, tall
/// centered rows, and the focused option's description in a bottom panel. The L/R
/// edge buttons (so rotation still works) are kept.
pub fn build_system_page(
    settings: &UserSettings,
    radio: &RadioSnapshot,
    dev: &DevSnapshot,
    focus: CubeFocus,
    open_entry: Option<SystemMenuEntryId>,
) -> MenuPageModel<CubePage, CubeAction> {
    let sys_model = SystemMenuModel::build(settings, radio, dev);
    let mut model = MenuPageModel::new(
        CubePage::System,
        "SYSTEM",
        MenuColor::rgba(0.03, 0.04, 0.10, 0.96),
    );
    let title = match open_entry {
        None => "SYSTEM".to_string(),
        Some(id) => format!("SYSTEM \u{2022} {}", id.label().to_uppercase()),
    };
    model.text(
        50.0,
        12.0,
        SYSTEM_TITLE_SIZE,
        &title,
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );

    let rows = system_rows(&sys_model, open_entry);
    let focused = system_focus_index(focus, rows.len());
    // Lay the rows as one centered vertical column. The pitch is sized to fill the
    // list rect for the CURRENT row count (so a sparse category reads large), but
    // each row height is capped at the touch maximum.
    let count = rows.len().max(1) as f32;
    let step = SYSTEM_LIST_RECT.h / count;
    let row_h = (step * 0.82).min(SYSTEM_ROW_MAX_H);
    for (idx, row) in rows.iter().enumerate() {
        let y = SYSTEM_LIST_RECT.y + idx as f32 * step;
        model.control(
            MenuRect {
                x: SYSTEM_LIST_RECT.x,
                y,
                w: SYSTEM_LIST_RECT.w,
                h: row_h,
            },
            MenuControlKind::OptionToggle,
            system_row_label(&sys_model, *row),
            // No per-row detail hint: the focused option's description now lives
            // in the dedicated BOTTOM panel, so the tall rows stay uncluttered.
            None,
            idx == focused,
            false,
            system_row_action(&sys_model, *row),
        );
    }

    add_system_detail_panel(&mut model, &sys_model, &rows, focused);
    add_edge_buttons(&mut model, CubePage::System, focus);
    model
}

/// The System face's BOTTOM detail panel: the focused option's label (its current
/// state) plus a wrapped description. A wide, short panel along the bottom (the
/// items page keeps its right-side panel; the System face moves it down so the
/// option column can be centered + enlarged).
fn add_system_detail_panel(
    model: &mut MenuPageModel<CubePage, CubeAction>,
    sys_model: &SystemMenuModel,
    rows: &[SystemRow],
    focused: usize,
) {
    model.panel(
        SYSTEM_DESC_RECT,
        MenuColor::rgba(0.035, 0.046, 0.105, 0.96),
        None,
    );
    let cx = SYSTEM_DESC_RECT.x + SYSTEM_DESC_RECT.w * 0.5;
    let (label, description) = rows
        .get(focused)
        .map(|row| {
            (
                system_row_label(sys_model, *row),
                system_row_description(sys_model, *row),
            )
        })
        .unwrap_or_else(|| ("System".to_string(), "System options.".to_string()));
    // Header: the focused row's label (its live value), then the wrapped
    // description below it inside the bottom band.
    model.text(
        cx,
        SYSTEM_DESC_RECT.y + 4.0,
        2.4,
        &label,
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );
    let mut lines = wrap_text(&description, SYSTEM_DESC_WRAP_COLS);
    lines.truncate(3);
    for (line_idx, line) in lines.into_iter().enumerate() {
        model.text(
            cx,
            SYSTEM_DESC_RECT.y + 8.5 + line_idx as f32 * 3.0,
            SYSTEM_ROW_FONT * 0.78,
            line,
            MenuTextAlign::Center,
            MenuColor::rgba(0.88, 0.94, 1.0, 0.96),
        );
    }
}

fn placeholder_page(
    page: CubePage,
    title: &str,
    body: &str,
    focus: CubeFocus,
) -> MenuPageModel<CubePage, CubeAction> {
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
    // Placeholder pages keep the edge buttons AND honour the live cursor focus, so
    // landing on an edge button after a page turn (Fix 1) highlights it. Only the
    // EdgeLeft/EdgeRight focuses matter here; an item/system focus simply leaves both
    // edge buttons un-highlighted (harmless — the placeholder has no other controls).
    add_edge_buttons(&mut model, page, focus);
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
                let next_len =
                    line.chars().count() + word.chars().count() + usize::from(needs_space);
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
        assert!(
            lines.len() <= LABEL_MAX_LINES,
            "label wraps to few lines: {label:?}"
        );
        for line in lines {
            assert!(
                line.chars().count() <= LABEL_WRAP_COLS,
                "line fits the cell: {line:?}"
            );
        }
    }

    #[test]
    fn items_page_has_one_detail_panel_not_per_cell_descriptions() {
        // Regression for the "24 overlapping descriptions" mush: NO grid cell may
        // carry the full item description as its detail text.
        let owned = OwnedItems::default();
        let page = build_items_page(&owned, None, CubeFocus::Item(Item::Blink.index()));
        for node in &page.nodes {
            if let ambition_inventory_ui::MenuNode::Control {
                detail: Some(d),
                kind,
                ..
            } = node
            {
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
        assert!(
            has_desc,
            "detail panel renders the focused item's description"
        );
    }

    #[test]
    fn system_page_top_level_shows_entry_list() {
        let settings = UserSettings::default();
        let focus = CubeFocus::System(0);
        // No entry open -> the top-level view is the SYSTEM entry list.
        let page = build_system_page(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
            focus,
            None,
        );
        let entries = page
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    ambition_inventory_ui::MenuNode::Control {
                        action: Some(CubeAction::OpenSystemEntry(_)),
                        ..
                    }
                )
            })
            .count();
        // Radio + Video + Audio + Controls + Gameplay + Language always drill in;
        // Developer also drills in only in dev builds (Reset Sandbox is an Action,
        // not OpenSystemEntry).
        let expected_drill = if crate::persistence::settings::system_menu::DEV_BUILD {
            7
        } else {
            6
        };
        assert_eq!(entries, expected_drill, "one drill row per non-action entry");
        // No raw settings toggles leak at the top level.
        let has_setting = page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_inventory_ui::MenuNode::Control {
                    action: Some(CubeAction::System(_)),
                    ..
                }
            )
        });
        assert!(!has_setting, "entry list does not show raw setting toggles");
        // Edge buttons are present so rotation still works.
        let has_edges = page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_inventory_ui::MenuNode::Control {
                    action: Some(CubeAction::ChangePage(_)),
                    ..
                }
            )
        });
        assert!(has_edges, "System page keeps the L/R edge buttons");
    }

    #[test]
    fn system_page_drilled_into_video_shows_curated_options_and_back() {
        let mut settings = UserSettings::default();
        settings.video.show_fps = true;
        let focus = CubeFocus::System(0);
        // Drill into Video -> its curated options (from the SYSTEM IR) + a Back row.
        let page = build_system_page(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
            focus,
            Some(SystemMenuEntryId::Video),
        );
        let options: Vec<_> = page
            .nodes
            .iter()
            .filter_map(|n| match n {
                ambition_inventory_ui::MenuNode::Control {
                    action: Some(CubeAction::System(o)),
                    ..
                } => Some(*o),
                _ => None,
            })
            .collect();
        // The cube's Video screen is the curated subset.
        assert_eq!(
            options,
            vec![
                SettingsOptionId::DisplayMode,
                SettingsOptionId::ShowFps,
                SettingsOptionId::CameraZoom,
            ]
        );

        // The FPS Overlay row's label reflects the ON state we set above.
        let has_on = page.nodes.iter().any(|n| matches!(
            n,
            ambition_inventory_ui::MenuNode::Control { action: Some(CubeAction::System(SettingsOptionId::ShowFps)), label, .. }
                if label.contains("ON")
        ));
        assert!(has_on, "FPS Overlay row reflects the current ON state");

        // A Back row (CloseSystemEntry) drills out to the entry list.
        let has_back = page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_inventory_ui::MenuNode::Control {
                    action: Some(CubeAction::CloseSystemEntry),
                    ..
                }
            )
        });
        assert!(has_back, "an open entry shows a Back row");
    }

    #[test]
    fn system_setting_label_tracks_settings_changes() {
        let mut settings = UserSettings::default();
        let model0 = SystemMenuModel::build(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        let off = system_row_label(&model0, SystemRow::Setting(SettingsOptionId::QuestHud));
        settings.gameplay.quest_hud_visible = !settings.gameplay.quest_hud_visible;
        let model1 = SystemMenuModel::build(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        let on = system_row_label(&model1, SystemRow::Setting(SettingsOptionId::QuestHud));
        assert_ne!(off, on, "toggling the setting changes the row label");
    }

    #[test]
    fn map_and_quest_edge_buttons_are_focusable_and_highlight() {
        // Fix 1: placeholder pages (Map / Quest) build real, focusable L/R edge
        // buttons that highlight from the live cursor focus — previously they were
        // hard-wired to `CubeFocus::Item(0)` so they could never highlight.
        for page in [CubePage::Map, CubePage::Quest] {
            let model = placeholder_page(page, "T", "body", CubeFocus::EdgeLeft);
            // Both edge buttons exist as Action controls with a ChangePage action.
            let edges: Vec<_> = model
                .nodes
                .iter()
                .filter(|n| {
                    matches!(
                        n,
                        ambition_inventory_ui::MenuNode::Control {
                            kind: MenuControlKind::Action,
                            action: Some(CubeAction::ChangePage(_)),
                            ..
                        }
                    )
                })
                .collect();
            assert_eq!(edges.len(), 2, "{page:?} has both L/R edge buttons");
            // With EdgeLeft focus, exactly the LEFT edge button reads selected.
            let left_selected = model.nodes.iter().any(|n| matches!(
                n,
                ambition_inventory_ui::MenuNode::Control { rect, selected: true, action: Some(CubeAction::ChangePage(_)), .. }
                    if rect.x < 10.0
            ));
            assert!(
                left_selected,
                "{page:?} left edge button highlights on EdgeLeft focus"
            );
        }
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
            ambition_inventory_ui::MenuNode::Control {
                action: Some(CubeAction::ChangePage(p)),
                rect,
                ..
            } if rect.x < 10.0 => Some(*p),
            _ => None,
        });
        assert_eq!(
            left,
            Some(CubePage::Map),
            "left edge button turns to viewer-left page"
        );
    }
}
