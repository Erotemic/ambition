//! The data seam between Ambition's live 24-item inventory and the reusable
//! `ambition_menu` 3D-cube OoT pause menu (#31).
//!
//! The game owns the item state (`crate::items`); this module builds the cube's
//! page MODELS from it via the lib's host-data seam (`ItemsOnlyPageSpec`, which is
//! deliberately renderer-agnostic — it can feed the Lunex cube, a Bevy-UI grid
//! fallback, or a test renderer). The cube RENDERER itself is the shared lib.
//!
//! This gives the "wire us up to use it" part: our `Item::ALL` (already 24 in OoT
//! grid order) → the cube's items page, with owned/equipped/selected reflected and
//! a host-defined [`MenuPageAction`] emitted back to the game.
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
//! * the [`DETAIL_PANEL_RECT`] on the right shows the [`MenuFocus`]ed item's name +
//!   wrapped description (filled by `lunex_kaleidoscope_app::kaleidoscope_sync_detail_panel`),
//! * the L/R page-turn buttons live in the *side margins* ([`EDGE_LEFT_RECT`] /
//!   [`EDGE_RIGHT_RECT`]) OUTSIDE the grid, exactly like the demo.

use ambition_menu::{
    InventoryItemNode, ItemsOnlyPageSpec, MenuColor, MenuControlKind, MenuPageModel, MenuRect,
    MenuTextAlign,
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

/// Detail-panel dynamic-text slots. The detail panel's CONTENT is cursor-dependent
/// but its LAYOUT is fixed, so the panel emits a fixed set of [`MenuDynamicText`]
/// slots (spawned empty) that the in-place updater
/// (`crate::lunex_kaleidoscope_app::kaleidoscope_sync_detail_text`) rewrites from
/// the live cursor — no face rebuild, so a hover never drops a `Pointer<Click>`.
///
/// Items face: slot 0 is the "SELECTED" header, slots `1..=DETAIL_VISIBLE_TOTAL`
/// are the wrapped name/description/status lines.
pub const ITEMS_DETAIL_BODY_SLOT0: u32 = 1;
/// Total body lines reserved on the Items detail panel (name + blank + desc +
/// blank + status, capped by [`DETAIL_VISIBLE_LINES`]).
pub const ITEMS_DETAIL_BODY_LINES: u32 = 12;
/// System face bottom panel: slot 100 is the focused row's label, slots
/// `101..` are its wrapped description lines.
pub const SYSTEM_DETAIL_LABEL_SLOT: u32 = 100;
pub const SYSTEM_DETAIL_BODY_SLOT0: u32 = 101;
/// Description lines reserved on the System bottom panel.
pub const SYSTEM_DETAIL_BODY_LINES: u32 = 3;

impl MenuPage {
    /// The neighbouring page when turning the ring left/right (wraps), matching
    /// [`MenuPage::ALL`] order.
    pub fn neighbor(self, dir: isize) -> MenuPage {
        let all = MenuPage::ALL;
        let cur = all.iter().position(|p| *p == self).unwrap_or(0);
        let next = (cur as isize + dir).rem_euclid(all.len() as isize) as usize;
        all[next]
    }

    /// The page that physically sits to the viewer's LEFT — i.e. the one that
    /// rotates to the front when the cube turns LEFT. This is the same
    /// inside-the-cube convention as the demo's `page_on_viewer_left`
    /// (`from_index(index + 1)`): pressing the LEFT affordance rotates the ring
    /// left, which brings the RIGHT-neighbour (`+1` in ring order) to the front.
    pub fn on_viewer_left(self) -> MenuPage {
        self.neighbor(1)
    }

    /// The page to the viewer's RIGHT (rotates to front when turning RIGHT).
    pub fn on_viewer_right(self) -> MenuPage {
        self.neighbor(-1)
    }
}

/// Append the Left/Right edge page-turn buttons to a page model. Emitted as real
/// `Action` controls so Bevy picking + directional focus can dispatch them. The
/// LEFT button turns the cube left → brings the viewer-left page to front
/// ([`MenuPage::on_viewer_left`]); mirrors the demo's `add_edge_buttons`.
fn add_edge_buttons(model: &mut MenuPageModel<MenuPage, MenuPageAction>, page: MenuPage) {
    // `selected: false` — the focus highlight is applied IN PLACE from the live
    // cursor (`kaleidoscope_sync_focus_visuals`), not baked here, so a hover does
    // not rebuild the face (which would drop a `Pointer<Click>`).
    model.control(
        EDGE_LEFT_RECT,
        MenuControlKind::Action,
        format!("<\n{}", page_label(page.on_viewer_left())),
        Some("turn cube left".to_string()),
        false,
        false,
        Some(MenuPageAction::ChangePage(page.on_viewer_left())),
    );
    model.control(
        EDGE_RIGHT_RECT,
        MenuControlKind::Action,
        format!(">\n{}", page_label(page.on_viewer_right())),
        Some("turn cube right".to_string()),
        false,
        false,
        Some(MenuPageAction::ChangePage(page.on_viewer_right())),
    );
}

fn page_label(page: MenuPage) -> &'static str {
    match page {
        MenuPage::Items => "Items",
        MenuPage::Map => "Map",
        MenuPage::Quest => "Quest",
        MenuPage::System => "System",
    }
}

