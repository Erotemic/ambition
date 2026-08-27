//! Unit tests for the reusable projectile primitives.

use ambition_platformer2d_core::Vec2;

use crate::spawn::{ProjectileSpawner, SpawnFailure};
use crate::{
    FireballChargeTuning, MotionDirection, MotionInputBuffer, MotionTechnique, ProjectileBody,
    ProjectileKind, ProjectileSolidHit,
};
use ambition_platformer2d_core::{aabb_from_min_size, Aabb, AabbExt};

// The named gesture patterns now live in content; these mirror
// `ambition_content::input_techniques` so the recognizer mechanics stay tested
// against the reusable matcher here in the projectile-kit engine tests.
fn qcf() -> MotionTechnique {
    use MotionDirection::{Down, DownLeft, DownRight, Left, Right};
    MotionTechnique::new(vec![
        vec![Down, DownRight, Right],
        vec![Down, DownLeft, Left],
    ])
}

fn qcf_grace() -> MotionTechnique {
    use MotionDirection::{Down, Left, Right};
    MotionTechnique::new(vec![vec![Down, Right], vec![Down, Left]])
}

fn hcf() -> MotionTechnique {
    use MotionDirection::{Down, DownLeft, DownRight, Left, Right};
    MotionTechnique {
        patterns: vec![
            vec![Right, DownRight, Down, DownLeft, Left],
            vec![Left, DownLeft, Down, DownRight, Right],
        ],
        invert_facing: true,
    }
}

#[test]
fn motion_buffer_recognizes_quarter_circle_right() {
    let mut buf = MotionInputBuffer::new(0.5);
    let mut t = 0.0;
    for dir in [
        MotionDirection::Down,
        MotionDirection::DownRight,
        MotionDirection::Right,
    ] {
        buf.push(dir, t);
        t += 0.05;
    }
    assert_eq!(qcf().detect(&buf), Some(1.0));
}

#[test]
fn motion_buffer_recognizes_quarter_circle_left() {
    let mut buf = MotionInputBuffer::new(0.5);
    let mut t = 0.0;
    for dir in [
        MotionDirection::Down,
        MotionDirection::DownLeft,
        MotionDirection::Left,
    ] {
        buf.push(dir, t);
        t += 0.05;
    }
    assert_eq!(qcf().detect(&buf), Some(-1.0));
}

#[test]
fn motion_buffer_recognizes_half_circle() {
    let mut buf = MotionInputBuffer::new(0.6);
    let mut t = 0.0;
    for dir in [
        MotionDirection::Right,
        MotionDirection::DownRight,
        MotionDirection::Down,
        MotionDirection::DownLeft,
        MotionDirection::Left,
    ] {
        buf.push(dir, t);
        t += 0.04;
    }
    // Half circle right-to-left: facing of the player should be left.
    assert_eq!(hcf().detect(&buf), Some(1.0));
}

#[test]
fn quarter_circle_tolerates_extra_samples() {
    let mut buf = MotionInputBuffer::new(1.0);
    let mut t = 0.0;
    for dir in [
        MotionDirection::Neutral,
        MotionDirection::Down,
        MotionDirection::DownRight,
        MotionDirection::Up, // noise
        MotionDirection::DownRight,
        MotionDirection::Right,
    ] {
        buf.push(dir, t);
        t += 0.04;
    }
    assert_eq!(qcf().detect(&buf), Some(1.0));
}

#[test]
fn motion_buffer_window_prunes_old_samples() {
    let mut buf = MotionInputBuffer::new(0.20);
    buf.push(MotionDirection::Down, 0.0);
    buf.push(MotionDirection::DownRight, 0.05);
    // Push something far in the future — old samples should be pruned.
    buf.push(MotionDirection::Right, 1.0);
    // Quarter circle should NOT detect because the older two
    // samples were dropped.
    assert_eq!(qcf().detect(&buf), None);
}

