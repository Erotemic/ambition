//! Generic encounter OBJECTIVES (§5): a small predicate vocabulary over
//! participants, elapsed time, and received signals.
//!
//! A conventional boss fight is `AllWithRoleDefeated(PrimaryTarget)`; a wave
//! arena `AllWithRoleDefeated(Minion)`; a survival section `Survive(secs)`; a
//! race/puzzle `ReceiveSignal(key)`. There is deliberately NO `Custom(String)`
//! escape hatch — if content needs a new fact it publishes a typed/stable-key
//! signal and the generic objective consumes it (§5), so the generic runtime
//! never interprets game names.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::participants::{EncounterParticipants, EncounterRole};

/// The generic objective predicate (§5). `All`/`Any` compose the leaves.
#[derive(Clone, Debug, PartialEq)]
pub enum Objective {
    /// Every member playing `role` is defeated (and at least one exists).
    AllWithRoleDefeated(EncounterRole),
    /// Any member playing `role` is defeated.
    AnyWithRoleDefeated(EncounterRole),
    /// `secs` elapsed since the encounter went Active (survive the timer).
    Survive(f32),
    /// A signal with this key was received this encounter.
    ReceiveSignal(String),
    /// Every sub-objective is met.
    All(Vec<Objective>),
    /// Any sub-objective is met.
    Any(Vec<Objective>),
}

/// The win (and optional fail) condition of one encounter entity (§5).
#[derive(Component, Clone, Debug)]
pub struct EncounterObjective {
    /// The condition that completes the encounter.
    pub win: Objective,
    /// An optional condition that fails it (e.g. a `Protected` member died).
    pub fail: Option<Objective>,
}

impl EncounterObjective {
    /// A win-only objective (the common case).
    pub fn win(win: Objective) -> Self {
        Self { win, fail: None }
    }

    /// A win/lose objective.
    pub fn win_or_fail(win: Objective, fail: Objective) -> Self {
        Self {
            win,
            fail: Some(fail),
        }
    }
}

/// Evaluate a generic objective against the live encounter facts. `elapsed_secs`
/// is time since the encounter went Active; `signals` is the set of signal keys
/// received this encounter. Pure — headless-testable, order-independent.
pub fn objective_met(
    objective: &Objective,
    participants: &EncounterParticipants,
    elapsed_secs: f32,
    signals: &HashSet<String>,
) -> bool {
    match objective {
        Objective::AllWithRoleDefeated(role) => participants.all_with_role_defeated(*role),
        Objective::AnyWithRoleDefeated(role) => participants.any_with_role_defeated(*role),
        Objective::Survive(secs) => elapsed_secs >= *secs,
        Objective::ReceiveSignal(key) => signals.contains(key),
        Objective::All(subs) => subs
            .iter()
            .all(|s| objective_met(s, participants, elapsed_secs, signals)),
        Objective::Any(subs) => subs
            .iter()
            .any(|s| objective_met(s, participants, elapsed_secs, signals)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::participants::{EncounterParticipant, EncounterRole, Ownership};

    fn parts(members: Vec<(&str, EncounterRole, bool)>) -> EncounterParticipants {
        EncounterParticipants::new(
            members
                .into_iter()
                .map(|(id, role, alive)| EncounterParticipant {
                    id: id.into(),
                    entity: None,
                    role,
                    ownership: Ownership::Adopted,
                    alive,
                })
                .collect(),
        )
    }

    #[test]
    fn boss_objective_completes_when_the_primary_target_dies() {
        let obj = Objective::AllWithRoleDefeated(EncounterRole::PrimaryTarget);
        let alive = parts(vec![("boss", EncounterRole::PrimaryTarget, true)]);
        let dead = parts(vec![("boss", EncounterRole::PrimaryTarget, false)]);
        let none = HashSet::new();
        assert!(!objective_met(&obj, &alive, 0.0, &none));
        assert!(objective_met(&obj, &dead, 0.0, &none));
    }

    #[test]
    fn survive_and_signal_and_compose() {
        let empty = EncounterParticipants::default();
        assert!(objective_met(
            &Objective::Survive(3.0),
            &empty,
            3.5,
            &HashSet::new()
        ));
        let mut signals = HashSet::new();
        signals.insert("gate_reached".to_string());
        assert!(objective_met(
            &Objective::ReceiveSignal("gate_reached".into()),
            &empty,
            0.0,
            &signals
        ));
        // Any([survive(never), signal(present)]) → true via the signal leaf.
        assert!(objective_met(
            &Objective::Any(vec![
                Objective::Survive(999.0),
                Objective::ReceiveSignal("gate_reached".into()),
            ]),
            &empty,
            0.0,
            &signals
        ));
    }
}
