//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
// The parent module imports only the handful of Bevy items its systems need,
// so the App-level tests below bring in their own.
use bevy::prelude::{default, App, Messages, Update};

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
    armor_hitstop_time: 0.070,
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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
        false,
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
fn goblin_melee_knockback_is_an_absolute_launch_speed() {
    let feel = Platformer2dFeelTuningMonolith::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0);
    let knockback = crate::combat::HitKnockback {
        dir: 1.0,
        magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
        source_pos,
        impact_pos: victim_pos,
        launch_dir: None,
    };

    let vel = resolved_body_knockback_velocity(
        victim_pos,
        1.0,
        DOWN,
        false,
        Some(&knockback),
        ae::Vec2::ZERO,
        feel,
    );

    assert!(
        (vel.length() - 120.0).abs() < 1e-3,
        "an authored 120 px/s goblin melee launch must remain 120 px/s, not become a 120x feel multiplier: {vel:?}"
    );
    assert!(
        vel.x > 0.0 && vel.y < 0.0,
        "launch stays up-and-away: {vel:?}"
    );
}

#[test]
fn absolute_launch_speed_does_not_scale_hitstun_as_a_bare_number() {
    let knockback = crate::combat::HitKnockback {
        dir: 1.0,
        magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
        source_pos: ae::Vec2::ZERO,
        impact_pos: ae::Vec2::ZERO,
        launch_dir: None,
    };
    assert_eq!(knockback_reaction_scale(Some(&knockback)), 1.0);

    let scaled = crate::combat::HitKnockback {
        magnitude: crate::combat::HitKnockbackMagnitude::FeelScale(0.6),
        ..knockback
    };
    assert_eq!(knockback_reaction_scale(Some(&scaled)), 0.6);
}

#[test]
fn knockback_impulse_is_frame_equivalent() {
    let feel = Platformer2dFeelTuningMonolith::default();
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
            magnitude: crate::combat::HitKnockbackMagnitude::FeelScale(1.0),
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
fn scaled_launch_speed_conjugates_under_rotated_gravity() {
    // C4: a growth-scaled authored speed under rotated gravity produces the
    // conjugated trajectory. The speed remains an engine-unit magnitude; only
    // the local launch direction rotates with gravity.
    let feel = Platformer2dFeelTuningMonolith::default();
    let launch_speed = scaled_knockback(100.0, 2.0, 30, 2.0); // == 130 px/s
    let default_dir = ae::Vec2::new(feel.enemy_knockback_x, -feel.enemy_knockback_y).normalize();
    let local_expected = default_dir * launch_speed;
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
            magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(launch_speed),
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
            "growth-scaled launch speed must conjugate for {gravity_dir:?}: {local_vel:?}"
        );
    }
}

// --- CM1: the authored launch DIRECTION (smash-style fixed angles) ---

