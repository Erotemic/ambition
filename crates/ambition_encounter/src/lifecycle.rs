//! The ONE encounter lifecycle authority (E8/E9).
//!
//! Every encounter entity — a wave arena, a boss wrap, a signal-driven puzzle —
//! carries an [`EncounterLifecycle`] and moves through the same generic phases:
//! `Inactive → Starting → Active → Completed | Failed`. Transitions come from
//! exactly two places:
//!
//! 1. **Commands** ([`EncounterCommand`], the generic ingress): adapters — the
//!    wave trigger, the boss wrap, content — request `Start`, `Complete`,
//!    `Fail`, `Signal`, or `Reset`. No adapter mutates the phase directly.
//! 2. **Objectives** ([`EncounterObjective`](crate::EncounterObjective),
//!    evaluated by the reducer): while `Active`, the win (and optional fail)
//!    objective is evaluated against participants, elapsed time, and received
//!    signals. Completion is a generic decision, never wave- or boss-specific
//!    code.
//!
//! The reducer ([`EncounterLifecycle::reduce`]) is pure and headless-testable;
//! [`reduce_encounter_lifecycles`] is its one ECS registration (in
//! [`EncounterLifecycleSet`], ordered by the runtime after the adapters that
//! refresh participant liveness and emit commands).

use std::collections::BTreeSet;

use bevy::prelude::*;

use ambition_persistence::save_data::PersistedEncounterState;

use crate::entity::Encounter;
use crate::events::{EncounterEvent, EncounterEventMsg};
use crate::objective::{objective_met, EncounterObjective};
use crate::participants::EncounterParticipants;

/// The generic lifecycle phase of one encounter. Wave-specific facts
/// (wave index, remaining mobs) live on the wave policy component
/// ([`EncounterWaves`](crate::EncounterWaves)), not here.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum EncounterPhase {
    /// Not started. The trigger/adapter may start it.
    #[default]
    Inactive,
    /// Started, counting down an authored intro window before going Active.
    /// Presentation effects (lock, camera, music) already apply.
    Starting { remaining: f32 },
    /// In flight: elapsed time accrues, signals collect, objectives evaluate.
    Active,
    /// The win objective was met (or `Complete` was commanded).
    Completed,
    /// The fail objective was met (or `Fail` was commanded).
    Failed,
}

impl Eq for EncounterPhase {}

impl EncounterPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Starting { .. } => "starting",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// In flight — started and not yet terminal.
    pub fn in_flight(self) -> bool {
        matches!(self, Self::Starting { .. } | Self::Active)
    }

    /// Whether an encounter in this phase seals its authored exits.
    pub fn locks_exits(self) -> bool {
        self.in_flight()
    }
}

/// The one lifecycle authority component. Every encounter entity carries one;
/// the reducer is the only writer of `phase`.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterLifecycle {
    pub phase: EncounterPhase,
    /// Authored intro window (seconds spent in `Starting` after a `Start`
    /// command before the encounter goes `Active`). Definition datum, set at
    /// spawn; `0.0` skips straight to `Active`.
    pub intro_seconds: f32,
    /// Seconds since the encounter went `Active` (E9). Objective
    /// `Survive(secs)` evaluates against this.
    pub elapsed_active: f32,
    /// Signal keys received this activation (E9). Objective
    /// `ReceiveSignal(key)` evaluates against this. `BTreeSet` — determinism
    /// contract (iterated for snapshot/hash).
    pub signals: BTreeSet<String>,
}

impl EncounterLifecycle {
    /// A lifecycle with an authored intro window.
    pub fn with_intro(intro_seconds: f32) -> Self {
        Self {
            intro_seconds,
            ..Self::default()
        }
    }

    /// Reconstruct the phase from a `PersistedEncounterState` (save load).
    pub fn apply_persisted(&mut self, persisted: PersistedEncounterState) {
        self.phase = match persisted {
            PersistedEncounterState::Untouched => EncounterPhase::Inactive,
            PersistedEncounterState::Cleared => EncounterPhase::Completed,
            PersistedEncounterState::Failed => EncounterPhase::Failed,
        };
    }

