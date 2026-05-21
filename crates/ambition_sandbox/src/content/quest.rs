//! Sandbox-side quest registry + Bevy systems.
//!
//! The engine owns the quest data shape (`QuestSpec`, `QuestState`,
//! `QuestAdvanceEvent`). This module wires that into the live game:
//! - holds the registry as a Bevy resource
//! - rehydrates quest progress from `SandboxSave` on startup
//! - listens for advance events (NPC talks, encounter clears, boss
//!   defeats, flag flips) and routes them into the registry
//! - exposes a `quest_log_lines` helper the HUD can render
//! - grants completion rewards (e.g. the pirate treasure payout)
//!
//! Today the sandbox carries one tutorial quest, "First Steps", as a
//! proof-of-concept. Future content adds more by appending to
//! `default_quest_specs()`.

use std::collections::BTreeMap;

use ambition_engine as ae;
use bevy::prelude::*;

use crate::inventory::{ItemKind, PlayerInventory};

/// Save flag set once the pirate-treasure reward has been granted, so
/// the payout fires exactly once across save/reload cycles.
pub const PIRATE_TREASURE_REWARD_FLAG: &str = "pirate_treasure_reward_granted";

/// Items the pirate admiral hands over when the treasure is returned.
/// Kept as a const so the payout is data-defined (and test-pinable)
/// rather than buried in a system body.
pub const PIRATE_TREASURE_REWARD: &[(ItemKind, u32)] = &[
    (ItemKind::HealthPotion, 3),
    (ItemKind::SpareBattery, 2),
    (ItemKind::DataChip, 1),
];

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
/// clearing the goblin encounter, and defeating the prototype boss — exactly
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
                    "Clear the goblin encounter.",
                    ae::QuestStepCondition::EncounterCleared("goblin_encounter".into()),
                ),
                ae::QuestStepSpec::new(
                    "Defeat the clockwork warden.",
                    ae::QuestStepCondition::BossDefeated("clockwork_warden".into()),
                ),
            ],
        ),
        ae::QuestSpec::new(
            "test_switch_quest",
            "Test the Memory",
            "Verify that the world remembers what you do.",
            vec![ae::QuestStepSpec::new(
                "Toggle the persistence test switch.",
                ae::QuestStepCondition::FlagSet("test_switch_toggled".into()),
            )],
        ),
        // Quest lab proof: minimal RoomEntered-driven quest. Auto-
        // starts at boot, advances when the player enters the
        // quest_lab room, completes when they walk back to the
        // basement.
        ae::QuestSpec::new(
            "quest_lab_visit",
            "Visit the Quest Lab",
            "Walk into the quest lab and back to verify quest progression.",
            vec![
                ae::QuestStepSpec::new(
                    "Enter the quest lab from the basement door.",
                    ae::QuestStepCondition::RoomEntered("quest_lab".into()),
                ),
                ae::QuestStepSpec::new(
                    "Return to the basement.",
                    ae::QuestStepCondition::RoomEntered("central_hub_complex".into()),
                ),
            ],
        ),
        // Pirate cove bounty: the cove's hoard was stolen by a
        // mockingbird. Auto-starts at boot so the player can take
        // either path first — slay the bird or chat up the admiral.
        // Step ordering encodes the *minimum* sequence: the chest has
        // to actually exist (i.e. the bird must be dead) before
        // returning it to the admiral can complete the quest. Talking
        // to the admiral first is fine — the FlagSet event simply
        // doesn't match step 0 and the quest stays put. The fallback
        // path (kill the bird first, then walk in) lands the player
        // at step 1 with no extra preamble required.
        ae::QuestSpec::new(
            "pirate_treasure",
            "The Plundered Hoard",
            "A mockingbird looted the pirate cove. Bring the chest back.",
            vec![
                ae::QuestStepSpec::new(
                    "Hunt the mockingbird and reclaim the chest.",
                    ae::QuestStepCondition::BossDefeated("mockingbird".into()),
                ),
                ae::QuestStepSpec::new(
                    "Return the treasure to the pirate admiral.",
                    ae::QuestStepCondition::FlagSet("npc_pirate_admiral_talked".into()),
                ),
            ],
        ),
        // Intro-v1 cartography route. Alice's sealed note → Bob's
        // field survey → first system boss + P5 Route Memory. Steps
        // are flag-set conditions wired to the PickupSpawn entities
        // placed in alice_relay, bob_relay, and first_system_boss.
        // Auto-starts at boot so the player sees the quest from the
        // moment they leave the lab.
        ae::QuestSpec::new(
            "intro_cartography_route",
            "Carry the Quiet Route",
            "Alice trusts you with a sealed note. Bob owes her a survey.",
            vec![
                ae::QuestStepSpec::new(
                    "Find Alice and accept her sealed route note.",
                    ae::QuestStepCondition::FlagSet("alice_route_note_carried".into()),
                ),
                ae::QuestStepSpec::new(
                    "Reach Bob and pick up his field survey.",
                    ae::QuestStepCondition::FlagSet("bob_field_survey_received".into()),
                ),
                ae::QuestStepSpec::new(
                    "Clear the first system encounter and bank route memory.",
                    ae::QuestStepCondition::FlagSet("intro_p5_route_memory_received".into()),
                ),
            ],
        ),
        // Intro-v1 P1 Stabilizer beat. Oiler is the social anchor in
        // Drain Market; talking to him plus picking up the stabilizer
        // entity (drain_alley spec) closes the beat.
        ae::QuestSpec::new(
            "intro_p1_stabilizer",
            "Stabilizer Drop",
            "Oiler can stabilize the under-town descent.",
            vec![
                ae::QuestStepSpec::new(
                    "Speak with Oiler in Drain Market.",
                    ae::QuestStepCondition::FlagSet("npc_oiler_intro_talked".into()),
                ),
                ae::QuestStepSpec::new(
                    "Pick up the stabilizer kit.",
                    ae::QuestStepCondition::FlagSet("p1_stabilizer_received".into()),
                ),
            ],
        ),
    ]
}

