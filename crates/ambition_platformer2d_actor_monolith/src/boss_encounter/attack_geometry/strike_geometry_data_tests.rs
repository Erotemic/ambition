//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod strike_geometry_data_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::*;
use ambition_characters::brain::BossAttackProfile;

/// The ORIGINAL hardcoded `volumes_for_profile` arms, verbatim — the reference the
/// `StrikeRect` DATA table (fable §C6) must reproduce byte-for-byte.
fn reference(attack: &BossAttackProfile, origin: ae::Vec2, size: ae::Vec2) -> Vec<ae::Aabb> {
    match attack.move_id().as_str() {
        "floor_slam" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.5 + 22.0),
            ae::Vec2::new(size.x * 0.75, 18.0),
        )],
        "side_sweep" => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.50, 0.0),
                ae::Vec2::new(size.x * 0.25, size.y * 0.72),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.50, 0.0),
                ae::Vec2::new(size.x * 0.25, size.y * 0.72),
            ),
        ],
        "full_body_pulse" => vec![ae::Aabb::new(origin, size * 0.70)],
        "hazard_column" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, 0.0),
            ae::Vec2::new(size.x * 0.30, size.y * 1.80),
        )],
        "wing_sweep" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.08),
            ae::Vec2::new(size.x * 0.56, size.y * 0.42),
        )],
        "dive_lane" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.42),
            ae::Vec2::new(size.x * 0.22, size.y * 0.72),
        )],
        "broadside" => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.34, 0.0),
                ae::Vec2::new(size.x * 0.18, size.y * 0.84),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.34, 0.0),
                ae::Vec2::new(size.x * 0.18, size.y * 0.84),
            ),
        ],
        "hand_slam" => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.40, size.y * 0.25),
                ae::Vec2::new(size.x * 0.14, size.y * 0.60),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.40, size.y * 0.25),
                ae::Vec2::new(size.x * 0.14, size.y * 0.60),
            ),
        ],
        "hand_sweep" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.15),
            ae::Vec2::new(size.x * 0.85, size.y * 0.28),
        )],
        "head_descent" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.05),
            ae::Vec2::new(size.x * 0.32, size.y * 0.38),
        )],
        "converging_shockwave" => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.48),
            ae::Vec2::new(size.x * 0.90, size.y * 0.08),
        )],
        // Special (or any non-geometry key) carries no body-mounted volume.
        _ => Vec::new(),
    }
}

#[test]
fn strike_geometry_is_byte_identical_to_the_old_hardcoded_match() {
    let profiles = [
        BossAttackProfile::Strike("floor_slam".to_string()),
        BossAttackProfile::Strike("side_sweep".to_string()),
        BossAttackProfile::Strike("full_body_pulse".to_string()),
        BossAttackProfile::Strike("wing_sweep".to_string()),
        BossAttackProfile::Strike("dive_lane".to_string()),
        BossAttackProfile::Strike("broadside".to_string()),
        BossAttackProfile::Strike("hand_slam".to_string()),
        BossAttackProfile::Strike("hand_sweep".to_string()),
        BossAttackProfile::Strike("head_descent".to_string()),
        BossAttackProfile::Strike("converging_shockwave".to_string()),
        BossAttackProfile::Strike("hazard_column".to_string()),
        BossAttackProfile::Special("overfit_volley".to_string()),
    ];
    // Sweep a couple of origins + body sizes so the affine `factor*size + const`
    // resolve is checked across scales (FloorSlam's fixed 22/18 px terms must NOT
    // scale; every other factor must).
    for origin in [ae::Vec2::ZERO, ae::Vec2::new(120.0, -40.0)] {
        for size in [ae::Vec2::new(30.0, 48.0), ae::Vec2::new(64.0, 96.0)] {
            for p in &profiles {
                let got: Vec<ae::Aabb> = strike_geometry(&p.move_id())
                    .iter()
                    .map(|r| r.to_aabb(origin, size))
                    .collect();
                let want = reference(p, origin, size);
                assert_eq!(got.len(), want.len(), "{p:?} volume count");
                for (g, w) in got.iter().zip(want.iter()) {
                    assert_eq!(g.center(), w.center(), "{p:?} center @ size {size:?}");
                    assert_eq!(g.half_size(), w.half_size(), "{p:?} half @ size {size:?}");
                }
            }
        }
    }
}

/// §C6 "out of core": a boss AUTHORS its own strike rects in its behavior profile
/// (RON-loaded here from the fixture), and that override REPLACES the built-in
/// geometry for exactly that move — while every other profile keeps the built-in
/// table. This is the seam a second game's boss uses to supply strike shapes with
/// no edit to core's `strike_geometry`.
#[test]
fn an_authored_override_replaces_the_built_in_geometry_for_that_move() {
    use crate::boss_encounter::behavior::BossBehaviorProfile;

    let mut behavior = BossBehaviorProfile::from_data(
        crate::boss_encounter::test_boss_catalog(),
        "clockwork_warden",
    );
    let size = ae::Vec2::new(80.0, 80.0);
    let pos = ae::Vec2::new(200.0, 100.0);
    let origin = pos + behavior.attack_origin_offset;

    // Author a single bespoke rect for the floor_slam move — deliberately unlike
    // the built-in FloorSlam so the swap is unambiguous.
    let authored = StrikeRect::scaled(ae::Vec2::new(0.0, 1.0), ae::Vec2::new(0.40, 0.40));
    behavior
        .strike_geometry
        .insert("floor_slam".to_string(), vec![authored]);

    // FloorSlam now resolves to the AUTHORED rect, not the built-in slab.
    let slam = volumes_for_profile(
        &BossAttackProfile::Strike("floor_slam".to_string()),
        pos,
        size,
        &behavior,
    );
    assert_eq!(slam.len(), 1);
    assert_eq!(slam[0].center(), authored.to_aabb(origin, size).center());
    assert_eq!(
        slam[0].half_size(),
        authored.to_aabb(origin, size).half_size()
    );
    assert_ne!(
        slam[0].half_size(),
        FLOOR_SLAM[0].to_aabb(origin, size).half_size(),
        "the override must NOT equal the built-in FloorSlam geometry"
    );

    // A profile with NO authored override still uses the built-in table.
    let sweep = volumes_for_profile(
        &BossAttackProfile::Strike("side_sweep".to_string()),
        pos,
        size,
        &behavior,
    );
    assert_eq!(
        sweep.len(),
        2,
        "SideSweep keeps its built-in two-box geometry"
    );
    assert_eq!(
        sweep[0].center(),
        SIDE_SWEEP[0].to_aabb(origin, size).center()
    );
}
