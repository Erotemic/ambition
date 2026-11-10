//! LOADOUT-AUTHORITY: a menu equips and stows a weapon through a stamped
//! request the simulation applies on its tick (`apply_hand_requests`), so the
//! sync test, which rewinds and resimulates every frame, sees the same hand on
//! every pass: one weapon in it after the equip, none after the stow.

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_options};
use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::combat::held_items::HeldItem;
use ambition_platformer2d::items::{Item, ItemEquipRequested, ItemStowRequested, OwnedItems};
use ambition_platformer2d::platformer::host_intents::write_host_intent;

/// The ids of every weapon held by a body the player's participant drives.
fn held(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    let world = sim.world_mut();
    world
        .query::<(&HeldItem, &ambition_platformer2d::characters::control::DrivingParticipant)>()
        .iter(world)
        .filter(|(_, driver)| driver.0 == PlayerSlot(0))
        .map(|(held, _)| held.id().to_string())
        .collect()
}

#[test]
fn a_menu_equip_and_stow_replay_to_the_same_hand_under_the_sync_test() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_options().with_sync_test_rollback())
        .expect("the sandbox boots under the sync test");
    sim.step_n(base(), 10);
    let catalog = ambition_platformer2d::items::item_catalog(sim.world_mut()).clone();
    sim.world_mut().resource_mut::<OwnedItems>().grant(&catalog, Item::GunSword, 1);
    assert!(held(&mut sim).is_empty(), "precondition: an empty hand");

    write_host_intent(sim.world_mut(), ItemEquipRequested { participant: PlayerSlot(0), item: Item::GunSword });
    sim.step_n(base(), 12);
    assert_eq!(sim.rollback_health(), Ok(()), "the equip resimulated to the same state");
    assert_eq!(held(&mut sim), vec!["gun_sword".to_string()], "one gun-sword in the player's hand");

    write_host_intent(sim.world_mut(), ItemStowRequested { participant: PlayerSlot(0) });
    sim.step_n(base(), 12);
    assert_eq!(sim.rollback_health(), Ok(()), "the stow resimulated to the same state");
    assert!(held(&mut sim).is_empty(), "the stow emptied the hand");
}
