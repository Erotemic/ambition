//! Bevy-side quest registry: the generic runtime half of the quest
//! system. Holds quest states as a resource, buffers advance events
//! from the simulation, drains them each frame, and mirrors progress
//! into the save file.
//!
//! Deliberately content-free: WHICH quests exist (specs, auto-start
//! list, completion payouts) is authored by the content layer, which
//! populates this registry at startup and hangs reward systems off
//! completed quest ids. The data shapes live in [`crate::quest`]
//! (Bevy-free); this module is the live-game wiring.

use std::collections::BTreeMap;

use bevy::prelude::*;

/// Sandbox quest registry. Keyed by quest id matching `QuestSpec::id`.
#[derive(Resource, Default, Clone)]
pub struct QuestRegistry {
    pub quests: BTreeMap<String, crate::quest::QuestState>,
    /// Pending advance events queued by the simulation half. Drained
    /// by `apply_quest_advance_events` each frame.
    ///
    /// ⛤ **PRIVATE, SO THE APPEND-ONLY SPLIT IS THE COMPILER'S RULE AND NOT A
    /// CONVENTION — 2026-09-18.** Five production systems in four crates queue
    /// into this (`update_boss_encounters`, `drive_wave_encounters`,
    /// `apply_wave_encounter_effects`, `apply_flag_effects`/`apply_quest_effects`,
    /// `push_room_entered_quest_events`) and exactly one drains it
    /// (`apply_quest_advance_events`, below). MEASURED before the change: every
    /// one of those producers already reached only [`Self::push_event`] and the
    /// field was touched outside this module in ONE place, a `#[cfg(test)]`
    /// helper. ⇒ The discipline was already real; what it lacked was enforcement,
    /// which is the difference between `ClassBRemapLog` (private `Vec`, adjudicated
    /// enforced) and a `pub` field that happens to be used correctly.
    pending_events: Vec<PendingQuestEvent>,
    /// The quest steps each cause's latest event moved, by cause: what a
    /// retraction of that cause puts back ([`Self::retract_caused_by`]).
    /// Written only by the drain, and a new event of a cause replaces the
    /// record of its last one, so this holds one entry per cause.
    caused_advances: BTreeMap<String, Vec<CausedQuestAdvance>>,
    pub initialized: bool,
}

/// One queued advance event, and the cause that can retract it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingQuestEvent {
    event: crate::quest::QuestAdvanceEvent,
    cause: Option<String>,
}

/// One quest step that an event with a cause moved: where the quest stood
/// before the event, and where the event left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausedQuestAdvance {
    pub quest: String,
    pub before: (crate::save_data::PersistedQuestState, u8),
    pub after: (crate::save_data::PersistedQuestState, u8),
}

/// The ids of the rooms that were live when `push_room_entered_quest_events`
/// last looked, sorted and without repeats — ROLLBACK STATE, not a system
/// `Local`.
///
/// The producer fires a `RoomEntered` quest event on the frame a room id
/// becomes live, so it has to remember which ids were live; a `Local`
/// remembers them across a rewind and lets a resimulation skip the push.
/// Registered beside `QuestRegistry` in this crate's rollback declaration.
///
/// A set, not one id: with two live rooms (Alice and Bob apart, OW1), each
/// room is entered on its own, and one id could remember only one of them.
/// With one live room the set has one member, and it flips as the one id did.
#[derive(bevy::prelude::Component, Debug, Default, Clone, PartialEq, Eq)]
pub struct LastQuestRoom(pub Vec<String>);

impl LastQuestRoom {
    /// Checksum projection: the count, then each room id in order.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
        let mut out = Vec::new();
        put_u64(&mut out, self.0.len() as u64);
        for room in &self.0 {
            put_str(&mut out, room);
        }
        checksum_bytes(&out)
    }
}

