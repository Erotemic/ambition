//! Ambition's authored quests + their completion payouts.
//!
//! The generic quest runtime (registry resource, advance-event
//! draining, save mirroring) lives in [`ambition_gameplay_core::quest::registry`]; the
//! Bevy-free data shapes in [`ambition_gameplay_core::quest`]. This module owns what is
//! specifically Ambition's: WHICH quests ship, which auto-start, and
//! named payouts like the pirate treasure.

use bevy::prelude::*;

use ambition_gameplay_core::items::{Item, OwnedItems};

/// Facade: the generic registry half moved to [`ambition_gameplay_core::quest::registry`].
/// Inbound `crate::quest::QuestRegistry` paths keep working.
pub use ambition_gameplay_core::quest::registry::{
    apply_quest_advance_events, push_room_entered_quest_events, QuestRegistry,
};

/// Save flag set once the pirate-treasure reward has been granted, so
/// the payout fires exactly once across save/reload cycles.
pub const PIRATE_TREASURE_REWARD_FLAG: &str = "pirate_treasure_reward_granted";

/// Items the pirate admiral hands over when the treasure is returned.
/// Kept as a const so the payout is data-defined (and test-pinable)
/// rather than buried in a system body.
pub const PIRATE_TREASURE_REWARD: &[(Item, u32)] = &[
    (Item::HealthCell, 3),
    (Item::SpareBattery, 2),
    (Item::DataChip, 1),
];

/// Quest ids that auto-start at boot so the player sees HUD entries
/// from the first frame. Starting is idempotent.
const AUTO_START_QUESTS: &[&str] = &[
    "first_steps",
    "test_switch_quest",
    "quest_lab_visit",
    "pirate_treasure",
    "intro_cartography_route",
    "intro_p1_stabilizer",
    "intro_first_system_boss",
];

/// Default quest specs the sandbox ships. The "First Steps" quest is
/// a tutorial that walks the player through talking to a hub NPC,
/// clearing the goblin encounter, and defeating the prototype boss — exactly
/// the systems the rest of this build pass introduces.
pub fn default_quest_specs() -> Vec<ambition_gameplay_core::quest::QuestSpec> {
    vec![
        ambition_gameplay_core::quest::QuestSpec::new(
            "first_steps",
            "First Steps",
            "Find your bearings as a new instance.",
            vec![
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Speak with someone in the hub.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet("met_any_hub_npc".into()),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Clear the goblin encounter.",
                    ambition_gameplay_core::quest::QuestStepCondition::EncounterCleared(
                        "goblin_encounter".into(),
                    ),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Defeat the clockwork warden.",
                    ambition_gameplay_core::quest::QuestStepCondition::BossDefeated(
                        "clockwork_warden".into(),
                    ),
                ),
            ],
        ),
        ambition_gameplay_core::quest::QuestSpec::new(
            "test_switch_quest",
            "Test the Memory",
            "Verify that the world remembers what you do.",
            vec![ambition_gameplay_core::quest::QuestStepSpec::new(
                "Toggle the persistence test switch.",
                ambition_gameplay_core::quest::QuestStepCondition::FlagSet("test_switch_toggled".into()),
            )],
        ),
        // Quest lab proof: minimal RoomEntered-driven quest. Auto-
        // starts at boot, advances when the player enters the
        // quest_lab room, completes when they walk back to the
        // basement.
        ambition_gameplay_core::quest::QuestSpec::new(
            "quest_lab_visit",
            "Visit the Quest Lab",
            "Walk into the quest lab and back to verify quest progression.",
            vec![
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Enter the quest lab from the basement door.",
                    ambition_gameplay_core::quest::QuestStepCondition::RoomEntered("quest_lab".into()),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Return to the basement.",
                    ambition_gameplay_core::quest::QuestStepCondition::RoomEntered(
                        "central_hub_complex".into(),
                    ),
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
        ambition_gameplay_core::quest::QuestSpec::new(
            "pirate_treasure",
            "The Plundered Hoard",
            "A mockingbird looted the pirate cove. Bring the chest back.",
            vec![
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Hunt the mockingbird and reclaim the chest.",
                    ambition_gameplay_core::quest::QuestStepCondition::BossDefeated("mockingbird".into()),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Return the treasure to the pirate admiral.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet(
                        "npc_pirate_admiral_talked".into(),
                    ),
                ),
            ],
        ),
        // Intro-v1 cartography route. Alice's sealed note → Bob's
        // field survey → first system boss + P5 Route Memory. Steps
        // are flag-set conditions wired to the PickupSpawn entities
        // placed in alice_relay, bob_relay, and first_system_boss.
        // Auto-starts at boot so the player sees the quest from the
        // moment they leave the lab.
        ambition_gameplay_core::quest::QuestSpec::new(
            "intro_cartography_route",
            "Carry the Quiet Route",
            "Alice trusts you with a sealed note. Bob owes her a survey.",
            vec![
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Find Alice and accept her sealed route note.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet(
                        "alice_route_note_carried".into(),
                    ),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Reach Bob and pick up his field survey.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet(
                        "bob_field_survey_received".into(),
                    ),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Clear the first system encounter and bank route memory.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet(
                        "intro_p5_route_memory_received".into(),
                    ),
                ),
            ],
        ),
        // Intro-v1 P1 Stabilizer beat. Oiler is the social anchor in
        // Drain Market; talking to him plus picking up the stabilizer
        // entity (drain_alley spec) closes the beat.
        ambition_gameplay_core::quest::QuestSpec::new(
            "intro_p1_stabilizer",
            "Stabilizer Drop",
            "Oiler can stabilize the under-town descent.",
            vec![
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Speak with Oiler in Drain Market.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet(
                        "npc_oiler_intro_talked".into(),
                    ),
                ),
                ambition_gameplay_core::quest::QuestStepSpec::new(
                    "Pick up the stabilizer kit.",
                    ambition_gameplay_core::quest::QuestStepCondition::FlagSet(
                        "p1_stabilizer_received".into(),
                    ),
                ),
            ],
        ),
        // Intro-v1 first system boss clear, tracked as a separate
        // single-step quest gated by BossDefeated("clockwork_warden")
        // (the boss profile the first_system_boss room reuses).
        // Mirrors the existing pirate_treasure / first_steps boss
        // hooks so the cartography quest stays flag-driven while the
        // boss-kill itself produces a separate durable record.
        ambition_gameplay_core::quest::QuestSpec::new(
            "intro_first_system_boss",
            "Capstone: First System",
            "Reach the system boss at the end of the gate stack and clear it.",
            vec![ambition_gameplay_core::quest::QuestStepSpec::new(
                "Defeat the system boss (clockwork_warden brain).",
                ambition_gameplay_core::quest::QuestStepCondition::BossDefeated(
                    "clockwork_warden".into(),
                ),
            )],
        ),
    ]
}

