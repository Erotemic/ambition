//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

#[test]
fn shield_blocks_only_hits_from_the_faced_side() {
    let player = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    // Controlled body facing local-right (+1) under normal gravity.
    assert!(
        shield_blocks_hit(true, 1.0, player, player + ae::Vec2::new(50.0, 0.0), down),
        "guards a hit from local right"
    );
    assert!(
        !shield_blocks_hit(true, 1.0, player, player + ae::Vec2::new(-50.0, 0.0), down),
        "a hit from behind (local left) lands"
    );
    // Facing local-left (-1) flips it.
    assert!(
        shield_blocks_hit(true, -1.0, player, player + ae::Vec2::new(-50.0, 0.0), down),
        "guards a hit from local left"
    );
    assert!(
        !shield_blocks_hit(true, -1.0, player, player + ae::Vec2::new(50.0, 0.0), down),
        "a hit from behind (local right) lands"
    );
    // No shield held -> never blocks; neutral facing -> guards either side.
    assert!(
        !shield_blocks_hit(false, 1.0, player, player + ae::Vec2::new(50.0, 0.0), down),
        "no shield, no block"
    );
    assert!(
        shield_blocks_hit(true, 0.0, player, player + ae::Vec2::new(-50.0, 0.0), down),
        "neutral facing guards either side"
    );
}

#[test]
fn shield_side_test_uses_the_controlled_body_frame() {
    let player = ae::Vec2::new(100.0, 200.0);
    let right_gravity = ae::Vec2::new(1.0, 0.0);
    // With right gravity, local-right is world-up.
    assert!(
        shield_blocks_hit(
            true,
            1.0,
            player,
            player + ae::Vec2::new(0.0, -50.0),
            right_gravity,
        ),
        "facing local-right should guard the world-up side under right gravity"
    );
    assert!(
        !shield_blocks_hit(
            true,
            1.0,
            player,
            player + ae::Vec2::new(0.0, 50.0),
            right_gravity,
        ),
        "world-down is behind a body facing local-right under right gravity"
    );
}

fn test_health(hp: i32) -> BodyHealth {
    BodyHealth::new(ambition_characters::actor::Health::new(hp))
}

const TEST_FEEL: BodyHitFeel = BodyHitFeel {
    hit_flash: 0.16,
    damage_invuln_time: 0.2,
    block_hit_flash: 0.16,
    block_invuln_floor: 0.2,
};

const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

#[test]
fn resolver_ignores_a_hit_inside_the_i_frame_window() {
    let mut combat = BodyCombat {
        damage_invuln_timer: 0.1,
        hit_flash: 0.5, // pre-poison: an Ignored hit must not touch state
        ..Default::default()
    };
    let mut health = test_health(5);
    let pos = ae::Vec2::new(100.0, 200.0);
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        false,
        1.0,
        pos,
        pos + ae::Vec2::new(50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(res, BodyHitResolution::Ignored);
    assert_eq!(health.current(), 5, "ignored hit deals no damage");
    assert_eq!(combat.hit_flash, 0.5, "ignored hit arms nothing");
}

#[test]
fn resolver_ignores_a_hit_on_a_dead_body() {
    let mut combat = BodyCombat::default();
    let mut health = test_health(5);
    health.damage(5);
    let pos = ae::Vec2::new(100.0, 200.0);
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        false,
        1.0,
        pos,
        pos + ae::Vec2::new(50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(res, BodyHitResolution::Ignored);
}

#[test]
fn resolver_shield_blocks_a_faced_hit_and_arms_the_guard_i_frame() {
    let mut combat = BodyCombat::default();
    let mut health = test_health(5);
    let pos = ae::Vec2::new(100.0, 200.0);
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        true,
        1.0,
        pos,
        pos + ae::Vec2::new(50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(res, BodyHitResolution::Blocked);
    assert_eq!(health.current(), 5, "a blocked hit deals no damage");
    assert!(
        combat.damage_invuln_timer >= TEST_FEEL.block_invuln_floor,
        "block arms the guard i-frame"
    );
    assert_eq!(combat.hit_flash, TEST_FEEL.block_hit_flash);
    // A hit from BEHIND the guard still lands.
    let mut combat = BodyCombat::default();
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        true,
        1.0,
        pos,
        pos + ae::Vec2::new(-50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 3,
            died: false
        }
    );
}

#[test]
fn resolver_scales_damage_arms_feel_and_floors_at_one() {
    let mut combat = BodyCombat::default();
    let mut health = test_health(10);
    let pos = ae::Vec2::new(0.0, 0.0);
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        3,
        2.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 6,
            died: false
        }
    );
    assert_eq!(health.current(), 4);
    assert_eq!(combat.hit_flash, TEST_FEEL.hit_flash);
    assert_eq!(combat.damage_invuln_timer, TEST_FEEL.damage_invuln_time);
    // A landed hit always deals at least 1 (assist can't zero it out).
    let mut combat = BodyCombat::default();
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        1,
        0.1,
        false,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 1,
            died: false
        }
    );
}

