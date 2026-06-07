//! Flat, tabbed `bevy_ui` renderer of a [`MenuPageModel`].
//!
//! This is the SECOND presentation of the backend-agnostic menu model (the first
//! being the bevy_lunex 3D cube in [`crate::render::kaleidoscope`]). Having two
//! real renderers of the same model empirically validates the engine/content
//! seam: this module draws the *same* controls, labels, actions and scroll window
//! the cube draws — just flat, with `bevy_ui` flex/grid layout instead of a 3D
//! projection.
//!
//! # What this module does (and does NOT do)
//!
//! It owns the **model → entity** mapping only. Given a description of the menu
//! ([`BevyUiMenuView`]: the tab set, the active tab index, the active page's
//! [`MenuPageModel`], and the focused control), [`spawn_bevy_ui_menu`] spawns a
//! `bevy_ui` tree:
//!
//! * a **tab bar** row at the top — one [`BevyUiMenuTab`] button per tab, the
//!   active one flagged + visually highlighted;
//! * the active page's **body** — panels/backgrounds as [`Node`]s, text/labels as
//!   [`Text`], interactive controls as focusable rows tagged with their
//!   [`AmbitionMenuControl`] (carrying `kind` + `action` + [`MenuFocusKey`]), the
//!   item grid laid out by the model's authored rects, and a **scrollbar** (track
//!   + thumb) for a [`MenuControlKind::Scrollbar`] node whose thumb scrolls.
//!
//! The host drives navigation + dispatch in a later phase. The seam it relies on
//! is the same as the cube's: every interactive control entity carries an
//! [`AmbitionMenuControl<Action>`] (so a picking/nav system can map an entity →
//! its `Action`) plus a [`MenuVisualState`] with `focused`/`selected` set, and the
//! focused control is additionally flagged with [`BevyUiMenuFocused`]. This module
//! puts **no game dispatch** in the engine — it only spawns tagged entities.
//!
//! Generic over `PageId` / `Action` exactly like the cube renderer; no
//! Ambition-specific types appear here.

use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;

use crate::{
    AmbitionMenuControl, AmbitionMenuRoot, MenuColor, MenuControlKind, MenuNode, MenuPageModel,
    MenuRect, MenuTextAlign, MenuVisualState, ScrollThumb,
};

/// Root marker for a spawned flat `bevy_ui` menu tree.
///
/// Despawn this entity to tear the menu down; respawn via [`spawn_bevy_ui_menu`]
/// when the view changes.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BevyUiMenuRoot;

/// Marker for the centered, fixed-size panel (the menu "window") that holds the
/// tab bar + body, sitting in the middle of the full-screen scrim root.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BevyUiMenuPanel;

/// Marker for the tab-bar row container.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BevyUiMenuTabBar;

/// Marker for the active page's body container.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BevyUiMenuBody;

/// One tab button in the tab bar.
///
/// `index` is the tab's position in the ordered tab set; `active` mirrors the
/// view's active tab so a host picking system can map a clicked tab → its index
/// without re-deriving it. The active tab is additionally highlighted visually.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct BevyUiMenuTab {
    pub index: usize,
    pub active: bool,
    /// Fix 4: keyboard focus is currently on THIS tab (the tab bar has focus and the
    /// cursor is on it). Drawn with a focus ring distinct from the active highlight.
    pub focused: bool,
}

/// Flag on the single focused control entity (the cursor).
///
/// Mirrors the cube's selection intent flat: the focused control also carries
/// `MenuVisualState { focused: true, .. }`; this marker lets the host find the
/// cursor entity directly.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BevyUiMenuFocused;

/// Marker for the scrollbar track entity (a `MenuControlKind::Scrollbar` node).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BevyUiMenuScrollbar {
    /// The thumb geometry the host computed (track fractions in `0..=1`).
    pub thumb: ScrollThumb,
}

/// Marker for the scrollbar thumb child (the grab handle / position indicator).
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BevyUiMenuScrollbarThumb;

/// A single tab descriptor: its stable page id + the label drawn on the button.
#[derive(Clone, Debug, PartialEq)]
pub struct BevyUiMenuTabSpec<PageId> {
    pub id: PageId,
    pub label: String,
}

