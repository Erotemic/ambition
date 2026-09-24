//! Flat `bevy_ui` renderer for [`MenuPageModel`].
//!
//! This module owns model-to-entity presentation only. It spawns tabs,
//! panels, labels, focusable controls, grids, and scrollbars, and tags
//! interactive entities with [`AmbitionMenuControl`] and focus/selection
//! state. Hosts do navigation and action dispatch separately. The renderer is
//! generic over page/action ids and shares the backend-agnostic model with
//! other menu presentations.

use crate::MenuFocusKey;
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
/// `index` is the tab's position in the ordered tab set. `active` mirrors the
/// view's active tab, so a host can map a clicked tab to its index directly.
#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct BevyUiMenuTab {
    pub index: usize,
    pub active: bool,
    /// Keyboard focus is on this tab (the tab bar has focus and the cursor is
    /// on it). Drawn with a focus ring distinct from the active highlight.
    pub focused: bool,
}

/// Flag on the single focused control entity (the cursor).
///
/// The focused control also carries `MenuVisualState { focused: true, .. }`.
/// This marker lets the host find the cursor entity directly.
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

/// Scheduling seam for flat Bevy-UI pointer/touch activation.
///
/// Hosts consume [`crate::MenuActionActivated`] / [`crate::MenuTabActivated`]
/// after this set, then route them through the same semantic command/dispatch
/// paths used by keyboard and controller input.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BevyUiMenuInteractionSet;

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
/// This is the renderer's single input, built by the host: the ordered tab
/// set, the active tab, the active page's [`MenuPageModel`], and the focused
/// control key. The renderer is a pure function of this view; it only spawns
/// the entity tree.
pub struct BevyUiMenuView<'a, PageId, Action> {
    /// The ordered tab set (page id + label), drawn left→right in the tab bar.
    pub tabs: &'a [BevyUiMenuTabSpec<PageId>],
    /// Index into `tabs` of the active tab (clamped on use).
    pub active_tab: usize,
    /// The active page's model: the body the renderer draws.
    pub page: &'a MenuPageModel<PageId, Action>,
    /// The focused control's focus key (cursor), if any control is focused.
    ///
    /// A control whose [`MenuFocusKey`](crate::MenuFocusKey) equals this is
    /// drawn focused and flagged with [`BevyUiMenuFocused`].
    pub focused: Option<crate::MenuFocusKey>,
    /// When keyboard focus is on the tab bar, the index of the tab under the
    /// cursor, drawn with a focus ring. `None` means focus is in the body; the
    /// active tab is still highlighted via [`BevyUiMenuTab::active`].
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

/// Derive a control's stable [`MenuFocusKey`] from its rect, the same way the
/// cube renderer does (ambition_menu_kaleidoscope). A key computed against
/// one renderer then addresses the same control in the other.
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

