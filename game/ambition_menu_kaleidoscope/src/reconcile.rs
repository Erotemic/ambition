//! Rewrite a live face in place so it draws a newly published page model.
//!
//! A face is a tree of planes and text meshes, and rebuilding it for a content
//! change costs more than the spawn: under Bevy 0.19 a freshly spawned solid
//! plane draws nothing on its first frame, so a respawned face showed its text
//! over an empty frame — the System face blinked on every scroll step. Here a
//! node whose value changed is patched on the entities already drawing it, and
//! only a node whose entity tree would differ (its [`NodeShape`]) is respawned.
//!
//! Each patch touches only what moved. Rewriting an unchanged `UiLayout` makes
//! Lunex rebuild the plane's mesh, and rewriting an unchanged `Text3d` reshapes
//! its glyphs, so every write below is guarded by the field it derives from.

use bevy::ecs::system::SystemParam;

use super::page::{
    active_depth, control_disabled, control_focus_key, control_icon_tint, control_plane_depth,
    control_visual_state, icon_material, plane_depth, rect_layout, solid_material, spawn_node,
    text_depth, text_layout, text_styling, thumb_layout, ControlParts, PlaneDepth, RenderedNode,
};
use super::*;

/// What entity tree a node spawns. Two values with the same shape differ only in
/// content a patch can rewrite; a different shape is a different tree.
#[derive(Clone, Copy, Debug, PartialEq)]
enum NodeShape {
    /// An actionable panel carries control components a plain one does not.
    Panel {
        actionable: bool,
    },
    Text,
    /// The slot is the host's handle on the line's content.
    DynamicText {
        slot: u32,
    },
    Control {
        kind: MenuControlKind,
        disabled: bool,
        icon: bool,
        detail: bool,
        thumb: bool,
    },
}

fn node_shape<Action>(node: &MenuNode<Action>) -> NodeShape {
    match node {
        MenuNode::Panel { action, .. } => NodeShape::Panel {
            actionable: action.is_some(),
        },
        MenuNode::Text { .. } => NodeShape::Text,
        MenuNode::DynamicText { slot, .. } => NodeShape::DynamicText { slot: *slot },
        MenuNode::Control {
            kind,
            detail,
            icon,
            action,
            thumb,
            ..
        } => NodeShape::Control {
            kind: *kind,
            disabled: control_disabled(*kind, action.is_some()),
            icon: icon.is_some(),
            detail: detail.is_some(),
            // `spawn_control` draws a thumb only for a list that scrolls.
            thumb: matches!(kind, MenuControlKind::Scrollbar)
                && thumb.is_some_and(|thumb| thumb.size < 1.0),
        },
    }
}

/// The live face entities a reconcile reads.
#[derive(SystemParam)]
pub struct LiveFaceNodes<'w, 's, Action: Send + Sync + 'static> {
    nodes: Query<
        'w,
        's,
        (
            Entity,
            &'static RenderedNode<Action>,
            &'static ChildOf,
            Option<&'static ControlParts>,
        ),
    >,
    materials_of: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    visuals: Query<'w, 's, &'static MenuVisualState>,
    depths: Query<'w, 's, &'static PlaneDepth>,
    children: Query<'w, 's, &'static Children>,
    controls: Query<'w, 's, (), With<AmbitionMenuControl<Action>>>,
}

/// Everything a reconcile writes through.
pub(super) struct FaceWriter<'a, 'w, 's> {
    pub(super) commands: &'a mut Commands<'w, 's>,
    pub(super) materials: &'a mut Assets<StandardMaterial>,
    pub(super) asset_server: &'a AssetServer,
    pub(super) config: &'a KaleidoscopeMenuConfig,
}

/// Bring the live `face`, last drawn from `rendered`, to draw `model` as
/// `active`, without replacing any entity whose node kept its shape.
pub(super) fn reconcile_face<PageId, Action>(
    out: &mut FaceWriter,
    live: &LiveFaceNodes<Action>,
    face: Entity,
    rendered: &RenderedFace<PageId, Action>,
    model: &MenuPageModel<PageId, Action>,
    active: bool,
) where
    Action: Clone + PartialEq + Send + Sync + 'static,
{
    if rendered.active != active {
        set_face_active(out.commands, live, face, active);
    }
    let mut drawn: Vec<Option<(Entity, &RenderedNode<Action>, Option<&ControlParts>)>> =
        vec![None; rendered.model.nodes.len()];
    for (entity, node, parent, parts) in &live.nodes {
        if parent.parent() == face && node.index < drawn.len() {
            drawn[node.index] = Some((entity, node, parts));
        }
    }
    for index in 0..drawn.len().max(model.nodes.len()) {
        let old = drawn.get(index).copied().flatten();
        let new = model.nodes.get(index);
        match (old, new) {
            (Some((_, old, _)), Some(new)) if old.node == *new => {}
            (Some((entity, old, parts)), Some(new)) if node_shape(&old.node) == node_shape(new) => {
                patch_node(out, live, entity, parts, &old.node, new, active);
                out.commands.entity(entity).insert(RenderedNode {
                    index,
                    node: new.clone(),
                });
            }
            (old, new) => {
                if let Some((entity, _, _)) = old {
                    out.commands.entity(entity).despawn();
                }
                if let Some(new) = new {
                    let (materials, asset_server, config) =
                        (&mut *out.materials, out.asset_server, out.config);
                    out.commands.entity(face).with_children(|ui| {
                        spawn_node(ui, materials, asset_server, config, index, new, active);
                    });
                }
            }
        }
    }
}

