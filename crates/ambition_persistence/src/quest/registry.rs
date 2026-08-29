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
    pub pending_events: Vec<crate::quest::QuestAdvanceEvent>,
    pub initialized: bool,
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
        for event in pending_events {
            put_str(&mut bytes, &event.label());
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

    pub fn push_event(&mut self, event: crate::quest::QuestAdvanceEvent) {
        self.pending_events.push(event);
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
    let mut changed_ids: Vec<String> = Vec::new();
    for event in events {
        for (id, state) in registry.quests.iter_mut() {
            if state.try_advance(&event) {
                changed_ids.push(id.clone());
            }
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
        pushed
            .pending_events
            .push(QuestAdvanceEvent::RoomEntered("hall".into()));
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
        a.pending_events
            .push(QuestAdvanceEvent::RoomEntered("hall".into()));
        b.pending_events
            .push(QuestAdvanceEvent::RoomEntered("cellar".into()));
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
        a.pending_events
            .push(QuestAdvanceEvent::RoomEntered("hall".into()));
        let b = a.clone();
        assert_eq!(a.checksum(), b.checksum());
    }
}