#[test]
fn resolver_reports_death_and_never_dies_takes_no_damage() {
    let mut combat = BodyCombat::default();
    let mut health = test_health(2);
    let pos = ae::Vec2::new(0.0, 0.0);
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        5,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 5,
            died: true
        }
    );
    assert!(!health.alive());
    // A `never_dies` body (training dummy) registers the hit but its HP
    // never moves.
    let mut combat = BodyCombat::default();
    let mut health = test_health(2);
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        5,
        1.0,
        true,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 5,
            died: false
        }
    );
    assert_eq!(health.current(), 2);
    // A headless body with no health component is damaged-but-undying.
    let mut combat = BodyCombat::default();
    let res = resolve_body_hit(
        &mut combat,
        None,
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        5,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 5,
            died: false
        }
    );
}

#[test]
fn knockback_impulse_is_frame_equivalent() {
    let feel = SandboxFeelTuning::default();
    let local_expected = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y);
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let source_pos = victim_pos - frame.side * 40.0;
        let knockback = crate::combat::HitKnockback {
            dir: 0.0,
            strength: 1.0,
            source_pos,
            impact_pos: victim_pos,
            launch_dir: None,
        };
        let vel = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            gravity_dir,
            false,
            Some(&knockback),
            ae::Vec2::ZERO,
            feel,
        );
        let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
        assert!(
            (local_vel - local_expected).length() < 1e-3,
            "knockback should resolve in local side/down for {gravity_dir:?}: {local_vel:?}"
        );
    }
}

// --- CM1: knockback scaling (the smash-percent axis) ---

#[test]
fn scaled_knockback_is_parity_at_zero_growth() {
    // growth == 0 returns the flat base for ANY damage/weight — the
    // byte-parity pin that keeps every un-authored volume unchanged.
    for dmg in [0, 5, 50, 999] {
        for w in [0.5, 1.0, 4.0] {
            assert_eq!(scaled_knockback(7.5, 0.0, dmg, w), 7.5);
        }
    }
}

#[test]
fn scaled_knockback_grows_with_damage_and_divides_by_weight() {
    // base + growth * damage / weight.
    assert_eq!(scaled_knockback(10.0, 2.0, 0, 1.0), 10.0);
    assert_eq!(scaled_knockback(10.0, 2.0, 30, 1.0), 70.0);
    // Twice the weight -> half the growth contribution.
    assert_eq!(scaled_knockback(10.0, 2.0, 30, 2.0), 40.0);
    // Monotonic in accumulated damage.
    assert!(scaled_knockback(10.0, 2.0, 60, 1.0) > scaled_knockback(10.0, 2.0, 30, 1.0));
    // Degenerate weight falls back to the reference body (never divides by 0).
    assert_eq!(scaled_knockback(10.0, 2.0, 10, 0.0), 30.0);
}

