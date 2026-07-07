//! Cube page-rendering: turns a `MenuPageModel` into the cube's 3D panel / text
//! / control / icon / scrollbar / selection-corner / nav-arrow entities. Split
//! out of the kaleidoscope renderer god-module; `super::*` brings the depth
//! constants, marker components, and config it spawns against.

use super::*;
pub(super) fn render_page_model<PageId, Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    config: &KaleidoscopeMenuConfig,
    model: &MenuPageModel<PageId, Action>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    // One full-page background at the dedicated background depth.
    spawn_panel(
        ui,
        materials,
        MenuRect::new(0.0, 0.0, 100.0, 100.0),
        menu_color(model.background),
        None::<Action>,
        active,
    );
    if config.draw_edge_frame {
        spawn_cube_edge_frame(ui, materials, active);
    }
    if config.draw_nav_arrows {
        spawn_nav_arrows(ui, materials, active);
    }
    for node in &model.nodes {
        match node {
            MenuNode::Panel {
                rect,
                color,
                action,
            } => spawn_panel(
                ui,
                materials,
                *rect,
                menu_color(*color),
                action.clone(),
                active,
            ),
            MenuNode::Text {
                x,
                y,
                size,
                text,
                align,
                color,
            } => {
                spawn_text(
                    ui,
                    materials,
                    *x,
                    *y,
                    *size,
                    text,
                    menu_align(*align),
                    menu_srgba(*color),
                    active,
                    None,
                );
            }
            MenuNode::DynamicText {
                slot,
                x,
                y,
                size,
                align,
                color,
            } => {
                // Spawned EMPTY; the host fills it in place by `slot` (see
                // `MenuDynamicText`). This keeps cursor-dependent text out of the
                // baked page data so a hover does not rebuild the face.
                spawn_text(
                    ui,
                    materials,
                    *x,
                    *y,
                    *size,
                    "",
                    menu_align(*align),
                    menu_srgba(*color),
                    active,
                    Some(MenuDynamicText { slot: *slot }),
                );
            }
            MenuNode::Control {
                rect,
                kind,
                label,
                detail,
                icon,
                selected,
                important,
                action,
                thumb,
            } => spawn_control(
                ui,
                materials,
                asset_server,
                config,
                *rect,
                *kind,
                label,
                detail.as_deref(),
                icon.as_deref(),
                *selected,
                *important,
                action.clone(),
                *thumb,
                active,
            ),
        }
    }
}

fn spawn_panel<Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<Action>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    spawn_panel_at_depth(
        ui,
        materials,
        rect,
        color,
        action.clone(),
        panel_depth(rect, action.is_some()),
        active,
    );
}