impl QuestRegistry {
    /// Canonical projection of the progression a rewind has to reproduce.
    ///
    /// ⭐ THE POINT IS THAT THE SYNC TEST CAN SEE THIS AT ALL. Registered with
    /// `rollback_resource_clone`, this resource contributed nothing to the
    /// session checksum but its PRESENCE, so a rewind that lost a `RoomEntered`
    /// push changed no checksum and no probe — the desync was structurally
    /// invisible to the developer proof pulse that exists to catch it.
    ///
    /// The `spec` is authored and constant, so only its `id` enters the hash:
    /// what a resimulation can disagree about is which quests EXIST and where
    /// each one stands, not the text of a step.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{
            checksum_bytes, put_bool, put_str, put_u64, put_u8,
        };
        // Destructured on purpose: a field added to this resource must be
        // answered for here or this stops compiling. A field that escapes the
        // projection is a desync nothing reports.
        let Self {
            quests,
            pending_events,
            caused_advances,
            initialized,
        } = self;
        let mut bytes = Vec::new();
        put_u64(&mut bytes, quests.len() as u64);
        // `BTreeMap`, so this walk is ordered by quest id on every peer.
        for (id, state) in quests {
            put_str(&mut bytes, id);
            put_str(&mut bytes, state.spec.id.as_str());
            put_u8(&mut bytes, state.progression as u8);
            put_u8(&mut bytes, state.step);
        }
        // Order is the push order both peers simulate, and `label()` is
        // exhaustive over the variants.
        put_u64(&mut bytes, pending_events.len() as u64);
        for PendingQuestEvent { event, cause } in pending_events {
            put_str(&mut bytes, &event.label());
            put_bool(&mut bytes, cause.is_some());
            put_str(&mut bytes, cause.as_deref().unwrap_or(""));
        }
        // `BTreeMap`, so this walk is ordered by cause on every peer.
        put_u64(&mut bytes, caused_advances.len() as u64);
        for (cause, advances) in caused_advances {
            put_str(&mut bytes, cause);
            put_u64(&mut bytes, advances.len() as u64);
            for advance in advances {
                put_str(&mut bytes, &advance.quest);
                for (progression, step) in [advance.before, advance.after] {
                    put_u8(&mut bytes, progression as u8);
                    put_u8(&mut bytes, step);
                }
            }
        }
        put_bool(&mut bytes, *initialized);
        checksum_bytes(&bytes)
    }

    pub fn ensure(&mut self, spec: crate::quest::QuestSpec) {
        let id = spec.id.clone();
        self.quests
            .entry(id)
            .or_insert_with(|| crate::quest::QuestState::new(spec));
    }

    pub fn get(&self, id: &str) -> Option<&crate::quest::QuestState> {
        self.quests.get(id)
    }

    pub fn start(&mut self, id: &str) -> bool {
        if let Some(state) = self.quests.get_mut(id) {
            state.start()
        } else {
            false
        }
    }

    /// QUEUE an advance event for the one drain. One of the two ways in.
    pub fn push_event(&mut self, event: crate::quest::QuestAdvanceEvent) {
        self.pending_events.push(PendingQuestEvent { event, cause: None });
    }

    /// QUEUE an advance event that `cause` can retract later. The drain
    /// records the steps it moves under `cause`, and
    /// [`Self::retract_caused_by`] puts them back. A boss road uses its
    /// placement id: a replay that retracts the defeat retracts its quest step.
    pub fn push_event_caused_by(
        &mut self,
        event: crate::quest::QuestAdvanceEvent,
        cause: impl Into<String>,
    ) {
        self.pending_events.push(PendingQuestEvent {
            event,
            cause: Some(cause.into()),
        });
    }

    /// What is queued and not yet drained — READ ONLY, for tests and probes
    /// that need to see a producer fire without stepping the reducer.
    pub fn pending_events(&self) -> impl Iterator<Item = &crate::quest::QuestAdvanceEvent> {
        self.pending_events.iter().map(|pending| &pending.event)
    }

    /// Retract what `cause` did: drop its events that are not drained yet,
    /// and put each quest its latest drained event moved back where it stood.
    /// Returns the ids of the quests it put back.
    ///
    /// A quest that moved on after that event goes back too: its steps are
    /// ordered, so a later step was reachable only through this one. A quest
    /// that stands before where the event left it (something else already put
    /// it back) stays.
    pub fn retract_caused_by(&mut self, cause: &str) -> Vec<String> {
        self.pending_events
            .retain(|pending| pending.cause.as_deref() != Some(cause));
        let Some(advances) = self.caused_advances.remove(cause) else {
            return Vec::new();
        };
        let mut put_back = Vec::new();
        // Latest first, so one event that moved a quest twice unwinds in order.
        for advance in advances.into_iter().rev() {
            let Some(state) = self.quests.get_mut(&advance.quest) else {
                continue;
            };
            if !reached((state.progression, state.step), advance.after) {
                continue;
            }
            (state.progression, state.step) = advance.before;
            put_back.push(advance.quest);
        }
        put_back
    }

    pub fn quest_log_lines(&self) -> Vec<String> {
        self.quests
            .values()
            .filter(|q| q.is_active() || q.is_complete())
            .map(|q| q.hud_summary())
            .collect()
    }

    pub fn active_quest_summary(&self) -> Option<String> {
        self.quests
            .values()
            .find(|q| q.is_active())
            .map(|q| q.hud_summary())
    }
}

