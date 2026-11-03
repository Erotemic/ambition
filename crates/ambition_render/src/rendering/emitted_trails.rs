//! Retained drawings of emitted trail snapshots.

use ambition_sim_view::{TrailSnapshot, TrailStripState};
use bevy::prelude::*;

use super::retained_drawings::{DrawingScope, RetainedDrawings};

#[derive(Component, Default)]
pub(super) struct TrailDrawing;

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
    mut drawings: RetainedDrawings<TrailDrawing>,
    mut drawn: Local<TrailSnapshot>,
) {
    if *drawn == snapshot {
        return;
    }
    let rooms: std::collections::BTreeSet<_> = snapshot.0.iter().map(|strip| strip.room).collect();
    for room in rooms {
        drawings.draw(DrawingScope::Room(room), |picture| {
            for strip in snapshot.0.iter().filter(|strip| strip.room == room) {
                picture.linestrip_2d(strip.points.iter().copied(), color(strip.state));
            }
        });
    }
    drawings.finish();
    *drawn = snapshot;
}

#[cfg(test)]
mod tests;