#[test]
fn scaled_knockback_conjugates_under_rotated_gravity() {
    // C4: a growth-scaled hit under rotated gravity produces the conjugated
    // trajectory — the scalar scaling is frame-agnostic, so the resolved
    // velocity stays identical in the victim's local frame under every
    // gravity, exactly like the flat case.
    let feel = SandboxFeelTuning::default();
    let strength = scaled_knockback(1.0, 0.05, 80, 1.25); // == 4.2
    let local_expected = ae::Vec2::new(
        feel.enemy_knockback_x * strength,
        -feel.enemy_knockback_y * strength,
    );
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let source_pos = victim_pos - frame.side * 40.0;
        let knockback = crate::combat::HitKnockback {
            dir: 0.0,
            strength,
            source_pos,
            impact_pos: victim_pos,
            launch_dir: None,
        };
        let vel = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            gravity_dir,
            false,
            Some(&knockback),
            ae::Vec2::ZERO,
            feel,
        );
        let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
        assert!(
            (local_vel - local_expected).length() < 1e-3,
            "growth-scaled knockback must conjugate for {gravity_dir:?}: {local_vel:?}"
        );
    }
}

// --- CM1: the authored launch DIRECTION (smash-style fixed angles) ---

#[test]
fn authored_launch_dir_sets_the_angle_and_keeps_the_default_speed() {
    let feel = SandboxFeelTuning::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0); // hit from local left
    let default_speed = ae::Vec2::new(feel.enemy_knockback_x, feel.enemy_knockback_y).length();

    // A pure up-launcher: (0, 1) launches straight against gravity.
    let up = crate::combat::HitKnockback {
        dir: 0.0,
        strength: 1.0,
        source_pos,
        impact_pos: victim_pos,
        launch_dir: Some(ae::Vec2::new(0.0, 1.0)),
    };
    let vel = resolved_body_knockback_velocity(
        victim_pos,
        1.0,
        down,
        false,
        Some(&up),
        ae::Vec2::ZERO,
        feel,
    );
    assert!(
        vel.x.abs() < 1e-3 && vel.y < 0.0,
        "a (0,1) launcher throws straight up (world -y): {vel:?}"
    );
    assert!(
        (vel.length() - default_speed).abs() < 1e-3,
        "the authored angle keeps the feel-tuned SPEED: |{vel:?}| vs {default_speed}"
    );

    // The lateral component mirrors to point AWAY from the source: hit
    // from the left ⇒ positive local x ⇒ world +x.
    let diag = crate::combat::HitKnockback {
        dir: 0.0,
        strength: 1.0,
        source_pos,
        impact_pos: victim_pos,
        launch_dir: Some(ae::Vec2::new(1.0, 1.0)),
    };
    let vel = resolved_body_knockback_velocity(
        victim_pos,
        1.0,
        down,
        false,
        Some(&diag),
        ae::Vec2::ZERO,
        feel,
    );
    assert!(
        vel.x > 0.0 && vel.y < 0.0,
        "a (1,1) launcher throws up-and-away from the source: {vel:?}"
    );
    // Mirrored source ⇒ mirrored lateral, same rise.
    let mirrored = crate::combat::HitKnockback {
        source_pos: victim_pos + ae::Vec2::new(40.0, 0.0),
        ..diag
    };
    let mvel = resolved_body_knockback_velocity(
        victim_pos,
        1.0,
        down,
        false,
        Some(&mirrored),
        ae::Vec2::ZERO,
        feel,
    );
    assert!(
        (mvel.x + vel.x).abs() < 1e-3 && (mvel.y - vel.y).abs() < 1e-3,
        "the authored angle mirrors with the away-from-source side: {vel:?} vs {mvel:?}"
    );
}

