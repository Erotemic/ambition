use super::*;
use ambition_characters::brain::action_set::{MeleeActionSpec, SwipeSpec};
use ambition_characters::brain::smash::CrowdingSignal;
use ambition_characters::brain::smash::SmashCfg;

fn obs_at(distance_x: f32, attacking: bool) -> ObservationFrame {
    ObservationFrame {
        self_pos: ae::Vec2::ZERO,
        self_vel: ae::Vec2::ZERO,
        self_facing: 1.0,
        self_on_ground: true,
        self_aerial: false,
        self_alive: true,
        self_captured: false,
        self_holding_captive: false,
        self_pummels_landed: 0,
        self_attacking: attacking,
        self_air_jumps_remaining: 0,
        attack_cooldown_remaining: 0.0,
        stun_remaining: 0.0,
        self_health_fraction: 1.0,
        target_pos: ae::Vec2::new(distance_x, 0.0),
        target_alive: true,
        to_target_x: distance_x,
        to_target_y: 0.0,
        distance_to_target: distance_x.abs(),
        down: ae::Vec2::new(0.0, 1.0),
        crowding: CrowdingSignal::default(),
        terrain: Default::default(),
        sim_time: 1.0,
        dt: 1.0 / 60.0,
    }
}

#[test]
fn approach_picks_walk_toward_target() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet::peaceful();
    let act = choose_action(&obs_at(300.0, false), BroadMode::Approach, &cfg, &actions);
    match act {
        SpecificAction::Walk { dir } => assert!(dir > 0.0),
        other => panic!("expected Walk, got {other:?}"),
    }
    let act = choose_action(&obs_at(-300.0, false), BroadMode::Approach, &cfg, &actions);
    match act {
        SpecificAction::Walk { dir } => assert!(dir < 0.0),
        other => panic!("expected Walk left, got {other:?}"),
    }
}

#[test]
fn engage_with_melee_in_range_emits_attack() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let act = choose_action(&obs_at(40.0, false), BroadMode::Engage, &cfg, &actions);
    assert!(
        matches!(act, SpecificAction::MeleeAttack { .. }),
        "got {act:?}"
    );
}

/// A Smash brain reaches directional moves by aiming its melee action. Unlike a
/// Fighter brain it does not enumerate a scored `attack_kit`; `MeleeAttack { dir }`
/// becomes `ActorControlFrame::attack_axis` in `emit`, and
/// `resolve_attack_gestures` turns that axis into `attack_up` / `attack_down` /
/// `attack_forward` out of the body's OWN `MovesetContract`, falling back to the
/// base attack when the character authored no directional variant.
///
///  so a goblin that authors an up-tilt throws it at a target above, and the
/// only thing choosing is this function. It had ONE test — that a melee comes out
/// at all — and the direction, which is the entire mechanism, was unasserted.
///
///  three cases and they are not redundant: a function returning a constant
/// forward swing passes any single one of them, and "up" and "down" are
/// different branches with different guards — down additionally requires being
/// AIRBORNE, because a grounded body above its target is on a platform and
/// swinging at the floor is not the read.
///
///  gravity-framed, never world-framed: `down` is the observation's, so a
/// rotated-gravity room gets the same reads (I10).
#[test]
fn an_engaged_swing_aims_at_where_the_target_actually_is() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let swing_dir =
        |obs: ObservationFrame| match choose_action(&obs, BroadMode::Engage, &cfg, &actions) {
            SpecificAction::MeleeAttack { dir } => dir,
            other => panic!("expected a melee swing, got {other:?}"),
        };

    // Level with the target: swing along the side axis, toward it.
    let level = swing_dir(obs_at(40.0, false));
    assert!(
        level.x > 0.5 && level.y.abs() < 0.5,
        "a level target should be swung at sideways, got {level:?}"
    );

    // Target ABOVE (authored geometry is y-down, so above is negative y).
    let mut above = obs_at(40.0, false);
    above.target_pos = ae::Vec2::new(40.0, -60.0);
    above.to_target_y = -60.0;
    let up = swing_dir(above);
    assert!(
        up.y < -0.5,
        "a target overhead should be swung at UPWARD — this is what reaches an \
         authored `attack_up` instead of the base attack — got {up:?}"
    );

    // Target BELOW and this body AIRBORNE: the down-air.
    let mut below = obs_at(40.0, false);
    below.target_pos = ae::Vec2::new(40.0, 60.0);
    below.to_target_y = 60.0;
    below.self_on_ground = false;
    below.self_aerial = true;
    let down = swing_dir(below);
    assert!(
        down.y > 0.5,
        "an airborne body over its target should swing DOWNWARD, which is what \
         reaches an authored `attack_air_down` — got {down:?}"
    );

    //  THE POISON: the same target below, but GROUNDED. A body standing on a
    // platform above its foe is not throwing a down-air; a rule that read only
    // the vertical offset would answer identically here and be wrong.
    let mut below_grounded = obs_at(40.0, false);
    below_grounded.target_pos = ae::Vec2::new(40.0, 60.0);
    below_grounded.to_target_y = 60.0;
    let grounded = swing_dir(below_grounded);
    assert!(
        grounded.y <= 0.5,
        "a GROUNDED body swung downward at a target below it, so the airborne \
         guard is gone and the pick is reading the offset alone: {grounded:?}"
    );
}

