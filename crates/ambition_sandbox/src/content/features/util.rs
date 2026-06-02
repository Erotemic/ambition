use super::*;

pub(super) fn player_is_standing_on(player: ae::Aabb, platform: ae::Aabb) -> bool {
    let horizontally_overlaps =
        player.right() > platform.left() + 2.0 && player.left() < platform.right() - 2.0;
    let near_top = (player.bottom() - platform.top()).abs() <= 8.0;
    horizontally_overlaps && near_top
}

pub(super) fn room_spec_paths(
    room: &crate::rooms::RoomSpec,
) -> Vec<(String, crate::actor::KinematicPath)> {
    let mut paths: Vec<(String, crate::actor::KinematicPath)> = Vec::new();
    for spec in &room.kinematic_paths {
        paths.push((spec.id.clone(), spec.path.clone()));
        if spec.name != spec.id {
            paths.push((spec.name.clone(), spec.path.clone()));
        }
    }
    paths
}

// Note: the older `blocked` / `blocked_y` predicates lived here.
// They were ad-hoc collision tests used by enemy / NPC sweep code,
// and their OneWay handling did not differentiate above-vs-below
// approaches — a hostile NPC chasing the player could not drop
// through a one-way platform, breaking the chase. Both paths now
// route through `crate::engine_core::step_kinematic`, which mirrors
// the player's sweep semantics exactly. Don't reintroduce the
// old helpers; if a new caller needs collision-aware motion, add
// it through `KinematicBody`.

pub(super) fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

pub(super) fn approximately_same_aabb(a: ae::Aabb, b: ae::Aabb) -> bool {
    // Pogo-bounce routing matches an engine-reported orb AABB against
    // sandbox-side breakable AABBs. The two are derived from the same
    // entity placement so the values agree to floating-point tolerance,
    // but a tiny epsilon avoids spurious mismatches if a future codepath
    // recomputes one of the AABBs from rounded coordinates.
    let eps = 0.5;
    (a.center() - b.center()).length() <= eps && (a.half_size() - b.half_size()).length() <= eps
}

pub(super) fn midpoint(a: ae::Vec2, b: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

/// Pick the SFX bank entry for a hazard contact based on the hazard's
/// authored name. Substring match keeps this resilient to naming
/// drift (e.g. `Lava Pit` and `lava_pool` both resolve to lava splash)
/// without coupling the engine to an SFX-asset enum. Falls back to a
/// generic player-damage clip when no keyword matches.
///
/// Long-term, a typed `HazardKind` field on the engine-side
/// `DamageVolume` would let this dispatch happen on a real enum;
/// until then the substring set is short enough to grep.
pub(super) fn hazard_sfx_id(name: &str) -> ambition_sfx::SfxId {
    let n = name.to_ascii_lowercase();
    if n.contains("lava") {
        ambition_sfx::ids::HAZARD_LAVA_SPLASH
    } else if n.contains("acid") {
        ambition_sfx::ids::HAZARD_ACID_SPLASH
    } else if n.contains("electric") || n.contains("shock") {
        ambition_sfx::ids::HAZARD_ELECTRIC_ARC
    } else if n.contains("saw") {
        ambition_sfx::ids::HAZARD_SAW_HIT
    } else if n.contains("spike") || n.contains("thorn") {
        ambition_sfx::ids::HAZARD_SPIKE_HIT
    } else {
        ambition_sfx::ids::PLAYER_DAMAGE
    }
}

pub(super) trait SignumOr {
    fn signum_or(self, fallback: f32) -> f32;
}

impl SignumOr for f32 {
    fn signum_or(self, fallback: f32) -> f32 {
        if self.abs() <= 0.001 {
            fallback
        } else {
            self.signum()
        }
    }
}

#[cfg(test)]
mod util_tests {
    //! Small pure feature helpers: the toward-target clamp, the
    //! standing-on-platform predicate (horizontal overlap + top contact),
    //! keyword hazard-SFX dispatch, AABB epsilon-equality, midpoint, and
    //! the zero-safe signum.
    use super::*;

    fn aabb(cx: f32, cy: f32, hx: f32, hy: f32) -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(cx, cy), ae::Vec2::new(hx, hy))
    }

    #[test]
    fn approach_moves_toward_target_and_clamps() {
        assert_eq!(approach(0.0, 10.0, 3.0), 3.0);
        assert_eq!(approach(8.0, 10.0, 3.0), 10.0); // capped, no overshoot
        assert_eq!(approach(10.0, 0.0, 3.0), 7.0);
        assert_eq!(approach(2.0, 0.0, 3.0), 0.0); // floored
        assert_eq!(approach(5.0, 5.0, 1.0), 5.0); // already there
    }

    #[test]
    fn player_is_standing_on_requires_overlap_and_top_contact() {
        let platform = aabb(50.0, 100.0, 50.0, 10.0); // top = 90
        assert!(player_is_standing_on(aabb(50.0, 67.0, 14.0, 23.0), platform)); // bottom = 90
        // Far above -> no top contact.
        assert!(!player_is_standing_on(aabb(50.0, 0.0, 14.0, 23.0), platform));
        // Off to the side -> no horizontal overlap.
        assert!(!player_is_standing_on(aabb(300.0, 67.0, 14.0, 23.0), platform));
    }

    #[test]
    fn hazard_sfx_id_dispatches_by_keyword_case_insensitively() {
        use ambition_sfx::ids;
        assert_eq!(hazard_sfx_id("Lava Pit"), ids::HAZARD_LAVA_SPLASH);
        assert_eq!(hazard_sfx_id("acid_pool"), ids::HAZARD_ACID_SPLASH);
        assert_eq!(hazard_sfx_id("SHOCK coil"), ids::HAZARD_ELECTRIC_ARC);
        assert_eq!(hazard_sfx_id("buzz saw"), ids::HAZARD_SAW_HIT);
        assert_eq!(hazard_sfx_id("thorn bush"), ids::HAZARD_SPIKE_HIT);
        assert_eq!(hazard_sfx_id("mystery goo"), ids::PLAYER_DAMAGE); // fallback
    }

    #[test]
    fn approximately_same_aabb_tolerates_small_epsilon() {
        let a = aabb(10.0, 10.0, 5.0, 5.0);
        assert!(approximately_same_aabb(a, aabb(10.2, 9.9, 5.0, 5.1)));
        assert!(!approximately_same_aabb(a, aabb(20.0, 10.0, 5.0, 5.0)));
    }

    #[test]
    fn midpoint_averages_the_two_points() {
        assert_eq!(
            midpoint(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(10.0, 4.0)),
            ae::Vec2::new(5.0, 2.0)
        );
    }

    #[test]
    fn signum_or_falls_back_inside_the_deadband() {
        assert_eq!(0.0_f32.signum_or(1.0), 1.0);
        assert_eq!(0.0005_f32.signum_or(-1.0), -1.0);
        assert_eq!(5.0_f32.signum_or(9.0), 1.0);
        assert_eq!((-5.0_f32).signum_or(9.0), -1.0);
    }
}
