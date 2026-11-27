use bevy::camera::visibility::RenderLayers;
use bevy::gizmos::retained::Gizmo;
use bevy::prelude::{
    Commands, Component, DetectChanges, DetectChangesMut, Entity, GlobalTransform, Local, Query,
    Ref, With, Without,
};

use super::{CardScope, RetainedEntities};
use crate::rendering::retained_drawings::DrawingScope;

#[derive(Component)]
pub(super) struct CardDrawing;

/// Draw each retained line overlay from the same buffer on both backends.
pub(super) fn realize_card_drawings(
    mut commands: Commands,
    sources: Query<
        (Entity, Ref<Gizmo>, &GlobalTransform),
        (With<DrawingScope>, Without<CardDrawing>),
    >,
    mut cards: Query<(&mut Gizmo, &mut GlobalTransform), With<CardDrawing>>,
    mut retained: Local<RetainedEntities<Entity>>,
) {
    retained.reconcile(&mut commands, |source| sources.contains(*source));
    for (source, drawing, place) in &sources {
        let for_cards = || Gizmo {
            // Keep the lines visible over the scene in an oblique view.
            depth_bias: -1.0,
            ..Gizmo::clone(&drawing)
        };
        if let Some((mut gizmo, mut transform)) = retained
            .get(&source)
            .and_then(|entity| cards.get_mut(*entity).ok())
        {
            if drawing.is_changed() {
                *gizmo = for_cards();
            }
            transform.set_if_neq(*place);
        } else {
            let card = commands.spawn((
                CardDrawing,
                CardScope::Source(source),
                for_cards(),
                *place,
                RenderLayers::none(),
            )).id();
            retained.insert(source, card);
        }
    }
}
