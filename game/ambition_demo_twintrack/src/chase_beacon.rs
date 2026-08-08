//! Authored TwinTrack character worldlines.
//!
//! These bodies are non-controlled plaza participants following prescribed
//! content trajectories. They write the canonical body state once, before the
//! relativity clock producer samples it; controlled-body movement continues to
//! use the shared flight limb exclusively.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::relativity2d::SpacetimeCoordinateTime2d;
use bevy::prelude::*;

use crate::{
    LaboratoryTwin, TwinTrackCharacter, TwinTrackExperiment, TwinTrackTrajectory, TAGGER_ID,
};

pub(crate) fn update_twintrack_character_worldlines(
    coordinate_time: Query<&SpacetimeCoordinateTime2d>,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut characters: Query<(&TwinTrackCharacter, &mut ae::BodyKinematics)>,
) {
    let Ok(time) = coordinate_time.single() else {
        return;
    };
    let tag_speed_scale = experiment
        .single()
        .map(|state| 1.0 + 0.15 * state.tag_hits.min(2) as f32)
        .unwrap_or(1.0);
    let t = time.seconds as f32;
    for (character, mut body) in &mut characters {
        match character.trajectory {
            TwinTrackTrajectory::Stationary => {
                body.vel = Vec2::ZERO;
            }
            TwinTrackTrajectory::Orbit {
                center,
                radius,
                angular_speed,
                phase,
            } => {
                let speed_scale = if character.id == TAGGER_ID {
                    tag_speed_scale
                } else {
                    1.0
                };
                let angular_speed = angular_speed * speed_scale;
                let angle = phase + angular_speed * t;
                let radial = Vec2::from_angle(angle);
                body.pos = center + radial * radius;
                body.vel = Vec2::new(-radial.y, radial.x) * (angular_speed * radius);
                body.facing = body.vel.x.signum();
            }
        }
    }
}
