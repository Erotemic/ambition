//! Character AI vocabulary shared by enemies, hostile NPCs, and bosses.
//!
//! The seldom_state component vocabulary in `ambition_actors::state_machines`
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
//! `docs/systems/actors-brains-and-character-content.md`) is to make those runtimes consume
//! `CharacterAiSnapshot` + `CharacterAiOutput` and let this module be
//! the single source of truth.

use ambition_engine_core::Vec2;

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

/// Coarse movement/attack intent paired with a [`CharacterAiMode`].
///
/// The sandbox supplies actor-specific speeds and collision rules, but this
/// engine-owned intent is the authority for whether an actor should hold,
/// patrol, chase, or request an attack windup this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CharacterAiIntent {
    /// No voluntary movement.
    #[default]
    Hold,
    /// Move along an authored/path-local patrol or fallback pacing lane.
    Patrol,
    /// Close on the target along the provided local side direction.
    Chase { direction_side: f32 },
    /// Caller should start an attack windup facing `direction_side` when its
    /// cooldown/archetype rules allow it.
    Attack { direction_side: f32 },
}

/// Engine-owned AI decision consumed by sandbox enemies/NPCs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CharacterAiOutput {
    pub mode: CharacterAiMode,
    pub intent: CharacterAiIntent,
}

impl CharacterAiOutput {
    pub fn committed(self) -> bool {
        self.mode.is_committed()
    }
}

/// Read-only view the AI evaluator needs each tick.
///
/// All input fields come from the actor's runtime. Keeping the
/// snapshot small and `Copy` makes the evaluator trivially testable.
#[derive(Clone, Copy, Debug)]
pub struct CharacterAiSnapshot {
    /// Actor origin in the evaluator's policy space.
    ///
    /// Direct unit tests often use world-like coordinates. Live actor brains
    /// feed this evaluator through `BrainSnapshot::to_character_ai_snapshot`,
    /// which first converts target deltas into the actor's acceleration frame.
    /// Therefore the evaluator treats the x component as the policy **side**
    /// axis, not necessarily raw world X.
    pub actor_pos: Vec2,
    /// Target/player origin in the same policy space as [`Self::actor_pos`].
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
    evaluate_character_ai_output(snap).mode
}

/// Pure evaluation: which mode + coarse intent should the actor run this tick?
pub fn evaluate_character_ai_output(snap: CharacterAiSnapshot) -> CharacterAiOutput {
    if !snap.alive {
        return CharacterAiOutput {
            mode: CharacterAiMode::Dead,
            intent: CharacterAiIntent::Hold,
        };
    }
    if snap.stun_remaining > 0.0 {
        return CharacterAiOutput {
            mode: CharacterAiMode::Stunned,
            intent: CharacterAiIntent::Hold,
        };
    }
    if snap.attack_active_remaining > 0.0 {
        return CharacterAiOutput {
            mode: CharacterAiMode::Attack,
            intent: CharacterAiIntent::Hold,
        };
    }
    if snap.attack_windup_remaining > 0.0 {
        return CharacterAiOutput {
            mode: CharacterAiMode::Telegraph,
            intent: CharacterAiIntent::Hold,
        };
    }
    if snap.attack_recover_remaining > 0.0 {
        return CharacterAiOutput {
            mode: CharacterAiMode::Recover,
            intent: CharacterAiIntent::Hold,
        };
    }
    let delta = snap.player_pos - snap.actor_pos;
    let dist = delta.length();
    let direction_side = if delta.x.abs() <= 0.001 {
        0.0
    } else {
        delta.x.signum()
    };
    if dist <= snap.attack_range.max(0.0) {
        CharacterAiOutput {
            mode: CharacterAiMode::Chase,
            intent: CharacterAiIntent::Attack { direction_side },
        }
    } else if dist <= snap.aggro_radius.max(0.0) {
        CharacterAiOutput {
            mode: CharacterAiMode::Chase,
            intent: CharacterAiIntent::Chase { direction_side },
        }
    } else if snap.patrol_enabled {
        CharacterAiOutput {
            mode: CharacterAiMode::Patrol,
            intent: CharacterAiIntent::Patrol,
        }
    } else {
        CharacterAiOutput {
            mode: CharacterAiMode::Idle,
            intent: CharacterAiIntent::Hold,
        }
    }
}

#[cfg(test)]
mod tests;
