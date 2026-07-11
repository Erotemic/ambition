//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn snap_with(distance: f32, alive: bool) -> CharacterAiSnapshot {
    CharacterAiSnapshot {
        actor_pos: Vec2::ZERO,
        player_pos: Vec2::new(distance, 0.0),
        aggro_radius: 200.0,
        attack_range: 60.0,
        attack_windup_remaining: 0.0,
        attack_active_remaining: 0.0,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
        alive,
        patrol_enabled: true,
    }
}

#[test]
fn dead_short_circuits() {
    let s = snap_with(10.0, false);
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Dead);
}

#[test]
fn stunned_short_circuits() {
    let mut s = snap_with(10.0, true);
    s.stun_remaining = 0.5;
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Stunned);
}

#[test]
fn active_attack_takes_precedence_over_distance() {
    let mut s = snap_with(800.0, true);
    s.attack_active_remaining = 0.05;
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Attack);
}

#[test]
fn windup_resolves_to_telegraph() {
    let mut s = snap_with(50.0, true);
    s.attack_windup_remaining = 0.20;
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Telegraph);
}

#[test]
fn recover_holds_until_zero() {
    let mut s = snap_with(50.0, true);
    s.attack_recover_remaining = 0.10;
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Recover);
}

#[test]
fn aggro_radius_resolves_to_chase() {
    let s = snap_with(150.0, true);
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Chase);
}

#[test]
fn chase_intent_reports_policy_side_not_world_axis() {
    let s = snap_with(-150.0, true);
    let out = evaluate_character_ai_output(s);
    assert_eq!(out.mode, CharacterAiMode::Chase);
    assert_eq!(
        out.intent,
        CharacterAiIntent::Chase {
            direction_side: -1.0,
        },
        "the x component of CharacterAiSnapshot is policy side-space"
    );
}

#[test]
fn within_attack_range_resolves_to_chase_for_caller_to_kick_off_windup() {
    let s = snap_with(40.0, true);
    // The caller decides to start the windup — this evaluator
    // doesn't manufacture the wind-up timer. So when the actor is
    // in range but hasn't been told to swing, "Chase" is the
    // right answer (close, holding position).
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Chase);
}

#[test]
fn far_with_patrol_resolves_to_patrol() {
    let s = snap_with(800.0, true);
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Patrol);
}

#[test]
fn far_without_patrol_resolves_to_idle() {
    let mut s = snap_with(800.0, true);
    s.patrol_enabled = false;
    assert_eq!(evaluate_character_ai(s), CharacterAiMode::Idle);
}

#[test]
fn character_ai_mode_is_dangerous_only_in_attack() {
    assert!(CharacterAiMode::Attack.is_dangerous());
    assert!(!CharacterAiMode::Idle.is_dangerous());
    assert!(!CharacterAiMode::Patrol.is_dangerous());
    assert!(!CharacterAiMode::Chase.is_dangerous());
    assert!(!CharacterAiMode::Telegraph.is_dangerous());
    assert!(!CharacterAiMode::Recover.is_dangerous());
    assert!(!CharacterAiMode::Stunned.is_dangerous());
    assert!(!CharacterAiMode::Dead.is_dangerous());
}

#[test]
fn character_ai_mode_is_committed_during_attack_window() {
    // Telegraph / Attack / Recover are the "committed" modes — the
    // actor is locked into the attack cycle and can't pivot mid-swing.
    assert!(CharacterAiMode::Telegraph.is_committed());
    assert!(CharacterAiMode::Attack.is_committed());
    assert!(CharacterAiMode::Recover.is_committed());
    // Other modes are interruptible.
    assert!(!CharacterAiMode::Idle.is_committed());
    assert!(!CharacterAiMode::Patrol.is_committed());
    assert!(!CharacterAiMode::Chase.is_committed());
    assert!(!CharacterAiMode::Stunned.is_committed());
    assert!(!CharacterAiMode::Dead.is_committed());
}

#[test]
fn character_ai_mode_labels_are_unique_and_non_empty() {
    let modes = [
        CharacterAiMode::Idle,
        CharacterAiMode::Patrol,
        CharacterAiMode::Chase,
        CharacterAiMode::Telegraph,
        CharacterAiMode::Attack,
        CharacterAiMode::Recover,
        CharacterAiMode::Stunned,
        CharacterAiMode::Dead,
    ];
    let labels: Vec<&str> = modes.iter().map(|m| m.label()).collect();
    for label in &labels {
        assert!(!label.is_empty());
    }
    for (i, a) in labels.iter().enumerate() {
        for b in &labels[i + 1..] {
            assert_ne!(a, b);
        }
    }
}
#[test]
fn output_reports_attack_intent_in_range() {
    let s = snap_with(30.0, true);
    let out = evaluate_character_ai_output(s);
    assert_eq!(out.mode, CharacterAiMode::Chase);
    assert!(matches!(out.intent, CharacterAiIntent::Attack { .. }));
}

#[test]
fn output_reports_patrol_intent_out_of_range() {
    let mut s = snap_with(300.0, true);
    s.patrol_enabled = true;
    let out = evaluate_character_ai_output(s);
    assert_eq!(out.mode, CharacterAiMode::Patrol);
    assert_eq!(out.intent, CharacterAiIntent::Patrol);
}