#[test]
fn projectile_spawner_blocks_when_on_cooldown() {
    let mut spawner = ProjectileSpawner::new(10.0, 0.0);
    let _ = spawner
        .try_spawn(
            ProjectileKind::Fireball,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            1.0,
        )
        .unwrap();
    let err = spawner
        .try_spawn(
            ProjectileKind::Fireball,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            1.0,
        )
        .unwrap_err();
    assert_eq!(err, SpawnFailure::Cooldown);
}

#[test]
fn projectile_spawner_blocks_when_out_of_resource() {
    let mut spawner = ProjectileSpawner::new(0.5, 0.0);
    let err = spawner
        .try_spawn(
            ProjectileKind::Fireball,
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            1.0,
        )
        .unwrap_err();
    assert_eq!(err, SpawnFailure::OutOfResource);
}

#[test]
fn projectile_body_expires_after_max_lifetime() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    let mut alive = true;
    for _ in 0..200 {
        alive = body.tick(0.016, Vec2::new(0.0, 1.0));
        if !alive {
            break;
        }
    }
    assert!(!alive);
    assert!(body.is_expired());
}

#[test]
fn fireball_arcs_downward() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    for _ in 0..30 {
        body.tick(0.016, Vec2::new(0.0, 1.0));
    }
    assert!(
        body.kin.pos.y > 0.0,
        "fireball should arc downward, got {}",
        body.kin.pos.y
    );
    assert!(body.kin.pos.x > 0.0);
}

#[test]
fn hadouken_travels_straight_horizontally() {
    let spec = ProjectileKind::Hadouken.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    for _ in 0..30 {
        body.tick(0.016, Vec2::new(0.0, 1.0));
    }
    assert!(body.kin.pos.y.abs() < 1e-3);
    assert!(body.kin.pos.x > 0.0);
}

fn block_aabb(min: Vec2, size: Vec2) -> Aabb {
    aabb_from_min_size(min, size)
}

/// A fireball travelling down + right that hits the *top* of a
/// floor block must bounce: vy reflects (now upward), the body
/// re-positions just above the block, and `bounces_remaining`
/// decrements.
#[test]
fn fireball_bounces_off_floor_top() {
    let spec = ProjectileKind::Fireball.spec(Vec2::new(100.0, 100.0), Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    // Force the body downward so the contact is unambiguously
    // "from above" (test the geometric branch independent of
    // whatever the spec's gravity has done so far).
    body.kin.vel = Vec2::new(200.0, 240.0);
    body.kin.pos = Vec2::new(150.0, 195.0);
    let starting_bounces = body.game.bounces_remaining;
    let floor = block_aabb(Vec2::new(0.0, 200.0), Vec2::new(400.0, 32.0));
    assert!(starting_bounces > 0, "fireball must spawn with bounces");
    let outcome = body.resolve_solid_hit(floor);
    assert_eq!(outcome, ProjectileSolidHit::Bounced);
    assert_eq!(body.game.bounces_remaining, starting_bounces - 1);
    assert!(
        body.kin.vel.y < 0.0,
        "vy must reflect upward after a floor bounce; got {}",
        body.kin.vel.y
    );
    // Body bottom edge must now be at or above the block top.
    assert!(body.aabb().bottom() <= floor.top() + 1.0);
}

/// Side / ceiling contacts (anything that isn't "fireball above
/// the block") must expire — including a fireball going up that
/// re-overlaps a ceiling.
#[test]
fn fireball_expires_on_non_floor_contact() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    // Side wall: body center is to the LEFT of the block center.
    // Side contact never bounces in this model.
    body.kin.pos = Vec2::new(180.0, 100.0);
    body.kin.vel = Vec2::new(360.0, 60.0);
    let wall = block_aabb(Vec2::new(190.0, 0.0), Vec2::new(32.0, 400.0));
    let outcome = body.resolve_solid_hit(wall);
    assert_eq!(outcome, ProjectileSolidHit::Expired);
}

