//! Retained drawings of emitted trail snapshots.

use ambition_sim_view::{TrailSnapshot, TrailStripState};
use bevy::prelude::*;

/// One retained drawing per live room band.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TrailDrawing(Option<usize>);

fn color(state: TrailStripState) -> Color {
    match state {
        TrailStripState::Emitting => Color::srgb(0.62, 0.47, 0.30),
        TrailStripState::Closed => Color::srgb(0.95, 0.78, 0.32),
        TrailStripState::Collapsing => Color::srgb(0.40, 0.32, 0.26),
        TrailStripState::CollapsingSelfLoop => Color::srgb(0.80, 0.56, 0.36),
    }
}

/// Rebuild drawings only when their strips change. An empty snapshot retires
/// the drawings when their trails or live rooms leave.
pub(super) fn draw_emitted_trails(
    In(snapshot): In<TrailSnapshot>,
    mut commands: Commands,
    drawings: Query<(Entity, &TrailDrawing, &bevy::gizmos::retained::Gizmo)>,
    mut assets: ResMut<Assets<bevy::gizmos::GizmoAsset>>,
    mut drawn: Local<TrailSnapshot>,
) {
    if *drawn == snapshot {
        return;
    }
    let mut by_band: std::collections::BTreeMap<Option<usize>, bevy::gizmos::GizmoAsset> =
        std::collections::BTreeMap::new();
    for strip in &snapshot.0 {
        by_band
            .entry(strip.band)
            .or_insert_with(bevy::gizmos::GizmoAsset::new)
            .linestrip_2d(strip.points.iter().copied(), color(strip.state));
    }
    for (entity, band, gizmo) in &drawings {
        match by_band.remove(&band.0) {
            Some(picture) => {
                let _ = assets.insert(gizmo.handle.id(), picture);
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (band, picture) in by_band {
        let mut drawing = commands.spawn((
            TrailDrawing(band),
            bevy::gizmos::retained::Gizmo {
                handle: assets.add(picture),
                ..default()
            },
            Name::new("player trail"),
        ));
        if let Some(position) = band {
            drawing.insert(bevy::camera::visibility::RenderLayers::none().with(
                ambition_platformer2d_shared_tangle::camera_layers::live_room_render_layer(
                    position,
                ),
            ));
        }
    }
    *drawn = snapshot;
}

#[cfg(test)]
mod tests;