#[test]
fn authored_launch_dir_sets_the_angle_and_keeps_the_authored_speed() {
    let feel = Platformer2dFeelTuningMonolith::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0); // hit from local left
    let authored_speed = 120.0;

    // A pure up-launcher: (0, 1) launches straight against gravity.
    let up = crate::combat::HitKnockback {
        dir: 0.0,
        magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(authored_speed),
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
        (vel.length() - authored_speed).abs() < 1e-3,
        "the authored angle keeps the authored SPEED: |{vel:?}| vs {authored_speed}"
    );

    // The lateral component mirrors to point AWAY from the source: hit
    // from the left ⇒ positive local x ⇒ world +x.
    let diag = crate::combat::HitKnockback {
        dir: 0.0,
        magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(authored_speed),
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
    let feel = Platformer2dFeelTuningMonolith::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let speed = 120.0;
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
            magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(speed),
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
    let feel = Platformer2dFeelTuningMonolith::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0);
    let base = crate::combat::HitKnockback {
        dir: 0.0,
        magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
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

/// **The meter is not the pool, and it does not stop where the pool does.** (S4)
///
/// This test used to assert the opposite — `h.damage(100)` on a 20 pool left
/// `damage_taken() == 20`, commented "clamps at the pool max" — and that
/// assertion WAS the defect, written down as intent. `damage_taken()` was
/// `max - current`, so it could not exceed `max` by construction, and
/// `Health::damage` returns early once the body is not `alive()`, so a hit
/// landing on an empty pool did not merely clamp: it was DROPPED. Knockback
/// growth scales off this meter, so a body that reached 100% stopped launching
/// farther, which is precisely what smash percent needs it to keep doing.
#[test]
fn the_damage_meter_accumulates_past_the_pool_it_is_measured_against() {
    let mut h = test_health(20);
    assert_eq!(h.damage_taken(), 0);
    h.damage(7);
    assert_eq!(h.damage_taken(), 7);
    h.damage(100);
    assert_eq!(
        h.damage_taken(),
        107,
        "the meter stopped at the pool max, so a body cannot be MORE hurt than \
         its pool is deep and knockback growth flatlines at 100%"
    );
}

/// **Percent is not health**, and the difference is expressible.
///
/// `Health::ratio` clamps to `0..=1` and is about the POOL. A HUD that needs to
/// print `188%` cannot get it from there at any amount of damage.
#[test]
fn damage_percent_is_unclamped_so_a_hud_can_print_188() {
    let mut h = test_health(50).with_policy(crate::combat::DeathPolicy::Unbounded);
    h.damage(94);
    assert!(
        (h.damage_percent() - 1.88).abs() < 1e-6,
        "damage_percent() = {} — a body at 94 damage over a 50 pool is at 188%",
        h.damage_percent()
    );
    assert_eq!(
        h.health.ratio(),
        1.0,
        "the POOL is untouched under Unbounded: its death comes from the world, \
         and a drained pool is what used to make it stop taking hits"
    );
}

/// **An `Unbounded` body keeps taking damage forever**, which is the whole
/// reason the variant exists — and what it could not do before S4.
///
/// At 100% the old shape had `alive()` go false, `resolve_body_hit` return
/// `Ignored` for every subsequent hit, and knockback stop growing. Selecting the
/// variant bought an immortal punching bag.
#[test]
fn an_unbounded_body_never_dies_to_the_meter_and_never_stops_feeling_it() {
    let mut h = test_health(10).with_policy(crate::combat::DeathPolicy::Unbounded);
    for _ in 0..20 {
        assert!(
            !h.damage(10),
            "the meter killed a body whose death is the world's"
        );
        assert!(h.alive(), "an Unbounded body stopped being alive");
    }
    assert_eq!(h.damage_taken(), 200);
    assert_eq!(h.current(), h.max(), "the pool drained under Unbounded");
}

/// The default policy is byte-unchanged: the pool still drains and still kills.
#[test]
fn an_hp_depleted_body_still_dies_exactly_when_it_always_did() {
    let mut h = test_health(20);
    assert!(!h.damage(19));
    assert!(h.alive());
    assert_eq!(h.current(), 1);
    assert!(h.damage(1), "the killing blow did not report the kill");
    assert!(!h.alive());
    assert_eq!(h.current(), 0);
}

/// Healing repays the meter too, or a healed body would keep launching as if it
/// were still hurt.
#[test]
fn healing_repays_the_meter_and_not_only_the_pool() {
    let mut h = test_health(20);
    h.damage(12);
    assert_eq!(h.damage_taken(), 12);
    h.heal(5);
    assert_eq!(h.damage_taken(), 7);
    assert_eq!(h.current(), 13);
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
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        4,
        1.0,
        false,
        TEST_FEEL,
        false,
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
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        4,
        1.0,
        false,
        TEST_FEEL,
        false,
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

/// `player_damage_multiplier` is the OUTGOING scale. It must not appear in the
/// incoming product — the exact bug this pins: the slider used to inflate
/// damage TAKEN too (GPT-dialog verification, 2026-07-19).
#[test]
fn incoming_multiplier_ignores_the_outgoing_damage_slider() {
    use ambition_persistence::settings::GameplaySettings;
    let mut weak = GameplaySettings::default();
    weak.player_damage_multiplier = 0.25;
    let mut strong = GameplaySettings::default();
    strong.player_damage_multiplier = 4.0;
    assert_eq!(
        incoming_player_damage_multiplier(&weak),
        incoming_player_damage_multiplier(&strong),
        "damage TAKEN must not move with the outgoing power slider"
    );
}

/// The incoming product is exactly difficulty × assist, per
/// `resolve_body_hit`'s documented contract.
#[test]
fn incoming_multiplier_is_difficulty_times_assist() {
    use ambition_persistence::settings::gameplay::AssistMode;
    use ambition_persistence::settings::GameplaySettings;
    let mut g = GameplaySettings::default();
    g.difficulty = ambition_persistence::settings::gameplay::Difficulty::Hard;
    g.assist = AssistMode::Off;
    assert_eq!(incoming_player_damage_multiplier(&g), 2.0);
    g.assist = AssistMode::On;
    assert_eq!(incoming_player_damage_multiplier(&g), 1.0, "assist halves");
}

/// The OUTGOING half of the invariant: the slider does scale what the
/// controlled body fires, at the projectile spec seam.
#[test]
fn outgoing_projectile_damage_scales_with_the_slider() {
    use ambition_projectiles::ProjectileKind;
    let origin = ambition_platformer2d_core::Vec2::new(0.0, 0.0);
    let dir = ambition_platformer2d_core::Vec2::new(1.0, 0.0);
    let base = ProjectileKind::Fireball.spec(origin, dir, 1.0).damage;
    let scaled = ProjectileKind::Fireball.spec(origin, dir, 4.0).damage;
    assert_eq!(scaled, base * 4, "outgoing damage follows the slider");
}

#[test]
fn kernel_reset_death_reports_the_pre_respawn_impact_position() {
    let mut app = App::new();
    app.add_message::<ActorDiedMessage>();
    app.add_systems(Update, publish_kernel_reset_death);

    let impact = ae::Vec2::new(321.0, -45.0);
    app.world_mut().spawn((
        crate::actor::PlayerEntity,
        crate::actor::PrimaryPlayer,
        crate::avatar::PlayerBodyFrameOutput {
            reset: Some(crate::avatar::BodyReset {
                cause: ae::ResetCause::Hazard,
                origin: impact,
            }),
            ..default()
        },
    ));

    app.update();

    let deaths: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<ActorDiedMessage>>()
        .drain()
        .collect();
    assert_eq!(deaths.len(), 1);
    assert_eq!(
        deaths[0].pos, impact,
        "the death fact must preserve where the hazard struck, not the spawn destination"
    );
}

/// Explicit victim identity outranks the legacy source-direction partition.
/// This is the downstream half of body-generic melee: a Player-effective fighter
/// may directly resolve another human-controlled body as its victim, while a
/// legacy broadcast `PlayerSlash` remains on the feature-damage path.
///
/// ⭐ **the victim carries `PlayerEntity`, the way a real one does.** The target
/// used to be stamped `HitTarget::Player(e)` and this fixture could spawn a bare
/// entity, because the stamp was the whole claim. There is one body variant now,
/// so the stager asks the world which population the victim is in — and a
/// fixture that spawns an unmarked entity is asserting about a body production
/// never builds.
#[test]
fn explicit_player_target_is_staged_even_for_an_attacker_side_source() {
    let mut app = App::new();
    app.add_message::<FeatureHitEvent>();
    app.init_resource::<crate::combat::events::PendingPlayerHitEvents>();
    app.add_systems(Update, stage_player_victim_hit_events);

    let victim = app
        .world_mut()
        .spawn(ambition_platformer2d_shared_tangle::markers::PlayerEntity)
        .id();
    // The poison: a body-targeted hit on a body this resolver does NOT own must
    // stay out of its rollback-registered FIFO.
    let other_body = app.world_mut().spawn_empty().id();
    let volume: ae::CombatVolume =
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)).into();
    app.world_mut().write_message(FeatureHitEvent {
        strike_sfx: None,
        volume: volume.clone(),
        damage: 3,
        source: crate::combat::HitSource::Melee,
        attacker: None,
        target: crate::combat::HitTarget::Body(victim),
        mode: crate::combat::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.world_mut().write_message(FeatureHitEvent {
        strike_sfx: None,
        volume: volume.clone(),
        damage: 3,
        source: crate::combat::HitSource::Melee,
        attacker: None,
        target: crate::combat::HitTarget::Volume,
        mode: crate::combat::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.world_mut().write_message(FeatureHitEvent {
        strike_sfx: None,
        volume,
        damage: 3,
        source: crate::combat::HitSource::Melee,
        attacker: None,
        target: crate::combat::HitTarget::Body(other_body),
        mode: crate::combat::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();
    let pending = &app
        .world()
        .resource::<crate::combat::events::PendingPlayerHitEvents>()
        .0;
    assert_eq!(
        pending.len(),
        1,
        "only the hit resolved onto a body THIS resolver owns belongs in its FIFO"
    );
    assert_eq!(pending[0].target, crate::combat::HitTarget::Body(victim));
}

/// A staged victim hit must not survive a room-lifecycle boundary: the void
/// system clears the FIFO when either boundary fact fired this frame, and
/// leaves it alone otherwise (the ordinary drain owns the no-boundary case).
#[test]
fn a_lifecycle_boundary_voids_staged_player_hits() {
    fn staged_hit() -> FeatureHitEvent {
        FeatureHitEvent {
            strike_sfx: None,
            volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)).into(),
            damage: 1,
            source: crate::combat::HitSource::Melee,
            attacker: None,
            target: crate::combat::HitTarget::Volume,
            mode: crate::combat::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        }
    }
    fn app_with_staged_hit() -> App {
        let mut app = App::new();
        app.add_message::<crate::combat::ResetRoomFeaturesEvent>()
            .add_message::<crate::rooms::RoomLoaded>()
            .init_resource::<crate::combat::events::PendingPlayerHitEvents>()
            .add_systems(Update, void_pending_player_hits_at_lifecycle_boundaries);
        app.world_mut()
            .resource_mut::<crate::combat::events::PendingPlayerHitEvents>()
            .0
            .push(staged_hit());
        app
    }

    // No boundary: the staged hit stays for the drain to consume.
    let mut quiet = app_with_staged_hit();
    quiet.update();
    assert_eq!(
        quiet
            .world()
            .resource::<crate::combat::events::PendingPlayerHitEvents>()
            .0
            .len(),
        1,
        "a quiet frame must not void the staged hit — draining is the resolver's job"
    );

    // Same-room reset boundary.
    let mut reset = app_with_staged_hit();
    reset
        .world_mut()
        .write_message(crate::combat::ResetRoomFeaturesEvent::default());
    reset.update();
    assert!(
        reset
            .world()
            .resource::<crate::combat::events::PendingPlayerHitEvents>()
            .0
            .is_empty(),
        "ResetRoomFeaturesEvent must void hits staged by the pre-reset population"
    );

    // Room (re)staging boundary — transitions, session resets, restores.
    let mut loaded = app_with_staged_hit();
    loaded.world_mut().write_message(crate::rooms::RoomLoaded {
        room_id: "test_room".to_string(),
    });
    loaded.update();
    assert!(
        loaded
            .world()
            .resource::<crate::combat::events::PendingPlayerHitEvents>()
            .0
            .is_empty(),
        "RoomLoaded must void hits staged by the outgoing room's population"
    );
}

#[test]
fn wallet_shield_spends_currency_before_a_lethal_hit_reaches_health() {
    use ambition_characters::actor::{BodyWallet, BodyWalletShield};

    let mut combat = BodyCombat::default();
    let mut health = test_health(1);
    let mut wallet = BodyWallet { balance: 7 };
    let shield = BodyWalletShield;
    let pos = ae::Vec2::new(10.0, 20.0);

    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        Some(WalletArmor::new(&mut wallet, &shield)),
        false,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        false,
        TEST_FEEL,
        false,
    );

    assert_eq!(res, BodyHitResolution::WalletShielded { spent: 7 });
    assert_eq!(health.current(), 1, "the lethal hit never reaches HP");
    assert_eq!(wallet.balance, 0, "the whole defensive balance is spent");
}

/// **Losing your rings gets the same BEAT as losing a powerup.**
///
/// Jon, from play: *"When SANIC is hit, there it seems like he is given no
/// iframes. He should also have some hitstun and be knocked back a bit."*
///
/// ⛔ **the lesson was learned one branch above and not applied to this one.**
/// The armor branch carries a comment that says so in Jon's earlier words about
/// Mary-O — *"AND THE BEAT, which this branch used to skip … she shrank
/// mid-stride with no beat, which reads as the hit not landing at all"* — and the
/// wallet branch, six lines below it, still skipped it. Spending a purse is the
/// same event: the most consequential thing that happens to Sanic short of
/// dying, and it happened with no pause at all.
///
/// ⚠ the hitstop ONLY, exactly as for armor. The recoil lock and the carried
/// launch belong to being thrown, and the outer handler already keeps the
/// physical reaction (`HitMode::Knockback` still knocks him off the ledge). What
/// is owed here is the pause that says it happened.
#[test]
fn losing_a_purse_arms_the_same_beat_as_losing_armor() {
    use ambition_characters::actor::{BodyWallet, BodyWalletShield};

    let mut combat = BodyCombat::default();
    let mut health = test_health(1);
    let mut wallet = BodyWallet { balance: 7 };
    let shield = BodyWalletShield;
    let pos = ae::Vec2::new(10.0, 20.0);

    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        Some(WalletArmor::new(&mut wallet, &shield)),
        false,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        false,
        TEST_FEEL,
        false,
    );

    assert_eq!(res, BodyHitResolution::WalletShielded { spent: 7 });
    assert!(
        combat.hitstop_timer >= TEST_FEEL.armor_hitstop_time,
        "spending a purse armed no hitstop ({}s), so the rings burst out of a \
         body that never paused — the same silence the armor branch was fixed \
         for",
        combat.hitstop_timer,
    );
    assert!(
        combat.damage_invuln_timer >= TEST_FEEL.damage_invuln_time,
        "and the i-frames the resolver arms for every other survivable hit"
    );
}

#[test]
fn empty_wallet_shield_does_not_make_the_body_immortal() {
    use ambition_characters::actor::{BodyWallet, BodyWalletShield};

    let mut combat = BodyCombat::default();
    let mut health = test_health(1);
    let mut wallet = BodyWallet { balance: 0 };
    let shield = BodyWalletShield;
    let pos = ae::Vec2::new(10.0, 20.0);

    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        Some(WalletArmor::new(&mut wallet, &shield)),
        false,
        1.0,
        pos,
        pos,
        DOWN,
        1,
        1.0,
        false,
        TEST_FEEL,
        false,
    );

    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 1,
            died: true
        }
    );
    assert_eq!(health.current(), 0);
}

