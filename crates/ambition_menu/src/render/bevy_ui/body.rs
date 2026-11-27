//! Retain the active body while its model changes.

use bevy::ecs::system::SystemParam;

use super::spawn::{panel_order, text_node};
use super::*;

#[derive(Component)]
pub(super) struct RenderedBody<Action: Send + Sync + 'static> {
    pub(super) nodes: Vec<Option<(Entity, MenuNode<Action>)>>,
    pub(super) focused: Option<crate::MenuFocusKey>,
}

/// Access the live bodies of flat menus.
#[derive(SystemParam)]
pub struct BevyUiMenuPageBodies<'w, 's, Action: Send + Sync + 'static> {
    bodies: Query<'w, 's, (Entity, &'static ChildOf, &'static mut RenderedBody<Action>)>,
    panels: Query<'w, 's, &'static ChildOf, With<BevyUiMenuPanel>>,
    children: Query<'w, 's, &'static Children>,
}

impl<Action: Clone + PartialEq + Send + Sync + 'static> BevyUiMenuPageBodies<'_, '_, Action> {
    /// Update a body when its tabs, font, and focus key stay the same.
    pub fn update<PageId>(
        &mut self,
        commands: &mut Commands,
        root: Entity,
        view: &BevyUiMenuView<'_, PageId, Action>,
        assets: Option<&AssetServer>,
        font: Option<&bevy::text::FontSource>,
    ) -> bool {
        let Some((body, _, mut rendered)) = self.bodies.iter_mut().find(|(_, parent, _)| {
            self.panels.get(parent.parent()).is_ok_and(|panel| panel.parent() == root)
        }) else {
            return false;
        };
        if rendered.focused != view.focused {
            return false;
        }
        let mut old = std::mem::take(&mut rendered.nodes);
        let previous: Vec<_> = old.iter().map(|node| node.as_ref().map(|(_, model)| model)).collect();
        let matches = crate::render::match_page_nodes(&previous, &view.page.nodes);
        commands.entity(body).with_children(|body| {
            // Spawns need the body relationship. Updates use the same command buffer.
            for (node, matched) in view.page.nodes.iter().zip(matches) {
                let entity = match matched.and_then(|index| old[index].take()) {
                    Some((entity, before)) if before == *node => Some(entity),
                    Some((entity, before)) => {
                        if patch_node(&mut body.commands(), &self.children, entity, &before, node) {
                            Some(entity)
                        } else {
                            body.commands().entity(entity).despawn();
                            spawn_node(body, view.menu, node, view.focused, assets, font)
                        }
                    }
                    None => spawn_node(body, view.menu, node, view.focused, assets, font),
                };
                rendered.nodes.push(entity.map(|entity| (entity, node.clone())));
            }
        });
        for (entity, _) in old.into_iter().flatten() {
            commands.entity(entity).despawn();
        }
        true
    }
}

fn patch_node<Action: Clone + PartialEq + Send + Sync + 'static>(
    commands: &mut Commands,
    children: &Query<&Children>,
    entity: Entity,
    before: &MenuNode<Action>,
    after: &MenuNode<Action>,
) -> bool {
    match (before, after) {
        (
            MenuNode::Panel { rect: old_rect, color: old_color, action: old_action, layer: old_layer },
            MenuNode::Panel { rect, color, action, layer },
        ) if old_action.is_some() == action.is_some() => {
            let mut panel = commands.entity(entity);
            if rect != old_rect {
                panel.insert(node_from_rect(*rect));
            }
            if color != old_color {
                panel.insert(BackgroundColor(to_color(*color)));
            }
            let order = panel_order(rect, *layer);
            if order != panel_order(old_rect, *old_layer) {
                panel.insert(ZIndex(order));
            }
            if rect != old_rect || action != old_action {
                if let Some(action) = action {
                    panel.insert(AmbitionMenuControl {
                        kind: MenuControlKind::Action,
                        action: Some(action.clone()),
                        focus: focus_key_for(*rect),
                    });
                }
            }
            true
        }
        (
            MenuNode::Text { x: old_x, y: old_y, size: old_size, text: old_text, align: old_align, color: old_color },
            MenuNode::Text { x, y, size, text, align, color },
        ) => {
            let Ok(children) = children.get(entity) else { return false; };
            let Some(label) = children.first().copied() else { return false; };
            if (x, y, align) != (old_x, old_y, old_align) {
                commands.entity(entity).insert(text_node(*x, *y, *align));
            }
            let mut label = commands.entity(label);
            if align != old_align {
                label.insert(TextLayout::new(to_justify(*align), bevy::text::LineBreak::NoWrap));
            }
            if size != old_size {
                label.insert(super::text::MenuTextFit::line(*size));
            }
            if text != old_text {
                label.insert(Text::new(text.clone()));
            }
            if color != old_color {
                label.insert(TextColor(to_color(*color)));
            }
            true
        }
        _ => false,
    }
}
