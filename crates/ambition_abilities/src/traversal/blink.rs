//! The blink rule: where a short teleport along a line ends, walls
//! permitting.
//!
//! The held blink is a procedural module (`ambition_content_modules::blink`)
//! that asks for a transit (`ambition.motion.transit`); the transit adapter
//! (`crate::extension::lower_transits`) ends it here. The actor teleport
//! intent ends here too.

use ambition_platformer2d_core::{self as ae, AabbExt};

/// Resolve a blink destination over `world`: teleport up to `distance` along the
/// unit `dir`, stopping a body-half (`half`, measured in the blink direction)
/// short of the first solid so the body never embeds, with a safety net that
/// falls back to `from` if the landing box would still overlap a solid.
///
/// The one teleport rule for every controller: the player's held-item blink
/// and any actor body that resolves a `blink` intent from its
/// `ActorControlFrame` call this (I2/I7), against the collision world it
/// occupies.
pub fn blink_target(
    world: &ae::World,
    from: ae::Vec2,
    dir: ae::Vec2,
    distance: f32,
    half: ae::Vec2,
) -> ae::Vec2 {
    // The pull-back uses the body's extent in the blink direction (half-height
    // for a vertical blink), or a diagonal blink embeds.
    let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
    let mut target = match ambition_platformer2d_core::cast::raycast_solids(
        world,
        from,
        dir,
        distance + margin,
        false,
    ) {
        Some((hit, _normal)) => hit - dir * margin,
        None => from + dir * distance,
    };
    // Safety net: the center ray can miss a wall the body's width would clip
    // (corners, grazing). If the landing box still overlaps a solid, stay at
    // the start.
    let landing = ae::Aabb::new(target, half);
    let embeds = world.blocks.iter().any(|b| {
        ae::collision_semantics::is_full_collision_surface(b.kind) && landing.strict_intersects(b.aabb)
    });
    if embeds {
        target = from;
    }
    target
}

#[cfg(test)]
mod tests;
