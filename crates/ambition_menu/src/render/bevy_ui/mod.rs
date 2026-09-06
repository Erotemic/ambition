//! Flat `bevy_ui` renderer for [`MenuPageModel`].
//!
//! This module owns model-to-entity presentation only. It spawns tabs, panels,
//! labels, focusable controls, grids, and scrollbars, tagging interactive
//! entities with [`AmbitionMenuControl`] and visual focus/selection state. Hosts
//! perform navigation and action dispatch separately. The renderer is generic
//! over page/action ids and shares the backend-agnostic model with other menu
//! presentations.

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

/// Spawn the flat tabbed menu under a fresh [`BevyUiMenuRoot`] and return its entity.
///
/// The panel is roughly where/size the kaleidoscope cube renders — a window in the middle of
/// the screen, NOT a full-screen layout. The body draws the page's nodes by absolute percent
/// rect (percent of the PANEL) so it matches the model's authored layout, while the tab bar
/// uses flex so tabs share the panel width evenly. Font handle used by all menu surfaces. The
/// renderer-agnostic menu crate does not own an asset path; the host supplies the resolved
/// handle. `None` uses Bevy's default font.
/// ⭐ A SOURCE, NOT A FACE. This carried a `Handle<Font>` until Bevy 0.19, which
/// meant the menu crate was handed ONE FILE — so "the menu font" and "the regular
/// weight" were the same fact and a menu could not ask for semibold at all. A
/// [`FontSource`](bevy::text::FontSource) is a family (or a generic category),
/// and the weight rides on the `TextFont` beside it, so the host publishes a
/// TYPEFACE and the menu chooses within it.
///
/// The renderer-agnostic menu crate still owns no path, no filename and now no
/// family name either: the host resolves it through
/// `ambition_render::ui_fonts::UiFonts`. `None` means nothing was resolved and
/// Bevy's built-in font is used — see the module note on why that is a decision.
#[derive(bevy::prelude::Resource, Default, Clone, Debug)]
pub struct MenuFont(pub Option<bevy::text::FontSource>);

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
    spawn_bevy_ui_menu_with_font(commands, view, assets, None)
}

