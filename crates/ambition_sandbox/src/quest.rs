//! Sandbox-side quest registry + Bevy systems.
//!
//! The engine owns the quest data shape (`QuestSpec`, `QuestState`,
//! `QuestAdvanceEvent`). This module wires that into the live game:
//! - holds the registry as a Bevy resource
//! - rehydrates quest progress from `SandboxSave` on startup
//! - listens for advance events (NPC talks, encounter clears, boss
//!   defeats, flag flips) and routes them into the registry
//! - exposes a `quest_log_lines` helper the HUD can render
//!
//! Today the sandbox carries one tutorial quest, "First Steps", as a
//! proof-of-concept. Future content adds more by appending to
//! `default_quest_specs()`.

use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::*;

/// Sandbox quest registry. Keyed by quest id matching `QuestSpec::id`.
#[derive(Resource, Default)]
pub struct QuestRegistry {
    pub quests: BTreeMap<String, ae::QuestState>,
    /// Pending advance events queued by the simulation half. Drained
    /// by `apply_quest_advance_events` each frame.
    pub pending_events: Vec<ae::QuestAdvanceEvent>,
    pub initialized: bool,
}

impl QuestRegistry {
    pub fn ensure(&mut self, spec: ae::QuestSpec) {
        let id = spec.id.clone();
        self.quests
            .entry(id)
            .or_insert_with(|| ae::QuestState::new(spec));
    }

    pub fn get(&self, id: &str) -> Option<&ae::QuestState> {
        self.quests.get(id)
    }

    pub fn start(&mut self, id: &str) -> bool {
        if let Some(state) = self.quests.get_mut(id) {
            state.start()
        } else {
            false
        }
    }

    pub fn push_event(&mut self, event: ae::QuestAdvanceEvent) {
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

/// Default quest specs the sandbox ships. The "First Steps" quest is
/// a tutorial that walks the player through talking to a hub NPC,
/// clearing the mob lab, and defeating the prototype boss — exactly
/// the systems the rest of this build pass introduces.
pub fn default_quest_specs() -> Vec<ae::QuestSpec> {
    vec![
        ae::QuestSpec::new(
            "first_steps",
            "First Steps",
            "Find your bearings as a new instance.",
            vec![
                ae::QuestStepSpec::new(
                    "Speak with someone in the hub.",
                    ae::QuestStepCondition::FlagSet("met_any_hub_npc".into()),
                ),
                ae::QuestStepSpec::new(
                    "Clear the mob lab.",
                    ae::QuestStepCondition::EncounterCleared("mob_lab".into()),
                ),
                ae::QuestStepSpec::new(
                    "Defeat the gradient sentinel.",
                    ae::QuestStepCondition::BossDefeated("gradient_sentinel".into()),
                ),
            ],
        ),
        ae::QuestSpec::new(
            "test_switch_quest",
            "Test the Memory",
            "Verify that the world remembers what you do.",
            vec![
                ae::QuestStepSpec::new(
                    "Toggle the persistence test switch.",
                    ae::QuestStepCondition::FlagSet("test_switch_toggled".into()),
                ),
            ],
        ),
    ]
}

/// Startup system: register specs and rehydrate from save.
pub fn populate_quest_registry(
    mut registry: ResMut<QuestRegistry>,
    save: Res<crate::save::SandboxSave>,
) {
    if registry.initialized {
        return;
    }
    for spec in default_quest_specs() {
        registry.ensure(spec);
    }
    let save_data = save.data();
    for (id, state) in registry.quests.iter_mut() {
        let (persisted, step) = save_data.quest(id);
        state.apply_persisted(persisted, step);
    }
    // Auto-start the tutorial so the player sees a HUD entry from
    // first frame. Idempotent — `start` is a no-op if it's already
    // running or done.
    if let Some(q) = registry.quests.get_mut("first_steps") {
        let _ = q.start();
    }
    if let Some(q) = registry.quests.get_mut("test_switch_quest") {
        let _ = q.start();
    }
    registry.initialized = true;
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
    registry.push_event(ae::QuestAdvanceEvent::RoomEntered(current));
}

/// Drain pending advance events into the registry and write quest
/// progress back to the save resource. Runs each frame.
pub fn apply_quest_advance_events(
    mut registry: ResMut<QuestRegistry>,
    mut save: ResMut<crate::save::SandboxSave>,
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

    fn first_steps_spec() -> ae::QuestSpec {
        default_quest_specs()
            .into_iter()
            .find(|s| s.id == "first_steps")
            .expect("first_steps spec")
    }

    #[test]
    fn ensure_inserts_idempotently() {
        let mut registry = QuestRegistry::default();
        let spec = first_steps_spec();
        registry.ensure(spec.clone());
        registry.ensure(spec);
        assert_eq!(registry.quests.len(), 1);
    }

    #[test]
    fn start_requires_existing_quest() {
        let mut registry = QuestRegistry::default();
        assert!(!registry.start("nonexistent"));
        registry.ensure(first_steps_spec());
        assert!(registry.start("first_steps"));
    }

    #[test]
    fn quest_log_lines_skips_inactive_unstarted_quests() {
        let mut registry = QuestRegistry::default();
        registry.ensure(first_steps_spec());
        // Default state is "unstarted", neither is_active nor is_complete.
        assert!(registry.quest_log_lines().is_empty());
        registry.start("first_steps");
        assert!(!registry.quest_log_lines().is_empty());
    }

    #[test]
    fn active_quest_summary_finds_one_active() {
        let mut registry = QuestRegistry::default();
        registry.ensure(first_steps_spec());
        assert!(registry.active_quest_summary().is_none());
        registry.start("first_steps");
        let summary = registry.active_quest_summary();
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("First Steps"));
    }

    #[test]
    fn push_event_buffers_pending() {
        let mut registry = QuestRegistry::default();
        registry.push_event(ae::QuestAdvanceEvent::FlagSet("foo".into()));
        registry.push_event(ae::QuestAdvanceEvent::FlagSet("bar".into()));
        assert_eq!(registry.pending_events.len(), 2);
    }
}
