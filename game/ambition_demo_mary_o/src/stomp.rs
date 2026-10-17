//! The ONE top/side contact authority for Mary-O's enemies, in the engine's
//! box type.
//!
//! The rule is `ambition_demo_mary_o_modules::stomp`, where the squash module
//! reads it too, so the native snake shells and a module can not disagree
//! about the same contact. This converts the boxes.
//!
//! Mary-O runs under screen gravity, so `+y` is DOWN: the player's feet are its
//! `max.y`, and a body's head is its `min.y`.

use ambition_platformer2d::engine_core as ae;

pub use ambition_demo_mary_o_modules::stomp::PlayerTouch;

fn bounds(aabb: ae::Aabb) -> ambition_demo_mary_o_modules::stomp::Bounds {
    ambition_demo_mary_o_modules::stomp::Bounds { min: [aabb.min.x, aabb.min.y], max: [aabb.max.x, aabb.max.y] }
}

/// Classify the player's contact with one body, or `None` if they are apart,
/// with the stomp band her pack states (`stomp.band`).
pub fn player_touch(body: ae::Aabb, player: ae::Aabb, player_vel: ae::Vec2) -> Option<PlayerTouch> {
    let band = crate::rules::mary_o_rules().stomp.band;
    ambition_demo_mary_o_modules::stomp::player_touch(bounds(body), bounds(player), [player_vel.x, player_vel.y], band)
}

#[cfg(test)]
mod tests;