impl<PageId> BevyUiMenuTabSpec<PageId> {
    pub fn new(id: PageId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
        }
    }
}

/// Everything the renderer needs to draw one frame of the flat menu.
///
/// This is the renderer's single input. The host builds it from its own state:
/// the ordered tab set, which tab is active, the active page's already-built
/// [`MenuPageModel`], and the focused control key (the cursor). The renderer is a
/// pure function of this view — it spawns the entity tree and nothing else.
pub struct BevyUiMenuView<'a, PageId, Action> {
    /// The ordered tab set (page id + label), drawn left→right in the tab bar.
    pub tabs: &'a [BevyUiMenuTabSpec<PageId>],
    /// Index into `tabs` of the active tab (clamped on use).
    pub active_tab: usize,
    /// The active page's model — the body the renderer draws.
    pub page: &'a MenuPageModel<PageId, Action>,
    /// The focused control's focus key (cursor), if any control is focused.
    ///
    /// A control whose derived [`MenuFocusKey`](crate::MenuFocusKey) equals this is
    /// drawn focused + flagged with [`BevyUiMenuFocused`]. `None` focuses nothing.
    pub focused: Option<crate::MenuFocusKey>,
    /// Fix 4: when keyboard focus is on the TAB BAR (not the body), the index of the
    /// tab the cursor is on. Drawn with a distinct focus ring so the user can see which
    /// tab UP/LEFT/RIGHT will act on. `None` = focus is in the body (the normal case);
    /// the active tab is still highlighted via [`BevyUiMenuTab::active`].
    pub focused_tab: Option<usize>,
}

/// Convert a renderer-neutral [`MenuColor`] into a Bevy [`Color`].
fn to_color(c: MenuColor) -> Color {
    Color::srgba(c.r, c.g, c.b, c.a)
}

/// Bevy text justification for a [`MenuTextAlign`].
fn to_justify(align: MenuTextAlign) -> Justify {
    match align {
        MenuTextAlign::Left => Justify::Left,
        MenuTextAlign::Center => Justify::Center,
        MenuTextAlign::Right => Justify::Right,
    }
}

/// Derive a control's stable [`MenuFocusKey`] from its rect, the SAME way the cube
/// renderer does (see `render::kaleidoscope`). Keeping this identical means a
/// `focused` key computed against one renderer addresses the same control in the
/// other — the cross-backend nav contract.
fn focus_key_for(rect: MenuRect) -> crate::MenuFocusKey {
    crate::MenuFocusKey {
        row: (rect.y * 10.0).round() as i32,
        col: (rect.x * 10.0).round() as i32,
        order: (rect.y * 100.0 + rect.x).round() as i32,
    }
}

/// Absolutely-positioned [`Node`] from a normalized page rect (percent space).
fn node_from_rect(rect: MenuRect) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(rect.x),
        top: Val::Percent(rect.y),
        width: Val::Percent(rect.w),
        height: Val::Percent(rect.h),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Background tint for a control, mirroring the cube's intent flat: focused/
/// selected reads bright-gold, important reads accented, disabled reads dim, the
/// scrollbar track reads dim, plain controls read a neutral blue.
fn control_bg(kind: MenuControlKind, focused: bool, selected: bool, important: bool) -> Color {
    if matches!(kind, MenuControlKind::Scrollbar) {
        return Color::srgba(0.10, 0.11, 0.16, 0.92);
    }
    // Fix 2: HIGHLIGHTED (cursor/hover) and SELECTED (equipped/active setting) must
    // read DISTINCT, mirroring the cube's `control_color(kind, selected, important)`
    // intent: selected is a warm accent, highlighted is the bright cursor color, and
    // the two together are the brightest. The cube distinguishes selected by color;
    // the flat backend additionally distinguishes the cursor (the cube does that with
    // a separate focus-ring system, which the flat renderer folds into the bg here).
    match (focused, selected) {
        // Highlighted AND selected → the brightest warm gold (the cursor sits on the
        // active item/setting).
        (true, true) => Color::srgba(0.99, 0.82, 0.34, 0.98),
        // Highlighted only (cursor/hover) → warm gold cursor color.
        (true, false) => Color::srgba(0.85, 0.70, 0.20, 0.96),
        // Selected only (equipped item / active setting, cursor elsewhere) → a muted
        // teal/blue accent, clearly different from the gold cursor.
        (false, true) => Color::srgba(0.16, 0.42, 0.46, 0.96),
        // Plain.
        (false, false) => {
            if important {
                Color::srgba(0.20, 0.30, 0.50, 0.96)
            } else {
                Color::srgba(0.09, 0.12, 0.26, 0.96)
            }
        }
    }
}

