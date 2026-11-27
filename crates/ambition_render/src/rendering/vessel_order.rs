//! Draw overlapping vessels as separate assemblies.

use std::cmp::Reverse;

use ambition_platformer2d_shared_tangle::camera_layers::RIGGED_IMPOSTOR_LAYER;
use ambition_platformer2d_shared_tangle::lifecycle::PresentationOf;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::ecs::entity::EntityHashMap;
use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::math::FloatOrd;
use bevy::prelude::{
    App, ChildOf, Entity, IntoScheduleConfigs, Or, Query, Res, ResMut, Resource, With,
};
use bevy::render::render_phase::{sort_phase_system, ViewSortedRenderPhases};
use bevy::render::{Extract, ExtractSchedule, Render, RenderApp, RenderSystems};

use super::actors::rigged::{RiggedPartSlot, RiggedPresentationOf};
use super::moving_platforms::{SeatedInVessel, VesselPanel, FRONT_LAYER_Z};
use super::posed_hand::HeldArtOf;
use super::worn_gear_visuals::WornGearVisual;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VesselSlot {
    Rear,
    Rider(Reverse<usize>),
    Near,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VesselDrawOrder {
    assembly: usize,
    slot: VesselSlot,
}

impl VesselDrawOrder {
    pub(super) fn key(
        order: Option<Self>,
        assembly_depth: f32,
        original_depth: f32,
    ) -> (FloatOrd, Option<Self>, FloatOrd) {
        let scene_depth = if order.is_some() { assembly_depth } else { original_depth };
        (FloatOrd(scene_depth), order, FloatOrd(original_depth))
    }
}

#[derive(Resource, Default)]
pub(super) struct VesselOrders(EntityHashMap<VesselDrawOrder>);

impl VesselOrders {
    pub(super) fn get(&self, entity: Entity) -> Option<VesselDrawOrder> {
        self.0.get(&entity).copied()
    }
}

pub(super) fn install(app: &mut App) {
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        let removed = render.remove_systems_in_set(
            Render,
            sort_phase_system::<Transparent2d>,
            ScheduleCleanupPolicy::RemoveSystemsOnly,
        );
        assert_eq!(removed.ok(), Some(1), "expected one Transparent2d sorter");
        render
            .init_resource::<VesselOrders>()
            .add_systems(ExtractSchedule, extract_vessel_orders)
            .add_systems(Render, sort_vessels_2d.in_set(RenderSystems::PhaseSort));
    }
}

type Attachment = Or<(With<HeldArtOf>, With<WornGearVisual>, With<PresentationOf>)>;

pub(super) fn extract_vessel_orders(
    panels: Extract<Query<(Entity, &VesselPanel)>>,
    riders: Extract<Query<(Entity, &SeatedInVessel)>>,
    rig_owners: Extract<Query<&RiggedPresentationOf>>,
    parts: Extract<Query<(Entity, &ChildOf, &RenderLayers), With<RiggedPartSlot>>>,
    attachments: Extract<Query<
        (Entity, Option<&HeldArtOf>, Option<&WornGearVisual>, Option<&PresentationOf>),
        Attachment,
    >>,
    mut orders: ResMut<VesselOrders>,
) {
    orders.0.clear();
    let mut vessels: Vec<_> = panels.iter()
        .filter_map(|(_, panel)| panel.near.then_some(&panel.vessel))
        .collect();
    vessels.sort_unstable();
    vessels.dedup();
    let assembly = |vessel| {
        let index = vessels.binary_search(&vessel).ok()?;
        let room = vessels[index].room;
        // Use the ordinary layer bands for a single vessel in a room.
        (index.checked_sub(1).is_some_and(|before| vessels[before].room == room)
            || vessels.get(index + 1).is_some_and(|after| after.room == room))
            .then_some(index)
    };
    for (entity, panel) in &panels {
        if let Some(assembly) = assembly(&panel.vessel) {
            let slot = if panel.near { VesselSlot::Near } else { VesselSlot::Rear };
            orders.0.insert(entity, VesselDrawOrder { assembly, slot });
        }
    }
    if orders.0.is_empty() {
        return;
    }
    for (entity, rider) in &riders {
        if let Some(assembly) = assembly(&rider.vessel) {
            let slot = VesselSlot::Rider(Reverse(rider.slot));
            orders.0.insert(entity, VesselDrawOrder { assembly, slot });
        }
    }
    let atlas_layers = RenderLayers::layer(RIGGED_IMPOSTOR_LAYER);
    for (entity, parent, layers) in &parts {
        if layers.intersects(&atlas_layers) {
            continue;
        }
        if let Ok(root) = rig_owners.get(parent.parent()) {
            if let Some(order) = orders.get(root.0) {
                orders.0.insert(entity, order);
            }
        }
    }
    for (entity, held, gear, subject) in &attachments {
        let root = held.map(|held| held.0)
            .or_else(|| gear.map(|gear| gear.order.root))
            .or_else(|| subject.map(|subject| subject.0));
        if let Some(order) = root.filter(|&root| riders.contains(root)).and_then(|root| orders.get(root)) {
            orders.0.insert(entity, order);
        }
    }
}

fn sort_vessels_2d(
    orders: Res<VesselOrders>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
) {
    for phase in phases.values_mut() {
        if orders.0.is_empty() {
            phase.sort();
        } else {
            // Use the same key for all draws in one assembly. The remaining
            // fields order its panels, then its parts inside each slot.
            phase.items.sort_by_key(|_, item| {
                VesselDrawOrder::key(orders.get(item.entity.1.id()), FRONT_LAYER_Z, item.sort_key.0)
            });
        }
    }
}