/// [`spawn_bevy_ui_menu_with_assets`], plus the font the host wants menus drawn
/// in. See [`MenuFont`] for why passing `None` is a decision and not a default.
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
            // A 0.55 alpha black darkens the gameplay enough to read the panel while keeping
            // the scene visible.
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
                            btn.spawn((
                                Text::new(tab.label.clone()),
                                // Tab labels are game-authored strings too — see
                                // the row-label note in `spawn.rs`. A `Text` with
                                // no `TextFont` still GETS one (required
                                // component), and that one is the ASCII subset.
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
                        spawn_node(body, node, view.focused, assets, font);
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

/// Feature C (flat backend): map a pointer's vertical SCREEN position over a scrollbar track's
/// screen rect into the neutral `0..=1` drag fraction (0 = top, 1 = bottom). Mirrors the cube's
/// the `ambition_menu_kaleidoscope` cube renderer `scrollbar_fraction`, but reads the track
/// rect from `bevy_ui`'s `ComputedNode`/`GlobalTransform` (2D, no camera projection). The
/// track's screen rect `(top_y, height)` in logical pixels from its `bevy_ui` layout. Scale
/// both to LOGICAL px via the node's `inverse_scale_factor` so they line up with the pointer
/// location, which the picking core reports in logical/window px.
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

/// Translate Bevy [`Interaction`] state into semantic menu activation.
///
/// Controls activate on release (`Pressed` -> `Hovered`), not on press. A
/// transition to `None` on the same entity cancels the press. If a page rebuild
/// replaces the entity for the same action, keep the action armed through the
/// rebuild frame because the new node remains `Interaction::None` until the next
/// `PreUpdate` focus pass.
fn publish_bevy_ui_menu_actions<Action>(
    rows: Query<(Entity, &Interaction, &AmbitionMenuControl<Action>), With<Button>>,
    pointers: Query<&bevy::picking::pointer::PointerLocation>,
    mut activated: MessageWriter<crate::MenuActionActivated<Action>>,
    // ⛔⛔ KEYED BY THE CONTROL, NOT BY WHAT IT DOES — and it took two goes.
    // This first held a `PressArm<String>` filled with `format!("{action:?}")`,
    // which made a DEBUGGING PRESENTATION part of a row's identity. Replacing
    // that with `Action` removed the `Debug` dependency and was still one layer
    // too coarse: an ACTION says what a row DOES and two rows may do the same
    // thing. Tap destructive row A once to arm it, tap destructive row B once,
    // and B — carrying an equal action — reads as already armed and fires on the
    // first tap.
    //
    // ⭐ `MenuFocusKey` IS THE IDENTITY THE MENU ALREADY HAS: *"stable
    // navigation identity for focusable controls"*, on the control component,
    // and stable across the republishes that move entities. `PressArm`'s own doc
    // asks flat lists to key by control identity; this is a flat menu.
    mut arm: Local<ambition_ui_nav::PressArm<MenuFocusKey>>,
    // The action and the entity behind the armed key. The ACTION is a payload
    // emitted on activation, not a name — it rides beside the arm rather than
    // inside it for the same reason the entity does.
    mut armed: Local<Option<(Action, Entity)>>,
    risk: Option<Res<crate::MenuDestructiveActions<Action>>>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    // The SECOND arm, and it is a different question from `arm` above. That one
    // asks "is a finger still down on this control"; this one asks "has this
    // destructive row already been tapped once". A gesture ends every frame the
    // pointer lifts; a confirm arm has to outlive that, or the guard would be
    // spent before the user could answer it.
    mut confirm_armed: Local<Option<MenuFocusKey>>,
) where
    Action: Clone + Send + Sync + 'static,
{
    // The pointer position, for the drag test. Multi-touch is approximated by
    // the first located pointer: `PressArm` treats a missing position as "no
    // evidence of a drag" and still activates, which is the safe direction —
    // an unreported drag costs a stray activation, a phantom drag costs every
    // tap on a device that reports no position at all.
    // ⛔ SCANNED ONLY WHEN SOMETHING IS PRESSED. This ran above the rows loop,
    // so every frame of every match paid a `PointerLocation` scan to answer a
    // question only the `Some(pressed)` arm below ever asks — and during
    // gameplay there are no menu rows and nothing is ever pressed. `at` is read
    // in exactly two places, both inside that arm, so deferring it changes
    // nothing except when the scan happens.
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
        // Still (or newly) held. A press on a DIFFERENT control replaces the
        // arm: two fingers on two rows is one gesture as far as this bridge is
        // concerned, and the later one is the live one.
        Some((focus, action, entity)) => {
            if arm.armed() == Some(&focus) {
                arm.moved(locate());
                // Re-anchor: a rebuild WHILE held moves the control, and the
                // leave test below compares against where it is now.
                *armed = Some((action, entity));
            } else {
                // Pressing a DIFFERENT row abandons any pending confirm: an
                // armed *Quit to Desktop* must not still be armed after the
                // user has gone and touched something else. ⛔ compared by
                // CONTROL, so a second row that happens to do the same thing is
                // a different row.
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
                        // The release landed on the row. Whether it ACTIVATES is
                        // the user's configured tap policy, and the policy is
                        // `ambition_input`'s — this bridge only supplies the two
                        // facts it is the one that knows: which row was released
                        // on, and whether that row is destructive.
                        let destructive = risk
                            .as_deref()
                            .is_some_and(|risk| (risk.is_destructive)(&action));
                        let tap_mode = settings
                            .as_deref()
                            .map(|settings| settings.controls.menu_tap_mode)
                            .unwrap_or_default();
                        // A pointer release IS the selection here, so target and
                        // selection are the same row by construction; the guard
                        // reduces to "was this row already armed".
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
                // Absent, or present at a NEW entity: mid-rebuild. Hold the
                // arm — the release will find the control again.
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

    // Edge-triggered. A pointer resting on a row holds `Hovered` every frame,
    // and a message per frame would turn "the mouse is here" into a stream the
    // host has to debounce — the same shape as the press latch above.
    // ⛔ THE ACTION, not its `Debug` text — same rule as the press arm above: a
    // hover edge is a claim about WHICH ROW, and two rows whose debugging
    // presentation agrees are still two rows.
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
/// Same rule as [`publish_bevy_ui_menu_actions`], for the same reason: a tab bar
/// is a strip of touch targets along the top of a scrollable page, so a finger
/// that lands on one and slides is trying to move the page, not change tabs.
/// Leaving this one activating on the way down while its neighbour in the same
/// file activated on the way up is exactly the drift a shared renderer exists to
/// prevent.
///
/// The identity here IS a row index — [`ambition_ui_nav::RowPress`] in its
/// original shape — because a tab bar's `index` is stable across the republishes
/// that move its entities.
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
            // A DIFFERENT entity (or none) is the tab bar mid-republish, whose fresh nodes read
            // `None` until the next frame's focus pass.
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
    // ⭐ NO TEXT-SIZE INSTALLER. Menu text sizes are authored as a percentage
    // of viewport height and spawned as `FontSize::Vh`, which the engine
    // resolves against the live UI render target; the seam a host used to have
    // to remember (and whose omission was invisible until somebody resized a
    // window) no longer exists.
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

/// Recolour a control when its runtime state changes, WITHOUT respawning it.
///
/// The colour was baked at spawn by `control_bg`, so the only way to change it
/// WAS to spawn again. Now [`MenuVisualState`] carries everything that function
/// needs — including the authored `important`, which is not runtime state and is
/// there precisely so a restyle never has to reach back into the page data.
///
///  `Changed<MenuVisualState>` — a quiet menu costs one empty query. Bevy sets
/// the change tick on any `&mut` deref, so a host that writes the same value
/// every frame pays for a colour write; write only on change, as the launcher's
/// own declarer does.
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

/// The control's KIND, on the entity.
///
/// `AmbitionMenuControl<Action>` already carries it, but that type is generic
/// over the host's action and a restyle system must not be — one restyle for
/// every menu in the app, whatever each one's actions are.
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
    app.add_message::<crate::MenuTabActivated>().add_systems(
        Update,
        publish_bevy_ui_menu_tabs.in_set(BevyUiMenuInteractionSet),
    );
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
