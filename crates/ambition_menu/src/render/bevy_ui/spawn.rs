use super::*;
/// Background panels sort by size, like the cube's DEPTH_BACKGROUND /
/// LARGE_PANEL / CARD bands: a near-full-page panel is furthest back.
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
    font: Option<&bevy::text::FontSource>,
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
                text_node(*x, *y, *align),
                Text::new(text.clone()),
                TextColor(to_color(*color)),
                TextFont {
                    // `size` is percent of viewport height; `Vh` is that unit.
                    font_size: FontSize::Vh(*size),
                    font: font.cloned().unwrap_or_default(),
                    ..default()
                },
                TextLayout::justify(to_justify(*align)),
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
            // Spawned empty; the host fills it in place by `slot`, like the
            // cube renderer (cursor-dependent text needs no body rebuild).
            body.spawn((
                text_node(*x, *y, *align),
                Text::new(String::new()),
                TextColor(to_color(*color)),
                TextFont {
                    // `size` is percent of viewport height; `Vh` is that unit.
                    font_size: FontSize::Vh(*size),
                    font: font.cloned().unwrap_or_default(),
                    ..default()
                },
                TextLayout::justify(to_justify(*align)),
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
            detail,
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
                detail.as_deref(),
                icon.as_deref(),
                *selected,
                *important,
                action,
                *thumb,
                focused,
                assets,
                font,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Place an absolutely positioned text node so its alignment is honored. A
/// centered line spans its container and centers in it, and a right-aligned
/// line spans up to its anchor. Only `Left` treats `x` as a left edge.
pub(super) fn text_node(x: f32, y: f32, align: MenuTextAlign) -> Node {
    let (left, width) = match align {
        MenuTextAlign::Left => (Val::Percent(x), Val::Auto),
        MenuTextAlign::Center => (Val::Percent(0.0), Val::Percent(100.0)),
        MenuTextAlign::Right => (Val::Percent(0.0), Val::Percent(x)),
    };
    Node {
        position_type: PositionType::Absolute,
        left,
        width,
        top: Val::Percent(y),
        ..default()
    }
}

/// Spawn one interactive control. Tagged like the cube renderer, so the
/// host's picking and navigation map entity to action and focus the same way
/// in both backends.
fn spawn_control<Action>(
    body: &mut RelatedSpawnerCommands<ChildOf>,
    rect: MenuRect,
    kind: MenuControlKind,
    label: &str,
    // The value a settings row shows (for example a volume number), drawn
    // like the kaleidoscope backend does (`page.rs`).
    detail: Option<&str>,
    icon: Option<&str>,
    selected: bool,
    important: bool,
    action: &Option<Action>,
    thumb: Option<ScrollThumb>,
    focused_key: Option<crate::MenuFocusKey>,
    assets: Option<&AssetServer>,
    font: Option<&bevy::text::FontSource>,
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
    // Black text only on the bright gold highlight. The teal selected-only
    // background is dark, so it keeps light text.
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
        // The kind, not generic, so one restyle system serves every menu
        // whatever its action type.
        super::AmbitionMenuControlKind(kind),
        MenuVisualState {
            focused: focused || selected,
            selected,
            disabled,
            important,
            ..default()
        },
        Name::new(if is_scrollbar { "scrollbar" } else { "control" }),
    ));

    // An item cell with an icon draws the sprite (an `ImageNode`), like the
    // cube's `spawn_icon`, tinted by state: dim when disabled (not owned),
    // warm when selected, white otherwise. Without an icon or `AssetServer`
    // (headless tests) it draws the label.
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
            c.spawn((
                Text::new(label.to_string()),
                // A `Text` without `TextFont` gets Bevy's default as a
                // required component: the ASCII-only `FiraMono-subset.ttf`.
                TextFont {
                    font: font.cloned().unwrap_or_default(),
                    ..default()
                },
                TextColor(label_color),
            ));
        });
    }

    // The value, beside the label. Only when present, so rows without a
    // detail keep their layout.
    if let Some(detail) = detail.filter(|d| !d.is_empty()) {
        control.with_children(|c| {
            c.spawn((
                Node {
                    margin: UiRect::left(Val::Px(12.0)),
                    ..default()
                },
                Text::new(detail.to_string()),
                TextFont {
                    font: font.cloned().unwrap_or_default(),
                    ..default()
                },
                TextColor(label_color),
            ));
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
        // Draw a thumb only when the list scrolls (`size < 1`), as the cube
        // does.
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
                    // The track owns the drag (it has `BevyUiMenuScrollbar`
                    // and the handlers). Without `IGNORE`, a press on the
                    // thumb goes to the thumb entity and the drag never
                    // starts. Same as the cube thumb.
                    Pickable::IGNORE,
                    Name::new("scrollbar thumb"),
                ));
            });
        }
    }
}
