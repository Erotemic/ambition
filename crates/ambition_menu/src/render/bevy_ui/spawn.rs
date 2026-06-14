//! Flat (bevy_ui) node spawning: turns a `MenuNode` rect/control/text/icon into
//! `Node` entities + the panel-layer z-sort. Split out of the grid renderer;
//! `use super::*` reaches the shared color/layout helpers + marker components.

use super::*;
/// Background panels sort by size, like the cube's DEPTH_BACKGROUND / LARGE_PANEL /
/// CARD bands: a near-full-page panel is the furthest back, a small card nearer.
pub(super) fn panel_layer(rect: &MenuRect) -> i32 {
    if rect.w > 98.0 && rect.h > 98.0 {
        0
    } else if rect.w > 40.0 || rect.h > 35.0 {
        1
    } else {
        2
    }
}

/// Spawn one [`MenuNode`] into the body container.
pub(super) fn spawn_node<Action>(
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
                    // The thumb sits ON TOP of the track but the track owns the
                    // drag (it carries `BevyUiMenuScrollbar` + the press/drag
                    // handlers). Without this, grabbing the thumb — the natural
                    // drag target — sends `Pointer<Press>` to the thumb entity,
                    // the press handler's `get_mut(press.entity)` misses, and the
                    // drag never starts. `IGNORE` lets the pick fall through to
                    // the track (mirrors the cube thumb).
                    Pickable::IGNORE,
                    Name::new("scrollbar thumb"),
                ));
            });
        }
    }
}
