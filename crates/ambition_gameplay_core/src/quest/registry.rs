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
#[derive(Resource, Default)]
pub struct QuestRegistry {
    pub quests: BTreeMap<String, crate::quest::QuestState>,
    /// Pending advance events queued by the simulation half. Drained
    /// by `apply_quest_advance_events` each frame.
    pub pending_events: Vec<crate::quest::QuestAdvanceEvent>,
    pub initialized: bool,
}

impl QuestRegistry {
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

/// Push a `RoomEntered` quest event whenever the active room
/// changes. Idempotent: only fires the frame the room id flips.
pub fn push_room_entered_quest_events(
    room_set: Res<crate::rooms::RoomSet>,
    mut registry: ResMut<QuestRegistry>,
    mut last_room: Local<Option<String>>,
) {
    let current = room_set.active_spec().id.clone();
    if last_room.as_deref() == Some(current.as_str()) {
        return;
    }
    *last_room = Some(current.clone());
    registry.push_event(crate::quest::QuestAdvanceEvent::RoomEntered(current));
}

/// Drain pending advance events into the registry and write quest
/// progress back to the save resource. Runs each frame.
pub fn apply_quest_advance_events(
    mut registry: ResMut<QuestRegistry>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
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