#[test]
fn authored_launch_dir_conjugates_under_rotated_gravity() {
    // C4: the authored angle is a LOCAL-frame fact, so the resolved
    // velocity is identical in the victim's side/down frame under every
    // gravity — the same conjugation invariant the flat + growth paths pin.
    let feel = SandboxFeelTuning::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let speed = ae::Vec2::new(feel.enemy_knockback_x, feel.enemy_knockback_y).length();
    let n = ae::Vec2::new(0.6, 0.8); // already unit-length
    let local_expected = ae::Vec2::new(n.x * speed, -n.y * speed);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let source_pos = victim_pos - frame.side * 40.0;
        let knockback = crate::combat::HitKnockback {
            dir: 0.0,
            strength: 1.0,
            source_pos,
            impact_pos: victim_pos,
            launch_dir: Some(n),
        };
        let vel = resolved_body_knockback_velocity(
            victim_pos,
            1.0,
            gravity_dir,
            false,
            Some(&knockback),
            ae::Vec2::ZERO,
            feel,
        );
        let local_vel = ae::Vec2::new(vel.dot(frame.side), vel.dot(frame.down));
        assert!(
            (local_vel - local_expected).length() < 1e-3,
            "authored launch must conjugate for {gravity_dir:?}: {local_vel:?}"
        );
    }
}

#[test]
fn zero_length_launch_dir_falls_back_to_the_default_diagonal() {
    // A degenerate authored vector (bad data) must not NaN the launch —
    // it reads as un-authored.
    let feel = SandboxFeelTuning::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0);
    let base = crate::combat::HitKnockback {
        dir: 0.0,
        strength: 1.0,
        source_pos,
        impact_pos: victim_pos,
        launch_dir: None,
    };
    let degenerate = crate::combat::HitKnockback {
        launch_dir: Some(ae::Vec2::ZERO),
        ..base
    };
    let expected = resolved_body_knockback_velocity(
        victim_pos,
        1.0,
        down,
        false,
        Some(&base),
        ae::Vec2::ZERO,
        feel,
    );
    let got = resolved_body_knockback_velocity(
        victim_pos,
        1.0,
        down,
        false,
        Some(&degenerate),
        ae::Vec2::ZERO,
        feel,
    );
    assert_eq!(expected, got);
}

#[test]
fn death_policy_gates_the_meter_kill() {
    use crate::combat::DeathPolicy;
    // HpDepleted (default) kills at the meter's max; Unbounded (smash
    // percent) never does — its death comes from the blast-zone gate.
    assert!(DeathPolicy::default().kills_at_max());
    assert!(DeathPolicy::HpDepleted.kills_at_max());
    assert!(!DeathPolicy::Unbounded.kills_at_max());
}

#[test]
fn damage_taken_is_the_accumulated_meter() {
    let mut h = test_health(20);
    assert_eq!(h.damage_taken(), 0);
    h.damage(7);
    assert_eq!(h.damage_taken(), 7);
    h.damage(100); // clamps at the pool max
    assert_eq!(h.damage_taken(), 20);
}

// --- CM2: directional influence ---

#[test]
fn di_is_inert_at_zero_budget_or_null_input() {
    let launch = ae::Vec2::new(300.0, -400.0);
    let down = ae::Vec2::new(0.0, 1.0);
    // Zero budget -> no DI, whatever the input.
    assert_eq!(
        di_adjust(launch, ae::Vec2::new(1.0, 0.0), down, 0.0),
        launch
    );
    // Null input -> no DI, even with a budget.
    assert_eq!(di_adjust(launch, ae::Vec2::ZERO, down, 0.35), launch);
    // Zero-length launch (no knockback) is left alone.
    assert_eq!(
        di_adjust(ae::Vec2::ZERO, ae::Vec2::new(1.0, 0.0), down, 0.35),
        ae::Vec2::ZERO
    );
}