/// Move a face into or out of the active depth bands, and hand its controls the
/// active-face marker that makes them highlight-eligible, or take it back.
fn set_face_active<Action>(
    commands: &mut Commands,
    live: &LiveFaceNodes<Action>,
    face: Entity,
    active: bool,
) where
    Action: Send + Sync + 'static,
{
    for entity in live.children.iter_descendants(face) {
        if let Ok(depth) = live.depths.get(entity) {
            commands.entity(entity).insert(active_depth(*depth, active));
        }
        if live.controls.contains(entity) {
            if active {
                commands
                    .entity(entity)
                    .insert(KaleidoscopeActiveFaceControl);
            } else {
                commands
                    .entity(entity)
                    .remove::<KaleidoscopeActiveFaceControl>();
            }
        }
    }
}

/// Rewrite `entity`, drawn from `old`, to draw `new`. The two share a shape.
fn patch_node<Action>(
    out: &mut FaceWriter,
    live: &LiveFaceNodes<Action>,
    entity: Entity,
    parts: Option<&ControlParts>,
    old: &MenuNode<Action>,
    new: &MenuNode<Action>,
    active: bool,
) where
    Action: Clone + PartialEq + Send + Sync + 'static,
{
    match (old, new) {
        (
            MenuNode::Panel {
                rect: old_rect,
                color: old_color,
                action: old_action,
            },
            MenuNode::Panel {
                rect,
                color,
                action,
            },
        ) => {
            let mut plane = out.commands.entity(entity);
            if old_rect != rect {
                plane.insert(rect_layout(*rect));
                let depth = panel_depth(*rect, action.is_some());
                if depth != panel_depth(*old_rect, old_action.is_some()) {
                    plane.insert(plane_depth(depth, active));
                }
            }
            if old_action != action {
                if let Some(action) = action {
                    plane.insert(AmbitionMenuControl {
                        kind: MenuControlKind::Action,
                        action: Some(action.clone()),
                        focus: MenuFocusKey::default(),
                    });
                }
            }
            if old_color != color {
                let color = menu_color(*color);
                write_material(out.materials, live, entity, solid_material(color));
                out.commands.entity(entity).insert(KaleidoscopeFade {
                    base_alpha: color.alpha(),
                });
                // An actionable panel wears its control colour, which the focus
                // recolor derives from its visual state; wake it as a spawn would.
                if action.is_some() {
                    wake_visual_state(out.commands, live, entity);
                }
            }
        }
        (
            MenuNode::Text {
                x: old_x,
                y: old_y,
                size: old_size,
                text: old_text,
                align: old_align,
                color: old_color,
            },
            MenuNode::Text {
                x,
                y,
                size,
                text,
                align,
                color,
            },
        ) => {
            patch_text_placement(
                out.commands,
                entity,
                (*old_x, *old_y, *old_size),
                (*x, *y, *size),
                active,
            );
            patch_text_style(
                out.commands,
                entity,
                (*old_align, *old_color),
                (*align, *color),
            );
            if old_text != text {
                out.commands
                    .entity(entity)
                    .insert(Text3d::new(text.clone()));
            }
        }
        (
            MenuNode::DynamicText {
                x: old_x,
                y: old_y,
                size: old_size,
                align: old_align,
                color: old_color,
                ..
            },
            MenuNode::DynamicText {
                x,
                y,
                size,
                align,
                color,
                ..
            },
        ) => {
            // The content is the host's, by slot; only the placement is the page's.
            patch_text_placement(
                out.commands,
                entity,
                (*old_x, *old_y, *old_size),
                (*x, *y, *size),
                active,
            );
            patch_text_style(
                out.commands,
                entity,
                (*old_align, *old_color),
                (*align, *color),
            );
        }
        (
            MenuNode::Control {
                rect: old_rect,
                label: old_label,
                detail: old_detail,
                icon: old_icon,
                selected: old_selected,
                important: old_important,
                action: old_action,
                thumb: old_thumb,
                ..
            },
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
            },
        ) => {
            let parts = parts.copied().unwrap_or_default();
            let disabled = control_disabled(*kind, action.is_some());
            let mut plane = out.commands.entity(entity);
            if old_rect != rect {
                plane.insert(rect_layout(*rect));
                let depth = control_plane_depth(*rect, *kind, action.is_some());
                if depth != control_plane_depth(*old_rect, *kind, old_action.is_some()) {
                    plane.insert(plane_depth(depth, active));
                }
            }
            if old_rect != rect || old_action != action {
                plane.insert(AmbitionMenuControl {
                    kind: *kind,
                    action: action.clone(),
                    focus: control_focus_key(*rect),
                });
            }
            if old_selected != selected || old_important != important {
                // The recolor follows the visual state in place, as on a spawn.
                // Hover is the pointer's, and the pointer has not moved.
                let hovered = live.visuals.get(entity).is_ok_and(|state| state.hovered);
                plane.insert((
                    KaleidoscopeControlStyle {
                        kind: *kind,
                        important: *important,
                        disabled,
                    },
                    control_visual_state(*selected, disabled, hovered),
                ));
            }
            if let (Some(label_text), true) = (parts.label, old_label != label) {
                out.commands
                    .entity(label_text)
                    .insert(Text3d::new(label.clone()));
            }
            if let (Some(detail_text), Some(detail)) = (parts.detail, detail) {
                if old_detail.as_ref() != Some(detail) {
                    out.commands
                        .entity(detail_text)
                        .insert(Text3d::new(detail.clone()));
                }
            }
            if let (Some(icon_plane), Some(icon)) = (parts.icon, icon) {
                if old_icon.as_ref() != Some(icon) || old_selected != selected {
                    let tint = control_icon_tint(disabled, *selected);
                    let image = out.asset_server.load::<Image>(icon.clone());
                    write_material(out.materials, live, icon_plane, icon_material(image, tint));
                    out.commands.entity(icon_plane).insert(KaleidoscopeFade {
                        base_alpha: tint.alpha(),
                    });
                }
            }
            if let (Some(thumb_plane), Some(thumb)) = (parts.thumb, thumb) {
                if old_thumb.as_ref() != Some(thumb) {
                    out.commands
                        .entity(thumb_plane)
                        .insert(thumb_layout(*thumb));
                }
            }
        }
        _ => unreachable!("patch_node is only called for nodes of one shape"),
    }
}

