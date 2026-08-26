use super::*;
// The parent module imports only the handful of Bevy items its systems need,
// so the App-level tests below bring in their own.
use bevy::prelude::{default, App, Messages, Update};

/// A guard that is UP, with a shield that is not a spendable resource — the
/// shape every body in this file has. `resolve_body_hit` takes the guard itself
/// now rather than a "is it up" bool, because a block SPENDS integrity and the
/// cost belongs inside the decision that grants the block; `ShieldTuning::OFF`
/// makes that spend a no-op, so these tests measure the block and nothing else.
/// A velocity nobody reads, for the tests that only ask whether a block
/// happens. `&mut ae::Vec2::ZERO` would borrow a fresh temporary of a `const`.
fn sink() -> ae::Vec2 {
    ae::Vec2::ZERO
}

fn raised_guard() -> ae::BodyShieldState {
    ae::BodyShieldState {
        active: true,
        ..Default::default()
    }
}

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

/// The body every fixture here wears.  hits are aimed at its CENTRE, so a
/// poke never fires and these keep measuring the facing and resource rules.
const TEST_BODY: ae::Vec2 = ae::Vec2::new(24.0, 40.0);

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
        None,
        1.0,
        pos,
        pos + ae::Vec2::new(50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos + ae::Vec2::new(50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        Some(GuardUnderFire {
            state: &mut raised_guard(),
            tuning: ae::ShieldTuning::OFF,
            body_size: TEST_BODY,
            vel: &mut sink(),
        }),
        1.0,
        pos,
        pos + ae::Vec2::new(50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        Some(GuardUnderFire {
            state: &mut raised_guard(),
            tuning: ae::ShieldTuning::OFF,
            body_size: TEST_BODY,
            vel: &mut sink(),
        }),
        1.0,
        pos,
        pos + ae::Vec2::new(-50.0, 0.0),
        DOWN,
        3,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        3,
        2.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        1,
        0.1,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        5,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        5,
        1.0,
        true,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        5,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
    let knockback = ambition_combat::HitKnockback {
        // An ordinary hit: it stuns.
        flinchless: false,
        dir: 1.0,
        magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
        source_pos,
        impact_pos: victim_pos,
        launch_dir: None,
        follow: None,
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

///  Hitstun scales with the LAUNCH, and never with the launch's bare
/// number.
///
/// The launch scales against a reference speed. Both claims hold: proportional
/// to the launch, and never the raw number.
#[test]
fn hitstun_scales_with_the_launch_and_never_with_its_bare_number() {
    let at = |magnitude| {
        knockback_reaction_scale(Some(&ambition_combat::HitKnockback {
            // An ordinary hit: it stuns.
            flinchless: false,
            dir: 1.0,
            magnitude,
            source_pos: ae::Vec2::ZERO,
            impact_pos: ae::Vec2::ZERO,
            launch_dir: None,
            follow: None,
        }))
    };
    use ambition_combat::HitKnockbackMagnitude::{FeelScale, LaunchSpeed};
    use ae::hit_response::{MAX_HITSTUN_SCALE, STANDARD_LAUNCH_SPEED};

    // A reference-strength launch is the standard reaction, so the shipped feel
    // numbers still mean what they said.
    assert!((at(LaunchSpeed(STANDARD_LAUNCH_SPEED)) - 1.0).abs() < 1e-6);
    //  twice the launch, twice the stun — the combo mechanic.
    assert!((at(LaunchSpeed(STANDARD_LAUNCH_SPEED * 2.0)) - 2.0).abs() < 1e-6);
    // …and a poke stuns less.
    assert!(at(LaunchSpeed(STANDARD_LAUNCH_SPEED * 0.25)) < 1.0);
    //  the original poison, still live: the scale is never the bare speed. A
    // 120 px/s launch must not arm 120x the standard hitstun.
    assert!(
        at(LaunchSpeed(120.0)) < 2.0,
        "a launch SPEED read as a scale is the bug this test was written for"
    );
    // A kill-percent launch is capped — it is a kill, not a combo starter.
    assert!((at(LaunchSpeed(STANDARD_LAUNCH_SPEED * 50.0)) - MAX_HITSTUN_SCALE).abs() < 1e-6);
    // A feel multiplier is still exactly itself.
    assert!((at(FeelScale(0.6)) - 0.6).abs() < 1e-6);
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
        let knockback = ambition_combat::HitKnockback {
            // An ordinary hit: it stuns.
            flinchless: false,
            dir: 0.0,
            magnitude: ambition_combat::HitKnockbackMagnitude::FeelScale(1.0),
            source_pos,
            impact_pos: victim_pos,
            launch_dir: None,
            follow: None,
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
        let knockback = ambition_combat::HitKnockback {
            // An ordinary hit: it stuns.
            flinchless: false,
            dir: 0.0,
            magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(launch_speed),
            source_pos,
            impact_pos: victim_pos,
            launch_dir: None,
            follow: None,
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

#[test]
fn authored_launch_dir_sets_the_angle_and_keeps_the_authored_speed() {
    let feel = Platformer2dFeelTuningMonolith::default();
    let victim_pos = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let source_pos = victim_pos - ae::Vec2::new(40.0, 0.0); // hit from local left
    let authored_speed = 120.0;

    // A pure up-launcher authors (0, -1): local `y` is TOWARD THE FEET, so "away from the feet"
    // is negative — the same convention every authored volume in the tree writes (`+y =
    // gravity-down`, ).
    let up = ambition_combat::HitKnockback {
        // An ordinary hit: it stuns.
        flinchless: false,
        dir: 0.0,
        magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(authored_speed),
        source_pos,
        impact_pos: victim_pos,
        launch_dir: Some(ae::Vec2::new(0.0, -1.0)),
        follow: None,
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
        "a (0,-1) launcher throws straight up (world -y): {vel:?}"
    );
    assert!(
        (vel.length() - authored_speed).abs() < 1e-3,
        "the authored angle keeps the authored SPEED: |{vel:?}| vs {authored_speed}"
    );

    // The lateral component mirrors to point AWAY from the source: hit
    // from the left  positive local x  world +x.
    let diag = ambition_combat::HitKnockback {
        // An ordinary hit: it stuns.
        flinchless: false,
        dir: 0.0,
        magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(authored_speed),
        source_pos,
        impact_pos: victim_pos,
        launch_dir: Some(ae::Vec2::new(1.0, -1.0)),
        follow: None,
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
        "a (1,-1) launcher throws up-and-away from the source: {vel:?}"
    );
    // Mirrored source  mirrored lateral, same rise.
    let mirrored = ambition_combat::HitKnockback {
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
    //  the authored vector IS the local launch direction, so the expected local velocity is
    // just `n * speed` — no negation anywhere.
    let n = ae::Vec2::new(0.6, -0.8); // already unit-length
    let local_expected = n * speed;
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let source_pos = victim_pos - frame.side * 40.0;
        let knockback = ambition_combat::HitKnockback {
            // An ordinary hit: it stuns.
            flinchless: false,
            dir: 0.0,
            magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(speed),
            source_pos,
            impact_pos: victim_pos,
            launch_dir: Some(n),
            follow: None,
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
    let base = ambition_combat::HitKnockback {
        // An ordinary hit: it stuns.
        flinchless: false,
        dir: 0.0,
        magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
        source_pos,
        impact_pos: victim_pos,
        launch_dir: None,
        follow: None,
    };
    let degenerate = ambition_combat::HitKnockback {
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
    use ambition_combat::DeathPolicy;
    // HpDepleted (default) kills at the meter's max; Unbounded (smash
    // percent) never does — its death comes from the blast-zone gate.
    assert!(DeathPolicy::default().kills_at_max());
    assert!(DeathPolicy::HpDepleted.kills_at_max());
    assert!(!DeathPolicy::Unbounded.kills_at_max());
}

/// The meter is not the pool, and it does not stop where the pool does. (S4)
///
/// Knockback growth scales off this meter, so a body that reached 100% stopped launching farther,
/// which is precisely what smash percent needs it to keep doing.
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

/// Percent is not health, and the difference is expressible.
///
/// `Health::ratio` clamps to `0..=1` and is about the POOL. A HUD that needs to
/// print `188%` cannot get it from there at any amount of damage.
#[test]
fn damage_percent_is_unclamped_so_a_hud_can_print_188() {
    let mut h = test_health(50).with_policy(ambition_combat::DeathPolicy::Unbounded);
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

/// An `Unbounded` body keeps taking damage forever, which is the whole
/// reason the variant exists — and what it could not do before S4.
///
/// At 100% the old shape had `alive()` go false, `resolve_body_hit` return
/// `Ignored` for every subsequent hit, and knockback stop growing. Selecting the
/// variant bought an immortal punching bag.
#[test]
fn an_unbounded_body_never_dies_to_the_meter_and_never_stops_feeling_it() {
    let mut h = test_health(10).with_policy(ambition_combat::DeathPolicy::Unbounded);
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        4,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        4,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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

/// `player_damage_multiplier` is the OUTGOING scale.
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
/// There is one body variant now, so the stager asks the world which population the victim is in —
/// and a fixture that spawns an unmarked entity is asserting about a body production never builds.
#[test]
fn explicit_player_target_is_staged_even_for_an_attacker_side_source() {
    let mut app = App::new();
    app.add_message::<FeatureHitEvent>();
    app.init_resource::<ambition_combat::events::PendingPlayerHitEvents>();
    app.add_systems(Update, stage_player_victim_hit_events);

    let victim = app
        .world_mut()
        .spawn(ambition_platformer2d_shared_tangle::markers::PlayerEntity)
        .id();
    // The poison: a body-targeted hit on a body this resolver does NOT own must
    // stay out of its rollback-registered FIFO.
    let other_body = app.world_mut().spawn_empty().id();
    let volume: ae::CombatVolume = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)).into();
    app.world_mut().write_message(FeatureHitEvent {
        strike_sfx: None,
        volume: volume.clone(),
        damage: 3,
        source: ambition_combat::HitSource::Melee,
        attacker: None,
        target: ambition_combat::HitTarget::Body(victim),
        mode: ambition_combat::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.world_mut().write_message(FeatureHitEvent {
        strike_sfx: None,
        volume: volume.clone(),
        damage: 3,
        source: ambition_combat::HitSource::Melee,
        attacker: None,
        target: ambition_combat::HitTarget::Volume,
        mode: ambition_combat::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    app.world_mut().write_message(FeatureHitEvent {
        strike_sfx: None,
        volume,
        damage: 3,
        source: ambition_combat::HitSource::Melee,
        attacker: None,
        target: ambition_combat::HitTarget::Body(other_body),
        mode: ambition_combat::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });

    app.update();
    let pending = &app
        .world()
        .resource::<ambition_combat::events::PendingPlayerHitEvents>()
        .0;
    assert_eq!(
        pending.len(),
        1,
        "only the hit resolved onto a body THIS resolver owns belongs in its FIFO"
    );
    assert_eq!(pending[0].target, ambition_combat::HitTarget::Body(victim));
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
            source: ambition_combat::HitSource::Melee,
            attacker: None,
            target: ambition_combat::HitTarget::Volume,
            mode: ambition_combat::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        }
    }
    fn app_with_staged_hit() -> App {
        let mut app = App::new();
        app.add_message::<ambition_combat::ResetRoomFeaturesEvent>()
            .add_message::<crate::rooms::RoomLoaded>()
            .init_resource::<ambition_combat::events::PendingPlayerHitEvents>()
            .add_systems(Update, void_pending_player_hits_at_lifecycle_boundaries);
        app.world_mut()
            .resource_mut::<ambition_combat::events::PendingPlayerHitEvents>()
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
            .resource::<ambition_combat::events::PendingPlayerHitEvents>()
            .0
            .len(),
        1,
        "a quiet frame must not void the staged hit — draining is the resolver's job"
    );

    // Same-room reset boundary.
    let mut reset = app_with_staged_hit();
    reset
        .world_mut()
        .write_message(ambition_combat::ResetRoomFeaturesEvent::default());
    reset.update();
    assert!(
        reset
            .world()
            .resource::<ambition_combat::events::PendingPlayerHitEvents>()
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
            .resource::<ambition_combat::events::PendingPlayerHitEvents>()
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
        false,
    );

    assert_eq!(res, BodyHitResolution::WalletShielded { spent: 7 });
    assert_eq!(health.current(), 1, "the lethal hit never reaches HP");
    assert_eq!(wallet.balance, 0, "the whole defensive balance is spent");
}

/// Losing your rings gets the same BEAT as losing a powerup.
///
///  the hitstop ONLY, exactly as for armor. The recoil lock and the carried
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        1,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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

/// Nothing defends against the edge of the world.
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
        let mut guard = shield_active.then(raised_guard);
        // The shove has its own test; these three only ask whether the block
        // happens, so the velocity is a sink.
        let mut scratch_vel = ae::Vec2::ZERO;
        let stopped = resolve_body_hit(
            &mut stopped_combat,
            Some(&mut stopped_health),
            None,
            None,
            guard.as_mut().map(|g| GuardUnderFire {
                state: g,
                tuning: ae::ShieldTuning::OFF,
                body_size: TEST_BODY,
                vel: &mut scratch_vel,
            }),
            1.0,
            pos,
            impact,
            DOWN,
            99,
            1.0,
            false,
            TEST_FEEL,
            // Not evading — these fixtures exercise the resolver itself.
            false,
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
        let mut guard = shield_active.then(raised_guard);
        // The shove has its own test; these three only ask whether the block
        // happens, so the velocity is a sink.
        let mut scratch_vel = ae::Vec2::ZERO;
        let blasted = resolve_body_hit(
            &mut blasted_combat,
            Some(&mut blasted_health),
            None,
            None,
            guard.as_mut().map(|g| GuardUnderFire {
                state: g,
                tuning: ae::ShieldTuning::OFF,
                body_size: TEST_BODY,
                vel: &mut scratch_vel,
            }),
            1.0,
            pos,
            impact,
            DOWN,
            99,
            1.0,
            false,
            TEST_FEEL,
            // Not evading — these fixtures exercise the resolver itself.
            false,
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

/// A training dummy that has left the stage is not training.
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        true,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        true,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
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
        None,
        1.0,
        pos,
        pos,
        DOWN,
        99,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
        true,
    );
    assert_eq!(res, BodyHitResolution::Ignored);
}

// ── The launch CHANNEL ────────────────────────────────────────────────

/// The reaction publishes the launch, and not only the velocity.
///
/// Writing `BodyKinematics::vel` is authoritative for an axis-swept body and a MIRROR for a
/// riding surface-momentum one, whose velocity is derived from `v_t` and republished every
/// step.
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
    let knockback = ambition_combat::HitKnockback {
        // An ordinary hit: it stuns.
        flinchless: false,
        dir: 1.0,
        magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
        source_pos: victim_pos - ae::Vec2::new(40.0, 0.0),
        impact_pos: victim_pos,
        launch_dir: None,
        follow: None,
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
        // The damage the hit dealt — what the contact freeze is computed from.
        HIT_DAMAGE,
        ae::Vec2::ZERO,
        Default::default(),
        // No budget and no ledge: this fixture measures the launch.
        None,
        None,
        None,
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

/// ⭐ AN AUTOLINK PULSE HOLDS ITS VICTIM INSTEAD OF LAUNCHING IT, through the
/// same shared reaction every other hit takes.
///
/// This is the primitive an authored multi-hit needs: the intermediate pulses
/// keep the victim inside the next hitbox and only the LAST one launches. ⛔ it
/// is NOT a capture — no relationship is formed, no clock runs, and the victim
/// keeps every verb it had; what holds it is that each pulse re-aims its
/// velocity, which is a hit reaction like any other.
///
/// ⭐ THE ASSERTION IS A DIRECTION, NOT A MAGNITUDE: the same knockback with the
/// follow REMOVED launches the victim AWAY from its attacker, and with it the
/// victim is aimed BACK toward the attacker. A test that only checked "some
/// velocity was written" would pass against the ordinary launch road.
#[test]
fn an_autolink_pulse_aims_the_victim_back_at_its_attacker() {
    const ATTACKER: ae::Vec2 = ae::Vec2::new(0.0, 0.0);
    // The victim has been knocked out in front and is drifting further out.
    const VICTIM: ae::Vec2 = ae::Vec2::new(60.0, 0.0);

    let pulse = |follow: Option<ae::hit_response::AutolinkFollow>| {
        let mut vel = ae::Vec2::new(400.0, 0.0);
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat::default();
        apply_body_hit_reaction(
            &mut vel,
            &mut flight,
            &mut combat,
            VICTIM,
            1.0,
            DOWN,
            false,
            Some(&crate::features::HitKnockback {
                // An ordinary hit: it stuns.
                flinchless: false,
                dir: 1.0,
                magnitude: crate::features::HitKnockbackMagnitude::LaunchSpeed(200.0),
                source_pos: ATTACKER,
                impact_pos: VICTIM,
                launch_dir: None,
                follow,
            }),
            HIT_DAMAGE,
            ae::Vec2::ZERO,
            Default::default(),
            None,
            None,
            None,
            Platformer2dFeelTuningMonolith::default(),
        );
        (vel, combat)
    };

    let (launched, _) = pulse(None);
    assert!(
        launched.x > 0.0,
        "the ordinary road stopped launching away from the attacker, so the \
         comparison below proves nothing: {launched:?}"
    );

    let (held, combat) = pulse(Some(ae::hit_response::AutolinkFollow {
        anchor_world: ae::hit_response::autolink_anchor_world(
            ae::Vec2::new(16.0, 0.0),
            ATTACKER,
            1.0,
            DOWN,
        ),
        carry: 1.0,
        pull: 20.0,
        max_speed: 900.0,
        source_vel: ae::Vec2::ZERO,
    }));
    assert!(
        held.x < 0.0,
        "the autolink pulse launched the victim away instead of pulling it back \
         toward the anchor: {held:?}"
    );
    assert!(
        combat.hitstun_timer > 0.0,
        "an autolink pulse is still a HIT — it owes its authored hitstun"
    );
    assert!(
        combat.recoil_lock_timer <= feel_meteor_floor(),
        "an autolink was charged the METEOR silence. The lock keys on \
         `velocity points toward the feet`, which is true of any anchor below \
         the attacker — a spinning move that gathers its victim underneath \
         would be punished for holding somebody"
    );
}

/// The ordinary (non-meteor) recoil lock, so the assertion above names a number
/// the tuning owns rather than a literal.
fn feel_meteor_floor() -> f32 {
    Platformer2dFeelTuningMonolith::default().knockback_recoil_lock_time
}

/// A hit that carries NO knockback publishes nothing — AND LEAVES THE BODY
/// GOING WHERE IT WAS GOING.
///
/// ⛔⛔ THE SECOND HALF IS THE ONE THAT WAS MISSING, and its absence hid a live
/// defect. `knockback_velocity(None)` returns a zero launch, and the reaction
/// wrote that zero straight into `*vel` — so a damage-only tick (a hazard, a
/// chip, a poison) stopped a running player dead. This fixture already set the
/// velocity to `(50, 0)` and then never asked what became of it: it asserted the
/// empty CHANNEL and agreed with the bug about the VELOCITY.
///
/// ⭐ and hitlag is asserted too, because the repair is a shape, not a skipped
/// write: a damage-only hit is still a HIT, and everything a hit does other than
/// throw the body still has to happen.
#[test]
fn a_hit_with_no_knockback_publishes_no_launch_and_keeps_the_ride() {
    const RIDE: ae::Vec2 = ae::Vec2::new(50.0, 0.0);
    let mut vel = RIDE;
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
        HIT_DAMAGE,
        ae::Vec2::ZERO,
        Default::default(),
        None,
        None,
        None,
        Platformer2dFeelTuningMonolith::default(),
    );

    assert_eq!(
        flight.pending_launch,
        ae::Vec2::ZERO,
        "no knockback, no launch: {:?}",
        flight.pending_launch
    );
    assert_eq!(
        vel, RIDE,
        "a damage-only hit erased the body's own velocity by writing a zero \
         launch over it"
    );
    assert!(
        combat.hitstop_timer > 0.0,
        "a damage-only hit skipped its hitlag — the freeze on contact is what \
         makes a hit read as a hit, and it is not the launch's to arm"
    );
    assert_eq!(
        combat.hitstun_timer, 0.0,
        "poison: nothing launched this body, so nothing may stun it either"
    );
}

///  the pure rule was tested and the RESOLVER's `Blocked` branch was not.
/// `shield_blocks_hit` has six unit tests above and `resolve_body_hit` had none
/// at all — so "the bubble blocks" was proven as geometry (*is this hit on the
/// faced side?*) and never as CONSEQUENCE (*does the body keep its health?*).
/// Those are different claims, and the second is the one P4.29 asks about: a
/// resolver that computed the geometry and then fell through to the damage path
/// would pass every test in this file.
///
///  four clauses, and each is a different OUTCOME of the same hit rather than
/// a different input:
///
/// ```text
///   shield up, hit from the front   -> Blocked   HP unchanged
///   shield DOWN, same hit           -> damaging  HP drops     (the poison)
///   shield up, hit from BEHIND      -> damaging  HP drops     (the direction)
///   parrying                        -> Ignored   HP unchanged (a different gate)
/// ```
///
///  the third clause is what stops this passing on a resolver that blocks
/// EVERYTHING while a shield is held, and the fourth separates the two defences:
/// a parry is `body_vulnerable`'s business and the hit never registers, while a
/// block registers and arms a guard i-frame.
#[test]
fn a_raised_shield_blocks_the_hit_and_a_lowered_one_does_not() {
    use ambition_characters::actor::{BodyHealth, Health};

    const START_HP: i32 = 10;
    let body = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let from_front = body + ae::Vec2::new(50.0, 0.0);
    let from_behind = body + ae::Vec2::new(-50.0, 0.0);

    // Facing local-right, so `from_front` is the guarded side.
    let hit = |shield_active: bool, impact: ae::Vec2| -> (BodyHitResolution, i32) {
        let mut combat = BodyCombat::default();
        let mut health = BodyHealth::new(Health::new(START_HP));
        let mut guard = shield_active.then(raised_guard);
        // The shove has its own test; these three only ask whether the block
        // happens, so the velocity is a sink.
        let mut scratch_vel = ae::Vec2::ZERO;
        let resolution = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            None,
            None,
            guard.as_mut().map(|g| GuardUnderFire {
                state: g,
                tuning: ae::ShieldTuning::OFF,
                body_size: TEST_BODY,
                vel: &mut scratch_vel,
            }),
            1.0,
            body,
            impact,
            down,
            3,
            1.0,
            false,
            BodyHitFeel {
                hit_flash: 0.0,
                damage_invuln_time: 0.0,
                block_hit_flash: 0.0,
                block_invuln_floor: 0.1,
                armor_hitstop_time: 0.0,
            },
            // Not evading — this fixture is about the guard.
            false,
            false,
        );
        (resolution, health.health.current)
    };

    let (blocked, hp) = hit(true, from_front);
    assert_eq!(blocked, BodyHitResolution::Blocked);
    assert_eq!(
        hp, START_HP,
        "a raised shield reported Blocked and the body lost health anyway — the \
         resolver computed the geometry and fell through to the damage path"
    );

    //  THE POISON: the same hit with the shield down must hurt, or the clause
    // above is satisfied by a body nothing can damage.
    let (unguarded, hp) = hit(false, from_front);
    assert_ne!(unguarded, BodyHitResolution::Blocked);
    assert!(
        hp < START_HP,
        "an unguarded body took no damage from the same hit, so this test proves \
         nothing about the shield"
    );

    //  and a shield is a SIDE, not a bubble: the back is open.
    let (from_the_back, hp) = hit(true, from_behind);
    assert_ne!(
        from_the_back,
        BodyHitResolution::Blocked,
        "a hit from behind was blocked, so the shield guards every direction and \
         facing decides nothing"
    );
    assert!(hp < START_HP);
}

/// A BLOCK COSTS THE BLOCKER GROUND, AWAY FROM THE HIT AND ALONG IT ONLY.
///
///  the third cost of shield pressure, after integrity and shieldstun. Without
/// it a guard near a ledge is a safe place to stand forever; with it the hits
/// themselves walk you toward the edge, which is what makes chip pressure a
/// thing a player has to answer.
#[test]
fn a_blocked_hit_shoves_the_blocker_laterally_away_from_it() {
    let mut combat = BodyCombat::default();
    let mut health = test_health(20);
    let body = ae::Vec2::new(100.0, 200.0);
    let down = ae::Vec2::new(0.0, 1.0);
    let mut guard = raised_guard();
    let mut vel = ae::Vec2::ZERO;
    let tuning = ae::ShieldTuning::PLATFORM_FIGHTER;

    let res = resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        None,
        Some(GuardUnderFire {
            state: &mut guard,
            tuning,
            vel: &mut vel,
            body_size: TEST_BODY,
        }),
        1.0,
        body,
        // Struck from the FRONT-RIGHT, so the shove goes left.
        body + ae::Vec2::new(50.0, 0.0),
        down,
        10,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
        false,
    );

    assert_eq!(res, BodyHitResolution::Blocked);
    assert!(
        vel.x < 0.0,
        "a blocked hit from the right pushed the blocker {vel:?}, not away from it"
    );
    assert_eq!(
        vel.y, 0.0,
        "a block shoved the blocker along GRAVITY — it is a push, not a launch"
    );
    assert_eq!(vel.x, -(tuning.pushback_per_damage * 10.0));
}

/// AND A GUARD THAT IS NOT A RESOURCE STILL BLOCKS FOR FREE.
#[test]
fn an_unlimited_guard_is_not_pushed() {
    let mut combat = BodyCombat::default();
    let mut health = test_health(20);
    let body = ae::Vec2::new(100.0, 200.0);
    let mut guard = raised_guard();
    let mut vel = ae::Vec2::ZERO;

    resolve_body_hit(
        &mut combat,
        Some(&mut health),
        None,
        None,
        Some(GuardUnderFire {
            state: &mut guard,
            tuning: ae::ShieldTuning::OFF,
            body_size: TEST_BODY,
            vel: &mut vel,
        }),
        1.0,
        body,
        body + ae::Vec2::new(50.0, 0.0),
        ae::Vec2::new(0.0, 1.0),
        10,
        1.0,
        false,
        TEST_FEEL,
        // Not evading — these fixtures exercise the resolver itself.
        false,
        false,
    );

    assert_eq!(vel, ae::Vec2::ZERO);
}

/// An ordinary hit's damage, so these fixtures reach the reaction the way a
/// match does: the contact freeze is computed FROM the damage, and a `0` here
/// would model a hit that dealt none.
const HIT_DAMAGE: i32 = 10;

/// Run one hit through the reaction and report the recoil lock it charged.
fn meteor_reaction(
    launch_dir: ae::Vec2,
    grounded: bool,
    feel: Platformer2dFeelTuningMonolith,
) -> f32 {
    let body = ae::Vec2::new(100.0, 150.0);
    let knockback = ambition_combat::HitKnockback {
        // An ordinary hit: it stuns.
        flinchless: false,
        dir: 1.0,
        magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(300.0),
        source_pos: body,
        impact_pos: body,
        launch_dir: Some(launch_dir),
        follow: None,
    };
    let mut vel = ae::Vec2::ZERO;
    let mut flight = ae::BodyFlightState::default();
    let mut combat = BodyCombat::default();
    apply_body_hit_reaction(
        &mut vel,
        &mut flight,
        &mut combat,
        body,
        1.0,
        ae::Vec2::new(0.0, 1.0),
        false,
        Some(&knockback),
        HIT_DAMAGE,
        ae::Vec2::ZERO,
        VictimStance {
            grounded,
            ..Default::default()
        },
        None,
        None,
        None,
        feel,
    );
    combat.recoil_lock_timer
}

/// CROUCHING TAKES LESS OF THE LAUNCH, AND ONLY WHEN THE GAME SAYS SO.
///
///  three assertions, and the first two are the pair: a standing body takes the
/// whole launch and a crouching one takes the declared fraction, so a version
/// that scaled EVERYBODY would fail the first and one that scaled nobody the
/// second. The third is the floor — an undeclared world's `1.0` must leave
/// crouching worth nothing but a shorter hurtbox, which is every room in
/// Ambition.
#[test]
fn crouching_takes_less_of_the_launch_when_the_rules_declare_it() {
    let launched = |crouching: bool, scale: f32| {
        let knockback = ambition_combat::HitKnockback {
            // An ordinary hit: it stuns.
            flinchless: false,
            dir: 1.0,
            magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(400.0),
            source_pos: ae::Vec2::ZERO,
            impact_pos: ae::Vec2::ZERO,
            launch_dir: Some(ae::Vec2::new(1.0, 0.0)),
            follow: None,
        };
        let mut feel = Platformer2dFeelTuningMonolith::default();
        feel.crouch_cancel_scale = scale;
        let mut vel = ae::Vec2::ZERO;
        let mut flight = ae::BodyFlightState::default();
        let mut combat = BodyCombat::default();
        apply_body_hit_reaction(
            &mut vel,
            &mut flight,
            &mut combat,
            ae::Vec2::ZERO,
            1.0,
            ae::Vec2::new(0.0, 1.0),
            false,
            Some(&knockback),
            HIT_DAMAGE,
            ae::Vec2::ZERO,
            VictimStance {
                grounded: true,
                crouching,
            },
            None,
            None,
            None,
            feel,
        );
        vel.length()
    };

    let standing = launched(false, 0.85);
    let ducked = launched(true, 0.85);
    assert!(
        standing > 0.0,
        "the fixture launched nobody, so this measured nothing"
    );
    assert!(
        (ducked - standing * 0.85).abs() < 0.01,
        "a crouching body took {ducked} where the declared 0.85 of {standing} was owed"
    );
    assert_eq!(
        launched(true, 1.0),
        standing,
        "an undeclared world charged a crouching body a discount it never declared"
    );
}

/// A SPIKE ON AN AIRBORNE BODY BUYS A LONGER SILENCE.
///
///  the mechanic, and what it is FOR: a launch that merely points down is a big
/// hit; one you cannot answer on the way down is a kill. The genre's "meteor
/// cancel" is this window ENDING, so there is no second verb to press and
/// nothing here to trigger.
#[test]
fn a_downward_launch_on_an_airborne_body_locks_it_longer() {
    let mut feel = Platformer2dFeelTuningMonolith::default();
    feel.knockback_recoil_lock_time = 0.10;
    feel.meteor_lock_time = 0.45;

    // Body-local +y is toward the feet: straight down.
    assert_eq!(
        meteor_reaction(ae::Vec2::new(0.0, 1.0), false, feel),
        0.45,
        "an airborne spike took the ordinary recoil, so it is just a big hit"
    );

    //  the floor, three ways.
    assert_eq!(
        meteor_reaction(ae::Vec2::new(0.0, 1.0), true, feel),
        0.10,
        "a body standing on the floor was charged a recovery window for being \
         driven into a floor it was already on"
    );
    assert_eq!(
        meteor_reaction(ae::Vec2::new(0.0, -1.0), false, feel),
        0.10,
        "an UPWARD launch was treated as a meteor"
    );
    let mut off = feel;
    off.meteor_lock_time = 0.0;
    assert_eq!(
        meteor_reaction(ae::Vec2::new(0.0, 1.0), false, off),
        0.10,
        "a game with no meteor rule got one anyway"
    );
}

/// A WINDBOX AUTHORS ZERO AND THE BODY KEEPS ITS HEALTH.
///
/// ⛔⛔ THE SHARED RESOLVER UNDID THE HITBOX LAYER'S WORK ONE SEAM LATER. The
/// producer already preserves an authored `0` and says so in its own comment;
/// this road then ran `damage.max(1)` on every hit alike, so a gust that was
/// never meant to hurt anybody took a point of health per pulse. A repeating
/// windbox killed a body it was only supposed to push.
///
/// ⭐ THE FLOOR IS STILL RIGHT FOR A STRIKE. A heavily staled or scaled-down
/// attack must not silently round away to nothing — that is what the `max(1)`
/// is for, and the third arm keeps it.
#[test]
fn an_authored_zero_damage_windbox_takes_no_health() {
    let hit = |raw: i32, multiplier: f32| -> (BodyHitResolution, i32) {
        let mut combat = BodyCombat::default();
        let mut health = test_health(10);
        let pos = ae::Vec2::new(100.0, 200.0);
        let res = resolve_body_hit(
            &mut combat,
            Some(&mut health),
            None,
            None,
            None,
            1.0,
            pos,
            pos + ae::Vec2::new(50.0, 0.0),
            DOWN,
            raw,
            multiplier,
            false,
            TEST_FEEL,
            false,
            false,
        );
        (res, health.current())
    };

    // ARM 1 — THE WINDBOX. Zero in, zero out, and the body is untouched.
    let (res, health) = hit(0, 1.0);
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 0,
            died: false
        },
        "an authored zero-damage volume was resolved as something other than a \
         zero-damage hit"
    );
    assert_eq!(
        health, 10,
        "a WINDBOX took health ({health} left of 10) — it is a push, not an attack, \
         and a repeating one would eventually kill somebody it never damaged"
    );

    // ARM 2 — AND STALING CANNOT MANUFACTURE DAMAGE EITHER. A zero scaled by
    // anything is still zero, which is the arm that catches a floor applied
    // after the multiplier.
    let (_, health) = hit(0, 0.5);
    assert_eq!(health, 10, "a scaled windbox took health");

    // ARM 3 — THE FLOOR SURVIVES FOR A REAL STRIKE. Without this the fix could
    // be "never floor anything", and a heavily staled attack would round to
    // nothing and read as the game dropping hits.
    let (res, health) = hit(1, 0.1);
    assert_eq!(
        res,
        BodyHitResolution::Damaged {
            damage: 1,
            died: false
        },
        "a real strike scaled below one point rounded away to nothing"
    );
    assert_eq!(health, 9, "the floored strike took no health");
}
/// ⭐⭐ A GUST OFFERS NO GUARD TO SPEND, ON EITHER ROAD.
///
/// A windbox authors *"pushes, and nothing else"* — explicitly NO SHIELD — and
/// both damage roads used to hand `resolve_body_hit` a guard for every contact
/// alike, so a gust spent shield integrity, charged shieldstun and shoved the
/// defender exactly like a strike.
///
/// ⛔⛔ THIS ARM EXISTS BECAUSE THE ROAD FIXTURES CANNOT SEE IT. Measured
/// 2026-08-25: they run on `ShieldTuning::OFF`, where an ordinary BLOCK leaves
/// `depleted`, `stun_timer` and `break_timer` all at zero — so a guard that
/// declines a gust and a guard that engages it for nothing are the same
/// observation. What separates them is whether a guard is OFFERED at all, and
/// that is one decision in one place now: there is nowhere else a
/// `GuardUnderFire` can be built.
///
/// ⭐ THE ARMS STRADDLE `flinchless` AND NOTHING ELSE — same state, same tuning,
/// same velocity, one flag apart. The `None`-knockback arm is the third legal
/// case: a damage-only tick still meets a raised guard.
#[test]
fn a_windbox_offers_no_guard_and_a_strike_offers_one() {
    fn knockback(flinchless: bool) -> ambition_combat::HitKnockback {
        ambition_combat::HitKnockback {
            flinchless,
            dir: 1.0,
            magnitude: ambition_combat::HitKnockbackMagnitude::LaunchSpeed(120.0),
            source_pos: ae::Vec2::ZERO,
            impact_pos: ae::Vec2::new(10.0, 0.0),
            launch_dir: None,
            follow: None,
        }
    }

    let offered = |knockback: Option<ambition_combat::HitKnockback>| {
        let mut state = ae::BodyShieldState::default();
        let mut vel = ae::Vec2::ZERO;
        GuardUnderFire::offered_to(
            knockback.as_ref(),
            &mut state,
            ae::ShieldTuning::default(),
            &mut vel,
            TEST_BODY,
        )
        .is_some()
    };

    assert!(
        offered(Some(knockback(false))),
        "an ordinary strike met no guard at all, so the refusal below is a \
         constructor that never offers one"
    );
    assert!(
        !offered(Some(knockback(true))),
        "a GUST was offered a guard to spend — its authored contract says no \
         shield, and the fact rides the knockback to this exact decision"
    );
    assert!(
        offered(None),
        "a damage-only tick with no knockback at all was refused a guard — a \
         hazard tick still meets a raised shield"
    );
}