/// Background tint for a control: focused/selected is gold, important is
/// accented, disabled and the scrollbar track are dim, plain controls are
/// neutral blue.
fn control_bg(kind: MenuControlKind, focused: bool, selected: bool, important: bool) -> Color {
    if matches!(kind, MenuControlKind::Scrollbar) {
        return Color::srgba(0.10, 0.11, 0.16, 0.92);
    }
    // Highlighted (cursor/hover) and selected (equipped/active setting) must
    // look different; both together are brightest. The cube shows the cursor
    // with a separate focus ring; the flat backend folds it into the
    // background.
    match (focused, selected) {
        // Highlighted and selected: brightest gold.
        (true, true) => Color::srgba(0.99, 0.82, 0.34, 0.98),
        // Highlighted only: gold cursor color.
        (true, false) => Color::srgba(0.85, 0.70, 0.20, 0.96),
        // Selected only: muted teal, distinct from the gold cursor.
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

/// The font source for all menu surfaces.
///
/// A [`FontSource`](bevy::text::FontSource) is a family (or generic
/// category); the weight is on the `TextFont`, so a menu can choose a weight
/// within the host's typeface. The menu crate owns no path, file, or family
/// name: the host resolves it through `ambition_render::ui_fonts::UiFonts`.
/// `None` means nothing was resolved and Bevy's built-in font is used.
#[derive(bevy::prelude::Resource, Default, Clone, Debug)]
pub struct MenuFont(pub Option<bevy::text::FontSource>);

/// Spawn the flat tabbed menu under a fresh [`BevyUiMenuRoot`] and return its
/// entity.
///
/// The panel is a centered window about the size of the kaleidoscope cube,
/// not a full-screen layout. The body draws page nodes by absolute percent
/// rect (percent of the panel), matching the model's layout. The tab bar uses
/// flex so tabs share the panel width.
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

/// Like [`spawn_bevy_ui_menu`], with an optional [`AssetServer`] so item cells
/// can draw their icon image. With `None` (for example a headless test with
/// no `AssetPlugin`), icons fall back to the label. The host always passes
/// one, so the Items tab shows the same icons as the cube.
pub fn spawn_bevy_ui_menu_with_assets<PageId, Action>(
    commands: &mut Commands,
    view: &BevyUiMenuView<PageId, Action>,
    assets: Option<&AssetServer>,
) -> Entity
where
    PageId: Clone + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    spawn_bevy_ui_menu_with_font(commands, view, assets, None)
}

/// [`spawn_bevy_ui_menu_with_assets`], plus the font the host wants menus drawn
/// in. See [`MenuFont`].
pub fn spawn_bevy_ui_menu_with_font<PageId, Action>(
    commands: &mut Commands,
    view: &BevyUiMenuView<PageId, Action>,
    assets: Option<&AssetServer>,
    font: Option<&bevy::text::FontSource>,
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
            // 0.55 black dims gameplay enough to read the panel and keeps the
            // scene visible.
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            // On top of the gameplay HUD so the menu's buttons get the pointer.
            GlobalZIndex(1000),
            BevyUiMenuRoot,
            AmbitionMenuRoot,
            Name::new("bevy_ui menu root"),
        ))
        .id();

    commands.entity(root).with_children(|root| {
        root.spawn((
            Node {
                width: Val::Percent(64.0),
                height: Val::Percent(74.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // Translucent dark window (0.90), so content reads clearly with a
            // hint of the world behind. The model's `page.background` is
            // near-transparent for the cube's opaque face, so the flat renderer
            // supplies this backing.
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
                        // A tab under the keyboard cursor gets a border focus
                        // ring, distinct from the active tab's fill.
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
                            btn.spawn((
                                Text::new(tab.label.clone()),
                                // Tab labels are game-authored strings (see the
                                // row-label note in `spawn.rs`). A `Text`
                                // without `TextFont` gets the default one,
                                // which is the ASCII subset.
                                TextFont {
                                    font: font.cloned().unwrap_or_default().into(),
                                    ..default()
                                },
                                TextColor(label_color),
                            ));
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
                    // Not tagged `AmbitionMenuPage`: that is the cube's face
                    // marker, and `rebuild_cube_faces` despawns every
                    // `AmbitionMenuPage` when `ActiveMenuPages` changes, which
                    // would empty this panel.
                    BevyUiMenuBody,
                    Name::new("menu body"),
                ))
                .with_children(|body| {
                    for node in &view.page.nodes {
                        spawn_node(body, node, view.focused, assets, font);
                    }
                });
        });
    });

    root
}

// Draw-order layers that mirror the cube's depth bands. Without them, bevy_ui
// sibling order paints a later background panel over earlier text. A per-node
// `ZIndex` gives back-to-front order: panels, then controls, then text.
const LAYER_CONTROL: i32 = 10;
const LAYER_TEXT: i32 = 20;

/// The scrollbar track's screen rect `(top_y, height)` in logical pixels,
/// from its `bevy_ui` layout. Scaled by `inverse_scale_factor` to match the
/// pointer location, which picking reports in logical pixels. The flat
/// counterpart of the kaleidoscope cube's `scrollbar_fraction`.
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

