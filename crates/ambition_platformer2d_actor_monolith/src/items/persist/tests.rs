use super::*;
use ambition_items::Item;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

fn app_with(save: AmbitionGameSave, owned: OwnedItems, wallet: i32) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(save);
    app.insert_resource(owned);
    app.init_resource::<crate::session::durable_horizon::SaveRestored>();
    app.init_resource::<crate::items::pickup::minted_horizon::MintedItemBaseline>();
    app.init_resource::<crate::items::pickup::minted_horizon::OwnedItemsBaseline>();
    app.add_systems(
        Update,
        (
            restore_inventory_from_save,
            |mut restored: ResMut<crate::session::durable_horizon::SaveRestored>| {
                restored.0 = true;
            },
            persist_inventory_to_save,
        )
            .chain(),
    );
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, BodyWallet { balance: wallet }))
        .id();
    (app, player)
}

#[test]
fn a_loaded_save_restores_items_and_wallet_over_the_starter() {
    let mut save = AmbitionGameSave::default();
    // HealthCell is a stacking consumable; Bomb is a unique weapon (cap 1).
    save.data_mut().set_inventory(
        vec![
            ambition_persistence::save_data::PersistedItem::new(ambition_items::builtin_item_catalog().dialog_id(Item::HealthCell), 4),
            ambition_persistence::save_data::PersistedItem::new(ambition_items::builtin_item_catalog().dialog_id(Item::Bomb), 1),
        ],
        137,
    );
    // Live state is the starter (Fireball etc.), wallet 0.
    let (mut app, player) = app_with(save, OwnedItems::starter(ambition_items::builtin_item_catalog()), 0);
    app.update();
    let owned = app.world().resource::<OwnedItems>();
    assert_eq!(
        owned.count(Item::HealthCell),
        4,
        "restored the saved consumable count"
    );
    assert_eq!(owned.count(Item::Bomb), 1, "restored the unique weapon");
    assert_eq!(
        owned.count(Item::Fireball),
        0,
        "the saved set REPLACES the starter"
    );
    assert_eq!(
        app.world().get::<BodyWallet>(player).unwrap().balance,
        137,
        "restored the saved wallet"
    );
}

#[test]
fn a_fresh_save_keeps_the_starter_and_then_persists_it() {
    // inventory_saved == false → fresh; keep the live starter + wallet.
    let (mut app, _player) = app_with(AmbitionGameSave::default(), OwnedItems::starter(ambition_items::builtin_item_catalog()), 25);
    app.update();
    let owned = app.world().resource::<OwnedItems>();
    assert!(
        owned.count(Item::Fireball) > 0,
        "fresh save keeps the starter"
    );
    // …and persist wrote the starter + wallet back into the save.
    let data = app.world().resource::<AmbitionGameSave>().data().clone();
    assert!(
        data.inventory_saved(),
        "the fresh inventory is now marked saved"
    );
    assert_eq!(data.wallet(), 25, "wallet persisted");
    assert!(
        data.items()
            .iter()
            .any(|i| i.id == ambition_items::builtin_item_catalog().dialog_id(Item::Fireball)),
        "the starter items were written to the save"
    );
}

#[test]
fn a_fresh_process_adopts_the_post_load_bag_as_its_checkpoint_baseline() {
    let mut save = AmbitionGameSave::default();
    save.data_mut().set_inventory(
        vec![ambition_persistence::save_data::PersistedItem::new(
            ambition_items::builtin_item_catalog().dialog_id(Item::HealthCell),
            4,
        )],
        0,
    );

    // Deliberately start from a DIFFERENT live bag.
    let (mut app, _player) = app_with(save, OwnedItems::starter(ambition_items::builtin_item_catalog()), 0);
    app.update();

    let baseline = app
        .world()
        .resource::<crate::items::pickup::minted_horizon::OwnedItemsBaseline>();
    assert_eq!(baseline.remembered().count(Item::HealthCell), 4);
    assert_eq!(
        baseline.remembered().count(Item::Fireball),
        0,
        "the checkpoint baseline must be the bag AFTER load, not the starter bag that existed before it",
    );
}

#[test]
fn round_trips_the_owned_counts_by_id() {
    // to_persisted / apply_persisted survive a round-trip (the storage half).
    // Consumables stack; unique items (Bomb) cap at 1 via grant.
    let mut owned = OwnedItems::default();
    owned.grant(ambition_items::builtin_item_catalog(), Item::Bomb, 1);
    owned.grant(ambition_items::builtin_item_catalog(), Item::HealthCell, 5);
    owned.grant(ambition_items::builtin_item_catalog(), Item::ManaCell, 2);
    let persisted = owned.to_persisted(ambition_items::builtin_item_catalog());
    let mut restored = OwnedItems::starter(ambition_items::builtin_item_catalog());
    restored.apply_persisted(ambition_items::builtin_item_catalog(), &persisted);
    assert_eq!(restored.count(Item::Bomb), 1);
    assert_eq!(restored.count(Item::HealthCell), 5);
    assert_eq!(restored.count(Item::ManaCell), 2);
    assert_eq!(
        restored.count(Item::Fireball),
        0,
        "apply replaces, not merges"
    );
}