/// **Nothing defends against the edge of the world.**
///
/// Every gate in `resolve_body_hit` is a defence against BEING HIT — i-frames,
/// the shield, a worn armor row, a wallet balance, and `never_dies`. Nothing
/// hit a body that left the stage; the world stopped. So the blast zone is
/// `unstoppable` and passes all of them.
///
/// The i-frame case is the one that matters by the frame: the launch that
/// throws a body off the stage is the SAME event that arms its knockback
/// invulnerability — 0.2s on an actor, 0.75s on the player — so gating the
/// blast zone on vulnerability fails hardest at exactly the case it exists for.
/// The victim crosses the line and falls for up to 45 frames with nothing
/// happening, then dies far below where it should have.
#[test]
fn an_unstoppable_hit_passes_every_defence_a_body_has() {
    let each_defence: [(&str, BodyCombat, bool); 2] = [
        (
            "i-frames",
            BodyCombat {
                damage_invuln_timer: 0.75,
                ..Default::default()
            },
            false,
        ),
        ("a raised shield", BodyCombat::default(), true),
    ];
    for (what, combat, shield_active) in each_defence {
        let pos = ae::Vec2::new(100.0, 200.0);
        // The shield only blocks a hit arriving from the guarded side, so the
        // impact is placed in front of the body's facing.
        let impact = pos + ae::Vec2::new(50.0, 0.0);

        let mut stopped_combat = combat.clone();
        let mut stopped_health = test_health(5);
        let stopped = resolve_body_hit(
            &mut stopped_combat,
            Some(&mut stopped_health),
            None,
            None,
            shield_active,
            1.0,
            pos,
            impact,
            DOWN,
            99,
            1.0,
            false,
            TEST_FEEL,
            false,
        );
        assert_ne!(
            stopped,
            BodyHitResolution::Damaged {
                damage: 99,
                died: true
            },
            "fixture: {what} must actually stop an ORDINARY hit, or the \
             unstoppable arm below proves nothing"
        );
        assert_eq!(stopped_health.current(), 5, "{what}: ordinary hit absorbed");

        let mut blasted_combat = combat.clone();
        let mut blasted_health = test_health(5);
        let blasted = resolve_body_hit(
            &mut blasted_combat,
            Some(&mut blasted_health),
            None,
            None,
            shield_active,
            1.0,
            pos,
            impact,
            DOWN,
            99,
            1.0,
            false,
            TEST_FEEL,
            true,
        );
        assert_eq!(
            blasted,
            BodyHitResolution::Damaged {
                damage: 99,
                died: true
            },
            "{what} stopped the BLAST ZONE. You cannot be invulnerable to the \
             edge of the world."
        );
        assert!(!blasted_health.alive(), "{what}: the blast zone killed it");
    }
}

