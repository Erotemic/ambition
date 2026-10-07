//! Ambition's authored quests + their completion payouts.
//!
//! The generic quest runtime (registry resource, advance-event
//! draining, save mirroring) lives in [`ambition_persistence::quest::registry`]; the
//! Bevy-free data shapes live in [`ambition_persistence::quest`]. This module owns what is
//! specifically Ambition's: WHICH quests ship, which auto-start, and
//! named payouts like the pirate treasure.

use bevy::prelude::*;

use ambition_combat::GameplayBanner;
use ambition_items::{Item, OwnedItems};

pub use ambition_platformer2d_actor_monolith::quest::push_room_entered_quest_events;
/// Inbound `crate::quest::QuestRegistry` paths keep working.
pub use ambition_persistence::quest::registry::{apply_quest_advance_events, QuestRegistry};

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

/// The quests `pack` ships: the `quest_book` it lowered from
/// `assets/data/quests.ron`.
pub fn quest_specs_of(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Vec<ambition_persistence::quest::QuestSpec> {
    ambition_persistence::quest::content_schema::lowered_quest_book(pack)
        .expect("Ambition's pack declares data/quests.ron as its quest_book")
        .clone()
}

/// The quests of Ambition's SHIPPED pack: for a validator or a test whose
/// subject is the shipped product, not a composition.
pub fn shipped_quest_specs() -> Vec<ambition_persistence::quest::QuestSpec> {
    quest_specs_of(crate::pack::shipped())
}

/// Startup system: register this App's authored specs and rehydrate
/// from save. Content-side because it names the quests; the registry it fills
/// is generic.
///
/// The quest book is read from the App's [`crate::pack::SelectedContentPack`].
/// A world without one is a composition that never installed the content
/// plugin: the system panics, and does not answer from the boot pack. The quest
/// book does not reload; it is read once, at startup.
pub fn populate_quest_registry(
    mut registry: ResMut<QuestRegistry>,
    selected: Res<crate::pack::SelectedContentPack>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
) {
    if registry.initialized {
        return;
    }
    for spec in quest_specs_of(selected.get()) {
        registry.ensure(spec);
    }
    let save_data = save.data();
    for (id, state) in registry.quests.iter_mut() {
        let (persisted, step) = save_data.quest(id);
        state.apply_persisted(persisted, step);
    }
    // ⭐ THE QUESTS SAY SO THEMSELVES. This was a separate `AUTO_START_QUESTS`
    // list of ids looked up with `get_mut`, so an id that stopped matching a
    // spec started nothing and reported nothing. Reading the flag off the spec
    // means a quest cannot be missing from a list it is not named in.
    for state in registry.quests.values_mut() {
        if state.spec.auto_start {
            let _ = state.start();
        }
    }
    registry.initialized = true;
}

/// Apply the items in `PIRATE_TREASURE_REWARD` to the inventory and
/// return a banner string for the HUD. Pure helper so tests can drive
/// the payout without spinning up Bevy.
pub fn grant_pirate_treasure_reward(
    inventory: &mut OwnedItems,
    items: &ambition_items::ItemCatalog,
) -> String {
    for (item, count) in PIRATE_TREASURE_REWARD {
        inventory.grant(items, *item, *count);
    }
    "TREASURE RETURNED — Admiral pays out the hoard".to_string()
}

/// The payout follows the quest: granted once when the quest completes, and
/// taken back if a replay retracts the boss defeat it depended on and the
/// quest is no longer complete (BOSS-REPLAY-RETRACTION). Today only the
/// pirate-treasure quest has a payout; new quests that need rewards on
/// completion can extend the match.
pub fn grant_quest_completion_rewards(
    registry: Res<QuestRegistry>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    mut inventory: ResMut<OwnedItems>,
    items: ambition_items::ItemCatalogRead,
    mut banner_state: ResMut<GameplayBanner>,
) {
    let Some(state) = registry.quests.get("pirate_treasure") else {
        return;
    };
    let paid = save.data().flag(PIRATE_TREASURE_REWARD_FLAG);
    if !state.is_complete() {
        if paid {
            for (item, count) in PIRATE_TREASURE_REWARD {
                inventory.take(*item, *count);
            }
            save.data_mut().set_flag(PIRATE_TREASURE_REWARD_FLAG, false);
        }
        return;
    }
    if paid {
        return;
    }
    let banner = grant_pirate_treasure_reward(&mut inventory, items.get());
    save.data_mut().set_flag(PIRATE_TREASURE_REWARD_FLAG, true);
    banner_state.show(banner, 3.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pirate_treasure_spec() -> ambition_persistence::quest::QuestSpec {
        shipped_quest_specs()
            .into_iter()
            .find(|s| s.id == "pirate_treasure")
            .expect("pirate_treasure spec")
    }

    fn pirate_treasure_state() -> ambition_persistence::quest::QuestState {
        let mut state = ambition_persistence::quest::QuestState::new(pirate_treasure_spec());
        state.start();
        state
    }

    #[test]
    fn pirate_treasure_completes_when_bird_defeated_then_admiral_talked() {
        let mut state = pirate_treasure_state();
        assert!(state.is_active());
        assert!(state.try_advance(
            &ambition_persistence::quest::QuestAdvanceEvent::BossDefeated("mockingbird".into())
        ));
        assert!(state.is_active());
        assert!(
            state.try_advance(&ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
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
            !state.try_advance(&ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
                "npc_pirate_admiral_talked".into()
            ))
        );
        assert_eq!(state.step, 0);
        assert!(state.is_active());
        // Kill the bird → step advances.
        assert!(state.try_advance(
            &ambition_persistence::quest::QuestAdvanceEvent::BossDefeated("mockingbird".into())
        ));
        assert_eq!(state.step, 1);
        // Walk back and talk again → completes.
        assert!(
            state.try_advance(&ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
                "npc_pirate_admiral_talked".into()
            ))
        );
        assert!(state.is_complete());
    }

    #[test]
    fn grant_pirate_treasure_reward_adds_each_item_listed_in_payout() {
        let mut inventory = OwnedItems::default();
        let banner = grant_pirate_treasure_reward(&mut inventory, ambition_items::builtin_item_catalog());
        for (item, count) in PIRATE_TREASURE_REWARD {
            assert_eq!(inventory.count(*item), *count);
        }
        assert!(banner.contains("TREASURE"));
    }

    /// The quests are content: a quest added to `data/quests.ron` reaches the
    /// lowered book through Ambition's own pack compile, and a book the schema
    /// refuses refuses the whole pack. The control is the unedited compile,
    /// whose book is the one the game registers.
    #[test]
    fn a_quest_authored_in_the_pack_is_the_quest_the_game_gets() {
        use ambition_persistence::quest::content_schema::lowered_quest_book;
        let shipped = crate::pack::compile_pack().expect("the shipped pack compiles");
        assert_eq!(lowered_quest_book(&shipped).cloned(), Some(shipped_quest_specs()));

        let added = r#"(id: "pack_probe", title: "Probe", summary: "S",
            steps: [(description: "D", condition: FlagSet("probe_flag"))])"#;
        let edited = crate::pack::compile_pack_with(|path, text| {
            if path != "data/quests.ron" {
                return text;
            }
            let end = text.rfind(']').expect("the book is a list");
            format!("{}, {added}\n]", text[..end].trim_end().trim_end_matches(','))
        })
        .expect("a book with one more quest compiles");
        let ids: Vec<String> = lowered_quest_book(&edited)
            .expect("the edited pack lowers a book")
            .iter()
            .map(|quest| quest.id.clone())
            .collect();
        assert_eq!(ids.last().map(String::as_str), Some("pack_probe"), "{ids:?}");

        let doubled = crate::pack::compile_pack_with(|path, text| {
            if path != "data/quests.ron" {
                return text;
            }
            let first = r#"(id: "first_steps", title: "Again", summary: "S",
                steps: [(description: "D", condition: FlagSet("x"))])"#;
            let end = text.rfind(']').expect("the book is a list");
            format!("{}, {first}\n]", text[..end].trim_end().trim_end_matches(','))
        });
        assert!(
            format!("{:?}", doubled.expect_err("a doubled id refuses the pack")).contains("two quests have the id"),
        );
    }

    // ⭐⭐ `auto_start_ids_all_exist_in_default_specs` WAS HERE AND IS DELETED
    // ON PURPOSE — it is no longer expressible. It checked that every id in a
    // separate `AUTO_START_QUESTS` list matched a shipped spec, because a typo
    // or a rename would silently never start the quest. The flag now lives on
    // `QuestSpec` itself, so there is no second list to disagree with, and the
    // property the test defended is a consequence of the type rather than a
    // thing to assert. ⇒ the test did not become redundant, its SUBJECT did.
    #[test]
    fn the_quests_that_should_greet_the_player_are_marked_auto_start() {
        // ⛔ NOT a restatement of the deleted test. That one guarded a LIST
        // against the specs; this guards the shipped CONTENT DECISION — that
        // these seven are the ones with a HUD entry from the first frame — so a
        // quest silently losing its `.auto_start()` is still caught.
        let specs = shipped_quest_specs();
        let mut marked: Vec<&str> = specs
            .iter()
            .filter(|s| s.auto_start)
            .map(|s| s.id.as_str())
            .collect();
        marked.sort_unstable();
        let mut expected: Vec<&str> = vec![
            "first_steps",
            "test_switch_quest",
            "quest_lab_visit",
            "pirate_treasure",
            "intro_cartography_route",
            "intro_p1_stabilizer",
            "intro_first_system_boss",
        ];
        expected.sort_unstable();
        assert_eq!(marked, expected);
    }
}

