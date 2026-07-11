//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::super::observation::CrowdingSignal;
use super::super::SmashCfg;
use super::*;
use crate::brain::action_set::{MeleeActionSpec, SwipeSpec};

fn obs_at(distance_x: f32, attacking: bool) -> ObservationFrame {
    ObservationFrame {
        self_pos: ae::Vec2::ZERO,
        self_vel: ae::Vec2::ZERO,
        self_facing: 1.0,
        self_on_ground: true,
        self_aerial: false,
        self_alive: true,
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

/// §A1 subsumption: the AUTONOMOUS special cadence is deliberately OFF (a naive
/// "fire while melee recharges" spammed the move and broke the damage-triggered
/// regroup kit) — so even a fighter WITH a signature special holds in Engage on
/// cooldown, same as one without. The moveset is still the executor; possession
/// fires the special via `special_pressed`. Re-enabling autonomous firing is a
/// feel/AI cadence pass (a real special cooldown) for Jon against the landed system.
#[test]
fn engage_on_cooldown_holds_even_with_a_signature_special() {
    use crate::brain::action_set::SpecialActionSpec;
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