#[test]
fn engage_without_melee_capability_does_not_attack() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet::peaceful(); // no melee
    let act = choose_action(&obs_at(40.0, false), BroadMode::Engage, &cfg, &actions);
    assert!(!matches!(act, SpecificAction::MeleeAttack { .. }));
}

#[test]
fn engage_on_cooldown_holds_instead_of_attacking() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let mut obs = obs_at(40.0, false);
    obs.attack_cooldown_remaining = 0.5;
    let act = choose_action(&obs, BroadMode::Engage, &cfg, &actions);
    assert_eq!(act, SpecificAction::Idle, "got {act:?}");
}

/// §A1 subsumption: the AUTONOMOUS special cadence is deliberately OFF (a naive "fire while melee
/// recharges" spammed the move and broke the damage-triggered regroup kit) — so even a fighter WITH
/// a signature special holds in Engage on cooldown, same as one without. The moveset is still the
/// executor; possession fires the special via `special_pressed`.
#[test]
fn engage_on_cooldown_holds_even_with_a_signature_special() {
    use ambition_characters::brain::action_set::SpecialActionSpec;
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        special: Some(SpecialActionSpec::Special("cellular_pulse".to_string())),
        ..Default::default()
    };
    let mut obs = obs_at(40.0, false);
    obs.attack_cooldown_remaining = 0.5; // melee recharging
    let act = choose_action(&obs, BroadMode::Engage, &cfg, &actions);
    assert_eq!(act, SpecificAction::Idle, "got {act:?}");
}

#[test]
fn retreat_walks_away_from_target() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet::peaceful();
    // Target to the right (positive x) → retreat = walk left.
    let act = choose_action(&obs_at(20.0, false), BroadMode::Retreat, &cfg, &actions);
    match act {
        SpecificAction::Walk { dir } => assert!(dir < 0.0),
        other => panic!("expected Walk left, got {other:?}"),
    }
}

#[test]
fn reposition_front_actor_pushes_through_toward_target() {
    // Target is to the LEFT (negative x). The actor is the
    // "front" (closer to target than the ally behind), so the
    // crowding away_dir points LEFT (away from the ally that
    // sits to the right of the actor). away_dir.x sign matches
    // toward_target.x sign → walk forward.
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet::peaceful();
    let mut obs = obs_at(-300.0, false); // target on left
    obs.crowding.away_dir = ae::Vec2::new(-1.0, 0.0); // ally is to the right of us
    let act = choose_action(&obs, BroadMode::Reposition, &cfg, &actions);
    match act {
        SpecificAction::Walk { dir } => assert!(
            dir < 0.0,
            "front actor should push left toward target; got {dir}"
        ),
        other => panic!("expected Walk, got {other:?}"),
    }
}

#[test]
fn reposition_back_actor_holds_rather_than_retreats() {
    // Target on the LEFT, but away_dir points RIGHT (ally is
    // to our left, between us and target). Walking away from
    // the centroid would mean retreating to the right. The back
    // actor holds instead.
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet::peaceful();
    let mut obs = obs_at(-300.0, false);
    obs.crowding.away_dir = ae::Vec2::new(1.0, 0.0);
    let act = choose_action(&obs, BroadMode::Reposition, &cfg, &actions);
    assert_eq!(
        act,
        SpecificAction::Idle,
        "back actor should hold; got {act:?}"
    );
}

#[test]
fn mid_swing_emits_idle_regardless_of_mode() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let obs = obs_at(40.0, true); // self_attacking = true
    for mode in [
        BroadMode::Approach,
        BroadMode::Retreat,
        BroadMode::Engage,
        BroadMode::Reposition,
    ] {
        let act = choose_action(&obs, mode, &cfg, &actions);
        assert_eq!(act, SpecificAction::Idle, "mode={mode:?}");
    }
}