/// A press on the scrollbar marks the track held by that pointer (so
/// [`bevy_ui_scrollbar_press_drag`] tracks it) and jumps the scroll to the
/// pressed position. Mirrors the cube's `scrollbar_press`.
fn bevy_ui_scrollbar_press(
    press: On<Pointer<Press>>,
    bars: Query<&BevyUiMenuScrollbar>,
    mut drag: ResMut<crate::ScrollbarDragState>,
    mut out: MessageWriter<crate::MenuScrollDragged>,
) {
    if bars.get(press.entity).is_ok() {
        // Geometry comes from the last good rect kept by
        // `bevy_ui_maintain_track_rect`: a respawned node's `ComputedNode` is
        // zero on the press frame.
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

/// Keep the [`ScrollbarDragState`](crate::ScrollbarDragState) track rect at
/// the grid scrollbar's last good screen rect. Never overwrite it with the
/// zero a node reports on the frame it respawns.
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

/// While dragging on the scrollbar, emit the fraction for the pointer's
/// position. `bevy_ui` picking drives `Pointer<Drag>` reliably, so this is
/// the primary path; the press-drag tracker is a backup.
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

/// Releasing the pointer ends the drag on every track it held (a release can
/// land off the thumb). Mirrors the cube's `scrollbar_release`.
fn bevy_ui_scrollbar_release(
    release: On<Pointer<Release>>,
    mut drag: ResMut<crate::ScrollbarDragState>,
) {
    if drag.pressed_by == Some(release.pointer_id) {
        drag.pressed_by = None;
    }
}

/// While a pointer is held on a scrollbar
/// ([`ScrollbarDragState`](crate::ScrollbarDragState)), emit the fraction for
/// its live position each frame. The track is found by component, so the drag
/// survives the republish that respawns it.
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
    // Use the cached track rect: it stays valid across the respawn that zeroes
    // the new node's layout.
    if let Some(fraction) =
        scrollbar_fraction_from_rect(drag.track_top_y, drag.track_height, loc.position.y)
    {
        out.write(crate::MenuScrollDragged { fraction });
    }
}

/// Translate Bevy [`Interaction`] state into semantic menu activation.
///
/// Controls activate on release (`Pressed` -> `Hovered`), not on press. A
/// change to `None` on the same entity cancels. If a rebuild replaces the
/// entity for the same action, the arm holds through the rebuild frame,
/// because the new node stays `Interaction::None` until the next `PreUpdate`
/// focus pass.
fn publish_bevy_ui_menu_actions<Action>(
    rows: Query<(Entity, &Interaction, &AmbitionMenuControl<Action>), With<Button>>,
    pointers: Query<&bevy::picking::pointer::PointerLocation>,
    mut activated: MessageWriter<crate::MenuActionActivated<Action>>,
    // Keyed by the control (`MenuFocusKey`), not by its action. Two rows can
    // do the same thing; keyed by action, tapping destructive row A then row
    // B would fire B on its first tap. `MenuFocusKey` is stable across the
    // republishes that move entities.
    mut arm: Local<ambition_ui_nav::PressArm<MenuFocusKey>>,
    // The action and entity behind the armed key. The action is a payload
    // emitted on activation.
    mut armed: Local<Option<(Action, Entity)>>,
    risk: Option<Res<crate::MenuDestructiveActions<Action>>>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    // A second arm with a different question: `arm` asks whether a finger is
    // still down; this asks whether a destructive row was already tapped once.
    // It must outlive the gesture so the user can answer the confirm.
    mut confirm_armed: Local<Option<MenuFocusKey>>,
) where
    Action: Clone + Send + Sync + 'static,
{
    // Pointer position for the drag test. Multi-touch uses the first located
    // pointer. `PressArm` treats a missing position as no drag and still
    // activates: a missed drag costs a stray activation, but a phantom drag
    // would block every tap on a device with no position. Scanned only when
    // something is pressed, because there are usually no menu rows.
    let locate = || {
        pointers
            .iter()
            .find_map(|p| p.location())
            .map(|l| l.position)
    };

    let mut pressed: Option<(MenuFocusKey, Action, Entity)> = None;
    let mut armed_now: Option<(Entity, Interaction)> = None;
    for (entity, interaction, control) in &rows {
        let Some(action) = control.action.clone() else {
            continue;
        };
        if arm.armed() == Some(&control.focus) {
            armed_now = Some((entity, *interaction));
        }
        if *interaction == Interaction::Pressed && pressed.is_none() {
            pressed = Some((control.focus, action, entity));
        }
    }

    match pressed {
        // Held. A press on a different control replaces the arm; the later
        // finger is the live one.
        Some((focus, action, entity)) => {
            if arm.armed() == Some(&focus) {
                arm.moved(locate());
                // Re-anchor: a rebuild while held moves the control, and the
                // leave test compares against its current place.
                *armed = Some((action, entity));
            } else {
                // Pressing a different row cancels a pending confirm. Compared
                // by control, so a row with the same action is still a
                // different row.
                if confirm_armed.as_ref() != Some(&focus) {
                    *confirm_armed = None;
                }
                arm.press(focus, locate());
                *armed = Some((action, entity));
            }
        }
        None if arm.is_armed() => {
            let pressed_at = armed.as_ref().map(|(_, entity)| *entity);
            match armed_now {
                // Came up ON the armed control.
                Some((_, Interaction::Hovered)) => {
                    let released = arm.release_anywhere();
                    if let (Some(focus), Some((action, _))) = (released, armed.take()) {
                        // Whether a release activates is the tap policy in
                        // `ambition_input`. This bridge supplies only which row
                        // was released on and whether it is destructive.
                        let destructive = risk
                            .as_deref()
                            .is_some_and(|risk| (risk.is_destructive)(&action));
                        let tap_mode = settings
                            .as_deref()
                            .map(|settings| settings.controls.menu_tap_mode)
                            .unwrap_or_default();
                        // A release is the selection here, so the guard only
                        // asks whether this row was already armed.
                        let press =
                            tap_mode.resolve_press(focus, &focus, destructive, &mut confirm_armed);
                        if press == ambition_input::settings::MenuPointerPress::Confirm {
                            activated.write(crate::MenuActionActivated { action });
                        }
                    }
                }
                // Same entity, no longer under the pointer: it was left.
                Some((entity, Interaction::None)) if Some(entity) == pressed_at => {
                    arm.clear();
                    *armed = None;
                }
                // Absent, or at a new entity: mid-rebuild. Hold the arm; the
                // release finds the control again.
                _ => {}
            }
        }
        None => {}
    }
}

/// Publish hover as a preview, distinct from activation. Only the first hovered
/// row is emitted; overlapping pickable rows are treated as a layout error rather
/// than exposing query order to the host.
fn publish_bevy_ui_menu_previews<Action>(
    rows: Query<(&Interaction, &AmbitionMenuControl<Action>), With<Button>>,
    mut previewed: MessageWriter<crate::MenuActionPreviewed<Action>>,
    mut last: Local<Option<Action>>,
) where
    Action: Clone + PartialEq + Send + Sync + 'static,
{
    let hovered = rows
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Hovered)
        .and_then(|(_, control)| control.action.clone());

    // Edge-triggered: a pointer resting on a row stays `Hovered`, and a
    // message per frame would force the host to debounce. Compared by action
    // value, not its `Debug` text.
    if hovered == *last {
        return;
    }
    *last = hovered.clone();
    if let Some(action) = hovered {
        previewed.write(crate::MenuActionPreviewed { action });
    }
}

