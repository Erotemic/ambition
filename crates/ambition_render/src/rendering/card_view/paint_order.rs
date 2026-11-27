//! Paint order for sprite cards in a 3D view.

use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::prelude::{
    App, Entity, IntoScheduleConfigs, Or, Query, Res, ResMut, Resource, With,
};
use bevy::render::render_phase::{sort_phase_system, ViewSortedRenderPhases};
use bevy::render::view::ExtractedView;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

use super::{
    CardHeldArt, CardPart, CardScenery, CardSpriteBody, CardTerrain, CardViewCamera,
    CAMERA_DISTANCE,
};

#[derive(Resource, Default)]
struct CardPaintOrder {
    cards: EntityHashSet,
    views: EntityHashSet,
}

pub(super) fn install(app: &mut App) {
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        let removed = render.remove_systems_in_set(
            Render,
            sort_phase_system::<Transparent3d>,
            ScheduleCleanupPolicy::RemoveSystemsOnly,
        );
        assert_eq!(removed.ok(), Some(1), "expected one Transparent3d sorter");
        render
            .init_resource::<CardPaintOrder>()
            .add_systems(ExtractSchedule, extract_cards)
            .add_systems(Render, sort_by_card_depth.in_set(RenderSystems::PhaseSort));
    }
}

fn extract_cards(
    cameras: Extract<Query<Entity, With<CardViewCamera>>>,
    cards: Extract<
        Query<
            Entity,
            Or<(
                With<CardPart>,
                With<CardHeldArt>,
                With<CardSpriteBody>,
                With<CardScenery>,
                With<CardTerrain>,
                With<super::labels::CardLabel>,
                With<super::world_mesh::CardWorldMesh>,
            )>,
        >,
    >,
    mut order: ResMut<CardPaintOrder>,
) {
    order.views.clear();
    order.views.extend(cameras.iter());
    order.cards.clear();
    if !order.views.is_empty() {
        order.cards.extend(cards.iter());
    }
}

/// Compare sprite planes at a shared point. A tilted view can otherwise
/// sort parts out of paint order because their mesh centers differ in X or Y.
fn sort_by_card_depth(
    order: Res<CardPaintOrder>,
    views: Query<&ExtractedView>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    for view in &views {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        if order.views.contains(&view.retained_view_entity.main_entity.id()) {
            let camera = view.world_from_view;
            let back = *camera.back();
            let center = (camera.translation() - back * CAMERA_DISTANCE).with_z(0.0);
            for item in phase.items.values_mut() {
                // The phase keeps the mesh center between frames. Write only
                // the sort key. The camera distance can hide small depth gaps.
                item.distance = match item.sorting_info {
                    TransparentSortingInfo3d::Sorted { mesh_center, depth_bias } => {
                        if order.cards.contains(&item.entity.1.id()) {
                            mesh_center.z * back.z + depth_bias
                        } else {
                            (mesh_center - center).dot(back) + depth_bias
                        }
                    }
                    TransparentSortingInfo3d::AlwaysOnTop => f32::NEG_INFINITY,
                }
            }
        } else {
            phase.recalculate_sort_keys(view);
        }
        phase.sort();
    }
}