/// Once `bounces_remaining` reaches zero, even a top-of-block
/// contact returns Expired — the fireball has used its budget.
#[test]
fn fireball_expires_when_bounce_budget_exhausted() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    body.game.bounces_remaining = 0;
    body.kin.vel = Vec2::new(200.0, 240.0);
    body.kin.pos = Vec2::new(150.0, 195.0);
    let floor = block_aabb(Vec2::new(0.0, 200.0), Vec2::new(400.0, 32.0));
    let outcome = body.resolve_solid_hit(floor);
    assert_eq!(outcome, ProjectileSolidHit::Expired);
}

/// One-way platforms must bounce a fireball coming down on top
/// just like a solid floor does. Otherwise the player can shoot
/// across thick floors but loses the bounce on thin ledges, which
/// feels arbitrary.
#[test]
fn fireball_bounces_off_one_way_platform_top() {
    let spec = ProjectileKind::Fireball.spec(Vec2::new(100.0, 100.0), Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    body.kin.vel = Vec2::new(200.0, 240.0);
    body.kin.pos = Vec2::new(150.0, 195.0);
    let starting_bounces = body.game.bounces_remaining;
    let platform = block_aabb(Vec2::new(0.0, 200.0), Vec2::new(400.0, 8.0));
    let outcome = body.resolve_one_way_hit(platform);
    assert_eq!(outcome, ProjectileSolidHit::Bounced);
    assert_eq!(body.game.bounces_remaining, starting_bounces - 1);
    assert!(body.kin.vel.y < 0.0);
    assert!(body.aabb().bottom() <= platform.top() + 1.0);
}

/// Side, ceiling, and below contacts on a one-way platform must
/// pass through — the platform is non-solid from those directions,
/// so a fireball flying horizontally past a thin ledge or rising up
/// into one from below shouldn't be stopped or expired.
#[test]
fn fireball_passes_through_one_way_on_non_top_contact() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    // From below, moving upward — not a landing.
    body.kin.pos = Vec2::new(150.0, 220.0);
    body.kin.vel = Vec2::new(200.0, -240.0);
    let bounces_before = body.game.bounces_remaining;
    let pos_before = body.kin.pos;
    let vel_before = body.kin.vel;
    let platform = block_aabb(Vec2::new(0.0, 200.0), Vec2::new(400.0, 8.0));
    let outcome = body.resolve_one_way_hit(platform);
    assert_eq!(outcome, ProjectileSolidHit::Passthrough);
    assert_eq!(body.game.bounces_remaining, bounces_before);
    assert_eq!(body.kin.pos, pos_before);
    assert_eq!(body.kin.vel, vel_before);
}

/// A fireball with no bounce budget left passes through a one-way
/// platform instead of expiring (a solid floor would expire it). This
/// keeps the platform feeling non-solid from any non-bounce angle.
#[test]
fn fireball_with_no_bounces_passes_through_one_way_top() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    body.game.bounces_remaining = 0;
    body.kin.vel = Vec2::new(200.0, 240.0);
    body.kin.pos = Vec2::new(150.0, 195.0);
    let platform = block_aabb(Vec2::new(0.0, 200.0), Vec2::new(400.0, 8.0));
    let outcome = body.resolve_one_way_hit(platform);
    assert_eq!(outcome, ProjectileSolidHit::Passthrough);
}

/// Hadouken spawns with 0 bounces, so the very first solid hit
/// expires it regardless of contact face. This pins the
/// "horizontal projectile that disappears on first wall" UX.
#[test]
fn hadouken_expires_on_first_solid_hit() {
    let spec = ProjectileKind::Hadouken.spec(Vec2::new(50.0, 100.0), Vec2::new(1.0, 0.0), 1.0);
    let mut body = ProjectileBody::from_spec(spec);
    assert_eq!(body.game.bounces_remaining, 0);
    let wall = block_aabb(Vec2::new(60.0, 0.0), Vec2::new(32.0, 400.0));
    let outcome = body.resolve_solid_hit(wall);
    assert_eq!(outcome, ProjectileSolidHit::Expired);
}