/// Spawn the flat tabbed menu under a fresh [`BevyUiMenuRoot`] and return its
/// entity. The host should despawn the previous root before respawning on change.
///
/// Layout: a full-screen absolute root acting as a centered scrim (so clicks
/// outside the panel land on the scrim, not the world), with a centered,
/// fixed-size PANEL holding a tab-bar row at the top then the page body filling
/// the rest. The panel is roughly where/size the kaleidoscope cube renders — a
/// window in the middle of the screen, NOT a full-screen layout. The body draws
/// the page's nodes by absolute percent rect (percent of the PANEL) so it matches
/// the model's authored layout, while the tab bar uses flex so tabs share the
/// panel width evenly. A high [`GlobalZIndex`] keeps the menu on top so its
/// `bevy_ui` buttons receive `Interaction`/picking before anything underneath.
pub fn spawn_bevy_ui_menu<PageId, Action>(
    commands: &mut Commands,
    view: &BevyUiMenuView<PageId, Action>,
) -> Entity
where
    PageId: Clone + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    spawn_bevy_ui_menu_with_assets(commands, view, None)
}

/// Like [`spawn_bevy_ui_menu`], but with an optional [`AssetServer`] so item cells
/// can render their ICON image (Fix 3). When `assets` is `None` (e.g. a headless
/// test on `MinimalPlugins` with no `AssetPlugin`), icons fall back to the label —
/// the cube renderer is unaffected. The host (which always has an `AssetServer`)
/// calls this so the Grid's Items tab shows the same sprite icons the cube does.
pub fn spawn_bevy_ui_menu_with_assets<PageId, Action>(
    commands: &mut Commands,
    view: &BevyUiMenuView<PageId, Action>,
    assets: Option<&AssetServer>,
) -> Entity
where
    PageId: Clone + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    let active_tab = view.active_tab.min(view.tabs.len().saturating_sub(1));
    // Full-screen scrim: centers the panel and dims/blocks the world behind it.
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Fix 1: TRANSLUCENT scrim. The despawn bug that blanked the body (which
            // forced an opaque workaround) is fixed, so the menu can dim-and-show the
            // world behind it again. A 0.55 alpha black darkens the gameplay enough to
            // read the panel while keeping the scene visible.
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            // On top of the gameplay HUD so the menu's buttons get the pointer.
            GlobalZIndex(1000),
            BevyUiMenuRoot,
            AmbitionMenuRoot,
            Name::new("bevy_ui menu root"),
        ))
        .id();

    commands.entity(root).with_children(|root| {
        // --- Centered fixed-size panel (the window) --------------------------
        root.spawn((
            Node {
                width: Val::Percent(64.0),
                height: Val::Percent(74.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // Fix 1: TRANSLUCENT dark window. A near-opaque (0.90) dark panel keeps the
            // body content crisply readable while letting a hint of the dimmed world
            // bleed through the window — the "translucent dark window" look. (The model's
            // own `page.background` is near-transparent for the cube's opaque 3D face;
            // the flat renderer supplies this panel so content has a backing.)
            BackgroundColor(Color::srgba(0.07, 0.09, 0.14, 0.90)),
            BevyUiMenuPanel,
            Name::new("menu panel"),
        ))
        .with_children(|panel| {
            // --- Tab bar ---------------------------------------------------------
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(44.0),
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    BevyUiMenuTabBar,
                    Name::new("menu tab bar"),
                ))
                .with_children(|bar| {
                    for (i, tab) in view.tabs.iter().enumerate() {
                        let active = i == active_tab;
                        let tab_focused = view.focused_tab == Some(i);
                        let bg = if active {
                            Color::srgba(0.85, 0.70, 0.20, 0.98)
                        } else {
                            Color::srgba(0.10, 0.13, 0.22, 0.94)
                        };
                        let label_color = if active {
                            Color::BLACK
                        } else {
                            Color::srgba(0.85, 0.90, 0.98, 0.98)
                        };
                        // Fix 4: a tab the keyboard cursor sits on gets a bright focus
                        // ring (a border) so the user sees which tab UP/LEFT/RIGHT acts
                        // on, distinct from the active tab's filled highlight.
                        let (border, border_color) = if tab_focused {
                            (
                                UiRect::all(Val::Px(3.0)),
                                Color::srgba(0.99, 0.82, 0.34, 1.0),
                            )
                        } else {
                            (UiRect::ZERO, Color::NONE)
                        };
                        bar.spawn((
                            Button,
                            Node {
                                flex_grow: 1.0,
                                height: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border,
                                ..default()
                            },
                            BackgroundColor(bg),
                            BorderColor::all(border_color),
                            BevyUiMenuTab {
                                index: i,
                                active,
                                focused: tab_focused,
                            },
                            Name::new(format!("tab[{i}]")),
                        ))
                        .with_children(|btn| {
                            btn.spawn((Text::new(tab.label.clone()), TextColor(label_color)));
                        });
                    }
                });

            // --- Active page body -----------------------------------------------
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    // NOTE: deliberately NOT tagged `AmbitionMenuPage`. That marker
                    // is the CUBE's face marker, and the cube's `rebuild_cube_faces`
                    // system despawns every `AmbitionMenuPage` entity whenever the
                    // shared `ActiveMenuPages` changes — which was despawning THIS
                    // flat body (and all its content children) out from under us,
                    // leaving an empty panel that only flashed content on respawn.
                    // The flat renderer uses its own `BevyUiMenuBody` marker only.
                    BevyUiMenuBody,
                    Name::new("menu body"),
                ))
                .with_children(|body| {
                    for node in &view.page.nodes {
                        spawn_node(body, node, view.focused, assets);
                    }
                });
        });
    });

    root
}