    /// Project the live phase onto the persisted shape. In-flight collapses to
    /// `Untouched`: the save represents a resumable terminal state, not a
    /// mid-fight snapshot (that is the snapshot substrate's job, E11).
    pub fn to_persisted(&self) -> PersistedEncounterState {
        match self.phase {
            EncounterPhase::Inactive | EncounterPhase::Starting { .. } | EncounterPhase::Active => {
                PersistedEncounterState::Untouched
            }
            EncounterPhase::Completed => PersistedEncounterState::Cleared,
            EncounterPhase::Failed => PersistedEncounterState::Failed,
        }
    }

    /// THE reducer (pure, headless): apply this tick's commands, advance the
    /// intro/active clocks by `dt`, and evaluate the objective. Returns the
    /// events the caller publishes ([`EncounterEventMsg`] in ECS; trace/tests
    /// directly).
    ///
    /// Command semantics:
    /// - `Start`: only from `Inactive` (a stale terminal phase needs an
    ///   explicit `Reset` first — re-arming is a policy decision, not a
    ///   side effect of starting).
    /// - `Complete` / `Fail`: force a terminal phase from in-flight (an
    ///   external authority — scripted beat, environmental kill — decided).
    /// - `Signal`: recorded while in flight; win/fail objectives consume it.
    /// - `Reset`: return to `Inactive` from any phase, clearing this
    ///   activation's elapsed time and signals.
    pub fn reduce<'c>(
        &mut self,
        dt: f32,
        commands: impl IntoIterator<Item = &'c EncounterCommandKind>,
        participants: &EncounterParticipants,
        objective: Option<&EncounterObjective>,
    ) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        for command in commands {
            match command {
                EncounterCommandKind::Start => {
                    if matches!(self.phase, EncounterPhase::Inactive) {
                        self.elapsed_active = 0.0;
                        self.signals.clear();
                        self.phase = if self.intro_seconds > 0.0 {
                            EncounterPhase::Starting {
                                remaining: self.intro_seconds,
                            }
                        } else {
                            EncounterPhase::Active
                        };
                        events.push(EncounterEvent::Started);
                        events.push(EncounterEvent::LockChanged { locked: true });
                    }
                }
                EncounterCommandKind::Complete => {
                    if self.phase.in_flight() {
                        self.complete(&mut events);
                    }
                }
                EncounterCommandKind::Fail => {
                    if self.phase.in_flight() {
                        self.fail(&mut events);
                    }
                }
                EncounterCommandKind::Signal(key) => {
                    if self.phase.in_flight() && self.signals.insert(key.clone()) {
                        events.push(EncounterEvent::SignalReceived { key: key.clone() });
                    }
                }
                EncounterCommandKind::Reset => {
                    if !matches!(self.phase, EncounterPhase::Inactive) {
                        let was_locked = self.phase.locks_exits();
                        self.phase = EncounterPhase::Inactive;
                        self.elapsed_active = 0.0;
                        self.signals.clear();
                        events.push(EncounterEvent::Reset);
                        if was_locked {
                            events.push(EncounterEvent::LockChanged { locked: false });
                        }
                    }
                }
            }
        }

        // Intro countdown.
        if let EncounterPhase::Starting { remaining } = self.phase {
            let next = remaining - dt;
            self.phase = if next > 0.0 {
                EncounterPhase::Starting { remaining: next }
            } else {
                EncounterPhase::Active
            };
        }

        // Active: accrue time, then evaluate fail-before-win (a tick where
        // both hold is a loss — the protected actor died even if the last
        // minion fell the same instant).
        if matches!(self.phase, EncounterPhase::Active) {
            self.elapsed_active += dt;
            if let Some(objective) = objective {
                if let Some(fail) = &objective.fail {
                    if objective_met(fail, participants, self.elapsed_active, &self.signals) {
                        self.fail(&mut events);
                        return events;
                    }
                }
                if objective_met(
                    &objective.win,
                    participants,
                    self.elapsed_active,
                    &self.signals,
                ) {
                    self.complete(&mut events);
                }
            }
        }
        events
    }

    fn complete(&mut self, events: &mut Vec<EncounterEvent>) {
        self.phase = EncounterPhase::Completed;
        events.push(EncounterEvent::Completed);
        events.push(EncounterEvent::LockChanged { locked: false });
    }

    fn fail(&mut self, events: &mut Vec<EncounterEvent>) {
        self.phase = EncounterPhase::Failed;
        events.push(EncounterEvent::Failed);
        events.push(EncounterEvent::LockChanged { locked: false });
    }
}