/// Grace QCF detector accepts the easier 2-step keyboard motion
/// (Down → Right) without requiring the diagonal midpoint that a
/// 4-key arrow setup can't easily reach.
#[test]
fn grace_quarter_circle_recognizes_two_step() {
    let mut buf = MotionInputBuffer::new(0.5);
    let mut t = 0.0;
    for dir in [MotionDirection::Down, MotionDirection::Right] {
        buf.push(dir, t);
        t += 0.04;
    }
    assert_eq!(qcf_grace().detect(&buf), Some(1.0));
    // The grace shape is a SUBSEQUENCE of the full QCF, so a
    // 3-step input also satisfies it.
    let mut buf = MotionInputBuffer::new(0.5);
    let mut t = 0.0;
    for dir in [
        MotionDirection::Down,
        MotionDirection::DownRight,
        MotionDirection::Right,
    ] {
        buf.push(dir, t);
        t += 0.04;
    }
    assert_eq!(qcf_grace().detect(&buf), Some(1.0));
}

/// The grace shape rejects a "straight forward press" (Right
/// only) — the player must have crouched at some point. Without
/// this, holding Right would always count as a Hadouken on the
/// next fire press.
#[test]
fn grace_quarter_circle_rejects_straight_forward_only() {
    let mut buf = MotionInputBuffer::new(0.5);
    buf.push(MotionDirection::Right, 0.0);
    buf.push(MotionDirection::Right, 0.04);
    assert_eq!(qcf_grace().detect(&buf), None);
}

/// Fireball charge tiers scale damage and hitbox size on the
/// spec. Hadouken / Super ignore the tier — they don't charge.
#[test]
fn charge_tier_scales_fireball_size_and_damage() {
    let dir = Vec2::new(1.0, 0.0);
    let baseline = ProjectileKind::Fireball.spec(Vec2::ZERO, dir, 1.0);
    let medium = ProjectileKind::Fireball.charged_spec(baseline, 1);
    let heavy = ProjectileKind::Fireball.charged_spec(baseline, 2);
    // Size monotonically increases with tier.
    assert!(medium.half_extent.x > baseline.half_extent.x);
    assert!(heavy.half_extent.x > medium.half_extent.x);
    // Damage increases exponentially: tier 0 = 1x, tier 1 = 4x, tier 2 = 16x.
    assert_eq!(medium.damage, baseline.damage * 4);
    assert_eq!(heavy.damage, baseline.damage * 16);
    // Hadouken with a charge tier ignores the request.
    let hadouken_baseline = ProjectileKind::Hadouken.spec(Vec2::ZERO, dir, 1.0);
    let hadouken_charged = ProjectileKind::Hadouken.charged_spec(hadouken_baseline, 2);
    assert_eq!(hadouken_charged.damage, hadouken_baseline.damage);
    assert_eq!(hadouken_charged.half_extent, hadouken_baseline.half_extent);
}

/// `FireballChargeTuning::tier_for_hold` quantizes hold-seconds
/// into 0/1/2. The thresholds are an authoring concern, but the
/// monotonicity contract is critical: a longer hold never
/// returns a smaller tier.
#[test]
fn fireball_charge_thresholds_quantize_monotonically() {
    let tuning = FireballChargeTuning::DEFAULT;
    assert_eq!(tuning.tier_for_hold(0.0), 0);
    assert_eq!(tuning.tier_for_hold(0.10), 0);
    assert_eq!(tuning.tier_for_hold(0.50), 1);
    assert_eq!(tuning.tier_for_hold(1.20), 2);
    // Monotonic over a wide range.
    let mut last = 0u8;
    for ms in (0..2000).step_by(50) {
        let t = tuning.tier_for_hold(ms as f32 / 1000.0);
        assert!(t >= last, "tier went backward at {ms}ms ({t} < {last})");
        last = t;
    }
}