/// Startup system: register specs and rehydrate from save.
pub fn populate_quest_registry(
    mut registry: ResMut<QuestRegistry>,
    save: Res<crate::persistence::save::SandboxSave>,
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
    if let Some(q) = registry.quests.get_mut("quest_lab_visit") {
        let _ = q.start();
    }
    if let Some(q) = registry.quests.get_mut("pirate_treasure") {
        let _ = q.start();
    }
    if let Some(q) = registry.quests.get_mut("intro_cartography_route") {
        let _ = q.start();
    }
    if let Some(q) = registry.quests.get_mut("intro_p1_stabilizer") {
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

/// Apply the items in `PIRATE_TREASURE_REWARD` to the inventory and
/// return a banner string for the HUD. Pure helper so tests can drive
/// the payout without spinning up Bevy.
pub fn grant_pirate_treasure_reward(inventory: &mut PlayerInventory) -> String {
    for (kind, count) in PIRATE_TREASURE_REWARD {
        inventory.add(*kind, *count);
    }
    "TREASURE RETURNED — Admiral pays out the hoard".to_string()
}

/// Detect newly-completed quests with payouts and grant their rewards
/// once. Today only the pirate-treasure quest has a payout; new
/// quests that need rewards on completion can extend the match.
pub fn grant_quest_completion_rewards(
    registry: Res<QuestRegistry>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut inventory: ResMut<PlayerInventory>,
    mut banner_state: ResMut<crate::features::GameplayBanner>,
) {
    let Some(state) = registry.quests.get("pirate_treasure") else {
        return;
    };
    if !state.is_complete() {
        return;
    }
    if save.data().flag(PIRATE_TREASURE_REWARD_FLAG) {
        return;
    }
    let banner = grant_pirate_treasure_reward(&mut inventory);
    save.data_mut().set_flag(PIRATE_TREASURE_REWARD_FLAG, true);
    banner_state.show(banner, 3.0);
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

    fn pirate_treasure_spec() -> ae::QuestSpec {
        default_quest_specs()
            .into_iter()
            .find(|s| s.id == "pirate_treasure")
            .expect("pirate_treasure spec")
    }

    fn pirate_treasure_state() -> ae::QuestState {
        let mut state = ae::QuestState::new(pirate_treasure_spec());
        state.start();
        state
    }

    #[test]
    fn pirate_treasure_completes_when_bird_defeated_then_admiral_talked() {
        let mut state = pirate_treasure_state();
        assert!(state.is_active());
        assert!(state.try_advance(&ae::QuestAdvanceEvent::BossDefeated("mockingbird".into())));
        assert!(state.is_active());
        assert!(state.try_advance(&ae::QuestAdvanceEvent::FlagSet(
            "npc_pirate_admiral_talked".into()
        )));
        assert!(state.is_complete());
    }

    /// Fallback path: the player wanders into the mockingbird arena
    /// and downs the bird before ever speaking to the admiral. The
    /// quest must still progress from step 0 to step 1, then complete
    /// once the player walks back and talks to the admiral. The
    /// pre-kill admiral flag (if any) is irrelevant.
    #[test]
    fn pirate_treasure_handles_admiral_talk_before_kill_as_a_no_op() {
        let mut state = pirate_treasure_state();
        // Talk to admiral first — wrong condition for step 0 (which
        // wants BossDefeated). Quest must stay put.
        assert!(!state.try_advance(&ae::QuestAdvanceEvent::FlagSet(
            "npc_pirate_admiral_talked".into()
        )));
        assert_eq!(state.step, 0);
        assert!(state.is_active());
        // Kill the bird → step advances.
        assert!(state.try_advance(&ae::QuestAdvanceEvent::BossDefeated("mockingbird".into())));
        assert_eq!(state.step, 1);
        // Walk back and talk again → completes.
        assert!(state.try_advance(&ae::QuestAdvanceEvent::FlagSet(
            "npc_pirate_admiral_talked".into()
        )));
        assert!(state.is_complete());
    }

    #[test]
    fn grant_pirate_treasure_reward_adds_each_item_listed_in_payout() {
        let mut inventory = PlayerInventory::default();
        let banner = grant_pirate_treasure_reward(&mut inventory);
        for (kind, count) in PIRATE_TREASURE_REWARD {
            assert_eq!(inventory.count(*kind), *count);
        }
        assert!(banner.contains("TREASURE"));
    }
}
