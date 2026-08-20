//! Unit-level proof that the durable horizon's domain adapters agree on the
//! on-disk form and on load-to-checkpoint adoption.
//!
//! Reconstruction itself is covered by the app-level save/load fixtures; these
//! tests pin the translation and the one-shot resume request.

use super::*;
use ambition_characters::actor::BodyWallet;
use ambition_persistence::save_data::PersistedMintedItem;
use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
use ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

use crate::items::pickup::minted_horizon::{
    MintedItemBaseline, MintedItemDescription, OwnedItemsBaseline,
};

fn horizon_app() -> App {
    let mut app = App::new();
    app.add_message::<ResetToCheckpoint>()
        .init_resource::<AmbitionGameSave>()
        .init_resource::<SaveRestored>()
        .init_resource::<AuthoredOccurrences>()
        .init_resource::<OccurrenceBaseline>()
        .init_resource::<CustodyBaseline>()
        .init_resource::<MintedItemBaseline>()
        .init_resource::<OwnedItemsBaseline>()
        .init_resource::<crate::items::OwnedItems>();
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        BodyWallet { balance: 0 },
    ));
    app
}

#[test]
fn every_whereabouts_survives_the_write_and_the_read() {
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;
    {
        let mut ledger = app.world_mut().resource_mut::<AuthoredOccurrences>();
        ledger.adopt_rows(
            [
                (SimId::placement("carried"), OccurrenceWhereabouts::InCustody),
                (
                    SimId::placement("dropped"),
                    OccurrenceWhereabouts::Placed {
                        room: "portal_bridge".into(),
                        at: Vec2::new(-48.4, 96.6),
                    },
                ),
                (SimId::placement("eaten"), OccurrenceWhereabouts::Consumed),
            ]
            .into_iter()
            .collect(),
        );
    }
    install_durable_save_horizon(&mut app);
    app.update();

    let written = app.world().resource::<AmbitionGameSave>().data().clone();
    assert_eq!(written.occurrences.len(), 3, "every row reaches the file");

    let mut reloaded = horizon_app();
    reloaded.world_mut().resource_mut::<AmbitionGameSave>().0 = written;
    install_durable_save_horizon(&mut reloaded);
    reloaded.update();

    let ledger = reloaded.world().resource::<AuthoredOccurrences>();
    assert_eq!(
        ledger.whereabouts(&SimId::placement("carried")),
        Some(&OccurrenceWhereabouts::InCustody),
    );
    assert_eq!(
        ledger.whereabouts(&SimId::placement("dropped")),
        Some(&OccurrenceWhereabouts::Placed {
            room: "portal_bridge".into(),
            at: Vec2::new(-48.0, 97.0),
        }),
    );
    assert_eq!(
        ledger.whereabouts(&SimId::placement("eaten")),
        Some(&OccurrenceWhereabouts::Consumed),
        "a terminal disposition the file drops is one a load undoes",
    );
}

#[test]
fn a_load_seeds_every_domain_baseline_and_requests_the_resume() {
    let mut app = horizon_app();
    {
        let mut data = app.world().resource::<AmbitionGameSave>().data().clone();
        data.occurrences = vec![PersistedOccurrence::new(
            "placement:carried",
            PersistedWhereabouts::InCustody,
        )];
        data.custody = vec![PersistedCustody::new("placement:carried", "slot:0")];
        data.minted_items = vec![PersistedMintedItem {
            occurrence: "slot:0/0".into(),
            parent: "slot:0".into(),
            sequence: 0,
            held_item: "javelin".into(),
        }];
        app.world_mut().resource_mut::<AmbitionGameSave>().0 = data;
    }
    install_durable_save_horizon(&mut app);
    app.update();

    assert_eq!(
        app.world()
            .resource::<OccurrenceBaseline>()
            .remembered()
            .whereabouts(&SimId::placement("carried")),
        Some(&OccurrenceWhereabouts::InCustody),
    );
    assert_eq!(
        app.world()
            .resource::<CustodyBaseline>()
            .custodian_of(&SimId::placement("carried")),
        Some(&SimId::player_slot(0)),
    );
    assert_eq!(
        app.world()
            .resource::<MintedItemBaseline>()
            .description_of(&SimId::from_snapshot("slot:0/0".into())),
        Some(&MintedItemDescription {
            origin: SpawnOrigin::Dynamic {
                parent: SimId::player_slot(0),
                sequence: 0,
            },
            held_item: "javelin".into(),
        }),
        "the item-domain durable adopter restores provenance with the description",
    );

    let mut resets = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ResetToCheckpoint>>();
    assert_eq!(
        resets.drain().count(),
        1,
        "a load becomes a checkpoint resume only after every domain adopter lands",
    );
}

#[test]
fn an_untouched_world_writes_no_occurrence_rows() {
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;
    install_durable_save_horizon(&mut app);
    app.update();
    let data = app.world().resource::<AmbitionGameSave>().data();
    assert!(data.occurrences.is_empty());
    assert!(data.custody.is_empty());
    assert!(data.minted_items.is_empty());
}

#[test]
fn a_load_with_nothing_remembered_asks_for_no_resume() {
    let mut app = horizon_app();
    install_durable_save_horizon(&mut app);
    app.update();
    let mut resets = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ResetToCheckpoint>>();
    assert_eq!(
        resets.drain().count(),
        0,
        "a world already in its authored state must not be rebuilt to reach it"
    );
}