/// Pin the +Y-DOWN convention of `MotionDirection::from_axis`.
/// The sandbox's `ControlFrame::axis_y` is also +Y-DOWN
/// (player presses Down → axis_y > 0), so the correct sandbox
/// → engine call is `from_axis(axis_x, axis_y, threshold)` with
/// NO sign flip. A previous version of the sandbox negated y
/// here under the (incorrect) assumption that the engine used
/// +Y up; the result was that every "press Down" sample
/// arrived at the buffer as `Up` and QCF detection silently
/// failed forever. This test exists so any future refactor
/// that "corrects" the convention has to break it explicitly.
#[test]
fn motion_direction_uses_y_down_like_sandbox() {
    // Down (sandbox: axis_y > 0) → MotionDirection::Down.
    assert_eq!(
        MotionDirection::from_axis(0.0, 1.0, 0.5),
        MotionDirection::Down
    );
    // Up (sandbox: axis_y < 0) → MotionDirection::Up.
    assert_eq!(
        MotionDirection::from_axis(0.0, -1.0, 0.5),
        MotionDirection::Up
    );
    // Down + Right → DownRight (matches the diagonal a player
    // hits on the way through a 3-step QCF).
    assert_eq!(
        MotionDirection::from_axis(0.7, 0.7, 0.5),
        MotionDirection::DownRight
    );
}

/// End-to-end: a Down → Right sequence pushed using the same convention
/// `charge_projectile_input` uses must be recognized as the grace QCF.
#[test]
fn down_then_right_via_from_axis_recognizes_grace_qcf() {
    let mut buf = MotionInputBuffer::new(0.5);
    let mut t = 0.0;
    // Sandbox-convention input: axis_y = 1.0 means Down.
    for (ax, ay) in [(0.0_f32, 1.0_f32), (1.0, 0.0)] {
        // PASS THROUGH (no sign flip) — must match the sandbox.
        let dir = MotionDirection::from_axis(ax, ay, 0.55);
        buf.push(dir, t);
        t += 0.04;
    }
    assert_eq!(
        qcf_grace().detect(&buf),
        Some(1.0),
        "Down-then-Right via from_axis must register as grace QCF"
    );
}

/// HadoukenSuper has strictly stronger stats than the grace
/// Hadouken. Pinning the relative ordering so a future tuning
/// pass doesn't accidentally make the harder gesture weaker.
#[test]
fn hadouken_super_dominates_hadouken_stats() {
    assert!(ProjectileKind::HadoukenSuper.damage() > ProjectileKind::Hadouken.damage());
    assert!(ProjectileKind::HadoukenSuper.cost() > ProjectileKind::Hadouken.cost());
    assert!(ProjectileKind::HadoukenSuper.speed() > ProjectileKind::Hadouken.speed());
    let super_hb = ProjectileKind::HadoukenSuper.half_extent();
    let normal_hb = ProjectileKind::Hadouken.half_extent();
    assert!(super_hb.x > normal_hb.x);
    assert!(super_hb.y > normal_hb.y);
}

#[test]
fn motion_direction_quantization() {
    assert_eq!(
        MotionDirection::from_axis(0.05, 0.05, 0.2),
        MotionDirection::Neutral
    );
    assert_eq!(
        MotionDirection::from_axis(0.8, 0.0, 0.2),
        MotionDirection::Right
    );
    assert_eq!(
        MotionDirection::from_axis(0.6, 0.6, 0.2),
        MotionDirection::DownRight
    );
    assert_eq!(
        MotionDirection::from_axis(-0.7, 0.7, 0.2),
        MotionDirection::DownLeft
    );
}

#[test]
fn outgoing_damage_multiplier_scales_damage() {
    let spec = ProjectileKind::Hadouken.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 2.0);
    // Hadouken default is 3 damage; 2x = 6.
    assert_eq!(spec.damage, 6);
}

#[test]
fn from_spec_lowers_kind_data_onto_body() {
    let spec = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    let body = ProjectileBody::from_spec(spec);
    // The named kind no longer rides the (generic) body; its lowered data does —
    // Fireball authors a 2-bounce budget.
    assert_eq!(
        body.game.bounces_remaining,
        ProjectileKind::Fireball.bounces()
    );
    assert_eq!(body.kin.pos, Vec2::ZERO);
}

