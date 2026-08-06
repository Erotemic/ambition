//! The chase beacon's PRESCRIBED worldline — content geometry, not a body move.
//!
//! The causal-pursuit phase needs a target whose future is known exactly, so the
//! participant can be shown the difference between where its light says it is
//! and where a new signal has to be sent. That worldline is a straight line in
//! the laboratory chart, evaluated ANALYTICALLY from the phase's start event:
//!
//! ```text
//! pos(t) = start + velocity * (t - pursuit_start_time)
//! ```
//!
//! ⭐ **analytic rather than integrated, deliberately.** Accumulating
//! `pos += vel * dt` drifts, and the acceptance test asserts that the retarded
//! event the optics solver finds is null-separated from reception to within a
//! sampled tolerance. A worldline that is exactly the line the solver assumes is
//! the difference between a tolerance that measures the SOLVER and one that
//! measures the beacon's own float error.
//!
//! ⚠ **why this is its own file.** `engine.pose-writes-are-authority-only` and
//! its velocity twin forbid bare `body.pos = ` writes, because a body relocated
//! outside `transit_body` keeps contacts, a ledge anchor, model-private
//! attachment and a collapsed sweep that described where it used to be. The
//! beacon has NONE of those: it is a marker, a `BodyKinematics` and a `SimId`,
//! with no `MotionModel` and no contact clusters, so there is nothing to
//! reconcile and `transit_body` cannot even be called on it. The waiver is
//! therefore scoped to THIS FILE rather than to the demo — `lib.rs` moves the
//! traveler, who is a full actor, and must stay covered. It has already lost
//! that fix twice.
//!
//! ⛔ **do not put anything else in here.** A skip path is a hole in a check;
//! this one is only honest while the file's whole content is one prop's
//! prescribed geometry.

use ambition_platformer2d::actor as ae;
use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use ambition_platformer2d::relativity2d::{ActiveSpacetime2d, SpacetimeCoordinateTime2d};
use bevy::prelude::*;

use crate::{
    LaboratoryTwin, TwinTrackChaseBeacon, TwinTrackExperiment, TwinTrackPhase,
    PURSUIT_TARGET_START_X, PURSUIT_TARGET_START_Y, PURSUIT_TARGET_VELOCITY_X,
    PURSUIT_TARGET_VELOCITY_Y,
};

/// Where the beacon waits before the pursuit begins.
pub(crate) fn beacon_start() -> Vec2 {
    Vec2::new(PURSUIT_TARGET_START_X, PURSUIT_TARGET_START_Y)
}

/// The beacon's constant coordinate velocity during the pursuit.
pub(crate) fn beacon_velocity() -> Vec2 {
    Vec2::new(PURSUIT_TARGET_VELOCITY_X, PURSUIT_TARGET_VELOCITY_Y)
}

pub(crate) fn update_twintrack_chase_beacon(
    coordinate_time: Query<
        &SpacetimeCoordinateTime2d,
        (With<SessionRoot>, With<ActiveSpacetime2d>),
    >,
    experiment: Query<&TwinTrackExperiment, With<LaboratoryTwin>>,
    mut beacons: Query<&mut ae::BodyKinematics, With<TwinTrackChaseBeacon>>,
) {
    let (Ok(time), Ok(experiment), Ok(mut body)) = (
        coordinate_time.single(),
        experiment.single(),
        beacons.single_mut(),
    ) else {
        return;
    };
    let start = beacon_start();
    let velocity = beacon_velocity();
    match experiment.phase {
        // Parked at its start event. Written every tick rather than once, so a
        // rewind across the pursuit puts it back where the phase found it.
        TwinTrackPhase::Ready | TwinTrackPhase::DopplerLock | TwinTrackPhase::AwaitEcho => {
            body.pos = start;
            body.vel = Vec2::ZERO;
            body.facing = 1.0;
        }
        TwinTrackPhase::Pursuit => {
            let elapsed = (time.seconds - experiment.pursuit_start_time).max(0.0) as f32;
            body.pos = start + velocity * elapsed;
            body.vel = velocity;
            body.facing = velocity.x.signum();
        }
        // ⚠ it STOPS but does not return: the intercept event is a fact of the
        // run, and a beacon that snapped home would contradict the trace the
        // observatory is still drawing.
        TwinTrackPhase::Inbound | TwinTrackPhase::Complete => {
            body.vel = Vec2::ZERO;
        }
    }
}
