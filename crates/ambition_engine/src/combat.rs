//! Backend-neutral combat helpers.
//!
//! Bevy should render slash previews, play hit sounds, and spawn particles, but
//! the shape of an attack is game logic.  Keeping the hitbox computation here
//! lets tests and future headless validators reason about combat without a
//! renderer.

use crate::geometry::Aabb;
use crate::math::Vec2;
use crate::movement::Player;

/// Compute the current slash/pogo hitbox for a player.
///
/// `axis_y` follows `InputState`: negative means up, positive means down.
/// `forced_pogo` is used by layouts that expose downward slash/pogo as a
/// dedicated face-button verb rather than requiring down + attack.
pub fn slash_hitbox(player: &Player, axis_y: f32, forced_pogo: bool) -> Aabb {
    let body = player.aabb();
    if forced_pogo || axis_y > 0.25 {
        Aabb::new(
            Vec2::new(body.center.x, body.bottom() + 24.0),
            Vec2::new(body.half.x * 0.95, 26.0),
        )
    } else if axis_y < -0.25 {
        Aabb::new(
            Vec2::new(body.center.x, body.top() - 22.0),
            Vec2::new(body.half.x * 1.10, 24.0),
        )
    } else {
        Aabb::new(
            Vec2::new(body.center.x + player.facing * (body.half.x + 30.0), body.center.y - 2.0),
            Vec2::new(34.0, 24.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec2;

    #[test]
    fn forward_slash_is_in_front_of_facing_direction() {
        let mut player = Player::new(Vec2::new(100.0, 100.0));
        player.facing = 1.0;
        let right = slash_hitbox(&player, 0.0, false);
        player.facing = -1.0;
        let left = slash_hitbox(&player, 0.0, false);
        assert!(right.center.x > player.pos.x);
        assert!(left.center.x < player.pos.x);
    }
}