/// Whether a quest standing at `at` has reached `mark`: as far along, or
/// further. A finished quest (completed or failed) is past every step.
fn reached(
    at: (crate::save_data::PersistedQuestState, u8),
    mark: (crate::save_data::PersistedQuestState, u8),
) -> bool {
    use crate::save_data::PersistedQuestState as State;
    let rank = |(state, step): (State, u8)| match state {
        State::NotStarted => (0, 0),
        State::InProgress => (1, step),
        State::Completed | State::Failed => (2, 0),
    };
    rank(at) >= rank(mark)
}

/// Drain pending advance events into the registry and write quest
/// progress back to the save resource. Runs each frame.
pub fn apply_quest_advance_events(
    mut registry: ResMut<QuestRegistry>,
    mut save: ResMut<crate::save::AmbitionGameSave>,
) {
    let events = std::mem::take(&mut registry.pending_events);
    if events.is_empty() {
        return;
    }
    let registry = &mut *registry;
    let mut changed_ids: Vec<String> = Vec::new();
    for PendingQuestEvent { event, cause } in events {
        let mut advances = Vec::new();
        for (id, state) in registry.quests.iter_mut() {
            let before = (state.progression, state.step);
            if state.try_advance(&event) {
                changed_ids.push(id.clone());
                advances.push(CausedQuestAdvance {
                    quest: id.clone(),
                    before,
                    after: (state.progression, state.step),
                });
            }
        }
        if let Some(cause) = cause {
            registry.caused_advances.insert(cause, advances);
        }
    }
    if changed_ids.is_empty() {
        return;
    }
    for id in changed_ids {
        if let Some(state) = registry.quests.get(&id) {
            save.data_mut()
                .set_quest(&id, state.progression, state.step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, title: &str) -> crate::quest::QuestSpec {
        crate::quest::QuestSpec::new(
            id,
            title,
            "test quest",
            vec![crate::quest::QuestStepSpec::new(
                "Set the flag.",
                crate::quest::QuestStepCondition::FlagSet("test_flag".into()),
            )],
        )
    }

    #[test]
    fn ensure_inserts_idempotently() {
        let mut registry = QuestRegistry::default();
        registry.ensure(spec("q", "Q"));
        registry.ensure(spec("q", "Q"));
        assert_eq!(registry.quests.len(), 1);
    }

    #[test]
    fn start_requires_existing_quest() {
        let mut registry = QuestRegistry::default();
        assert!(!registry.start("nonexistent"));
        registry.ensure(spec("q", "Q"));
        assert!(registry.start("q"));
    }

    #[test]
    fn quest_log_lines_skips_inactive_unstarted_quests() {
        let mut registry = QuestRegistry::default();
        registry.ensure(spec("q", "Q"));
        // Default state is "unstarted", neither is_active nor is_complete.
        assert!(registry.quest_log_lines().is_empty());
        registry.start("q");
        assert!(!registry.quest_log_lines().is_empty());
    }

    #[test]
    fn active_quest_summary_finds_one_active() {
        let mut registry = QuestRegistry::default();
        registry.ensure(spec("q", "Quiet Quest"));
        assert!(registry.active_quest_summary().is_none());
        registry.start("q");
        let summary = registry.active_quest_summary();
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("Quiet Quest"));
    }

    #[test]
    fn push_event_buffers_pending() {
        let mut registry = QuestRegistry::default();
        registry.push_event(crate::quest::QuestAdvanceEvent::FlagSet("foo".into()));
        registry.push_event(crate::quest::QuestAdvanceEvent::FlagSet("bar".into()));
        assert_eq!(registry.pending_events.len(), 2);
    }

    /// A registry with one started quest: a boss step, then a flag step.
    fn two_step_quest() -> QuestRegistry {
        use crate::quest::{QuestSpec, QuestStepCondition, QuestStepSpec};
        let mut registry = QuestRegistry::default();
        registry.ensure(QuestSpec::new(
            "q",
            "Q",
            "test quest",
            vec![
                QuestStepSpec::new("Defeat it.", QuestStepCondition::BossDefeated("boss".into())),
                QuestStepSpec::new("Report.", QuestStepCondition::FlagSet("reported".into())),
            ],
        ));
        registry.start("q");
        registry
    }

    fn drain(registry: QuestRegistry) -> QuestRegistry {
        let mut app = App::new();
        app.insert_resource(registry)
            .init_resource::<crate::save::AmbitionGameSave>()
            .add_systems(Update, apply_quest_advance_events);
        app.update();
        app.world_mut().remove_resource::<QuestRegistry>().expect("still there")
    }

    fn step(registry: &QuestRegistry) -> u8 {
        registry.get("q").expect("authored").step
    }

    /// A retraction puts back the step its cause moved, and the steps that
    /// followed it; an event that is not drained yet is dropped.
    #[test]
    fn a_retraction_puts_back_the_step_its_cause_moved_and_what_followed() {
        use crate::quest::QuestAdvanceEvent;
        let defeat = || QuestAdvanceEvent::BossDefeated("boss".into());

        let mut retracted = two_step_quest();
        retracted.push_event_caused_by(defeat(), "placement");
        let mut retracted = drain(retracted);
        let after_defeat = step(&retracted);
        let put_back = retracted.retract_caused_by("placement");

        let mut moved_on = two_step_quest();
        moved_on.push_event_caused_by(defeat(), "placement");
        moved_on.push_event(QuestAdvanceEvent::FlagSet("reported".into()));
        let mut moved_on = drain(moved_on);
        let moved_on_put_back = moved_on.retract_caused_by("placement");

        let mut queued = two_step_quest();
        queued.push_event_caused_by(defeat(), "placement");
        let queued_put_back = queued.retract_caused_by("placement");
        let queued = drain(queued);

        assert_eq!(
            (
                (after_defeat, put_back, step(&retracted)),
                (moved_on_put_back, step(&moved_on), moved_on.get("q").map(|q| q.is_complete())),
                (queued_put_back, step(&queued)),
            ),
            (
                (1, vec!["q".to_string()], 0),
                (vec!["q".to_string()], 0, Some(false)),
                (Vec::new(), 0),
            ),
            "((step after the defeat, put back, step after), (moved on: put back, step, complete), \
             (queued: put back, step after the drain))"
        );
    }
}

#[cfg(test)]
mod checksum_tests {
    use super::QuestRegistry;
    use crate::quest::{QuestAdvanceEvent, QuestSpec, QuestState};

    fn registry_with(quest: &str) -> QuestRegistry {
        let mut registry = QuestRegistry::default();
        registry.quests.insert(
            quest.to_string(),
            QuestState::new(QuestSpec {
                id: quest.to_string(),
                title: "t".into(),
                summary: "s".into(),
                steps: Vec::new(),
                auto_start: false,
            }),
        );
        registry
    }

    /// ⭐ THE POSITIVE CONTROL FOR THE WHOLE CHANGE. `push_room_entered_quest_events`
    /// is guarded by a `Local` that does not rewind, so a resimulation can skip
    /// the push. That is only DETECTABLE if a missing pending event moves the
    /// checksum — before this projection existed it did not.
    #[test]
    fn a_missing_room_entered_push_moves_the_checksum() {
        let base = registry_with("q");
        let mut pushed = base.clone();
        pushed.push_event(QuestAdvanceEvent::RoomEntered("hall".into()));
        assert_ne!(
            base.checksum(),
            pushed.checksum(),
            "a dropped RoomEntered push must be visible to the session checksum"
        );
    }

    /// The other half: WHICH room was entered has to matter too, or a
    /// resimulation that pushes a different event still agrees.
    #[test]
    fn two_different_room_entries_do_not_share_a_checksum() {
        let mut a = registry_with("q");
        let mut b = a.clone();
        a.push_event(QuestAdvanceEvent::RoomEntered("hall".into()));
        b.push_event(QuestAdvanceEvent::RoomEntered("cellar".into()));
        assert_ne!(a.checksum(), b.checksum());
    }

    /// A quest that advanced a step must not hash like one that did not.
    #[test]
    fn an_advanced_step_moves_the_checksum() {
        let base = registry_with("q");
        let mut advanced = base.clone();
        advanced.quests.get_mut("q").expect("present").step = 1;
        assert_ne!(base.checksum(), advanced.checksum());
    }

    /// ⛔ AND THE ARM THAT CATCHES A CHECKSUM THAT CANNOT AGREE: equal state must
    /// hash equal, or every frame is a false mismatch and the tool is worse than
    /// blind.
    #[test]
    fn equal_registries_agree() {
        let mut a = registry_with("q");
        a.push_event(QuestAdvanceEvent::RoomEntered("hall".into()));
        let b = a.clone();
        assert_eq!(a.checksum(), b.checksum());
    }
}