/// Translate flat-menu tab taps into a renderer-neutral tab message.
///
/// Same release rule as [`publish_bevy_ui_menu_actions`]: a tab bar sits on a
/// scrollable page, so a finger that lands and slides is scrolling, not
/// changing tabs.
///
/// Keyed by row index ([`ambition_ui_nav::RowPress`]), because a tab's
/// `index` is stable across the republishes that move its entities.
fn publish_bevy_ui_menu_tabs(
    tabs: Query<(Entity, &Interaction, &BevyUiMenuTab), With<Button>>,
    pointers: Query<&bevy::picking::pointer::PointerLocation>,
    mut activated: MessageWriter<crate::MenuTabActivated>,
    mut arm: Local<ambition_ui_nav::RowPress>,
    mut armed_entity: Local<Option<Entity>>,
) {
    let at = pointers
        .iter()
        .find_map(|p| p.location())
        .map(|l| l.position);

    let mut pressed: Option<(usize, Entity)> = None;
    let mut armed_now: Option<(Entity, Interaction)> = None;
    for (entity, interaction, tab) in &tabs {
        if arm.armed() == Some(&tab.index) {
            armed_now = Some((entity, *interaction));
        }
        if *interaction == Interaction::Pressed && pressed.is_none() {
            pressed = Some((tab.index, entity));
        }
    }

    match pressed {
        Some((index, entity)) => {
            if arm.armed() == Some(&index) {
                arm.moved(at);
                *armed_entity = Some(entity);
            } else {
                arm.press(index, at);
                *armed_entity = Some(entity);
            }
        }
        None if arm.is_armed() => match armed_now {
            Some((_, Interaction::Hovered)) => {
                if let Some(index) = arm.release_anywhere() {
                    activated.write(crate::MenuTabActivated { index });
                }
                *armed_entity = None;
            }
            // A different entity (or none) means the tab bar is mid-republish;
            // new nodes read `None` until the next focus pass.
            Some((entity, Interaction::None)) if Some(entity) == *armed_entity => {
                arm.clear();
                *armed_entity = None;
            }
            _ => {}
        },
        None => {}
    }
}