/// ⛔⛤ **RESET NEW GAME WIPED THE SAVE AND THE NEXT PERSISTENCE PASS PUT THE OLD
/// RUN BACK — MEASURED BY A 2026-09-13 REVIEW, AND THIS ARM REPRODUCES IT.**
///
/// The reset writes `AmbitionGameSaveData::default()` and leaves `SaveRestored`
/// TRUE. If the old run's bag were still live, `persist_inventory_to_save` would
/// see the fresh save differ from it and write the OLD inventory and wallet back.
///
/// A New Game is a checkpoint restore to the fresh baseline, so this drives the
/// item domain's two commit reducers with the inputs a New Game pins: the
/// starter bag, no mints, and the fresh-run marker.
///
/// ⭐ **IT TICKS THE MIRROR, WHICH IS THE WHOLE POINT.** The damage is done by
/// the next persistence pass, so the test lets one happen.
#[test]
fn a_new_game_does_not_write_the_old_runs_inventory_back_into_the_fresh_save() {
    use crate::items::pickup::minted_horizon::{
        restore_owned_items_to_checkpoint, start_the_item_domain_fresh,
        ItemCheckpointRestoreInputs, OwnedItemsBaseline,
    };
    use bevy::ecs::system::RunSystemOnce as _;
    let mut save = AmbitionGameSave::default();
    save.data_mut().set_inventory(
        vec![ambition_persistence::save_data::PersistedItem::new(
            ambition_items::builtin_item_catalog().dialog_id(Item::Bomb),
            1,
        )],
        137,
    );
    let mut owned = OwnedItems::starter(ambition_items::builtin_item_catalog());
    owned.grant(ambition_items::builtin_item_catalog(), Item::Bomb, 1);
    let (mut app, player) = app_with(save, owned, 137);
    app.update();

    // ⚠ THE PREMISE: an old run actually in the save, the bag and the checkpoint.
    assert_eq!(app.world().resource::<AmbitionGameSave>().data().wallet(), 137);
    assert_eq!(
        app.world().resource::<OwnedItems>().count(Item::Bomb),
        1,
        "the fixture never acquired the old run's weapon"
    );
    assert_eq!(
        app.world().resource::<OwnedItemsBaseline>().remembered().count(Item::Bomb),
        1,
        "the fixture's checkpoint never held the old run's weapon"
    );

    // The commit's own work: the file is wiped, then the domain reducers run
    // with the pinned fresh inputs installed.
    *app.world_mut().resource_mut::<AmbitionGameSave>() = AmbitionGameSave::default();
    let mut starter = OwnedItemsBaseline::default();
    starter.adopt(OwnedItems::starter(ambition_items::builtin_item_catalog()));
    app.world_mut().insert_resource(ItemCheckpointRestoreInputs {
        minted: Default::default(),
        owned: starter,
    });
    app.world_mut()
        .insert_resource(ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore);
    app.world_mut()
        .run_system_once(restore_owned_items_to_checkpoint)
        .unwrap();
    app.world_mut()
        .run_system_once(start_the_item_domain_fresh)
        .unwrap();
    app.world_mut().remove_resource::<ItemCheckpointRestoreInputs>();
    app.world_mut()
        .remove_resource::<ambition_platformer2d_shared_tangle::lifecycle::FreshRunRestore>();
    // ⭐ AND TWO ORDINARY FRAMES, which is where the defect used to land.
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<OwnedItems>().count(Item::Bomb),
        0,
        "the new run began holding the old run's weapon"
    );
    assert_eq!(
        app.world().get::<BodyWallet>(player).unwrap().balance,
        0,
        "the new run began with the old run's money"
    );
    let data = app.world().resource::<AmbitionGameSave>().data();
    assert_eq!(
        data.wallet(),
        0,
        "the freshly wiped save was repopulated with the old run's wallet by the \
         next persistence pass"
    );
    assert!(
        data.items()
            .iter()
            .all(|item| item.id != ambition_items::builtin_item_catalog().dialog_id(Item::Bomb)),
        "the freshly wiped save was repopulated with the old run's weapon: {:?}",
        data.items()
    );
    // ⭐ THE STARTER SET SURVIVES, so this is a FRESH RUN rather than an empty
    // one — a reducer that cleared everything would pass every assertion above.
    assert_eq!(
        app.world().resource::<OwnedItems>().count(Item::Fireball),
        1,
        "a new game began with an EMPTY bag rather than the starter set"
    );
    // ⭐ AND THE CHECKPOINT IS THE STARTER BAG. It was reset to an EMPTY bag
    // before 2026-09-29, so a death after a New Game and before any shrine
    // restored nothing to the bag.
    assert_eq!(
        app.world().resource::<OwnedItemsBaseline>().remembered(),
        &OwnedItems::starter(ambition_items::builtin_item_catalog()),
        "a death after this New Game would restore the wrong bag"
    );
}
