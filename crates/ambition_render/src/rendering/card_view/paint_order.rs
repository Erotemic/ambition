//! Paint order for sprite cards in a 3D view.

use std::collections::HashMap;

use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::ecs::entity::EntityHashSet;
use bevy::prelude::{
    App, Entity, GlobalTransform, IntoScheduleConfigs, Or, Query, Res, ResMut, Resource, Vec2, With,
};
use bevy::render::render_phase::{sort_phase_system, ViewSortedRenderPhases};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

use super::{
    CardHeldArt, CardPart, CardPortal, CardScenery, CardSpriteBody, CardTerrain, CardViewCamera,
    CAMERA_DISTANCE,
};

#[derive(Resource, Default)]
struct CardPaintOrder {
    cards: EntityHashSet,
    view_centers: HashMap<Entity, Vec2>,
}

pub(super) fn install(app: &mut App) {
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .init_resource::<CardPaintOrder>()
            .add_systems(ExtractSchedule, extract_cards)
            .add_systems(
                Render,
                sort_by_card_depth
                    .in_set(RenderSystems::PhaseSort)
                    .before(sort_phase_system::<Transparent3d>),
            );
    }
}

fn extract_cards(
    cameras: Extract<Query<(Entity, &GlobalTransform), With<CardViewCamera>>>,
    cards: Extract<
        Query<
            Entity,
            Or<(
                With<CardPart>,
                With<CardHeldArt>,
                With<CardSpriteBody>,
                With<CardScenery>,
                With<CardTerrain>,
                With<CardPortal>,
                With<super::world_mesh::CardWorldMesh>,
            )>,
        >,
    >,
    mut order: ResMut<CardPaintOrder>,
) {
    order.view_centers.clear();
    order
        .view_centers
        .extend(cameras.iter().map(|(id, camera)| {
            (
                id,
                (camera.translation() - *camera.back() * CAMERA_DISTANCE).truncate(),
            )
        }));
    order.cards.clear();
    if !order.view_centers.is_empty() {
        order.cards.extend(cards.iter());
    }
}

/// Compare sprite planes at a shared point. A tilted view can otherwise
/// sort parts out of paint order because their mesh centers differ in X or Y.
fn sort_by_card_depth(
    order: Res<CardPaintOrder>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    for (view, phase) in phases.iter_mut() {
        let Some(center) = order.view_centers.get(&view.main_entity.id()) else {
            continue;
        };
        for item in phase.items.values_mut() {
            if let TransparentSortingInfo3d::Sorted { mesh_center, .. } = &mut item.sorting_info {
                if order.cards.contains(&item.entity.1.id()) {
                    *mesh_center = center.extend(mesh_center.z);
                }
            }
        }
    }
}