/// ⭐ THE BOOMERANG GOES OUT, STOPS, AND COMES HOME, and each of those three is
/// asserted separately — a shot that merely decelerated would pass a test that
/// only checked "it is not as far as it should be".
#[test]
fn a_boomerang_turns_around_and_returns_to_where_it_was_thrown() {
    const OUT_S: f32 = 0.34;
    const DT: f32 = 1.0 / 60.0;
    let mut spec = ProjectileKind::Hadouken.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    spec.boomerang_return_s = Some(OUT_S);
    // The ROUND TRIP, which is what `ProjectileFlight::boomerang` authors — the
    // return acceleration is `-v0 / out_s`, so the shot is back at the throw
    // point at exactly `2 * out_s`. ⛔ This used to restate the old `+ 0.15`
    // and so agreed with a tail that expired 79px behind the hand.
    spec.max_lifetime = OUT_S * 2.0;
    let mut body = ProjectileBody::from_spec(spec);

    // OUT: half the outbound leg in, it is moving away.
    for _ in 0..(OUT_S / DT / 2.0) as usize {
        body.tick(DT, Vec2::new(0.0, 1.0));
    }
    let halfway = body.kin.pos.x;
    assert!(halfway > 0.0, "the tail must go out, and it is at {halfway}");
    assert!(body.kin.vel.x > 0.0, "…and still be going out");

    // STOP: at the turnaround its speed along the throw is gone.
    while body.game.age < OUT_S {
        body.tick(DT, Vec2::new(0.0, 1.0));
    }
    let apex = body.kin.pos.x;
    assert!(
        body.kin.vel.x.abs() < 20.0,
        "the tail must have stopped at the turnaround, and it is doing {}",
        body.kin.vel.x
    );
    assert!(
        apex > halfway,
        "…having travelled further out first ({halfway} -> {apex})"
    );

    // HOME: it passes back through the throw point, moving the other way.
    let mut returned = false;
    while body.tick(DT, Vec2::new(0.0, 1.0)) {
        if body.kin.pos.x <= 0.0 {
            returned = true;
            break;
        }
    }
    assert!(
        returned,
        "the tail must come back through the hand that threw it; it expired at \
         x={}",
        body.kin.pos.x
    );
    assert!(body.kin.vel.x < 0.0, "…still travelling homeward");

    // ⛔⛔ AND IT IS CAUGHT, NOT MERELY PASSING. "Eventually crossed zero" is
    // satisfied by a tail that sails on through and expires as a fast rearward
    // projectile — which is exactly what it did: 79.2px behind the hand at
    // 603 px/s (GPT 5.6, 2026-08-27). Run it to despawn and BOUND the overshoot.
    let mut last = body.kin.pos.x;
    while body.tick(DT, Vec2::new(0.0, 1.0)) {
        last = body.kin.pos.x;
    }
    let overshoot = -last;
    assert!(
        overshoot < 16.0,
        "the tail expired {overshoot:.1}px BEHIND the hand that threw it — a \
         boomerang that keeps going is a second projectile nobody aimed"
    );
}

/// ⛔ THE PAIRED ARM. The same shot without a turnaround keeps going, so the
/// test above is about `boomerang_return_s` and not about the stepper having
/// quietly acquired a drag term.
#[test]
fn a_shot_with_no_turnaround_never_comes_back() {
    const DT: f32 = 1.0 / 60.0;
    let spec = ProjectileKind::Hadouken.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
    assert_eq!(spec.boomerang_return_s, None);
    let mut body = ProjectileBody::from_spec(spec);
    let mut last = f32::NEG_INFINITY;
    while body.tick(DT, Vec2::new(0.0, 1.0)) {
        assert!(
            body.kin.pos.x >= last,
            "a straight shot must never travel backwards"
        );
        last = body.kin.pos.x;
    }
    assert!(last > 0.0);
}