// Draw-order layers mirroring the cube's depth bands. The flat renderer uses
// bevy_ui sibling order otherwise, which paints a later background Panel ON TOP of
// earlier text/controls (the model relies on the cube's depth field to sort). A
// per-node `ZIndex` restores back-to-front order: panels behind, controls above,
// text/labels on top.
const LAYER_CONTROL: i32 = 10;
const LAYER_TEXT: i32 = 20;

/// Background panels sort by size, like the cube's DEPTH_BACKGROUND / LARGE_PANEL /
/// CARD bands: a near-full-page panel is the furthest back, a small card nearer.
fn panel_layer(rect: &MenuRect) -> i32 {
    if rect.w > 98.0 && rect.h > 98.0 {
        0
    } else if rect.w > 40.0 || rect.h > 35.0 {
        1
    } else {
        2
    }
}

/// Spawn one [`MenuNode`] into the body container.
fn spawn_node<Action>(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    node: &MenuNode<Action>,
    focused: Option<crate::MenuFocusKey>,
    assets: Option<&AssetServer>,
) where
    Action: Clone + Send + Sync + 'static,
{
    match node {
        MenuNode::Panel { rect, color, .. } => {
            body.spawn((
                node_from_rect(*rect),
                BackgroundColor(to_color(*color)),
                ZIndex(panel_layer(rect)),
                Name::new("panel"),
            ));
        }
        MenuNode::Text {
            x,
            y,
            size,
            text,
            align,
            color,
        } => {
            body.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(*x),
                    top: Val::Percent(*y),
                    ..default()
                },
                Text::new(text.clone()),
                TextColor(to_color(*color)),
                TextFont {
                    font_size: *size,
                    ..default()
                },
                TextLayout::new_with_justify(to_justify(*align)),
                ZIndex(LAYER_TEXT),
                Name::new("text"),
            ));
        }
        MenuNode::DynamicText {
            slot,
            x,
            y,
            size,
            align,
            color,
        } => {
            // Spawned empty; the host fills it in place by `slot`, exactly like the
            // cube renderer (cursor-dependent text needs no body rebuild).
            body.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(*x),
                    top: Val::Percent(*y),
                    ..default()
                },
                Text::new(String::new()),
                TextColor(to_color(*color)),
                TextFont {
                    font_size: *size,
                    ..default()
                },
                TextLayout::new_with_justify(to_justify(*align)),
                crate::MenuDynamicText { slot: *slot },
                crate::MenuDynamicTextContent(String::new()),
                ZIndex(LAYER_TEXT),
                Name::new("dynamic text"),
            ));
        }
        MenuNode::Control {
            rect,
            kind,
            label,
            detail: _,
            icon,
            selected,
            important,
            action,
            thumb,
        } => {
            spawn_control(
                body,
                *rect,
                *kind,
                label,
                icon.as_deref(),
                *selected,
                *important,
                action,
                *thumb,
                focused,
                assets,
            );
        }
    }
}