/// Startup system: register Ambition's authored specs and rehydrate
/// from save. Content-side because it names the shipped quests; the
/// registry it fills is generic.
pub fn populate_quest_registry(
    mut registry: ResMut<QuestRegistry>,
    save: Res<ambition_gameplay_core::persistence::save::SandboxSave>,
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
    for id in AUTO_START_QUESTS {
        if let Some(q) = registry.quests.get_mut(*id) {
            let _ = q.start();
        }
    }
    registry.initialized = true;
}

/// Apply the items in `PIRATE_TREASURE_REWARD` to the inventory and
/// return a banner string for the HUD. Pure helper so tests can drive
/// the payout without spinning up Bevy.
pub fn grant_pirate_treasure_reward(inventory: &mut OwnedItems) -> String {
    for (item, count) in PIRATE_TREASURE_REWARD {
        inventory.grant(*item, *count);
    }
    "TREASURE RETURNED — Admiral pays out the hoard".to_string()
}

/// Detect newly-completed quests with payouts and grant their rewards
/// once. Today only the pirate-treasure quest has a payout; new
/// quests that need rewards on completion can extend the match.
pub fn grant_quest_completion_rewards(
    registry: Res<QuestRegistry>,
    mut save: ResMut<ambition_gameplay_core::persistence::save::SandboxSave>,
    mut inventory: ResMut<OwnedItems>,
    mut banner_state: ResMut<ambition_gameplay_core::features::GameplayBanner>,
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

    fn pirate_treasure_spec() -> ambition_gameplay_core::quest::QuestSpec {
        default_quest_specs()
            .into_iter()
            .find(|s| s.id == "pirate_treasure")
            .expect("pirate_treasure spec")
    }

    fn pirate_treasure_state() -> ambition_gameplay_core::quest::QuestState {
        let mut state = ambition_gameplay_core::quest::QuestState::new(pirate_treasure_spec());
        state.start();
        state
    }

    #[test]
    fn pirate_treasure_completes_when_bird_defeated_then_admiral_talked() {
        let mut state = pirate_treasure_state();
        assert!(state.is_active());
        assert!(
            state.try_advance(&ambition_gameplay_core::quest::QuestAdvanceEvent::BossDefeated(
                "mockingbird".into()
            ))
        );
        assert!(state.is_active());
        assert!(
            state.try_advance(&ambition_gameplay_core::quest::QuestAdvanceEvent::FlagSet(
                "npc_pirate_admiral_talked".into()
            ))
        );
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
        assert!(
            !state.try_advance(&ambition_gameplay_core::quest::QuestAdvanceEvent::FlagSet(
                "npc_pirate_admiral_talked".into()
            ))
        );
        assert_eq!(state.step, 0);
        assert!(state.is_active());
        // Kill the bird → step advances.
        assert!(
            state.try_advance(&ambition_gameplay_core::quest::QuestAdvanceEvent::BossDefeated(
                "mockingbird".into()
            ))
        );
        assert_eq!(state.step, 1);
        // Walk back and talk again → completes.
        assert!(
            state.try_advance(&ambition_gameplay_core::quest::QuestAdvanceEvent::FlagSet(
                "npc_pirate_admiral_talked".into()
            ))
        );
        assert!(state.is_complete());
    }

    #[test]
    fn grant_pirate_treasure_reward_adds_each_item_listed_in_payout() {
        let mut inventory = OwnedItems::default();
        let banner = grant_pirate_treasure_reward(&mut inventory);
        for (item, count) in PIRATE_TREASURE_REWARD {
            assert_eq!(inventory.count(*item), *count);
        }
        assert!(banner.contains("TREASURE"));
    }

    /// Every auto-start id must correspond to a shipped spec — a typo
    /// here would silently never start the quest.
    #[test]
    fn auto_start_ids_all_exist_in_default_specs() {
        let specs = default_quest_specs();
        for id in AUTO_START_QUESTS {
            assert!(
                specs.iter().any(|s| s.id == *id),
                "auto-start quest id {id:?} has no spec"
            );
        }
    }
}