fn spawn_panel_at_depth<Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    rect: MenuRect,
    color: Color,
    action: Option<Action>,
    depth: f32,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    let base_alpha = color.alpha();
    let material = materials.add(StandardMaterial {
        base_color: fade_color(color, base_alpha),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let mut entity = ui.spawn((
        Name::new("panel"),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(depth, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        KaleidoscopeFade { base_alpha },
    ));
    if let Some(action) = action {
        entity.insert((
            AmbitionMenuControl {
                kind: MenuControlKind::Action,
                action: Some(action),
                focus: MenuFocusKey::default(),
            },
            MenuVisualState::default(),
            // Mark as an interactive control so the `cube_3d_picking` candidate query
            // (filtered `With<KaleidoscopeControlStyle>`) treats an actionable panel as a
            // pick target, exactly like a `spawn_control` button.
            KaleidoscopeControlStyle {
                kind: MenuControlKind::Action,
                important: false,
                disabled: false,
            },
        ));
        if active {
            entity.insert(KaleidoscopeActiveFaceControl);
        }
    } else {
        entity.insert(Pickable::IGNORE);
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn spawn_text(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    size: f32,
    text: &str,
    align: TextAlign,
    color: Srgba,
    active: bool,
    // When set, tags the text as a host-filled dynamic line (its content is
    // rewritten in place by `slot`), so cursor-dependent text needs no rebuild.
    dynamic: Option<MenuDynamicText>,
) {
    let base_alpha = color.alpha;
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let mut entity = ui.spawn((
        Name::new("text"),
        KaleidoscopeFade { base_alpha },
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .anchor(Anchor::CENTER)
            .pack(),
        UiDepth::Set(page_depth(text_depth(y), active)),
        UiTextSize::from(Rh(size)),
        Text3d::new(text.to_string()),
        Text3dStyling {
            size: 64.0,
            color,
            align,
            font: Arc::from(FONT_FAMILY),
            weight: Weight::BOLD,
            ..Default::default()
        },
        MeshMaterial3d(material),
        Mesh3d::default(),
        Pickable::IGNORE,
    ));
    if let Some(dynamic) = dynamic {
        // Pair the marker with the live content channel (starts empty); the host
        // writes the string and `apply_dynamic_text` copies it into the `Text3d`.
        entity.insert((dynamic, MenuDynamicTextContent::default()));
    }
}

/// Copy each [`MenuDynamicTextContent`] the host has changed into its entity's
/// `Text3d`, so a host can rewrite a dynamic line in place (no rebuild). Only
/// changed contents are touched (cheap, idempotent).
pub(super) fn apply_dynamic_text(
    mut texts: Query<(&MenuDynamicTextContent, &mut Text3d), Changed<MenuDynamicTextContent>>,
) {
    for (content, mut text) in &mut texts {
        *text = Text3d::new(content.0.clone());
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_control<Action>(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    config: &KaleidoscopeMenuConfig,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    detail: Option<&str>,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: Option<Action>,
    thumb: Option<ScrollThumb>,
    active: bool,
) where
    Action: Clone + Send + Sync + 'static,
{
    // A scrollbar has no click `action` (it drives scroll via drag), but it is NOT
    // a disabled/greyed control — colour it with its live scrollbar colour, not the
    // dim disabled colour, and keep it pickable for drag (see below).
    let is_scrollbar = matches!(kind, MenuControlKind::Scrollbar);
    let disabled = action.is_none() && !is_scrollbar;
    // The scrollbar TRACK is drawn DIM (Fix 1): it's the full-height channel behind
    // the brighter thumb, so it must not read as the solid bright blob it used to.
    let color = if disabled {
        disabled_control_color()
    } else if is_scrollbar {
        scrollbar_track_color()
    } else {
        control_color(kind, selected, important)
    };
    let base_alpha = color.alpha();
    let material = materials.add(StandardMaterial {
        base_color: fade_color(color, base_alpha),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let focus = MenuFocusKey {
        row: (rect.y * 10.0).round() as i32,
        col: (rect.x * 10.0).round() as i32,
        order: (rect.y * 100.0 + rect.x).round() as i32,
    };
    // Edge page-turn buttons (the narrow flanking L/R controls) live in their own
    // depth band so they don't z-fight with the item-grid action planes (both would
    // otherwise resolve to DEPTH_ACTION and flicker as the ring rotates).
    let control_depth = if is_scrollbar {
        // Dedicated band — never coplanar with the panel it overlays (no z-fight).
        DEPTH_SCROLLBAR
    } else if action.is_some() && is_edge_button_rect(rect) {
        DEPTH_EDGE_BUTTON
    } else {
        panel_depth(rect, action.is_some())
    };
    let mut entity = ui.spawn((
        Name::new("control"),
        UiLayout::window()
            .x(Rl(rect.x))
            .y(Rl(rect.y))
            .width(Rl(rect.w))
            .height(Rh(rect.h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(control_depth, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        AmbitionMenuControl {
            kind,
            action,
            focus,
        },
        KaleidoscopeControlStyle {
            kind,
            important,
            disabled,
        },
        MenuVisualState {
            focused: selected,
            selected,
            disabled,
            ..Default::default()
        },
        KaleidoscopeFade { base_alpha },
    ));
    // Only controls on the active face are highlight-eligible (focus keys collide
    // across the cube's simultaneously-spawned faces).
    if active {
        entity.insert(KaleidoscopeActiveFaceControl);
    }
    // Feature C: a scrollbar is a DRAGGABLE control even with no `action` (it emits
    // `MenuScrollDragged` via the drag observers, not a click action). Tag it so the
    // projection system + drag observers can find it, and keep it pickable below
    // (the `is_scrollbar` exception to the `disabled` IGNORE rule).
    if is_scrollbar {
        entity.insert(MenuScrollbar::default());
        // Fix 1: draw the bright THUMB as a child of the dim track, sized + positioned
        // by the host-supplied fractions. Only when the list actually scrolls
        // (`size < 1`); a full-size thumb means the list fits, so no thumb is drawn.
        if let Some(thumb) = thumb {
            if thumb.size < 1.0 {
                entity.with_children(|children| {
                    spawn_scrollbar_thumb(children, materials, thumb, active);
                });
            }
        }
    }
    // Disabled controls never participate in picking. Enabled controls are pickable
    // only when the host wants Bevy picking (`pickable_controls`); a host with its
    // own manual hit-test (the demo) keeps them `Pickable::IGNORE`. A scrollbar is the
    // exception: it is pickable (for drag) whenever picking is on, action or not.
    if (disabled && !is_scrollbar) || !config.pickable_controls {
        entity.insert(Pickable::IGNORE);
    }
    let draw_corners = config.draw_selection_corners;
    // The icon image (if any) is tinted by the control's owned/selected/equipped
    // state so the same focus styling reads on the picture as on a text cell:
    // dim when disabled (un-owned), bright-gold-ish when selected, full white
    // otherwise. Equipped (`important`) keeps full brightness but the cell bg
    // already carries the equipped accent, so the icon stays crisp white.
    let icon_handle = icon.map(|path| asset_server.load::<Image>(path.to_string()));
    let icon_tint = if disabled {
        // Dim un-owned items (alpha + value drop), matching the dimmed text cell.
        Color::srgba(0.55, 0.58, 0.66, 0.55)
    } else if selected {
        Color::srgb(1.0, 0.95, 0.78)
    } else {
        Color::WHITE
    };
    entity.with_children(|children| {
        // Spawn the selection corners on every focusable (actionable, non-scrollbar)
        // cell, but HIDDEN — `sync_selection_corner_visuals` reveals the focused
        // control's set in place. (Pre-click-fix this was baked from `selected`; the
        // build is now cursor-independent so the cursor visual is applied at runtime.)
        if draw_corners && !disabled && !is_scrollbar {
            spawn_selection_corners(children, materials, active);
        }
        if let Some(icon_handle) = icon_handle {
            // An item icon REPLACES the cell's text label (the name moves to the
            // detail panel). Centred, inset inside the cell so the cell bg + the
            // selection accent stay visible as a frame around the picture.
            spawn_icon(children, materials, icon_handle, icon_tint, active);
            // Keep the short action hint (detail) below the icon if present.
            if let Some(detail) = detail {
                spawn_text(
                    children,
                    materials,
                    50.0,
                    86.0,
                    10.5,
                    detail,
                    TextAlign::Center,
                    Srgba::rgb_u8(185, 196, 210),
                    active,
                    None,
                );
            }
            return;
        }
        let main_size = match kind {
            MenuControlKind::Item => 20.0,
            // System option rows want a noticeably bigger label than a generic
            // action button (Fix 2): the System face shows few, tall rows, so a
            // larger Rh-relative font keeps them readable + centered.
            MenuControlKind::OptionToggle => 34.0,
            _ => 22.0,
        };
        spawn_text(
            children,
            materials,
            50.0,
            44.0,
            main_size,
            label,
            TextAlign::Center,
            Srgba::rgb_u8(242, 234, 200),
            active,
            None,
        );
        if let Some(detail) = detail {
            spawn_text(
                children,
                materials,
                50.0,
                76.0,
                10.5,
                detail,
                TextAlign::Center,
                Srgba::rgb_u8(185, 196, 210),
                active,
                None,
            );
        }
    });
}

/// Fix 1: render the bright scrollbar THUMB as a child of the dim track. The
/// thumb's geometry is given as track fractions (`0..=1`); since it is a child of
/// the track plane, its `window` layout is relative to the track (0..100%), so the
/// thumb spans the full track width with `y = start*100%` and `height = size*100%`.
/// It sits a hair in front of the track ([`DEPTH_SCROLLBAR_THUMB`]) so the two solid
/// planes never z-fight. `Pickable::IGNORE`: the track owns the drag.
fn spawn_scrollbar_thumb(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    thumb: ScrollThumb,
    active: bool,
) {
    let (y, size) = scrollbar_thumb_layout(thumb);
    let color = scrollbar_thumb_color();
    let base_alpha = color.alpha();
    let material = materials.add(StandardMaterial {
        base_color: fade_color(color, base_alpha),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new("scrollbar thumb"),
        KaleidoscopeFade { base_alpha },
        UiLayout::window()
            .x(Rl(0.0))
            .y(Rl(y * 100.0))
            .width(Rl(100.0))
            .height(Rh(size * 100.0))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(DEPTH_SCROLLBAR_THUMB, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

/// Fix 1: clamp the host thumb fractions into the renderable `(y, size)` track
/// fractions. `size` is floored to a grabbable minimum (8%) and capped at the full
/// track; `y` (the thumb top) is positioned across the REMAINING travel
/// (`1 - size`) so the thumb never overflows the track bottom. Pure for testing.

/// Render an item's icon as a textured plane inside a control cell.
///
/// An unlit, `AlphaMode::Blend`, double-sided (`cull_mode: None`) `StandardMaterial`
/// whose `base_color_texture` is the item sprite and whose `base_color` is the
/// owned/selected tint — so the sprite respects the same focus styling a text cell
/// would. The plane is centred and inset (`window` at 18..82%) so the cell bg and
/// the selection corner-brackets frame the picture. `Pickable::IGNORE`: the parent
/// control plane owns the click, the icon is pure decoration on top of it.
fn spawn_icon(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    image: Handle<Image>,
    tint: Color,
    active: bool,
) {
    let base_alpha = tint.alpha();
    let material = materials.add(StandardMaterial {
        base_color: tint,
        base_color_texture: Some(image),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    ui.spawn((
        Name::new("item icon"),
        KaleidoscopeFade { base_alpha },
        UiLayout::window()
            .x(Rl(18.0))
            .y(Rl(14.0))
            .width(Rl(64.0))
            .height(Rh(64.0))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        // Sit just in front of the cell background / selection accent, behind the
        // top text band so any overlaid hint stays readable.
        UiDepth::Set(page_depth(DEPTH_ICON, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

fn spawn_selection_corners(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    active: bool,
) {
    let color = Color::WHITE;
    let base_alpha = color.alpha();
    // PERF (2026-06-10): all 8 corner pieces are the SAME white plane, toggled
    // only by VISIBILITY (`sync_selection_corner_visuals`) and never recolored, so
    // they share ONE material handle instead of each `materials.add()`-ing its own
    // (8 -> 1 per control; ~190 -> 24 across the inventory grid — fewer
    // StandardMaterial assets + GPU bind groups).
    let material = materials.add(StandardMaterial {
        base_color: fade_color(color, base_alpha),
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: true,
        ..default()
    });
    let l = 23.0;
    let t = 6.0;
    let pieces = [
        (0.0, 0.0, l, t),
        (0.0, 0.0, t, l),
        (100.0 - l, 0.0, l, t),
        (100.0 - t, 0.0, t, l),
        (0.0, 100.0 - t, l, t),
        (0.0, 100.0 - l, t, l),
        (100.0 - l, 100.0 - t, l, t),
        (100.0 - t, 100.0 - l, t, l),
    ];
    for (x, y, w, h) in pieces {
        spawn_corner_piece(ui, material.clone(), x, y, w, h, base_alpha, active);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_corner_piece(
    ui: &mut ChildSpawnerCommands,
    material: Handle<StandardMaterial>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    base_alpha: f32,
    active: bool,
) {
    ui.spawn((
        Name::new("selection corner"),
        SelectionCorner,
        // Start hidden; `sync_selection_corner_visuals` reveals the corners of the
        // focused control in place (the page is built cursor-independent so clicks
        // survive — see the click-fix — so the cursor visual can't be baked here).
        Visibility::Hidden,
        KaleidoscopeFade { base_alpha },
        UiLayout::window()
            .x(Rl(x))
            .y(Rl(y))
            .width(Rl(w))
            .height(Rh(h))
            .anchor(Anchor::TOP_LEFT)
            .pack(),
        UiDepth::Set(page_depth(DEPTH_SELECTION, active)),
        UiMeshPlane3d,
        MeshMaterial3d(material),
        Pickable::IGNORE,
    ));
}

/// Draw the left/right page-navigation affordance buttons on a face (the L/R
/// "switch subscreen" arrows). Ported from the demo's per-face `add_edge_buttons`
/// (same rects/look), but decorative here: the lib is generic over the host's
/// `Action`, and the host already owns page cycling via input. They render the
/// affordance from ONE place so both the demo and the game show them.
fn spawn_nav_arrows(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    active: bool,
) {
    // Match the demo's edge-button placement and the unselected Action color.
    let bg = control_color(MenuControlKind::Action, false, false);
    let left = MenuRect::new(1.8, 43.5, 7.5, 13.0);
    let right = MenuRect::new(90.7, 43.5, 7.5, 13.0);
    spawn_panel_at_depth(
        ui,
        materials,
        left,
        bg,
        None::<Action0>,
        DEPTH_ACTION,
        active,
    );
    spawn_panel_at_depth(
        ui,
        materials,
        right,
        bg,
        None::<Action0>,
        DEPTH_ACTION,
        active,
    );
    let glyph = Srgba::rgb_u8(242, 234, 200);
    spawn_text(
        ui,
        materials,
        left.x + left.w * 0.5,
        left.y + left.h * 0.5,
        5.0,
        "<",
        TextAlign::Center,
        glyph,
        active,
        None,
    );
    spawn_text(
        ui,
        materials,
        right.x + right.w * 0.5,
        right.y + right.h * 0.5,
        5.0,
        ">",
        TextAlign::Center,
        glyph,
        active,
        None,
    );
}

fn spawn_cube_edge_frame(
    ui: &mut ChildSpawnerCommands,
    materials: &mut Assets<StandardMaterial>,
    active: bool,
) {
    let color = Color::srgba(0.80, 0.92, 1.0, 0.62);
    // Cube borders sit in their own deterministic depth band so they do not
    // shimmer against the page/panel edges while the cube rotates.
    spawn_panel_at_depth(
        ui,
        materials,
        MenuRect::new(0.0, 0.0, 100.0, 0.7),
        color,
        None::<Action0>,
        DEPTH_EDGE,
        active,
    );
    spawn_panel_at_depth(
        ui,
        materials,
        MenuRect::new(0.0, 99.3, 100.0, 0.7),
        color,
        None::<Action0>,
        DEPTH_EDGE,
        active,
    );
    spawn_panel_at_depth(
        ui,
        materials,
        MenuRect::new(0.0, 0.0, 0.7, 100.0),
        color,
        None::<Action0>,
        DEPTH_EDGE,
        active,
    );
    spawn_panel_at_depth(
        ui,
        materials,
        MenuRect::new(99.3, 0.0, 0.7, 100.0),
        color,
        None::<Action0>,
        DEPTH_EDGE,
        active,
    );
}

/// Zero-sized stand-in `Action` for non-interactive decoration spawns (edges).
#[derive(Clone)]
enum Action0 {}

fn page_depth(depth: f32, active: bool) -> f32 {
    if active {
        depth
    } else {
        depth * 0.28
    }
}

fn text_depth(y: f32) -> f32 {
    DEPTH_TEXT_TOP - (y.round() % 37.0) * 0.0008
}

/// `color`'s rgb with an explicit `alpha` (Feature B). Used so a control/panel
/// material starts at its design alpha and [`fade_kaleidoscope_materials`] can scale
/// that alpha by the open `amount` without losing the rgb.
pub(super) fn fade_color(color: Color, alpha: f32) -> Color {
    let s = color.to_srgba();
    Color::srgba(s.red, s.green, s.blue, alpha)
}
