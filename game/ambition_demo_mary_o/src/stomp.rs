//! The ONE top/side contact authority for Mary-O's enemies.
//!
//! Every enemy rule in this demo turns on the same question: is the player ON this
//! body, or beside it? From the top it is a stomp — the snake shells, the AI Slop
//! squashes, and the player is ALWAYS safe. From a side it is a threat — contact
//! damage, or a running shell.
//!
//! That question is answered here, once, so the two enemies can never disagree about the same
//! contact.
//!
//! Mary-O runs under screen gravity, so `+y` is DOWN: the player's feet are its
//! `max.y`, and a body's head is its `min.y`.

use ambition_platformer2d::engine_core as ae;

/// Vertical tolerance (px) for "feet on its head": the band within which the
/// player's feet count as landing on top rather than hitting a side.
pub const STOMP_BAND: f32 = 16.0;

/// Which face of a body the player is touching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerTouch {
    /// Feet within [`STOMP_BAND`] of the head while not moving upward — coming
    /// down onto it, or already resting on it. Resting counts, and that is the
    /// whole point: a body under the player's feet must never hurt them.
    Top,
    /// Overlapping anywhere else — beside it, or rising into it from below.
    Side,
}

/// Classify the player's contact with one body, or `None` if they are apart.
pub fn player_touch(body: ae::Aabb, player: ae::Aabb, player_vel: ae::Vec2) -> Option<PlayerTouch> {
    let overlap_x = player.min.x < body.max.x && player.max.x > body.min.x;
    if !overlap_x {
        return None;
    }
    let feet = player.max.y;
    // The band is penetration tolerance, not reach: the player's feet must have
    // reached the head plane. Allowing `head - STOMP_BAND` classified a player
    // hovering up to sixteen pixels above an enemy as a stomp.
    //
    // Her feet on the same ground sat 9.48 below its head — comfortably inside 16 — and every
    // side run-in read as a stomp.
    //
    // Half the body is the rule every game of this kind actually uses: feet above
    // the enemy's middle is a stomp, below it is a hit. That is height-independent,
    // so a shorter enemy authored tomorrow cannot reopen this.
    let band = STOMP_BAND.min((body.max.y - body.min.y) * 0.5);
    let on_head = feet >= body.min.y && feet <= body.min.y + band;
    // Falling onto the head, or standing on it (`vel.y == 0`), is a stomp. Rising
    // INTO it from below is not — that is a hit, exactly like Mario.
    if on_head && player_vel.y >= 0.0 {
        return Some(PlayerTouch::Top);
    }
    let overlap_y = player.min.y < body.max.y && player.max.y > body.min.y;
    overlap_y.then_some(PlayerTouch::Side)
}

#[cfg(test)]
mod tests;