/// The cube faces (pages). `Items` is wired live from our inventory; the rest
/// mirror OoT's subscreen tabs as host-data-driven placeholders for now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuPage {
    Items,
    Map,
    Quest,
    System,
}

impl MenuPage {
    pub const ALL: [MenuPage; 4] = [
        MenuPage::Items,
        MenuPage::Map,
        MenuPage::Quest,
        MenuPage::System,
    ];
}

/// The cursor's logical position on the items page: either an item slot or one of
/// the flanking edge (page-turn) buttons. This is the game-side equivalent of the
/// demo's `MockAction`-as-selection, and the unit of [`crate::lunex_kaleidoscope_app`]'s
/// directional navigation (`move_spatial`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuFocus {
    EdgeLeft,
    EdgeRight,
    Item(usize),
    /// A System-face option row (index into [`SystemOption::ALL`]).
    System(usize),
}

impl Default for MenuFocus {
    fn default() -> Self {
        MenuFocus::Item(0)
    }
}

impl MenuFocus {
    /// The item slot index this focus refers to, clamped into range. Edge buttons
    /// report slot 0 so callers always have a valid item to describe.
    pub fn item_index(self) -> usize {
        match self {
            MenuFocus::Item(idx) => idx.min(crate::items::ITEM_COUNT - 1),
            _ => 0,
        }
    }
}

/// Actions the cube emits back to the game (the host consumes these — the cube
/// never mutates item state itself, matching the existing `bevy_ui_grid_menu` seam).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPageAction {
    Equip(Item),
    Use(Item),
    ChangePage(MenuPage),
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
    /// (`lunex_kaleidoscope_app::KaleidoscopeSystemNav`).
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
) -> ItemsOnlyPageSpec<MenuPage, MenuPageAction> {
    let mut spec = ItemsOnlyPageSpec::new(MenuPage::Items, "ITEMS")
        .with_grid(ITEM_GRID_ROWS, ITEM_GRID_COLS)
        .with_grid_rect(GRID_RECT);
    // No baked `selected_slot`: the focus highlight is applied IN PLACE from the
    // live cursor (`kaleidoscope_sync_focus_visuals`), so a hover does not rebuild
    // the items face (which would drop a `Pointer<Click>`).
    spec.selected_slot = None;
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
            // Render the item's sprite in the cell when it has one; the catalog
            // returns `None` for items with no authored art, which keeps the text
            // label (the lib falls back to text when `icon` is `None`). The item's
            // name still shows in the detail panel either way.
            if let Some(icon) = item.icon_path() {
                node = node.icon(icon);
            }
            if owns {
                // Held-item weapons/abilities equip; everything else "uses".
                let (action, label) = if item.held_item_id().is_some() {
                    (MenuPageAction::Equip(item), "Equip")
                } else {
                    (MenuPageAction::Use(item), "Use")
                };
                node = node.action(action).action_label(label);
            }
            node
        })
        .collect();
    spec
}

/// The Items cube face, built from our live inventory. Lays out the grid, the
/// right-hand detail panel for the [`MenuFocus`]ed item, and the flanking
/// page-turn buttons — matching the demo's `add_items_page` structure.
pub fn build_items_page(
    owned: &OwnedItems,
    equipped: Option<Item>,
) -> MenuPageModel<MenuPage, MenuPageAction> {
    let mut model = items_spec(owned, equipped).into_page_model();
    add_detail_panel(&mut model);
    add_edge_buttons(&mut model, MenuPage::Items);
    model
}

/// Render the single right-hand detail panel for the focused item: its name and
/// wrapped description. This is the demo's "SELECTED" panel — exactly ONE detail
/// region for the whole page, not one per cell.
///
/// The panel's CONTENT is cursor-dependent, so it is emitted as fixed, empty
/// [`MenuDynamicText`] slots; the in-place updater
/// (`crate::lunex_kaleidoscope_app::kaleidoscope_sync_detail_text`) fills them
/// from the live cursor each time it moves — no rebuild, so a hover never drops a
/// click. See [`items_detail_slot_text`] for the slot→string mapping.
fn add_detail_panel(model: &mut MenuPageModel<MenuPage, MenuPageAction>) {
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
    for line_idx in 0..ITEMS_DETAIL_BODY_LINES {
        model.dynamic_text(
            ITEMS_DETAIL_BODY_SLOT0 + line_idx,
            79.6,
            28.5 + line_idx as f32 * 3.2,
            1.55,
            MenuTextAlign::Center,
            MenuColor::rgba(0.88, 0.94, 1.0, 0.96),
        );
    }
}