/// Spawn one interactive control. Tagging mirrors the cube renderer so the host's
/// picking/nav can map entity → action/focus identically across backends.
#[allow(clippy::too_many_arguments)]
fn spawn_control<Action>(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: &Option<Action>,
    thumb: Option<ScrollThumb>,
    focused_key: Option<crate::MenuFocusKey>,
    assets: Option<&AssetServer>,
) where
    Action: Clone + Send + Sync + 'static,
{
    let is_scrollbar = matches!(kind, MenuControlKind::Scrollbar);
    let focus = focus_key_for(rect);
    let focused = focused_key == Some(focus);
    let disabled = action.is_none() && !is_scrollbar;
    let bg = if disabled {
        to_color(MenuColor::DISABLED)
    } else {
        control_bg(kind, focused, selected, important)
    };
    // Black text only on the bright gold highlight; the muted teal selected-only bg
    // is dark, so it keeps light text (Fix 2: selected ≠ highlighted, incl. text).
    let label_color = if focused {
        Color::BLACK
    } else {
        Color::srgba(0.90, 0.94, 1.0, 0.98)
    };

    let mut control = body.spawn((
        Button,
        node_from_rect(rect),
        BackgroundColor(bg),
        ZIndex(LAYER_CONTROL),
        AmbitionMenuControl {
            kind,
            action: action.clone(),
            focus,
        },
        MenuVisualState {
            focused: focused || selected,
            selected,
            disabled,
            ..default()
        },
        Name::new(if is_scrollbar { "scrollbar" } else { "control" }),
    ));

    // Fix 3: an item cell with an icon renders the sprite ICON (an `ImageNode`)
    // instead of a bare label, matching the cube's `spawn_icon`. The icon is tinted
    // by the cell's state the same way: dim when disabled (un-owned), warm when
    // selected, white otherwise. Falls back to the label when there is no icon or no
    // `AssetServer` (headless tests). The detail/name still lives in the detail panel.
    let icon_handle = icon
        .zip(assets)
        .map(|(path, server)| server.load::<Image>(path.to_string()));
    if let Some(handle) = icon_handle {
        let tint = if disabled {
            Color::srgba(0.55, 0.58, 0.66, 0.55)
        } else if focused || selected {
            Color::srgb(1.0, 0.95, 0.78)
        } else {
            Color::WHITE
        };
        control.with_children(|c| {
            c.spawn((
                ImageNode::new(handle).with_color(tint),
                Node {
                    width: Val::Percent(78.0),
                    height: Val::Percent(78.0),
                    ..default()
                },
                Name::new("item icon"),
            ));
        });
    } else if !label.is_empty() {
        control.with_children(|c| {
            c.spawn((Text::new(label.to_string()), TextColor(label_color)));
        });
    }

    if focused {
        control.insert(BevyUiMenuFocused);
    }

    if is_scrollbar {
        let thumb = thumb.unwrap_or(ScrollThumb {
            start: 0.0,
            size: 1.0,
        });
        control.insert(BevyUiMenuScrollbar { thumb });
        // Only draw a thumb when the list actually scrolls (`size < 1`); a
        // full-size thumb means the list fits, same rule as the cube.
        if thumb.size < 1.0 {
            let (top, height) = scrollbar_thumb_layout(thumb);
            control.with_children(|track| {
                track.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(0.0),
                        top: Val::Percent(top * 100.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(height * 100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.85, 0.78, 0.30, 0.96)),
                    BevyUiMenuScrollbarThumb,
                    Name::new("scrollbar thumb"),
                ));
            });
        }
    }
}