/// **A training dummy that has left the stage is not training.**
///
/// `never_dies` is the sandbag's whole point and it must survive any amount of
/// damage — but a `never_dies` body outside the world is worse than immortal,
/// it is a permanent event source: the blast gate is a position test that
/// re-fires every tick, so an immortal body past the line re-triggers its own
/// death forever, once per frame, banner and all.
#[test]
fn the_blast_zone_kills_a_body_that_cannot_be_damaged_to_death() {
    let pos = ae::Vec2::new(100.0, 200.0);
    let mut combat = BodyCombat::default();
    let mut health = test_health(5);
    let ordinary = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        true,
        TEST_FEEL,
        false,
    );
    assert_eq!(
        ordinary,
        BodyHitResolution::Damaged {
            damage: 99,
            died: false
        },
        "fixture: a never_dies body survives an ordinary lethal hit"
    );
    assert!(health.alive());

    let mut combat = BodyCombat::default();
    let mut health = test_health(5);
    let blasted = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        true,
        TEST_FEEL,
        true,
    );
    assert_eq!(
        blasted,
        BodyHitResolution::Damaged {
            damage: 99,
            died: true
        },
        "a never_dies body that left the world must DIE, or it sits outside the \
         stage re-triggering its own death every tick forever"
    );
    assert!(!health.alive());
}