/// What an [`EncounterCommand`] asks of a lifecycle. See
/// [`EncounterLifecycle::reduce`] for the exact semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncounterCommandKind {
    Start,
    Complete,
    Fail,
    /// Record a stable signal key (content publishes facts; generic
    /// objectives consume them — never the reverse).
    Signal(String),
    /// Return to `Inactive` for a fresh attempt (re-arm, area exit,
    /// post-death cleanup).
    Reset,
}

/// The generic command ingress (E8): any adapter or content system starts,
/// signals, completes, fails, or resets an encounter by id through this one
/// message. The reducer is the only consumer.
#[derive(Message, Clone, Debug)]
pub struct EncounterCommand {
    /// Target [`Encounter`] id.
    pub encounter: String,
    pub kind: EncounterCommandKind,
}

impl EncounterCommand {
    pub fn new(encounter: impl Into<String>, kind: EncounterCommandKind) -> Self {
        Self {
            encounter: encounter.into(),
            kind,
        }
    }

    pub fn signal(encounter: impl Into<String>, key: impl Into<String>) -> Self {
        Self::new(encounter, EncounterCommandKind::Signal(key.into()))
    }
}

/// The lifecycle authority's schedule slot. The runtime positions this set
/// (after the adapters that refresh participant liveness / emit commands,
/// before the effect adapters that react to lifecycle transitions); this crate
/// only registers [`reduce_encounter_lifecycles`] inside it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EncounterLifecycleSet;