#[test]
fn di_rotates_toward_held_input_bounded_by_the_budget() {
    let down = ae::Vec2::new(0.0, 1.0);
    // Launch straight "up" (world -y); hold fully perpendicular (local +x =
    // world +x). Speed is preserved and the vector rotates by exactly the
    // budget (perpendicular input, full throttle).
    let launch = ae::Vec2::new(0.0, -100.0);
    let max = 0.30_f32;
    let out = di_adjust(launch, ae::Vec2::new(1.0, 0.0), down, max);
    assert!((out.length() - 100.0).abs() < 1e-3, "DI preserves speed");
    let ang = (out.x / out.length()).asin(); // angle off vertical toward +x
    assert!(
        (ang - max).abs() < 1e-3,
        "rotates by the full budget: {ang}"
    );
    // Holding INTO the launch line (parallel) cannot DI — no rotation.
    let parallel = di_adjust(launch, ae::Vec2::new(0.0, -1.0), down, max);
    assert!(
        (parallel - launch).length() < 1e-3,
        "cannot DI along the launch"
    );
}

#[test]
fn di_conjugates_under_rotated_gravity() {
    // C4: the SAME local input under rotated gravity yields the conjugated
    // launch — DI is frame-agnostic, so the victim-local outgoing angle is
    // identical under every gravity.
    let max = 0.28_f32;
    let di_local = ae::Vec2::new(1.0, 0.0); // hold local-side
    let local_launch = ae::Vec2::new(0.0, -100.0); // straight up, body-local
    let mut expected_local: Option<ae::Vec2> = None;
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let launch_world = frame.to_world(local_launch);
        let out = di_adjust(launch_world, di_local, gravity_dir, max);
        let out_local = ae::Vec2::new(out.dot(frame.side), out.dot(frame.down));
        match expected_local {
            None => expected_local = Some(out_local),
            Some(e) => assert!(
                (out_local - e).length() < 1e-3,
                "DI must conjugate for {gravity_dir:?}: {out_local:?} vs {e:?}"
            ),
        }
    }
}

/// A3 armor-on-hit, through the ONE victim-side resolver: Mary-O's mushroom
/// big→small. The first hit is ABSORBED (the row downgrades, HP untouched, the
/// normal i-frames armed); the second — once the armor-less small row is all
/// that's worn — reaches HP. This is the exit-test assertion "one hit downgrades,
/// second hit damages HP".
#[test]
fn a3_worn_armor_absorbs_a_hit_downgrades_then_the_next_hit_damages_hp() {
    use ambition_characters::equipment::{EquipmentRow, OnHit, WornEquipment};

    let small = EquipmentRow {
        id: "mushroom_small".to_string(),
        ..Default::default()
    };
    let mut worn = WornEquipment::new(vec![EquipmentRow {
        id: "mushroom_big".to_string(),
        on_hit: Some(OnHit::ConsumeAsArmor {
            downgrade_to: Some(Box::new(small)),
        }),
        ..Default::default()
    }]);
    let mut combat = BodyCombat::default();
    let mut health = test_health(10);
    let pos = ae::Vec2::new(0.0, 0.0);

    // First hit: the mushroom absorbs it. Zero HP loss, the row downgrades to
    // small, and the SAME brief i-frames any hit arms are armed.
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        Some(&mut worn),
        false,
        1.0,
        pos,
        pos,
        DOWN,
        4,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(res, BodyHitResolution::Armored);
    assert_eq!(health.current(), 10, "worn armor spends itself, not HP");
    assert_eq!(
        combat.damage_invuln_timer, TEST_FEEL.damage_invuln_time,
        "armor arms the same brief i-frames a damaging hit would"
    );
    assert!(
        worn.wears("mushroom_small"),
        "big downgraded to small in place"
    );

    // Clear the i-frame the absorb armed so the next hit resolves; small carries
    // no armor, so this hit reaches HP.
    combat.damage_invuln_timer = 0.0;
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        Some(&mut worn),
        false,
        1.0,
        pos,
        pos,
        DOWN,
        4,
        1.0,
        false,
        TEST_FEEL,
    );
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 4,
            died: false
        }
    );
    assert_eq!(
        health.current(),
        6,
        "with the armor spent, the hit reaches HP"
    );
}
