//! Projectile portal transit — a small, fast in-flight shot threads a portal
//! aperture (carrying rotated momentum) instead of detonating on the portal
//! wall. This mirrors [`ambition_portal::portal_teleport_ground_items`] (the
//! "small object passes through a portal pair" precedent) for the unified
//! projectile pipeline, fixing Jon's report that fireballs/laser bolts explode
//! on a portal as if it were a solid wall.
//!
//! The core ([`try_projectile_portal_transit`]) is **pure + deterministic** —
//! no Bevy, no RNG — so the transit geometry (does it cross? where does it pop
//! out? which way does momentum rotate?) is headless-testable. Both the player
//! and enemy projectile systems call it BEFORE their shared world-collision
//! step: if it transited, the shot is now at the exit portal and the caller
//! skips collision for the tick (so it neither explodes on the entry face nor
//! double-tests a hit).

use crate::portal::{find_portal, portal_transform_velocity, PlacedPortal};
use ambition_engine_core::{self as ae, AabbExt};

/// Margin (px) past the exit face so a transited shot clears the thin portal
/// plane and isn't immediately re-tested as "entering" the exit. Matches the
/// spirit of `ambition_portal`'s (crate-private) `portal_exit_clearance`: the
/// body's half-size projected onto the exit normal, plus a hair of margin.
const PROJECTILE_EXIT_MARGIN: f32 = 5.0;

/// If `kin` (a moving projectile body) has entered a portal whose partner
/// exists, map its position + velocity through to the exit portal and return
/// `true`. The caller should then SKIP its world-collision step this tick.
///
/// Only a shot moving INTO the wall (`vel · entry_normal < 0`, since the normal
/// points out of the wall into the room) transits — so a shot that just popped
/// out of the exit face, now flying *along* the exit normal, isn't immediately
/// pulled back in. Speed is preserved through the pair rotation, so momentum
/// carries (a fast shot stays fast).
pub fn try_projectile_portal_transit(
    kin: &mut ae::BodyKinematics,
    portals: &[PlacedPortal],
) -> bool {
    if kin.vel == ae::Vec2::ZERO {
        return false;
    }
    let half = kin.size * 0.5;
    let body = ae::Aabb::new(kin.pos, half);
    for enter in portals {
        // Heading away from (or parallel to) this portal's face → not entering.
        if kin.vel.dot(enter.normal) >= 0.0 {
            continue;
        }
        let Some(exit) = find_portal(portals, enter.channel.partner()) else {
            continue;
        };
        if body.strict_intersects(ae::Aabb::new(enter.pos, enter.half_extent)) {
            kin.vel = portal_transform_velocity(kin.vel, enter.normal, exit.normal);
            let clearance = half.dot(exit.normal.abs()) + PROJECTILE_EXIT_MARGIN;
            kin.pos = exit.pos + exit.normal * clearance;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portal::{PortalChannel, PortalGunColor};

    fn kin(pos: ae::Vec2, vel: ae::Vec2) -> ae::BodyKinematics {
        ae::BodyKinematics {
            pos,
            vel,
            size: ae::Vec2::new(8.0, 8.0),
            facing: 1.0,
        }
    }

    fn portal(channel: PortalChannel, pos: ae::Vec2, normal: ae::Vec2) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos,
            normal,
            // A body-height opening on a vertical wall (tall, thin).
            half_extent: ae::Vec2::new(6.0, 40.0),
        }
    }

    // Physical convention: a portal's `normal` points OUT of the wall into the
    // room, so a shot ENTERS a portal by flying *against* its normal. For a shot
    // travelling +x, the entry wall faces -x (Blue) and we put the exit wall
    // facing +x (Orange) so the shot pops back out flying +x.
    fn blue_facing_left() -> PlacedPortal {
        portal(
            PortalGunColor::BLUE.channel(),
            ae::Vec2::new(100.0, 0.0),
            ae::Vec2::new(-1.0, 0.0),
        )
    }
    fn orange_facing_right() -> PlacedPortal {
        portal(
            PortalGunColor::ORANGE.channel(),
            ae::Vec2::new(500.0, 0.0),
            ae::Vec2::new(1.0, 0.0),
        )
    }

    /// A shot flying +x into Blue (whose face looks -x) should pop out of Orange
    /// flying along Orange's face normal — speed preserved, repositioned just
    /// outside Orange — NOT exploding on Blue's wall.
    #[test]
    fn a_shot_entering_blue_pops_out_of_orange_with_rotated_momentum() {
        let portals = [blue_facing_left(), orange_facing_right()];

        // A shot whose AABB overlaps Blue's frame, flying +x into the wall.
        let mut k = kin(ae::Vec2::new(99.0, 0.0), ae::Vec2::new(360.0, 0.0));
        assert!(try_projectile_portal_transit(&mut k, &portals));

        // Speed preserved through the pair rotation.
        assert!((k.vel.length() - 360.0).abs() < 1e-3);
        // Exits flying OUT of Orange's face (along its +x normal).
        assert!(
            k.vel.dot(orange_facing_right().normal) > 0.0,
            "expected to exit flying along Orange's normal, got {:?}",
            k.vel
        );
        // Repositioned to just outside Orange, not left at Blue.
        assert!(k.pos.x > 400.0, "shot should be teleported near Orange");
    }

    #[test]
    fn a_shot_not_touching_any_portal_does_not_transit() {
        let portals = [blue_facing_left(), orange_facing_right()];
        let mut k = kin(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(360.0, 0.0));
        assert!(!try_projectile_portal_transit(&mut k, &portals));
        assert_eq!(k.pos, ae::Vec2::new(0.0, 0.0));
    }

    /// A shot overlapping the exit face but flying *out* of it (along the exit
    /// normal) must NOT be yanked back through — otherwise it ping-pongs.
    #[test]
    fn a_shot_leaving_the_exit_face_is_not_pulled_back() {
        let portals = [blue_facing_left(), orange_facing_right()];
        // Sitting on Orange's frame, flying +x (along Orange's normal, leaving).
        let mut k = kin(ae::Vec2::new(501.0, 0.0), ae::Vec2::new(360.0, 0.0));
        assert!(!try_projectile_portal_transit(&mut k, &portals));
    }

    #[test]
    fn a_lone_portal_without_its_partner_never_transits() {
        let mut k = kin(ae::Vec2::new(99.0, 0.0), ae::Vec2::new(360.0, 0.0));
        assert!(!try_projectile_portal_transit(
            &mut k,
            &[blue_facing_left()]
        ));
    }

    #[test]
    fn a_resting_shot_never_transits() {
        let portals = [blue_facing_left(), orange_facing_right()];
        let mut k = kin(ae::Vec2::new(99.0, 0.0), ae::Vec2::ZERO);
        assert!(!try_projectile_portal_transit(&mut k, &portals));
    }
}
