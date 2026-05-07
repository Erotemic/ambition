//! Character AI vocabulary shared by enemies, hostile NPCs, and bosses.
//!
//! The seldom_state component vocabulary in `crate::state_machines`
//! (`EnemyIdle`, `EnemyPatrol`, `EnemyTelegraph`, `EnemyAttack`,
//! `EnemyRecover`, `EnemyStunned`, `EnemyDead`) describes the
//! per-entity state. This module owns the *evaluation* shape — the
//! pure function that, given a snapshot of an actor's situation,
//! returns the AI mode it should be in this tick.
//!
//! Keeping the evaluator pure (no Bevy, no `EnemyRuntime`) means:
//! - The same logic runs in headless tests deterministically.
//! - Sandbox enemies / hostile NPCs / boss minions can all share one
//!   AI without copy-pasting state machines.
//! - A future "character state machine" refactor can plug this in
//!   piecemeal without touching seldom_state component plumbing.
//!
//! Today the sandbox `EnemyRuntime` and `BossRuntime` carry their own
//! ad-hoc AI logic. The path forward (documented in
//! `docs/character_ai_refactor.md`) is to make those runtimes consume
//! `CharacterAiSnapshot` + `CharacterAiOutput` and let this module be
//! the single source of truth.

use crate::Vec2;

/// What the actor should be doing this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterAiMode {
    /// Stand still / wait for the player to enter aggro range.
    #[default]
    Idle,
    /// Move along an authored patrol path or pace the spawn point.
    Patrol,
    /// Player is in aggro range; close the distance.
    Chase,
    /// Player is in attack range; play the wind-up telegraph.
    Telegraph,
    /// Active hit window — the attack volume is dangerous.
    Attack,
    /// Post-attack recovery; vulnerable, can't act.
    Recover,
    /// Briefly disabled by a hit / pogo / story rule.
    Stunned,
    /// Dead — should not run AI.
    Dead,
}

impl CharacterAiMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Patrol => "patrol",
            Self::Chase => "chase",
            Self::Telegraph => "telegraph",
            Self::Attack => "attack",
            Self::Recover => "recover",
            Self::Stunned => "stunned",
            Self::Dead => "dead",
        }
    }

    pub fn is_dangerous(self) -> bool {
        matches!(self, Self::Attack)
    }

    pub fn is_committed(self) -> bool {
        matches!(self, Self::Telegraph | Self::Attack | Self::Recover)
    }
}

/// Read-only view the AI evaluator needs each tick.
///
/// All input fields come from the actor's runtime. Keeping the
/// snapshot small and `Copy` makes the evaluator trivially testable.
#[derive(Clone, Copy, Debug)]
pub struct CharacterAiSnapshot {
    pub actor_pos: Vec2,
    pub player_pos: Vec2,
    pub aggro_radius: f32,
    pub attack_range: f32,
    pub attack_windup_remaining: f32,
    pub attack_active_remaining: f32,
    pub attack_recover_remaining: f32,
    pub stun_remaining: f32,
    pub alive: bool,
    /// True when the actor should patrol when not engaged (e.g. has a
    /// path or a non-zero patrol speed). False makes "not engaged"
    /// resolve to `Idle` instead of `Patrol`.
    pub patrol_enabled: bool,
}

impl CharacterAiSnapshot {
    pub fn distance_to_player(&self) -> f32 {
        (self.player_pos - self.actor_pos).length()
    }
}

/// Pure evaluation: which mode should the actor be in this tick?
pub fn evaluate_character_ai(snap: CharacterAiSnapshot) -> CharacterAiMode {
    if !snap.alive {
        return CharacterAiMode::Dead;
    }
    if snap.stun_remaining > 0.0 {
        return CharacterAiMode::Stunned;
    }
    if snap.attack_active_remaining > 0.0 {
        return CharacterAiMode::Attack;
    }
    if snap.attack_windup_remaining > 0.0 {
        return CharacterAiMode::Telegraph;
    }
    if snap.attack_recover_remaining > 0.0 {
        return CharacterAiMode::Recover;
    }
    let dist = snap.distance_to_player();
    if dist <= snap.attack_range.max(0.0) {
        // In strike range but not currently swinging — caller will
        // start the attack windup; surface as Telegraph so the HUD
        // / animation shows the wind-up frame as soon as the timer
        // is set.
        CharacterAiMode::Chase
    } else if dist <= snap.aggro_radius.max(0.0) {
        CharacterAiMode::Chase
    } else if snap.patrol_enabled {
        CharacterAiMode::Patrol
    } else {
        CharacterAiMode::Idle
    }
}

#[cfg(test)]
mod tests {
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
}