/// Install pointer/touch activation for one host action type.
///
/// Call once for every concrete `Action` rendered through
/// [`spawn_bevy_ui_menu_with_assets`]. Different menu producers may coexist in
/// one App because each monomorphized [`AmbitionMenuControl<Action>`] is a
/// distinct ECS component type.
pub fn install_bevy_ui_menu_actions<Action>(app: &mut App)
where
    Action: Clone + PartialEq + Send + Sync + 'static,
{
    // No text-size installer: menu text is spawned as `FontSize::Vh`, which the
    // engine resolves against the live UI target.
    install_bevy_ui_menu_restyle(app);
    app.add_message::<crate::MenuActionActivated<Action>>()
        .add_message::<crate::MenuActionPreviewed<Action>>()
        .add_systems(
            Update,
            (
                publish_bevy_ui_menu_actions::<Action>,
                publish_bevy_ui_menu_previews::<Action>,
            )
                .in_set(BevyUiMenuInteractionSet),
        );
}

/// Recolor a control when its runtime state changes, without respawning it.
///
/// [`MenuVisualState`] carries everything `control_bg` needs, including the
/// authored `important`, so a restyle never reads page data. Filtered on
/// `Changed<MenuVisualState>`, so a quiet menu costs one empty query. Any
/// `&mut` deref marks a change, so hosts should write only on change.
pub fn restyle_bevy_ui_menu_controls(
    mut controls: Query<
        (
            &MenuVisualState,
            &mut BackgroundColor,
            &AmbitionMenuControlKind,
        ),
        Changed<MenuVisualState>,
    >,
) {
    for (state, mut background, kind) in &mut controls {
        let color = if state.disabled {
            to_color(MenuColor::DISABLED)
        } else {
            control_bg(kind.0, state.focused, state.selected, state.important)
        };
        if background.0 != color {
            background.0 = color;
        }
    }
}

/// The control's kind, on the entity.
///
/// `AmbitionMenuControl<Action>` also has it, but it is generic over the
/// host's action. One restyle system serves every menu in the app.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmbitionMenuControlKind(pub MenuControlKind);

/// Install the restyle system. Action-agnostic, so repeated installs from
/// several `install_bevy_ui_menu_actions::<A>` calls must not stack it.
pub fn install_bevy_ui_menu_restyle(app: &mut bevy::prelude::App) {
    if app.is_plugin_added::<BevyUiMenuRestylePlugin>() {
        return;
    }
    app.add_plugins(BevyUiMenuRestylePlugin);
}

/// Carries the once-only registration for [`restyle_bevy_ui_menu_controls`].
#[derive(Default)]
pub struct BevyUiMenuRestylePlugin;

impl bevy::prelude::Plugin for BevyUiMenuRestylePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            bevy::prelude::Update,
            restyle_bevy_ui_menu_controls.in_set(BevyUiMenuInteractionSet),
        );
    }
}

/// Install pointer/touch activation for the flat renderer's tab buttons.
///
/// This is separate from [`install_bevy_ui_menu_actions`] because an App may
/// render several action types, while the shared tab component/message must be
/// installed exactly once.
pub fn install_bevy_ui_menu_tabs(app: &mut App) {
    // Idempotent: a second call would add `publish_bevy_ui_menu_tabs` twice,
    // so one click would change tab and change back. Several compositions
    // call this (kaleidoscope menu, shell title screen). Keyed on a marker
    // for the system, because `add_message` is already idempotent and cannot
    // show whether the system was added.
    if app.world().contains_resource::<BevyUiMenuTabsInstalled>() {
        return;
    }
    app.init_resource::<BevyUiMenuTabsInstalled>();
    app.add_message::<crate::MenuTabActivated>().add_systems(
        Update,
        publish_bevy_ui_menu_tabs.in_set(BevyUiMenuInteractionSet),
    );
}

/// Marker: [`install_bevy_ui_menu_tabs`] has registered its system in this App.
#[derive(bevy::prelude::Resource, Default)]
struct BevyUiMenuTabsInstalled;

/// Install flat scrollbar drag handling: registers the
/// [`MenuScrollDragged`](crate::MenuScrollDragged) message (idempotent if the
/// cube added it) and the press/drag/release observers and tracker. The host
/// applies the emitted fraction to its scroll window (like the cube's
/// `kaleidoscope_apply_scroll_drag`).
pub fn install_bevy_ui_menu_scroll(app: &mut App) {
    app.add_message::<crate::MenuScrollDragged>();
    app.init_resource::<crate::ScrollbarDragState>();
    app.add_observer(bevy_ui_scrollbar_press);
    app.add_observer(bevy_ui_scrollbar_drag);
    app.add_observer(bevy_ui_scrollbar_release);
    // Maintain the last good rect before the tracker reads it each frame.
    app.add_systems(
        Update,
        (bevy_ui_maintain_track_rect, bevy_ui_scrollbar_press_drag).chain(),
    );
}

#[cfg(test)]
mod tests;

mod spawn;
use spawn::spawn_node;
