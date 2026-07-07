//! Flat, tabbed `bevy_ui` renderer of a [`MenuPageModel`].
//!
//! This is the SECOND presentation of the backend-agnostic menu model (the first
//! being the bevy_lunex 3D cube in the `ambition_menu_kaleidoscope` cube renderer). Having two
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
use bevy::ui::UiGlobalTransform;

use crate::{
    scrollbar_fraction_from_rect, scrollbar_thumb_layout, AmbitionMenuControl, AmbitionMenuRoot,
    MenuColor, MenuControlKind, MenuNode, MenuPageModel, MenuRect, MenuTextAlign, MenuVisualState,
    ScrollThumb,
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
/// renderer does (see ambition_menu_kaleidoscope). Keeping this identical means a
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

/// Feature C (flat backend): map a pointer's vertical SCREEN position over a
/// scrollbar track's screen rect into the neutral `0..=1` drag fraction (0 = top,
/// 1 = bottom). `None` if the track has no measured height yet. Mirrors the cube's
/// the `ambition_menu_kaleidoscope` cube renderer `scrollbar_fraction`, but reads the track rect
/// from `bevy_ui`'s `ComputedNode`/`GlobalTransform` (2D, no camera projection).
/// The track's screen rect `(top_y, height)` in logical pixels from its
/// `bevy_ui` layout. A UI node's computed screen rect lives in [`ComputedNode`]
/// (PHYSICAL px size) + [`UiGlobalTransform`] (PHYSICAL px center) — NOT its plain
/// `GlobalTransform`, which is identity for UI nodes (that was the long-standing
/// bug: the rect always read zero). Scale both to LOGICAL px via the node's
/// `inverse_scale_factor` so they line up with the pointer location, which the
/// picking core reports in logical/window px.
fn bevy_ui_track_rect(computed: &ComputedNode, transform: &UiGlobalTransform) -> (f32, f32) {
    let inv = computed.inverse_scale_factor();
    let height = computed.size().y * inv;
    let center_y = transform.translation.y * inv;
    (center_y - height * 0.5, height)
}

fn bevy_ui_scrollbar_fraction(
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
    pointer_y: f32,
) -> Option<f32> {
    let (top_y, height) = bevy_ui_track_rect(computed, transform);
    scrollbar_fraction_from_rect(top_y, height, pointer_y)
}

/// The pure track-rect → fraction mapping shared by the `bevy_ui` scrollbar
/// observers. `None` if the track has no measured height yet. 0 = top edge,
/// 1 = bottom edge; clamped.

/// Feature C: a press that lands on the `bevy_ui` scrollbar marks the track held by
/// that pointer (so [`bevy_ui_scrollbar_press_drag`] tracks the live position) and
/// immediately jumps the scroll to the pressed position (emits the neutral
/// fraction). Mirrors the cube's `scrollbar_press`.
fn bevy_ui_scrollbar_press(
    press: On<Pointer<Press>>,
    bars: Query<&BevyUiMenuScrollbar>,
    mut drag: ResMut<crate::ScrollbarDragState>,
    mut out: MessageWriter<crate::MenuScrollDragged>,
) {
    if bars.get(press.entity).is_ok() {
        // Mark the held pointer; geometry is the LAST KNOWN GOOD rect maintained by
        // `bevy_ui_maintain_track_rect` (a freshly-respawned node's ComputedNode is
        // zero on the press frame, so reading it directly here would jump nowhere).
        drag.pressed_by = Some(press.pointer_id);
        if let Some(fraction) = crate::scrollbar_fraction_from_rect(
            drag.track_top_y,
            drag.track_height,
            press.pointer_location.position.y,
        ) {
            out.write(crate::MenuScrollDragged { fraction });
        }
    }
}

/// Keep the shared [`ScrollbarDragState`](crate::ScrollbarDragState)
/// track rect refreshed with the grid scrollbar's LAST KNOWN GOOD screen rect — never
/// overwriting it with the zero a fresh node reports the frame it is respawned. The
/// press jump + the manual drag tracker both map against this always-valid rect.
fn bevy_ui_maintain_track_rect(
    bars: Query<(&ComputedNode, &UiGlobalTransform), With<BevyUiMenuScrollbar>>,
    mut drag: ResMut<crate::ScrollbarDragState>,
) {
    for (computed, transform) in &bars {
        let (top_y, height) = bevy_ui_track_rect(computed, transform);
        if height > f32::EPSILON {
            drag.track_top_y = top_y;
            drag.track_height = height;
        }
    }
}

/// Feature C: while dragging on the `bevy_ui` scrollbar, emit the neutral fraction
/// for the pointer's current position. `bevy_ui` picking drives `Pointer<Drag>`
/// reliably (unlike the cube's custom 3D backend), so this is the primary path; the
/// press+move tracker below is belt-and-braces.
fn bevy_ui_scrollbar_drag(
    drag: On<Pointer<Drag>>,
    bars: Query<(&BevyUiMenuScrollbar, &ComputedNode, &UiGlobalTransform)>,
    mut out: MessageWriter<crate::MenuScrollDragged>,
) {
    if let Ok((_, computed, transform)) = bars.get(drag.entity) {
        if let Some(fraction) =
            bevy_ui_scrollbar_fraction(computed, transform, drag.pointer_location.position.y)
        {
            out.write(crate::MenuScrollDragged { fraction });
        }
    }
}

/// Feature C: releasing the pointer ends the manual scrollbar drag on every track
/// held by that pointer (a release can land off the thumb). Mirrors the cube's
/// `scrollbar_release`.
fn bevy_ui_scrollbar_release(
    release: On<Pointer<Release>>,
    mut drag: ResMut<crate::ScrollbarDragState>,
) {
    if drag.pressed_by == Some(release.pointer_id) {
        drag.pressed_by = None;
    }
}

/// Feature C: while a pointer is held on a `bevy_ui` scrollbar
/// ([`ScrollbarDragState`](crate::ScrollbarDragState)), emit
/// the neutral fraction for its LIVE position each frame against the CURRENT track
/// — re-found by component, so the drag survives the per-step republish that
/// respawns the track entity.
fn bevy_ui_scrollbar_press_drag(
    pointers: Query<(
        &bevy::picking::pointer::PointerId,
        &bevy::picking::pointer::PointerLocation,
    )>,
    drag: Res<crate::ScrollbarDragState>,
    mut out: MessageWriter<crate::MenuScrollDragged>,
) {
    let Some(held) = drag.pressed_by else {
        return;
    };
    let Some(loc) = pointers
        .iter()
        .find(|(id, _)| **id == held)
        .and_then(|(_, loc)| loc.location())
    else {
        return;
    };
    // Map the live pointer onto the CACHED track rect — valid across the respawn
    // that zeroes the fresh node's `ComputedNode`/`GlobalTransform`.
    if let Some(fraction) =
        scrollbar_fraction_from_rect(drag.track_top_y, drag.track_height, loc.position.y)
    {
        out.write(crate::MenuScrollDragged { fraction });
    }
}

/// Install the flat `bevy_ui` scrollbar drag handling (Feature C): registers the
/// neutral [`MenuScrollDragged`](crate::MenuScrollDragged)
/// message (idempotent if already added by the cube) and the press/drag/release
/// observers + press-drag tracker. The HOST applies the emitted fraction to its own
/// scroll window (mirroring the cube's `kaleidoscope_apply_scroll_drag`).
pub fn install_bevy_ui_menu_scroll(app: &mut App) {
    app.add_message::<crate::MenuScrollDragged>();
    app.init_resource::<crate::ScrollbarDragState>();
    app.add_observer(bevy_ui_scrollbar_press);
    app.add_observer(bevy_ui_scrollbar_drag);
    app.add_observer(bevy_ui_scrollbar_release);
    // Maintain the last-known-good rect BEFORE the tracker reads it each frame.
    app.add_systems(
        Update,
        (bevy_ui_maintain_track_rect, bevy_ui_scrollbar_press_drag).chain(),
    );
}

#[cfg(test)]
mod tests;

mod spawn;
use spawn::spawn_node;
