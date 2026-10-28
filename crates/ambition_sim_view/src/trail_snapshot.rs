//! Trail geometry captured on the presentation clock.

use ambition_platformer2d_actor_monolith::avatar::trail::{PlayerTrail, TrailStatus};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use bevy::prelude::*;

/// The state that selects a trail strip's appearance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailStripState {
    Emitting,
    Closed,
    Collapsing,
    CollapsingSelfLoop,
}

/// One continuous strip in Bevy world coordinates, on its live room's band.
#[derive(Clone, Debug, PartialEq)]
pub struct TrailStrip {
    pub band: Option<usize>,
    pub points: Vec<Vec2>,
    pub state: TrailStripState,
}

/// Plain drawing facts for the current presentation frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrailSnapshot(pub Vec<TrailStrip>);

/// Capture and drawing can form one system pipe. The capture owns simulation
/// reads; the drawing consumes this value without access to trail state.
pub fn snapshot_emitted_trails(
    rooms: LiveRoomOf<ae::RoomGeometry>,
    trails: Query<(Entity, &PlayerTrail), With<PlayerEntity>>,
) -> TrailSnapshot {
    let mut strips = Vec::new();
    for (entity, trail) in &trails {
        let band = rooms.room_of(entity).and_then(|room| {
            ambition_platformer2d_shared_tangle::camera_layers::live_room_band(rooms.instances(), room)
        }).map(|layer| layer - ambition_platformer2d_shared_tangle::camera_layers::LIVE_ROOM_RENDER_LAYER_BASE);
        let Some(room) = rooms.of(entity) else {
            continue;
        };
        let to_bevy = |points: Vec<ae::Vec2>| {
            points
                .into_iter()
                .map(|point| {
                    ae::config::world_to_bevy(&room.0, point, ae::config::WORLD_Z_PLAYER - 0.1)
                        .truncate()
                })
                .collect()
        };
        let state = match trail.status {
            TrailStatus::Emitting => TrailStripState::Emitting,
            TrailStatus::Closed => TrailStripState::Closed,
            TrailStatus::Collapsing { .. } => TrailStripState::Collapsing,
        };
        for points in trail.render_polylines() {
            if points.len() >= 2 {
                strips.push(TrailStrip {
                    band,
                    points: to_bevy(points),
                    state,
                });
            }
        }
        for points in trail.collapsing_loop_polylines() {
            if points.len() >= 2 {
                strips.push(TrailStrip {
                    band,
                    points: to_bevy(points),
                    state: TrailStripState::CollapsingSelfLoop,
                });
            }
        }
    }
    TrailSnapshot(strips)
}

#[cfg(test)]
mod tests;