/// Drain [`EncounterCommand`]s and reduce every encounter entity's lifecycle.
/// The one ECS registration of the pure reducer.
pub fn reduce_encounter_lifecycles(
    dt: Res<ambition_platformer_primitives::time::SimDt>,
    mut commands_in: MessageReader<EncounterCommand>,
    mut events_out: MessageWriter<EncounterEventMsg>,
    mut encounters: Query<(
        &Encounter,
        &mut EncounterLifecycle,
        Option<&EncounterParticipants>,
        Option<&EncounterObjective>,
    )>,
) {
    // Group this tick's commands by encounter id. BTreeMap: commands for the
    // same encounter apply in arrival order, and the map itself never drives
    // entity iteration (the query does), but keep ordering canonical anyway.
    let mut by_id: std::collections::BTreeMap<&str, Vec<&EncounterCommandKind>> =
        std::collections::BTreeMap::new();
    for command in commands_in.read() {
        by_id
            .entry(command.encounter.as_str())
            .or_default()
            .push(&command.kind);
    }
    let no_participants = EncounterParticipants::default();
    for (encounter, mut lifecycle, participants, objective) in &mut encounters {
        let commands = by_id
            .get(encounter.id.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let events = lifecycle.reduce(
            dt.dt,
            commands.iter().copied(),
            participants.unwrap_or(&no_participants),
            objective,
        );
        for event in events {
            events_out.write(EncounterEventMsg {
                encounter: encounter.id.clone(),
                event,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objective::Objective;
    use crate::participants::{EncounterParticipant, EncounterRole};

    fn minions(states: &[bool]) -> EncounterParticipants {
        EncounterParticipants::new(
            states
                .iter()
                .enumerate()
                .map(|(i, alive)| {
                    let mut p =
                        EncounterParticipant::spawned(format!("m{i}"), None, EncounterRole::Minion);
                    p.alive = *alive;
                    p
                })
                .collect(),
        )
    }

    /// E8 exit: the reducer drives inactive → active → completed and
    /// inactive → active → failed with no boss- or wave-specific code.
    #[test]
    fn commands_drive_inactive_to_active_to_completed_or_failed() {
        let none = EncounterParticipants::default();

        let mut lc = EncounterLifecycle::default();
        assert_eq!(lc.phase, EncounterPhase::Inactive);
        let events = lc.reduce(0.0, [&EncounterCommandKind::Start], &none, None);
        assert_eq!(lc.phase, EncounterPhase::Active, "no intro → straight in");
        assert!(events.contains(&EncounterEvent::Started));
        assert!(events.contains(&EncounterEvent::LockChanged { locked: true }));

        let events = lc.reduce(0.0, [&EncounterCommandKind::Complete], &none, None);
        assert_eq!(lc.phase, EncounterPhase::Completed);
        assert!(events.contains(&EncounterEvent::Completed));
        assert!(events.contains(&EncounterEvent::LockChanged { locked: false }));

        let mut lc = EncounterLifecycle::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &none, None);
        let events = lc.reduce(0.0, [&EncounterCommandKind::Fail], &none, None);
        assert_eq!(lc.phase, EncounterPhase::Failed);
        assert!(events.contains(&EncounterEvent::Failed));
    }

    #[test]
    fn an_authored_intro_counts_down_before_active() {
        let mut lc = EncounterLifecycle::with_intro(1.0);
        lc.reduce(
            0.0,
            [&EncounterCommandKind::Start],
            &EncounterParticipants::default(),
            None,
        );
        assert!(matches!(lc.phase, EncounterPhase::Starting { .. }));
        lc.reduce(0.6, [], &EncounterParticipants::default(), None);
        assert!(matches!(lc.phase, EncounterPhase::Starting { .. }));
        lc.reduce(0.6, [], &EncounterParticipants::default(), None);
        assert_eq!(lc.phase, EncounterPhase::Active);
    }

    /// E9 exit: all minions defeated completes via the generic objective.
    #[test]
    fn all_minions_defeated_completes_the_objective() {
        let objective =
            EncounterObjective::win(Objective::AllWithRoleDefeated(EncounterRole::Minion));
        let mut lc = EncounterLifecycle::default();
        lc.reduce(
            0.0,
            [&EncounterCommandKind::Start],
            &minions(&[true, true]),
            Some(&objective),
        );
        assert_eq!(lc.phase, EncounterPhase::Active);
        lc.reduce(0.1, [], &minions(&[false, true]), Some(&objective));
        assert_eq!(lc.phase, EncounterPhase::Active, "one still alive");
        let events = lc.reduce(0.1, [], &minions(&[false, false]), Some(&objective));
        assert_eq!(lc.phase, EncounterPhase::Completed);
        assert!(events.contains(&EncounterEvent::Completed));
    }

    /// E9 exit: a survive-timer encounter with NO actors completes when its
    /// elapsed active time passes the objective.
    #[test]
    fn survive_timer_completes_with_no_actors() {
        let objective = EncounterObjective::win(Objective::Survive(2.0));
        let none = EncounterParticipants::default();
        let mut lc = EncounterLifecycle::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &none, Some(&objective));
        lc.reduce(1.0, [], &none, Some(&objective));
        assert_eq!(lc.phase, EncounterPhase::Active, "1.0s < 2.0s");
        lc.reduce(1.5, [], &none, Some(&objective));
        assert_eq!(lc.phase, EncounterPhase::Completed, "2.5s ≥ 2.0s");
    }

    /// E9 exit: a receive-signal encounter with NO actors completes when the
    /// signal arrives through the command ingress.
    #[test]
    fn receive_signal_completes_with_no_actors() {
        let objective = EncounterObjective::win(Objective::ReceiveSignal("gate_reached".into()));
        let none = EncounterParticipants::default();
        let mut lc = EncounterLifecycle::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &none, Some(&objective));
        lc.reduce(
            0.1,
            [&EncounterCommandKind::Signal("wrong_key".into())],
            &none,
            Some(&objective),
        );
        assert_eq!(lc.phase, EncounterPhase::Active, "unrelated signal");
        let events = lc.reduce(
            0.1,
            [&EncounterCommandKind::Signal("gate_reached".into())],
            &none,
            Some(&objective),
        );
        assert_eq!(lc.phase, EncounterPhase::Completed);
        assert!(events.contains(&EncounterEvent::SignalReceived {
            key: "gate_reached".into()
        }));
    }

    /// E9 exit: a protected participant's death fails the encounter through
    /// the generic fail objective — and fail is evaluated before win.
    #[test]
    fn protected_participant_death_fails_the_encounter() {
        let objective = EncounterObjective::win_or_fail(
            Objective::AllWithRoleDefeated(EncounterRole::Minion),
            Objective::AnyWithRoleDefeated(EncounterRole::Protected),
        );
        let mut escortee = EncounterParticipant::adopted(
            "escortee",
            bevy::prelude::Entity::PLACEHOLDER,
            EncounterRole::Protected,
        );
        let minion = |alive: bool| {
            let mut p = EncounterParticipant::spawned("m0", None, EncounterRole::Minion);
            p.alive = alive;
            p
        };
        let mut lc = EncounterLifecycle::default();
        lc.reduce(
            0.0,
            [&EncounterCommandKind::Start],
            &EncounterParticipants::new(vec![escortee.clone(), minion(true)]),
            Some(&objective),
        );
        assert_eq!(lc.phase, EncounterPhase::Active);

        // The escortee dies the same tick the last minion falls: FAIL wins.
        escortee.alive = false;
        let events = lc.reduce(
            0.1,
            [],
            &EncounterParticipants::new(vec![escortee, minion(false)]),
            Some(&objective),
        );
        assert_eq!(
            lc.phase,
            EncounterPhase::Failed,
            "fail evaluates before win"
        );
        assert!(events.contains(&EncounterEvent::Failed));
    }

    #[test]
    fn reset_returns_any_phase_to_inactive_and_clears_the_activation() {
        let none = EncounterParticipants::default();
        let mut lc = EncounterLifecycle::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &none, None);
        lc.reduce(
            1.0,
            [&EncounterCommandKind::Signal("s".into())],
            &none,
            None,
        );
        assert!(lc.elapsed_active > 0.0);
        assert!(!lc.signals.is_empty());
        let events = lc.reduce(0.0, [&EncounterCommandKind::Reset], &none, None);
        assert_eq!(lc.phase, EncounterPhase::Inactive);
        assert_eq!(lc.elapsed_active, 0.0);
        assert!(lc.signals.is_empty());
        assert!(events.contains(&EncounterEvent::LockChanged { locked: false }));

        // Start is refused from a terminal phase without a Reset.
        let mut lc = EncounterLifecycle::default();
        lc.reduce(0.0, [&EncounterCommandKind::Start], &none, None);
        lc.reduce(0.0, [&EncounterCommandKind::Complete], &none, None);
        lc.reduce(0.0, [&EncounterCommandKind::Start], &none, None);
        assert_eq!(
            lc.phase,
            EncounterPhase::Completed,
            "no restart from terminal"
        );
        // Reset then Start in the SAME tick re-arms cleanly.
        lc.reduce(
            0.0,
            [&EncounterCommandKind::Reset, &EncounterCommandKind::Start],
            &none,
            None,
        );
        assert_eq!(lc.phase, EncounterPhase::Active);
    }

    #[test]
    fn persisted_round_trip_collapses_in_flight_to_untouched() {
        let mut lc = EncounterLifecycle::default();
        lc.phase = EncounterPhase::Active;
        assert_eq!(lc.to_persisted(), PersistedEncounterState::Untouched);
        lc.apply_persisted(PersistedEncounterState::Cleared);
        assert_eq!(lc.phase, EncounterPhase::Completed);
        assert_eq!(lc.to_persisted(), PersistedEncounterState::Cleared);
    }
}
