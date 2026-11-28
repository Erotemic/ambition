//! Paint order for sprite cards in a 3D view.

use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::prelude::{
    App, Component, Entity, IntoScheduleConfigs, Query, Res, ResMut, Resource, With,
};
use bevy::render::render_phase::{sort_phase_system, ViewSortedRenderPhases};
use bevy::render::view::ExtractedView;
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

use super::{scenery_paint_depth, CardScope, CardViewCamera, CAMERA_DISTANCE, CARD_BODY_STEP};
use crate::rendering::moving_platforms::FRONT_LAYER_Z;
use crate::rendering::vessel_order::{extract_vessel_orders, VesselDrawOrder, VesselOrders};

#[derive(Resource, Default)]
struct CardPaintOrder {
    depths: EntityHashMap<f32>,
    views: EntityHashSet,
    vessels: EntityHashMap<VesselDrawOrder>,
}

/// Order a transparent card in the scene.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(in crate::rendering) struct CardPaintDepth(pub(in crate::rendering) f32);

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
            .add_systems(ExtractSchedule, extract_cards.after(extract_vessel_orders))
            .add_systems(Render, sort_by_card_depth.in_set(RenderSystems::PhaseSort));
    }
}

fn extract_cards(
    cameras: Extract<Query<Entity, With<CardViewCamera>>>,
    cards: Extract<Query<(Entity, &CardPaintDepth, Option<&CardScope>)>>,
    vessel_orders: Option<Res<VesselOrders>>,
    mut order: ResMut<CardPaintOrder>,
) {
    order.views.clear();
    order.views.extend(cameras.iter());
    order.depths.clear();
    order.vessels.clear();
    if !order.views.is_empty() {
        for (card, depth, scope) in &cards {
            order.depths.insert(card, depth.0);
            if let Some(CardScope::Source(source)) = scope {
                if let Some(vessel) = vessel_orders.as_deref().and_then(|orders| orders.get(*source)) {
                    order.vessels.insert(card, vessel);
                }
            }
        }
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
                        let entity = item.entity.1.id();
                        if let Some(depth) = order.depths.get(&entity) {
                            depth * back.z + depth_bias
                        } else {
                            (mesh_center - center).dot(back) + depth_bias
                        }
                    }
                    TransparentSortingInfo3d::AlwaysOnTop => f32::NEG_INFINITY,
                }
            }
            if !order.vessels.is_empty() {
                let vessel_depth = scenery_paint_depth(FRONT_LAYER_Z, CARD_BODY_STEP) * back.z;
                phase.items.sort_by_key(|_, item| {
                    VesselDrawOrder::key(
                        order.vessels.get(&item.entity.1.id()).copied(),
                        vessel_depth,
                        item.distance,
                    )
                });
                continue;
            }
        } else {
            phase.recalculate_sort_keys(view);
        }
        phase.sort();
    }
}