/// The Items detail-panel text keyed by dynamic-text slot, for the focused item.
/// Empty slots (lines beyond the item's text) map to `""` so a previously longer
/// description is cleared in place. This is the in-place equivalent of the old
/// baked detail panel.
pub fn items_detail_slot_text(
    owned: &OwnedItems,
    equipped: Option<Item>,
    focus: MenuFocus,
) -> Vec<(u32, String)> {
    let item = Item::from_index(focus.item_index()).unwrap_or(Item::ALL[0]);
    let lines = detail_lines(owned, equipped, item);
    (0..ITEMS_DETAIL_BODY_LINES)
        .map(|i| {
            let slot = ITEMS_DETAIL_BODY_SLOT0 + i;
            let text = lines.get(i as usize).cloned().unwrap_or_default();
            (slot, text)
        })
        .collect()
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
/// `ActiveMenuPages<MenuPage, MenuPageAction>`. `focus` highlights the items page's
/// cursor and selects the detail-panel item.
#[allow(clippy::too_many_arguments)]
pub fn build_inventory_pages(
    owned: &OwnedItems,
    equipped: Option<Item>,
    focus: MenuFocus,
    settings: &UserSettings,
    radio: &RadioSnapshot,
    dev: &DevSnapshot,
    // The effective System scroll-window start (Features C/D). Drives which System
    // rows render + the scrollbar thumb position.
    system_window_start: usize,
    open_entry: Option<SystemMenuEntryId>,
) -> Vec<MenuPageModel<MenuPage, MenuPageAction>> {
    vec![
        build_items_page(owned, equipped),
        placeholder_page(
            MenuPage::Map,
            "MAP",
            "Area map: discovered rooms, anchors, portal markers (host data TODO).",
        ),
        placeholder_page(
            MenuPage::Quest,
            "QUEST",
            "Quest status + key items from save data (host data TODO).",
        ),
        build_system_page(settings, radio, dev, focus, system_window_start, open_entry),
    ]
}

/// Index of the focused System row, clamped to the live row count. Non-System
/// focuses (e.g. a stale items cursor) default to the first row so callers always
/// have a valid row to describe in the detail panel.
fn system_focus_index(focus: MenuFocus, row_count: usize) -> usize {
    let max = row_count.saturating_sub(1);
    match focus {
        MenuFocus::System(idx) => idx.min(max),
        _ => 0,
    }
}

/// One System-face row. The cube's `MenuFocus::System` cursor is an index into the
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
pub fn system_rows(
    model: &SystemMenuModel,
    open_entry: Option<SystemMenuEntryId>,
) -> Vec<SystemRow> {
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
fn system_row_action(model: &SystemMenuModel, row: SystemRow) -> Option<MenuPageAction> {
    match row {
        SystemRow::Entry(id) => match model.entry(id).map(|e| &e.target) {
            Some(SystemMenuTarget::Action(action)) => Some(MenuPageAction::SystemAction(*action)),
            _ => Some(MenuPageAction::OpenSystemEntry(id)),
        },
        SystemRow::Setting(o) => Some(MenuPageAction::System(o)),
        SystemRow::Option(o) => Some(MenuPageAction::SystemOption(o)),
        SystemRow::Back => Some(MenuPageAction::CloseSystemEntry),
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
/// category does not produce absurdly tall rows. Fix 2: bumped from 8.5 so the 6
/// visible rows are noticeably taller (bigger touch targets + bigger Rh-relative
/// font) while 6 rows + title + bottom panel still fit the face.
const SYSTEM_ROW_MAX_H: f32 = 10.5;
/// Description wrap width inside the wider bottom panel.
const SYSTEM_DESC_WRAP_COLS: usize = 60;
/// Max System rows shown at once before the list becomes a windowed scroll list
/// (Fix 3/4). Long screens — Radio (~26 stations) and Developer (~15 toggles) —
/// otherwise cram every row into the list rect, producing unreadably thin rows.
/// At/under this count the whole list shows; over it, the window follows the
/// cursor (same mechanic as the Bevy-UI pause menu's `RADIO_VISIBLE_ROWS`). Sized
/// for large, finger-readable rows that still leave room for the bottom panel: 6
/// rows give each a noticeably bigger touch target + font than the old 7.
pub const SYSTEM_VISIBLE_ROWS: usize = 6;

/// The largest valid scroll-window START for a list of `total` rows: the last
/// position that still fills the visible window (so the bottom row is reachable
/// without overscrolling past the end). `0` when the list fits.
pub fn system_max_window_start(total: usize) -> usize {
    total.saturating_sub(SYSTEM_VISIBLE_ROWS)
}

/// The cursor-derived scroll-window START (the window that keeps the focused row
/// visible). The default when no explicit scroll override is in effect.
pub fn system_window_start(rows: &[SystemRow], focus: MenuFocus) -> usize {
    let focused = system_focus_index(focus, rows.len());
    if rows.len() <= SYSTEM_VISIBLE_ROWS {
        return 0;
    }
    crate::ui_nav::visible_window_start(focused, rows.len(), SYSTEM_VISIBLE_ROWS)
}

/// The EFFECTIVE scroll-window START for the System face (Features C/D).
///
/// When the host holds an explicit scroll override (set by a scrollbar drag or the
/// mouse wheel), that override — clamped into `[0, system_max_window_start]` — wins,
/// so the visible window scrolls INDEPENDENTLY of the keyboard selection. With no
/// override (`None`) it falls back to the cursor-following window
/// ([`system_window_start`]). The face's STRUCTURE (which rows render) depends only
/// on this start, so the republish keys off it: a hover that does not shift the
/// window does NOT rebuild the face (which would drop a `Pointer<Click>`).
pub fn system_effective_window_start(
    rows: &[SystemRow],
    focus: MenuFocus,
    scroll_override: Option<usize>,
) -> usize {
    if rows.len() <= SYSTEM_VISIBLE_ROWS {
        return 0;
    }
    match scroll_override {
        Some(start) => start.min(system_max_window_start(rows.len())),
        None => system_window_start(rows, focus),
    }
}

/// The visible window of System rows for an explicit window START + a scroll
/// indicator. When the list fits ([`SYSTEM_VISIBLE_ROWS`]) the whole list shows
/// with no indicator. When it overflows, the `start` (the effective scroll
/// position) selects the visible slice. Returns the windowed `(absolute_index,
/// row)` slice plus an `"n/total"` indicator string (1-based first-visible row).
fn system_visible_window(
    rows: &[SystemRow],
    start: usize,
) -> (Vec<(usize, SystemRow)>, Option<String>) {
    let total = rows.len();
    if total <= SYSTEM_VISIBLE_ROWS {
        let slice = rows.iter().copied().enumerate().collect();
        return (slice, None);
    }
    let start = start.min(system_max_window_start(total));
    let end = (start + SYSTEM_VISIBLE_ROWS).min(total);
    let slice = (start..end).map(|i| (i, rows[i])).collect();
    // 1-based "first-visible/total" indicator (e.g. "8/26") so the scroll position in
    // the full list is legible even though only a window renders.
    let indicator = format!("{}/{}", start + 1, total);
    (slice, Some(indicator))
}

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
    focus: MenuFocus,
    // The EFFECTIVE scroll-window start (cursor-derived OR a drag/wheel override —
    // see [`system_effective_window_start`]). Drives which rows render + the thumb.
    window_start: usize,
    open_entry: Option<SystemMenuEntryId>,
) -> MenuPageModel<MenuPage, MenuPageAction> {
    let sys_model = SystemMenuModel::build(settings, radio, dev);
    let mut model = MenuPageModel::new(
        MenuPage::System,
        "SYSTEM",
        MenuColor::rgba(0.03, 0.04, 0.10, 0.96),
    );
    let rows = system_rows(&sys_model, open_entry);
    let _ = focus;
    // Fix 3/4: long screens (Radio ~26, Developer ~15) become a windowed scroll
    // list — only the visible window of rows is rendered, the window is the
    // effective scroll position, and the title gains an "n/total" scroll indicator.
    // Short screens show every row with no indicator (unchanged).
    let (window, indicator) = system_visible_window(&rows, window_start);

    let title = match open_entry {
        None => "SYSTEM".to_string(),
        Some(id) => format!("SYSTEM \u{2022} {}", id.label().to_uppercase()),
    };
    let title = match indicator {
        Some(ind) => format!("{title}  {ind}"),
        None => title,
    };
    model.text(
        50.0,
        12.0,
        SYSTEM_TITLE_SIZE,
        &title,
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );

    // Lay the VISIBLE rows as one centered vertical column. The pitch is sized to
    // fill the list rect for the visible row count (so a sparse window reads large),
    // but each row height is capped at the touch maximum. `slot` is the row's
    // position WITHIN the window; `abs_idx` is its absolute index into the full row
    // list — the latter drives selection highlight + the dispatched action, so the
    // keyboard cursor (which navigates the full list) and the windowed render agree.
    let count = window.len().max(1) as f32;
    let step = SYSTEM_LIST_RECT.h / count;
    let row_h = (step * 0.82).min(SYSTEM_ROW_MAX_H);
    for (slot, (abs_idx, row)) in window.iter().enumerate() {
        let y = SYSTEM_LIST_RECT.y + slot as f32 * step;
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
            // `selected: false` — the row highlight is applied IN PLACE from the
            // live cursor (`kaleidoscope_sync_focus_visuals`), so hovering a row
            // does not rebuild the face. `abs_idx` is unused for highlight now but
            // still anchors the row's action to its absolute list index.
            {
                let _ = abs_idx;
                false
            },
            false,
            system_row_action(&sys_model, *row),
        );
    }

    // Feature C: a draggable scrollbar appears only when the list overflows the
    // visible window. The full-height track is one `Scrollbar` control (draggable
    // via the lib's pointer-drag observers → `MenuScrollDragged`); a thumb panel on
    // top reflects the current scroll fraction + window size. The host applies the
    // emitted drag fraction to the scroll position (`KaleidoscopeScroll`).
    let total = rows.len();
    if total > SYSTEM_VISIBLE_ROWS {
        add_system_scrollbar(&mut model, window_start, total);
    }

    add_system_detail_panel(&mut model);
    add_edge_buttons(&mut model, MenuPage::System);
    model
}

/// Vertical scrollbar track rect for the System list (right of [`SYSTEM_LIST_RECT`]).
const SYSTEM_SCROLLBAR_RECT: MenuRect = MenuRect {
    x: 86.0,
    y: 20.0,
    w: 3.0,
    h: 52.0,
};

/// Feature C / Fix 1: author the System face's draggable scrollbar. Emitted as ONE
/// `MenuPageModel::scrollbar` node: the lib draws a DIM full-height track with a
/// BRIGHT thumb child sized to the visible fraction and positioned by the current
/// `window_start`. The thumb both shows the scroll position and is the visible grab
/// target; the lib's pointer-drag observers turn a drag on the track into the neutral
/// `MenuScrollDragged` fraction. The host computes the thumb geometry (it knows
/// visible-count / total / window-start); the lib owns the track+thumb visuals.
fn add_system_scrollbar(
    model: &mut MenuPageModel<MenuPage, MenuPageAction>,
    window_start: usize,
    total: usize,
) {
    let (thumb_start, thumb_size) = system_scrollbar_thumb(window_start, total);
    model.scrollbar(SYSTEM_SCROLLBAR_RECT, thumb_start, thumb_size);
}

/// Fix 1: the System scrollbar thumb geometry as track fractions `0..=1`.
/// `size` = visible rows / total rows (how much of the list is on screen); `start` =
/// scroll position as a fraction of the remaining window travel (0 = top window, 1 =
/// bottom window). Pure so it is headlessly unit-testable.
pub fn system_scrollbar_thumb(window_start: usize, total: usize) -> (f32, f32) {
    let size = (SYSTEM_VISIBLE_ROWS as f32 / total as f32).clamp(0.0, 1.0);
    let max_start = system_max_window_start(total).max(1) as f32;
    let start = (window_start as f32 / max_start).clamp(0.0, 1.0);
    (start, size)
}

/// The System face's BOTTOM detail panel: the focused option's label (its current
/// state) plus a wrapped description. A wide, short panel along the bottom (the
/// items page keeps its right-side panel; the System face moves it down so the
/// option column can be centered + enlarged).
/// The System face's BOTTOM detail panel. Its CONTENT (focused row's label +
/// description) is cursor-dependent, so it is emitted as fixed, empty
/// [`MenuDynamicText`] slots filled in place by
/// `crate::lunex_kaleidoscope_app::kaleidoscope_sync_detail_text`
/// (see [`system_detail_slot_text`]) — no rebuild on hover, so the click survives.
fn add_system_detail_panel(model: &mut MenuPageModel<MenuPage, MenuPageAction>) {
    model.panel(
        SYSTEM_DESC_RECT,
        MenuColor::rgba(0.035, 0.046, 0.105, 0.96),
        None,
    );
    let cx = SYSTEM_DESC_RECT.x + SYSTEM_DESC_RECT.w * 0.5;
    // Header: the focused row's label (filled in place).
    model.dynamic_text(
        SYSTEM_DETAIL_LABEL_SLOT,
        cx,
        SYSTEM_DESC_RECT.y + 4.0,
        2.4,
        MenuTextAlign::Center,
        MenuColor::rgba(1.0, 0.84, 0.38, 1.0),
    );
    for line_idx in 0..SYSTEM_DETAIL_BODY_LINES {
        model.dynamic_text(
            SYSTEM_DETAIL_BODY_SLOT0 + line_idx,
            cx,
            SYSTEM_DESC_RECT.y + 8.5 + line_idx as f32 * 3.0,
            SYSTEM_ROW_FONT * 0.78,
            MenuTextAlign::Center,
            MenuColor::rgba(0.88, 0.94, 1.0, 0.96),
        );
    }
}

/// The System bottom-panel text keyed by dynamic-text slot, for the focused row.
/// The in-place equivalent of the old baked bottom panel; empty slots clear stale
/// lines from a previously longer description.
pub fn system_detail_slot_text(
    sys_model: &SystemMenuModel,
    rows: &[SystemRow],
    focused: usize,
) -> Vec<(u32, String)> {
    let (label, description) = rows
        .get(focused)
        .map(|row| {
            (
                system_row_label(sys_model, *row),
                system_row_description(sys_model, *row),
            )
        })
        .unwrap_or_else(|| ("System".to_string(), "System options.".to_string()));
    let mut out = vec![(SYSTEM_DETAIL_LABEL_SLOT, label)];
    let mut lines = wrap_text(&description, SYSTEM_DESC_WRAP_COLS);
    lines.truncate(SYSTEM_DETAIL_BODY_LINES as usize);
    for i in 0..SYSTEM_DETAIL_BODY_LINES {
        let slot = SYSTEM_DETAIL_BODY_SLOT0 + i;
        let text = lines.get(i as usize).cloned().unwrap_or_default();
        out.push((slot, text));
    }
    out
}

fn placeholder_page(
    page: MenuPage,
    title: &str,
    body: &str,
) -> MenuPageModel<MenuPage, MenuPageAction> {
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
    // Placeholder pages keep the edge buttons; their focus highlight is applied in
    // place from the live cursor (`kaleidoscope_sync_focus_visuals`), so landing on
    // an edge button after a page turn (Fix 1) highlights it without a rebuild.
    add_edge_buttons(&mut model, page);
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
        let spec = items_spec(&owned, None);
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
        let spec = items_spec(&owned, Some(Item::Blink));
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
    fn item_cells_carry_a_sprite_icon_when_one_exists_else_fall_back_to_text() {
        // Items with authored art emit an `icon` on their grid control; items
        // without art carry `None` (the lib then renders the text label).
        let owned = OwnedItems::default();
        let page = build_items_page(&owned, None);
        // Item-grid controls are emitted in catalog slot order, so the icon list
        // lines up 1:1 with `Item::ALL`.
        let icons: Vec<Option<String>> = page
            .nodes
            .iter()
            .filter_map(|n| match n {
                ambition_menu::MenuNode::Control {
                    kind: MenuControlKind::Item,
                    icon,
                    ..
                } => Some(icon.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(icons.len(), Item::ALL.len(), "one control per catalog item");
        for (item, icon) in Item::ALL.into_iter().zip(icons.iter()) {
            match item.icon_path() {
                Some(path) => assert_eq!(
                    icon.as_deref(),
                    Some(path),
                    "{item:?} should carry its sprite icon"
                ),
                None => assert!(icon.is_none(), "{item:?} has no art → text fallback"),
            }
        }
        // Sanity: at least one of each (a real sprite + a real text fallback).
        assert!(icons.iter().any(|i| i.is_some()), "some items have icons");
        assert!(
            icons.iter().any(|i| i.is_none()),
            "some items fall back to text"
        );
    }

    #[test]
    fn items_page_has_one_detail_panel_not_per_cell_descriptions() {
        // Regression for the "24 overlapping descriptions" mush: NO grid cell may
        // carry the full item description as its detail text.
        let owned = OwnedItems::default();
        let page = build_items_page(&owned, None);
        for node in &page.nodes {
            if let ambition_menu::MenuNode::Control {
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
        // The detail panel is now cursor-INDEPENDENT page data: it reserves a fixed
        // set of EMPTY dynamic-text slots (filled in place from the live cursor),
        // so the page itself never bakes the description (a hover would otherwise
        // rebuild the face and drop a `Pointer<Click>` — the deferred Bug 2).
        let has_dynamic_slots = page
            .nodes
            .iter()
            .filter(|n| matches!(n, ambition_menu::MenuNode::DynamicText { .. }))
            .count();
        assert!(
            has_dynamic_slots >= ITEMS_DETAIL_BODY_LINES as usize,
            "items detail panel reserves dynamic-text slots for in-place fill"
        );
        // The in-place text for a focused item renders its description (this is what
        // `kaleidoscope_sync_detail_text` writes into the dynamic slots each move).
        let slot_text = items_detail_slot_text(&owned, None, MenuFocus::Item(Item::Blink.index()));
        let joined: String = slot_text
            .iter()
            .map(|(_, s)| s.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains(Item::Blink.description()),
            "focused item's description is supplied by the in-place detail slots: {joined:?}"
        );
    }

    #[test]
    fn system_page_top_level_shows_entry_list() {
        let settings = UserSettings::default();
        let focus = MenuFocus::System(0);
        // No entry open -> the top-level view is the SYSTEM entry list.
        let page = build_system_page(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
            focus,
            0,
            None,
        );
        let entries = page
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    ambition_menu::MenuNode::Control {
                        action: Some(MenuPageAction::OpenSystemEntry(_)),
                        ..
                    }
                )
            })
            .count();
        // Radio + Video + Audio + Controls + Gameplay + Language always drill in
        // (6 rows; Shaders is no longer a top-level entry — it rides under Video).
        // Reset All Settings is always present but is an Action (no drill). The 7
        // top-level rows (6 drill entries + the Reset All Settings action) now
        // overflow the SYSTEM_VISIBLE_ROWS (6) window (Fix 2); the first window still
        // shows all 6 drill entries, so exactly 6 drill rows are emitted.
        let expected_drill = 6;
        assert_eq!(
            entries, expected_drill,
            "one drill row per non-action entry"
        );
        // No raw settings toggles leak at the top level.
        let has_setting = page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::System(_)),
                    ..
                }
            )
        });
        assert!(!has_setting, "entry list does not show raw setting toggles");
        // Edge buttons are present so rotation still works.
        let has_edges = page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::ChangePage(_)),
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
        let focus = MenuFocus::System(0);
        // Drill into Video -> its curated options (from the SYSTEM IR) + a Back row.
        let page = build_system_page(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
            focus,
            0,
            Some(SystemMenuEntryId::Video),
        );
        let options: Vec<_> = page
            .nodes
            .iter()
            .filter_map(|n| match n {
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::System(o)),
                    ..
                } => Some(*o),
                _ => None,
            })
            .collect();
        // The cube's Video screen leads with the basic Video rows; the shader
        // subpage now rides under Video, so the full screen overflows the visible
        // window — the first window shows the 3 basic rows then the leading shader
        // sliders.
        assert_eq!(
            &options[..3],
            &[
                SettingsOptionId::DisplayMode,
                SettingsOptionId::CameraZoom,
                SettingsOptionId::CameraAspect,
            ]
        );
        // Shaders are reachable under Video: the FULL row list (pre-window) carries
        // every shader option as a Setting row.
        let sys_model = SystemMenuModel::build(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
        );
        let all_rows = system_rows(&sys_model, Some(SystemMenuEntryId::Video));
        for shader in [
            SettingsOptionId::ShaderStrength,
            SettingsOptionId::ShaderVignetteStrength,
        ] {
            assert!(
                all_rows.contains(&SystemRow::Setting(shader)),
                "{shader:?} is reachable under Video"
            );
        }

        // The FPS Overlay row reflects the ON state we set above. ShowFps now sits
        // past the first visible window (the full player-facing Video set leads the
        // screen), so verify the live label off the IR rather than the windowed page.
        let video_entry = sys_model.entry(SystemMenuEntryId::Video).unwrap();
        let crate::menu::ir::system::SystemMenuTarget::Settings(opts) = &video_entry.target else {
            panic!("video drills into a settings screen");
        };
        let fps = opts
            .iter()
            .find(|o| o.id == SettingsOptionId::ShowFps)
            .expect("ShowFps is on the Video screen");
        assert_eq!(fps.value_label, "ON", "FPS Overlay reflects the ON state");

        // A Back row drills out to the entry list. The Video screen now overflows
        // the visible window (24 rows), so Back is the LAST row in the full list
        // rather than always on the first window; assert it via the row list.
        assert_eq!(
            all_rows.last(),
            Some(&SystemRow::Back),
            "an open entry ends with a Back row"
        );
        // Scrolling to the end brings the Back row into the rendered window.
        let end_start = system_max_window_start(all_rows.len());
        let page_end = build_system_page(
            &settings,
            &RadioSnapshot::default(),
            &DevSnapshot::default(),
            MenuFocus::System(all_rows.len() - 1),
            end_start,
            Some(SystemMenuEntryId::Video),
        );
        let has_back = page_end.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::CloseSystemEntry),
                    ..
                }
            )
        });
        assert!(has_back, "scrolling to the end renders the Back row");
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
    fn map_and_quest_edge_buttons_are_focusable() {
        // Fix 1: placeholder pages (Map / Quest) build real, focusable L/R edge
        // buttons. The focus HIGHLIGHT is now applied in place from the live cursor
        // (`kaleidoscope_sync_focus_visuals`) rather than baked into the page data,
        // so the page only needs to emit the two clickable edge controls; landing on
        // one after a page turn highlights it without a rebuild.
        for page in [MenuPage::Map, MenuPage::Quest] {
            let model = placeholder_page(page, "T", "body");
            // Both edge buttons exist as Action controls with a ChangePage action.
            let edges: Vec<_> = model
                .nodes
                .iter()
                .filter(|n| {
                    matches!(
                        n,
                        ambition_menu::MenuNode::Control {
                            kind: MenuControlKind::Action,
                            action: Some(MenuPageAction::ChangePage(_)),
                            ..
                        }
                    )
                })
                .collect();
            assert_eq!(edges.len(), 2, "{page:?} has both L/R edge buttons");
            // The page data is cursor-independent: NO edge button is baked selected.
            let any_baked_selected = model.nodes.iter().any(|n| {
                matches!(
                    n,
                    ambition_menu::MenuNode::Control {
                        selected: true,
                        action: Some(MenuPageAction::ChangePage(_)),
                        ..
                    }
                )
            });
            assert!(
                !any_baked_selected,
                "{page:?} edge highlight is applied in place, not baked"
            );
        }
    }

    #[test]
    fn short_system_screens_show_every_row_without_an_indicator() {
        // Fix 3/4: a screen that fits shows all rows and adds NO scroll indicator.
        let rows: Vec<SystemRow> = (0..SYSTEM_VISIBLE_ROWS)
            .map(|i| SystemRow::Option(SystemOptionId::Radio(i)))
            .collect();
        let (window, indicator) = system_visible_window(&rows, 0);
        assert_eq!(window.len(), rows.len(), "all rows visible when they fit");
        assert!(indicator.is_none(), "no indicator for a short screen");
        // Absolute indices are identity for a non-windowed list.
        for (slot, (abs, _)) in window.iter().enumerate() {
            assert_eq!(slot, *abs);
        }
    }

    #[test]
    fn long_system_screens_window_the_list_and_follow_the_cursor() {
        // Fix 3/4: a Radio-sized screen (26 rows) windows to SYSTEM_VISIBLE_ROWS and
        // the window follows the cursor, mapping windowed slots back to absolute rows.
        let total = 26usize;
        let rows: Vec<SystemRow> = (0..total)
            .map(|i| SystemRow::Option(SystemOptionId::Radio(i)))
            .collect();

        // Cursor at the top: window starts at 0, indicator reads "1/26".
        let (window, indicator) = system_visible_window(&rows, 0);
        assert_eq!(window.len(), SYSTEM_VISIBLE_ROWS, "list is windowed");
        assert_eq!(window.first().unwrap().0, 0);
        assert_eq!(indicator.as_deref(), Some("1/26"));

        // Cursor mid-list: the focused absolute row stays inside the rendered window.
        let focused = 13;
        let (window, indicator) = system_visible_window(&rows, focused);
        assert_eq!(window.len(), SYSTEM_VISIBLE_ROWS);
        assert!(
            window.iter().any(|(abs, _)| *abs == focused),
            "the focused row scrolls into the visible window"
        );
        assert_eq!(
            indicator.as_deref(),
            Some("14/26"),
            "1-based n/total indicator"
        );

        // Cursor at the bottom: the window clamps to the list end (no overflow).
        let (window, _) = system_visible_window(&rows, total - 1);
        assert_eq!(window.len(), SYSTEM_VISIBLE_ROWS);
        assert_eq!(window.last().unwrap().0, total - 1, "last row reachable");
    }

    #[test]
    fn long_system_page_renders_only_a_window_of_clickable_rows() {
        // The built System page for a long Radio screen renders exactly
        // SYSTEM_VISIBLE_ROWS option controls (all clickable), not all 26.
        let settings = UserSettings::default();
        let radio = RadioSnapshot {
            stations: (0..26).map(|i| (i, format!("Station {i}"))).collect(),
            active: Some(0),
        };
        let focus = MenuFocus::System(13);
        let sys_model = SystemMenuModel::build(&settings, &radio, &DevSnapshot::default());
        let rows = system_rows(&sys_model, Some(SystemMenuEntryId::Radio));
        // Cursor-derived window (no override) keeps the focused station in view.
        let window_start = system_window_start(&rows, focus);
        let page = build_system_page(
            &settings,
            &radio,
            &DevSnapshot::default(),
            focus,
            window_start,
            Some(SystemMenuEntryId::Radio),
        );
        let option_rows = page
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    ambition_menu::MenuNode::Control {
                        action: Some(MenuPageAction::SystemOption(_)),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(
            option_rows, SYSTEM_VISIBLE_ROWS,
            "a long Radio screen renders only the visible window of station rows"
        );
        // The window includes the focused station (index 13). The highlight itself
        // is applied IN PLACE from the live cursor (not baked as `selected`), so the
        // page only needs to RENDER the focused row inside the window.
        let has_focused = page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    action: Some(MenuPageAction::SystemOption(SystemOptionId::Radio(13))),
                    ..
                }
            )
        });
        assert!(
            has_focused,
            "the focused station scrolls into the visible window"
        );
    }

    #[test]
    fn scrollbar_thumb_geometry_reflects_window_and_total() {
        // Fix 1: thumb size = visible/total; start = window fraction of travel.
        // 26-row list, 6 visible: size = 6/26 ≈ 0.2308.
        let total = 26usize;
        let (start_top, size) = system_scrollbar_thumb(0, total);
        assert!(
            (size - SYSTEM_VISIBLE_ROWS as f32 / total as f32).abs() < 1e-4,
            "thumb size = visible/total: {size}"
        );
        assert!(
            size < 1.0,
            "an overflowing list scrolls (thumb < full track)"
        );
        assert!(
            (start_top - 0.0).abs() < 1e-4,
            "top window → thumb at the top"
        );

        // Bottom window → thumb at the bottom (start == 1.0).
        let max = system_max_window_start(total);
        let (start_bottom, _) = system_scrollbar_thumb(max, total);
        assert!(
            (start_bottom - 1.0).abs() < 1e-4,
            "bottom window → thumb start 1.0: {start_bottom}"
        );

        // A mid window lands between.
        let (start_mid, _) = system_scrollbar_thumb(max / 2, total);
        assert!(
            start_mid > 0.0 && start_mid < 1.0,
            "mid window → thumb mid-track: {start_mid}"
        );
    }

    #[test]
    fn long_system_page_emits_one_scrollbar_node_with_thumb() {
        // Fix 1: a long Radio screen emits exactly one Scrollbar control carrying the
        // thumb geometry (the lib draws the track + thumb from it).
        let settings = UserSettings::default();
        let radio = RadioSnapshot {
            stations: (0..26).map(|i| (i, format!("Station {i}"))).collect(),
            active: Some(0),
        };
        let page = build_system_page(
            &settings,
            &radio,
            &DevSnapshot::default(),
            MenuFocus::System(0),
            0,
            Some(SystemMenuEntryId::Radio),
        );
        let thumbs: Vec<_> = page
            .nodes
            .iter()
            .filter_map(|n| match n {
                ambition_menu::MenuNode::Control {
                    kind: MenuControlKind::Scrollbar,
                    thumb: Some(t),
                    ..
                } => Some(*t),
                _ => None,
            })
            .collect();
        assert_eq!(thumbs.len(), 1, "exactly one scrollbar node with a thumb");
        assert!(thumbs[0].size < 1.0, "thumb shows the list scrolls");
        assert!(
            (thumbs[0].start - 0.0).abs() < 1e-4,
            "top window → thumb top"
        );

        // A short screen emits NO scrollbar node. A 3-station Radio screen (3 rows +
        // Back = 4) fits inside SYSTEM_VISIBLE_ROWS, so no scrollbar is drawn. (The
        // TOP-LEVEL entry list has 7 rows and now overflows the 6-row window — Fix 2 —
        // so it is no longer a valid "fits" case; drill into a short screen instead.)
        let short_radio = RadioSnapshot {
            stations: (0..3).map(|i| (i, format!("Station {i}"))).collect(),
            active: Some(0),
        };
        let short_page = build_system_page(
            &settings,
            &short_radio,
            &DevSnapshot::default(),
            MenuFocus::System(0),
            0,
            Some(SystemMenuEntryId::Radio),
        );
        let any_scrollbar = short_page.nodes.iter().any(|n| {
            matches!(
                n,
                ambition_menu::MenuNode::Control {
                    kind: MenuControlKind::Scrollbar,
                    ..
                }
            )
        });
        assert!(!any_scrollbar, "a fitting list draws no scrollbar");
    }

    #[test]
    fn viewer_left_button_turns_to_the_right_neighbor() {
        // Pressing LEFT rotates the cube left, bringing the +1 ring neighbour to
        // front (matches the demo's page_on_viewer_left = index + 1).
        assert_eq!(MenuPage::Items.on_viewer_left(), MenuPage::Map);
        assert_eq!(MenuPage::Items.on_viewer_right(), MenuPage::System);
        let owned = OwnedItems::default();
        let page = build_items_page(&owned, None);
        let left = page.nodes.iter().find_map(|n| match n {
            ambition_menu::MenuNode::Control {
                action: Some(MenuPageAction::ChangePage(p)),
                rect,
                ..
            } if rect.x < 10.0 => Some(*p),
            _ => None,
        });
        assert_eq!(
            left,
            Some(MenuPage::Map),
            "left edge button turns to viewer-left page"
        );
    }
}
