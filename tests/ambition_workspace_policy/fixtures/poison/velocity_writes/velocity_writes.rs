//! Poison fixture: the bare velocity-write shapes the authority guard must catch.
//!
//! ⛔ THE VELOCITY RULE CARRIES NINETEEN WAIVERS AND HAD NO POISON. Its pose
//! twin has had one since the day it was written; this half was enforced by
//! nothing but the absence of new violations, which is the same evidence a
//! deleted rule produces. Every entry below is one of the shapes `forbid` lists.

fn bare_stop(kin: &mut BodyKinematics) {
    kin.vel = Vec2::ZERO;
}

fn bare_impulse(clusters: &mut BodyClustersMut<'_>, push: Vec2) {
    clusters.kinematics.vel += push;
    clusters.kinematics.vel -= push;
}

fn bare_axis_write(kin: &mut BodyKinematics, launch: f32) {
    kin.vel.y = launch;
}

/// The receiver-name hole the rule's own rationale names: a `BodyKinematics`
/// bound as `body` is the spelling TwinTrack's traveler used.
fn bare_named_body(body: &mut BodyKinematics, push: Vec2) {
    body.vel += push;
}
