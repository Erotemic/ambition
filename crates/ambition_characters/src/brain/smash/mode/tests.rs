//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::super::observation::CrowdingSignal;
use super::*;
use ambition_engine_core as ae;

fn obs_at(distance_x: f32) -> ObservationFrame {
    ObservationFrame {
        self_pos: ae::Vec2::ZERO,
        self_vel: ae::Vec2::ZERO,
        self_facing: 1.0,
        self_on_ground: true,
        self_aerial: false,
        self_alive: true,
        self_attacking: false,
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
fn idles_outside_aggro() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let obs = obs_at(2000.0);
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Idle);
}

#[test]
fn approaches_at_long_range() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let obs = obs_at(300.0); // outside engage_distance (70)
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Approach);
}

#[test]
fn engages_inside_attack_range() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let obs = obs_at(40.0); // inside attack_range (56)
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Engage);
}

#[test]
fn engages_when_point_blank_and_in_attack_range() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let obs = obs_at(20.0); // inside both too_close (30) and attack_range (56)
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Engage);
}

#[test]
fn retreats_when_too_close_but_not_in_attack_range() {
    let mut cfg = SmashCfg::STRIKER_DEFAULT;
    cfg.attack_range = 10.0;
    cfg.too_close_distance = 30.0;
    let mut state = SmashState::default();
    let obs = obs_at(20.0);
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Retreat);
}

#[test]
fn repositions_when_crowded() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let mut obs = obs_at(300.0);
    obs.crowding.pressure = 1.0; // way over 0.65 threshold
    obs.crowding.away_dir = ae::Vec2::new(-1.0, 0.0);
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Reposition,);
}

#[test]
fn hysteresis_prevents_approach_to_retreat_flip_within_dwell() {
    // Pin the contract: once committed to Approach, a brief dip
    // below `too_close_distance` (into the Retreat band, NOT swing
    // range) should not immediately flip to Retreat. Engage is the
    // only candidate that bypasses dwell. STRIKER_DEFAULT keeps
    // `too_close_distance` inside `attack_range` so a point-blank
    // pirate engages instead of retreating, which leaves no Retreat
    // band — so carve an explicit one the way
    // `retreats_when_too_close_but_not_in_attack_range` does.
    let mut cfg = SmashCfg::STRIKER_DEFAULT;
    cfg.attack_range = 10.0;
    cfg.too_close_distance = 30.0;
    let mut state = SmashState::default();
    let obs = obs_at(300.0);
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Approach);
    // Simulate caller's per-tick `mode_dwell_s += dt` ahead of
    // the second call: 0.1 s < MODE_MIN_DWELL_S = 0.18.
    state.mode_dwell_s = 0.1;
    let obs2 = obs_at(20.0);
    let chosen = choose_mode(&obs2, &cfg, &mut state);
    assert_eq!(chosen, BroadMode::Approach, "should stick within dwell");
}

#[test]
fn hysteresis_does_not_block_engage_transition() {
    // Engage is a hard exit from the dwell window — the brain
    // shouldn't waste a strike opportunity for hysteresis.
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let _ = choose_mode(&obs_at(300.0), &cfg, &mut state);
    assert_eq!(state.mode, BroadMode::Approach);
    state.mode_dwell_s = 0.05;
    let chosen = choose_mode(&obs_at(40.0), &cfg, &mut state);
    assert_eq!(chosen, BroadMode::Engage);
}

#[test]
fn stun_forces_idle() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let mut obs = obs_at(40.0); // would be Engage
    obs.stun_remaining = 0.5;
    assert_eq!(choose_mode(&obs, &cfg, &mut state), BroadMode::Idle);
}