/// Clamp host thumb fractions into a renderable `(top, height)` pair in `0..=1`,
/// matching the cube's `scrollbar_thumb_layout`: the height is floored grabbable,
/// and the top is placed across the remaining travel (`1 - height`) so the thumb
/// never overflows the track bottom.
fn scrollbar_thumb_layout(thumb: ScrollThumb) -> (f32, f32) {
    let start = thumb.start.clamp(0.0, 1.0);
    let size = thumb.size.clamp(0.08, 1.0);
    let travel = (1.0 - size).max(0.0);
    (start * travel, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MenuColor, MenuFocusKey};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Page {
        Inventory,
        System,
        Map,
        Quest,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Action {
        Equip,
        Setting,
    }

    fn tab_set() -> Vec<BevyUiMenuTabSpec<Page>> {
        vec![
            BevyUiMenuTabSpec::new(Page::Inventory, "Inventory"),
            BevyUiMenuTabSpec::new(Page::System, "System"),
            BevyUiMenuTabSpec::new(Page::Map, "Map"),
            BevyUiMenuTabSpec::new(Page::Quest, "Quest"),
        ]
    }

    /// A page with two actionable controls + a non-actionable label, and a
    /// scrolling scrollbar. Returns the page plus the focus key of the first
    /// control so tests can request it focused.
    fn sample_page() -> (MenuPageModel<Page, Action>, MenuFocusKey) {
        let mut page = MenuPageModel::new(Page::Inventory, "Inventory", MenuColor::BLUE_PANEL);
        page.text(
            50.0,
            4.0,
            5.0,
            "Inventory",
            MenuTextAlign::Center,
            MenuColor::WHITE,
        );
        let r0 = MenuRect::new(10.0, 20.0, 30.0, 8.0);
        let r1 = MenuRect::new(10.0, 30.0, 30.0, 8.0);
        page.control(
            r0,
            MenuControlKind::Item,
            "Health",
            None,
            false,
            false,
            Some(Action::Equip),
        );
        page.control(
            r1,
            MenuControlKind::Action,
            "Audio",
            None,
            false,
            false,
            Some(Action::Setting),
        );
        // A label with no action (not actionable).
        page.control(
            MenuRect::new(10.0, 40.0, 30.0, 8.0),
            MenuControlKind::Decoration,
            "Label",
            None,
            false,
            false,
            None,
        );
        // A scrolling scrollbar (size < 1 → thumb drawn).
        page.scrollbar(MenuRect::new(92.0, 20.0, 4.0, 60.0), 0.25, 0.5);
        let focus0 = focus_key_for(r0);
        (page, focus0)
    }

    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app
    }

    /// Queue the spawn, run one update so the command applies, then assert.
    fn spawn_view(app: &mut App, active_tab: usize, focused: Option<MenuFocusKey>) {
        let (page, _) = sample_page();
        let tabs = tab_set();
        app.world_mut().commands().queue(move |world: &mut World| {
            let view = BevyUiMenuView {
                tabs: &tabs,
                active_tab,
                page: &page,
                focused,
                focused_tab: None,
            };
            let mut commands = world.commands();
            spawn_bevy_ui_menu(&mut commands, &view);
        });
        app.update();
    }

    #[test]
    fn spawns_one_tab_button_per_tab_with_active_flagged() {
        let mut app = build_app();
        spawn_view(&mut app, 1, None);

        let mut q = app.world_mut().query::<&BevyUiMenuTab>();
        let mut tabs: Vec<_> = q.iter(app.world()).copied().collect();
        tabs.sort_by_key(|t| t.index);
        assert_eq!(tabs.len(), 4, "one button per tab");
        let active: Vec<usize> = tabs.iter().filter(|t| t.active).map(|t| t.index).collect();
        assert_eq!(active, vec![1], "exactly the active tab is flagged");
    }

    #[test]
    fn selected_and_highlighted_are_distinct_colors() {
        // Fix 2: highlighted (cursor/hover), selected (equipped/active), and the two
        // together must all read as DIFFERENT control backgrounds.
        let k = MenuControlKind::Item;
        let highlighted = control_bg(k, true, false, false);
        let selected = control_bg(k, false, true, false);
        let both = control_bg(k, true, true, false);
        let plain = control_bg(k, false, false, false);
        assert_ne!(highlighted, selected, "highlighted ≠ selected");
        assert_ne!(highlighted, both, "highlighted ≠ selected+highlighted");
        assert_ne!(selected, both, "selected ≠ selected+highlighted");
        assert_ne!(selected, plain, "selected ≠ plain");
        assert_ne!(highlighted, plain, "highlighted ≠ plain");
    }

    #[test]
    fn focused_tab_is_flagged_on_the_tab_button() {
        // Fix 4: when the view reports a focused tab (keyboard on the tab bar), that
        // tab button carries `focused: true` and no other does.
        let mut app = build_app();
        let (page, _) = sample_page();
        let tabs = tab_set();
        app.world_mut().commands().queue(move |world: &mut World| {
            let view = BevyUiMenuView {
                tabs: &tabs,
                active_tab: 0,
                page: &page,
                focused: None,
                focused_tab: Some(2),
            };
            let mut commands = world.commands();
            spawn_bevy_ui_menu(&mut commands, &view);
        });
        app.update();

        let mut q = app.world_mut().query::<&BevyUiMenuTab>();
        let focused: Vec<usize> = q
            .iter(app.world())
            .filter(|t| t.focused)
            .map(|t| t.index)
            .collect();
        assert_eq!(focused, vec![2], "exactly the focused tab is flagged");
    }

    #[test]
    fn controls_present_tagged_with_action_and_focus_key() {
        let mut app = build_app();
        spawn_view(&mut app, 0, None);

        let mut q = app.world_mut().query::<&AmbitionMenuControl<Action>>();
        let controls: Vec<_> = q.iter(app.world()).cloned().collect();
        // 2 actionable + 1 label + 1 scrollbar = 4 control entities.
        assert_eq!(controls.len(), 4);
        let actions: Vec<Action> = controls.iter().filter_map(|c| c.action).collect();
        assert!(actions.contains(&Action::Equip));
        assert!(actions.contains(&Action::Setting));
        // The item control carries the focus key derived from its rect.
        let item = controls
            .iter()
            .find(|c| c.action == Some(Action::Equip))
            .unwrap();
        assert_eq!(
            item.focus,
            focus_key_for(MenuRect::new(10.0, 20.0, 30.0, 8.0))
        );
    }

    #[test]
    fn focused_control_is_flagged_and_only_one() {
        let mut app = build_app();
        let (_, focus0) = sample_page();
        spawn_view(&mut app, 0, Some(focus0));

        let mut focused_q = app
            .world_mut()
            .query::<(&BevyUiMenuFocused, &AmbitionMenuControl<Action>)>();
        let flagged: Vec<_> = focused_q.iter(app.world()).collect();
        assert_eq!(flagged.len(), 1, "exactly one focused control");
        assert_eq!(flagged[0].1.action, Some(Action::Equip));

        let mut vs_q = app
            .world_mut()
            .query::<(&BevyUiMenuFocused, &MenuVisualState)>();
        let (_, vs) = vs_q.single(app.world()).unwrap();
        assert!(vs.focused, "focused control's visual state is focused");
    }

    #[test]
    fn scrollbar_spawns_track_and_thumb_with_right_fraction() {
        let mut app = build_app();
        spawn_view(&mut app, 0, None);

        let mut bar_q = app.world_mut().query::<&BevyUiMenuScrollbar>();
        let bars: Vec<_> = bar_q.iter(app.world()).copied().collect();
        assert_eq!(bars.len(), 1, "one scrollbar track");
        assert_eq!(
            bars[0].thumb,
            ScrollThumb {
                start: 0.25,
                size: 0.5
            }
        );

        let mut thumb_q = app.world_mut().query::<&BevyUiMenuScrollbarThumb>();
        assert_eq!(
            thumb_q.iter(app.world()).count(),
            1,
            "a scrolling scrollbar draws a thumb"
        );
    }

    #[test]
    fn full_size_scrollbar_draws_no_thumb() {
        let mut app = build_app();
        let mut page: MenuPageModel<Page, Action> =
            MenuPageModel::new(Page::System, "System", MenuColor::BLUE_PANEL);
        // size >= 1 → list fits → no thumb.
        page.scrollbar(MenuRect::new(92.0, 20.0, 4.0, 60.0), 0.0, 1.0);
        let tabs = tab_set();
        app.world_mut().commands().queue(move |world: &mut World| {
            let view = BevyUiMenuView {
                tabs: &tabs,
                active_tab: 1,
                page: &page,
                focused: None,
                focused_tab: None,
            };
            let mut commands = world.commands();
            spawn_bevy_ui_menu(&mut commands, &view);
        });
        app.update();

        let mut bar_q = app.world_mut().query::<&BevyUiMenuScrollbar>();
        assert_eq!(bar_q.iter(app.world()).count(), 1);
        let mut thumb_q = app.world_mut().query::<&BevyUiMenuScrollbarThumb>();
        assert_eq!(
            thumb_q.iter(app.world()).count(),
            0,
            "a non-scrolling list draws no thumb"
        );
    }

    #[test]
    fn item_cell_with_icon_spawns_an_image_node() {
        // Fix 3: an owned item cell carrying an icon path renders an `ImageNode`
        // (the sprite icon) when an `AssetServer` is available, like the cube does.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Image>();

        let mut page: MenuPageModel<Page, Action> =
            MenuPageModel::new(Page::Inventory, "Inventory", MenuColor::BLUE_PANEL);
        page.control_with_icon(
            MenuRect::new(10.0, 20.0, 12.0, 12.0),
            MenuControlKind::Item,
            "Health",
            None,
            Some("items/health.png"),
            false,
            false,
            Some(Action::Equip),
        );
        let tabs = tab_set();
        app.world_mut().commands().queue(move |world: &mut World| {
            let view = BevyUiMenuView {
                tabs: &tabs,
                active_tab: 0,
                page: &page,
                focused: None,
                focused_tab: None,
            };
            let assets = world.get_resource::<AssetServer>().cloned();
            let mut commands = world.commands();
            spawn_bevy_ui_menu_with_assets(&mut commands, &view, assets.as_ref());
        });
        app.update();

        let mut icon_q = app.world_mut().query::<&ImageNode>();
        assert_eq!(
            icon_q.iter(app.world()).count(),
            1,
            "an item cell with an icon spawns one ImageNode"
        );
    }

    #[test]
    fn item_cell_without_assets_falls_back_to_label() {
        // With no AssetServer (the cube/headless path), an icon cell still renders
        // its label and NO ImageNode — the renderer degrades gracefully.
        let mut app = build_app();
        let mut page: MenuPageModel<Page, Action> =
            MenuPageModel::new(Page::Inventory, "Inventory", MenuColor::BLUE_PANEL);
        page.control_with_icon(
            MenuRect::new(10.0, 20.0, 12.0, 12.0),
            MenuControlKind::Item,
            "Health",
            None,
            Some("items/health.png"),
            false,
            false,
            Some(Action::Equip),
        );
        let tabs = tab_set();
        app.world_mut().commands().queue(move |world: &mut World| {
            let view = BevyUiMenuView {
                tabs: &tabs,
                active_tab: 0,
                page: &page,
                focused: None,
                focused_tab: None,
            };
            let mut commands = world.commands();
            spawn_bevy_ui_menu(&mut commands, &view);
        });
        app.update();

        let mut icon_q = app.world_mut().query::<&ImageNode>();
        assert_eq!(
            icon_q.iter(app.world()).count(),
            0,
            "no assets → no ImageNode"
        );
    }

    #[test]
    fn thumb_layout_clamps_and_places_within_track() {
        // Top window → thumb at top.
        let (top, h) = scrollbar_thumb_layout(ScrollThumb {
            start: 0.0,
            size: 0.5,
        });
        assert!(top.abs() < 1e-6);
        assert!((h - 0.5).abs() < 1e-6);
        // Bottom window → thumb flush with bottom (top == 1 - height).
        let (top, h) = scrollbar_thumb_layout(ScrollThumb {
            start: 1.0,
            size: 0.5,
        });
        assert!((top + h - 1.0).abs() < 1e-6);
        // Tiny thumb floored grabbable.
        let (_, h) = scrollbar_thumb_layout(ScrollThumb {
            start: 0.5,
            size: 0.0,
        });
        assert!(h >= 0.08 - 1e-6);
    }
}
