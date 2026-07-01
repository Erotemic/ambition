//! The data seam between Ambition's live 24-item inventory and the reusable
//! `ambition_menu` 3D-cube OoT pause menu (#31).
//!
//! The game owns the item state (`ambition_gameplay_core::items`); this module builds the cube's
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

use ambition_gameplay_core::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use ambition_gameplay_core::persistence::settings::{
    DevSnapshot, RadioSnapshot, SettingsOption, SettingsOptionId, SettingsOptionKind,
    SystemMenuAction, SystemMenuEntryId, SystemMenuModel, SystemMenuTarget, SystemOptionId,
    UserSettings, VisualQualityProfile,
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
/// (`crate::menu::kaleidoscope_app::kaleidoscope_sync_detail_text`) rewrites from
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
/// demo's `MockAction`-as-selection, and the unit of [`crate::menu::kaleidoscope_app`]'s
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
            MenuFocus::Item(idx) => idx.min(ambition_gameplay_core::items::ITEM_COUNT - 1),
            _ => 0,
        }
    }
}

/// Actions the cube emits back to the game (the host consumes these — the cube
/// never mutates item state itself, matching the `crate::menu::effects` seam).
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
    /// Fix 2: a DIRECTIONAL step on a value-style System SETTINGS row (Slider/Cycle),
    /// for touch/mouse users. The `i32` is the step direction (`-1` decrease / `+1`
    /// increase) the host applies via `apply_settings_option(option, dir, …)` — the
    /// same path the keyboard's LEFT/RIGHT already drives. Emitted by the ◀ / ▶ click
    /// zones flanking the row (plain `System` confirm/select still steps +1).
    SystemStep(SettingsOptionId, i32),
    /// A non-settings System screen option (radio station / locale / dev toggle).
    /// Applied host-side against the matching live resource.
    SystemOption(SystemOptionId),
    /// An immediate, screen-less System action (Reset Sandbox).
    SystemAction(SystemMenuAction),
    /// Apply the pending visual-quality profile after the confirmation row is
    /// selected. The chosen profile lives in app-local menu state until then.
    ConfirmVisualQuality,
    /// Cancel a pending visual-quality profile change without dirtying settings.
    CancelVisualQuality,
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
/// (`crate::menu::kaleidoscope_app::kaleidoscope_sync_detail_text`) fills them
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
    build_inventory_pages_with_quality_prompt(
        owned,
        equipped,
        focus,
        settings,
        radio,
        dev,
        system_window_start,
        open_entry,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_inventory_pages_with_quality_prompt(
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
    pending_quality: Option<VisualQualityProfile>,
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
        build_system_page_with_quality_prompt(
            settings,
            radio,
            dev,
            focus,
            system_window_start,
            open_entry,
            pending_quality,
        ),
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
    /// Confirmation row for an in-flight visual quality profile change.
    QualityApply(VisualQualityProfile),
    /// Cancellation row for an in-flight visual quality profile change.
    QualityCancel,
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
    system_rows_with_quality_prompt(model, open_entry, None)
}

pub fn system_rows_with_quality_prompt(
    model: &SystemMenuModel,
    open_entry: Option<SystemMenuEntryId>,
    pending_quality: Option<VisualQualityProfile>,
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
                    let mut rows = Vec::with_capacity(options.len() + 3);
                    for option in options {
                        rows.push(SystemRow::Setting(option.id));
                        if entry.id == SystemMenuEntryId::Video
                            && option.id == SettingsOptionId::VisualQuality
                        {
                            if let Some(profile) = pending_quality {
                                rows.push(SystemRow::QualityApply(profile));
                                rows.push(SystemRow::QualityCancel);
                            }
                        }
                    }
                    rows
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

/// Build the System menu model that should be displayed while a visual-quality
/// change is pending. The persisted settings remain untouched until the user
/// chooses Apply; this temporary clone exists only so the Quality Profile row
/// itself keeps cycling visibly while the confirmation rows are shown.
pub fn system_menu_model_with_pending_quality(
    settings: &UserSettings,
    radio: &RadioSnapshot,
    dev: &DevSnapshot,
    pending_quality: Option<VisualQualityProfile>,
) -> SystemMenuModel {
    if let Some(profile) = pending_quality {
        let mut display_settings = settings.clone();
        display_settings.video.quality.profile = profile;
        SystemMenuModel::build(&display_settings, radio, dev)
    } else {
        SystemMenuModel::build(settings, radio, dev)
    }
}

/// Look up a settings option's live IR entry by id from the live model.
fn setting_entry(model: &SystemMenuModel, id: SettingsOptionId) -> Option<SettingsOption> {
    model.entries.iter().find_map(|e| match &e.target {
        SystemMenuTarget::Settings(options) => options.iter().find(|o| o.id == id).cloned(),
        _ => None,
    })
}

/// Fix 2: whether a settings row is a value-style control (Slider/Cycle), i.e. one
/// that the keyboard steps with LEFT/RIGHT and so should get flanking ◀ / ▶ touch
/// click zones. Toggles and Actions are select-only and get no step zones.
fn is_value_setting_row(model: &SystemMenuModel, id: SettingsOptionId) -> bool {
    setting_entry(model, id)
        .map(|o| {
            matches!(
                o.kind,
                SettingsOptionKind::Cycle { .. } | SettingsOptionKind::Slider { .. }
            )
        })
        .unwrap_or(false)
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
        SystemRow::QualityApply(profile) => format!("Apply quality: {}", profile.label()),
        SystemRow::QualityCancel => "Cancel quality change".to_string(),
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
        SystemRow::QualityApply(profile) => format!(
            "Confirm the {} visual-quality profile. Textures and room visuals reload immediately after this is applied.",
            profile.label()
        ),
        SystemRow::QualityCancel => {
            "Discard the pending visual-quality profile and keep the current setting.".to_string()
        }
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
        SystemRow::QualityApply(_) => Some(MenuPageAction::ConfirmVisualQuality),
        SystemRow::QualityCancel => Some(MenuPageAction::CancelVisualQuality),
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

/// Map a scrollbar's neutral `0..=1` drag fraction onto a System-window START row
/// for a list of `total` rows. `None` when the list fits (no scrolling). Single
/// source of truth shared by BOTH backends' scroll-drag appliers (the cube's
/// `kaleidoscope_apply_scroll_drag` + the grid's `grid_menu_apply_scroll_drag`),
/// which previously each open-coded the same `(fraction * max).round().min(max)`.
pub fn scroll_fraction_to_window_start(total: usize, fraction: f32) -> Option<usize> {
    if total <= SYSTEM_VISIBLE_ROWS {
        return None;
    }
    let max = system_max_window_start(total);
    Some(((fraction.clamp(0.0, 1.0) * max as f32).round() as usize).min(max))
}

/// The cursor-derived scroll-window START (the window that keeps the focused row
/// visible). The default when no explicit scroll override is in effect.
pub fn system_window_start(rows: &[SystemRow], focus: MenuFocus) -> usize {
    let focused = system_focus_index(focus, rows.len());
    if rows.len() <= SYSTEM_VISIBLE_ROWS {
        return 0;
    }
    ambition_ui_nav::visible_window_start(focused, rows.len(), SYSTEM_VISIBLE_ROWS)
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
    build_system_page_with_quality_prompt(
        settings,
        radio,
        dev,
        focus,
        window_start,
        open_entry,
        None,
    )
}

pub fn build_system_page_with_quality_prompt(
    settings: &UserSettings,
    radio: &RadioSnapshot,
    dev: &DevSnapshot,
    focus: MenuFocus,
    // The EFFECTIVE scroll-window start (cursor-derived OR a drag/wheel override —
    // see [`system_effective_window_start`]). Drives which rows render + the thumb.
    window_start: usize,
    open_entry: Option<SystemMenuEntryId>,
    pending_quality: Option<VisualQualityProfile>,
) -> MenuPageModel<MenuPage, MenuPageAction> {
    let sys_model = system_menu_model_with_pending_quality(settings, radio, dev, pending_quality);
    let mut model = MenuPageModel::new(
        MenuPage::System,
        "SYSTEM",
        MenuColor::rgba(0.03, 0.04, 0.10, 0.96),
    );
    let rows = system_rows_with_quality_prompt(&sys_model, open_entry, pending_quality);
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

        // Fix 2: a value-style row (Slider/Cycle) gets two flanking pickable click
        // zones so TOUCH/MOUSE users can step both ways (the keyboard already does
        // via LEFT/RIGHT). The ◀ zone dispatches a -1 step, the ▶ zone a +1 step,
        // through the same `apply_settings_option(option, dir, …)` IR path. They sit
        // INSIDE the row's left/right edges (so they overlay the row, not the
        // neighbours); spawned AFTER the row, they win the perspective pick over it.
        if let SystemRow::Setting(option) = *row {
            if is_value_setting_row(&sys_model, option) {
                let zone_w = (SYSTEM_LIST_RECT.w * 0.16).min(11.0);
                model.control(
                    MenuRect {
                        x: SYSTEM_LIST_RECT.x,
                        y,
                        w: zone_w,
                        h: row_h,
                    },
                    MenuControlKind::OptionChoice,
                    "\u{25C0}", // ◀
                    None,
                    false,
                    false,
                    Some(MenuPageAction::SystemStep(option, -1)),
                );
                model.control(
                    MenuRect {
                        x: SYSTEM_LIST_RECT.x + SYSTEM_LIST_RECT.w - zone_w,
                        y,
                        w: zone_w,
                        h: row_h,
                    },
                    MenuControlKind::OptionChoice,
                    "\u{25B6}", // ▶
                    None,
                    false,
                    false,
                    Some(MenuPageAction::SystemStep(option, 1)),
                );
            }
        }
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
/// `crate::menu::kaleidoscope_app::kaleidoscope_sync_detail_text`
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
mod tests;