/// The one gate `unstoppable` does NOT bypass, and the reason it must not.
///
/// The blast gate re-fires every tick while a body is past the margin, so
/// without this a corpse outside the world writes a lethal hit once per frame
/// for as long as it exists.
#[test]
fn even_an_unstoppable_hit_refuses_a_body_that_is_already_dead() {
    let pos = ae::Vec2::new(100.0, 200.0);
    let mut combat = BodyCombat::default();
    let mut health = test_health(5);
    health.damage(5);
    assert!(!health.alive(), "fixture: the body starts dead");
    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        None,
        false,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        false,
        TEST_FEEL,
        true,
    );
    assert_eq!(res, BodyHitResolution::Ignored);
}

// ── The launch CHANNEL (D6) ────────────────────────────────────────────────

/// **The reaction publishes the launch, and not only the velocity.**
///
/// Writing `BodyKinematics::vel` is authoritative for an axis-swept body and a
/// MIRROR for a riding surface-momentum one, whose velocity is derived from `v_t`
/// and republished every step. Sanic rides — so for as long as this reaction only
/// wrote `vel`, his knockback was applied faithfully to a field nothing read, and
/// the symptom was "no knockback" with every authored number non-zero.
///
/// This is the writer's half of the seam. `step_motion` drains it; the kernel's
/// own tests cover what the model then does with it.
#[test]
fn a_hit_publishes_its_launch_where_the_motion_model_will_find_it() {
    let feel = Platformer2dFeelTuningMonolith::default();
    let mut vel = ae::Vec2::ZERO;
    let mut flight = ae::BodyFlightState::default();
    let mut combat = BodyCombat::default();
    let victim_pos = ae::Vec2::new(100.0, 100.0);
    let knockback = crate::combat::HitKnockback {
        dir: 1.0,
        magnitude: crate::combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
        source_pos: victim_pos - ae::Vec2::new(40.0, 0.0),
        impact_pos: victim_pos,
        launch_dir: None,
    };

    apply_body_hit_reaction(
        &mut vel,
        &mut flight,
        &mut combat,
        victim_pos,
        1.0,
        DOWN,
        false,
        Some(&knockback),
        ae::Vec2::ZERO,
        feel,
    );

    assert_ne!(
        flight.pending_launch,
        ae::Vec2::ZERO,
        "the reaction must PUBLISH the launch — a velocity write alone is invisible \
         to a model that owns its own velocity"
    );
    assert_eq!(
        flight.pending_launch, vel,
        "and it must be the same launch the velocity got, or the two channels \
         disagree about what the hit did"
    );
}

/// A hit that carries NO knockback publishes nothing.
///
/// `Vec2::ZERO` is the channel's empty state, so this is what keeps a damage-only
/// hit from being drained as a launch of zero — and, more importantly, from
/// clearing a body's ride for no reason.
#[test]
fn a_hit_with_no_knockback_publishes_no_launch() {
    let mut vel = ae::Vec2::new(50.0, 0.0);
    let mut flight = ae::BodyFlightState::default();
    let mut combat = BodyCombat::default();

    apply_body_hit_reaction(
        &mut vel,
        &mut flight,
        &mut combat,
        ae::Vec2::ZERO,
        1.0,
        DOWN,
        false,
        None,
        ae::Vec2::ZERO,
        Platformer2dFeelTuningMonolith::default(),
    );

    assert_eq!(
        flight.pending_launch,
        ae::Vec2::ZERO,
        "no knockback, no launch: {:?}",
        flight.pending_launch
    );
}