fn patch_text_placement(
    commands: &mut Commands,
    entity: Entity,
    (old_x, old_y, old_size): (f32, f32, f32),
    (x, y, size): (f32, f32, f32),
    active: bool,
) {
    let mut text = commands.entity(entity);
    if (old_x, old_y) != (x, y) {
        text.insert(text_layout(x, y));
        if text_depth(old_y) != text_depth(y) {
            text.insert(plane_depth(text_depth(y), active));
        }
    }
    if old_size != size {
        text.insert(UiTextSize::from(Rh(size)));
    }
}

fn patch_text_style(
    commands: &mut Commands,
    entity: Entity,
    old: (MenuTextAlign, MenuColor),
    (align, color): (MenuTextAlign, MenuColor),
) {
    if old != (align, color) {
        commands.entity(entity).insert((
            text_styling(menu_srgba(color), menu_align(align)),
            KaleidoscopeFade {
                base_alpha: color.a,
            },
        ));
    }
}

/// Re-insert a control's visual state unchanged, so the in-place recolor runs
/// for it as it would for a freshly spawned control.
fn wake_visual_state<Action>(commands: &mut Commands, live: &LiveFaceNodes<Action>, entity: Entity)
where
    Action: Send + Sync + 'static,
{
    if let Ok(state) = live.visuals.get(entity) {
        commands.entity(entity).insert(*state);
    }
}

/// Write `material` into the asset `entity` already draws with, keeping the
/// handle: a new handle is a new material to prepare, and the plane would sit
/// out a frame. The fade sweep re-applies its alpha to the written value.
fn write_material<Action>(
    materials: &mut Assets<StandardMaterial>,
    live: &LiveFaceNodes<Action>,
    entity: Entity,
    material: StandardMaterial,
) where
    Action: Send + Sync + 'static,
{
    let Ok(handle) = live.materials_of.get(entity) else {
        return;
    };
    if let Some(mut current) = materials.get_mut(&handle.0) {
        *current = material;
    }
}
