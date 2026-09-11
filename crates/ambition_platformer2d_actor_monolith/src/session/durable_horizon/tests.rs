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
        .init_resource::<ambition_items::OwnedItems>();
    app.world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, BodyWallet { balance: 0 }));
    app
}

#[test]
fn every_whereabouts_survives_the_write_and_the_read() {
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;
    // the carried occurrence needs a LIVE, DURABLY-RESTORABLE custody behind it, because the
    // mirror will only put an `InCustody` claim on disk for a hand the load can rebuild — see
    // `persist_occurrence_horizon_to_save`. The row it wrote was the possessed-body shape, and that
    // is precisely the claim the horizon now declines to make.
    let holder = app.world_mut().spawn(SimId::player_slot(0)).id();
    app.world_mut().spawn((
        SimId::placement("carried"),
        ambition_held_items::ItemCustody::Held { holder },
    ));
    {
        let mut ledger = app.world_mut().resource_mut::<AuthoredOccurrences>();
        ledger.adopt_rows(
            [
                (
                    SimId::placement("carried"),
                    OccurrenceWhereabouts::InCustody,
                ),
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
    assert_eq!(written.occurrences().len(), 3, "every row reaches the file");

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

/// A CUSTODY CLAIM THE LOAD COULD NOT REBUILD DOES NOT REACH THE FILE.
///
/// The mirror's population is everything wearing `InCustodyOf`, and that
/// component has two owners: the item road derives it from `ItemCustody`, which
/// the save carries and the load applies again — and a POSSESSED BODY answers the
/// same query with nothing durable behind it. `PossessionState` is rollback
/// state, so a fresh process starts with nobody driving anything and the row's
/// other side simply does not exist.
///
/// Driven end to end by `a_save_taken_mid_possession_does_not_delete_the_enemy_in_a_fresh_process`;
/// this is the same rule at the translation layer, where it is one assertion.
///
/// both terms. The `Placed` row proves the mirror ran and wrote SOMETHING,
/// so "no custody row" cannot pass by the file being empty.
#[test]
fn an_in_custody_row_with_no_restorable_hand_behind_it_stays_out_of_the_file() {
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;
    {
        let mut ledger = app.world_mut().resource_mut::<AuthoredOccurrences>();
        ledger.adopt_rows(
            [
                // The possessed-body shape: a live `InCustody` row whose custodian
                // is possession state, which the save does not hold.
                (SimId::placement("driven"), OccurrenceWhereabouts::InCustody),
                (
                    SimId::placement("dropped"),
                    OccurrenceWhereabouts::Placed {
                        room: "portal_bridge".into(),
                        at: Vec2::new(8.0, 16.0),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );
    }
    install_durable_save_horizon(&mut app);
    app.update();

    let written = app.world().resource::<AmbitionGameSave>().data().clone();
    assert!(
        written
            .occurrences()
            .iter()
            .any(|row| row.id == "placement:dropped"),
        "the mirror wrote nothing at all, so the absence below proves nothing; \
         file was {:?}",
        written.occurrences()
    );
    assert!(
        !written
            .occurrences()
            .iter()
            .any(|row| row.id == "placement:driven"),
        "the file claims something is holding `placement:driven` while carrying \
         nothing that could rebuild the hand. On load nobody is holding it, and a \
         room build reading that row authors nothing: the occurrence is gone from \
         the world permanently. File was {:?}",
        written.occurrences()
    );
}

#[test]
fn a_load_seeds_every_domain_baseline_and_requests_the_resume() {
    let mut app = horizon_app();
    {
        let mut data = app.world().resource::<AmbitionGameSave>().data().clone();
        data.set_durable_horizon(
            vec![PersistedOccurrence::new(
                "placement:carried",
                PersistedWhereabouts::InCustody,
            )],
            vec![PersistedCustody::new("placement:carried", "slot:0")],
        );
        data.set_minted_items(vec![PersistedMintedItem {
            occurrence: "slot:0/0".into(),
            parent: "slot:0".into(),
            sequence: 0,
            held_item: "javelin".into(),
        }]);
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
    assert!(data.occurrences().is_empty());
    assert!(data.custody().is_empty());
    assert!(data.minted_items().is_empty());
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

/// ⭐⭐ **DURABILITY IS THE RELATION'S OWN STATEMENT, AND A `SessionOnly` ROW
/// NEVER REACHES THE FILE.**
///
/// ⛔⛤ **IT USED TO BE AN ABSENCE.** The filter asked whether the SUBJECT
/// carried `ambition_held_items::ItemCustody` — a marker belonging to another
/// domain — so a third producer of `InCustodyOf` became non-durable BY DEFAULT
/// and silently. The accepted-control writer map named that risk in writing:
/// *"anyone who adds one must decide durability on purpose, because the save
/// filter will not ask."* It asks now, and `CustodyDurability` has no `Default`,
/// so a producer cannot decline to answer.
///
/// ⛔ THE TWO SUBJECTS ARE IDENTICAL EXCEPT FOR THE FIELD. Same components, same
/// custodian, same room scope — so a filter that dropped both, or kept both,
/// fails here. That is the whole discrimination and it is why this is one test
/// with two bodies rather than two tests with one each.
#[test]
fn only_a_restored_custody_row_crosses_the_save_boundary() {
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;

    let hand = app.world_mut().spawn(SimId::player_slot(0)).id();
    let durable = SimId::placement("axe_in_a_hand");
    let session = SimId::placement("rider_on_a_mount");
    for (id, durability) in [
        (durable.clone(), CustodyDurability::Restored),
        (session.clone(), CustodyDurability::SessionOnly),
    ] {
        app.world_mut().spawn((
            id,
            InCustodyOf {
                custodian: hand,
                durability,
            },
            ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity,
        ));
    }

    install_durable_save_horizon(&mut app);
    app.update();

    let written = app.world().resource::<AmbitionGameSave>().data().clone();
    let held: Vec<&str> = written
        .custody()
        .iter()
        .map(|row| row.occurrence.as_str())
        .collect();
    // ⛔ THE ANTI-VACUITY TERM FIRST. "The session row is absent" is also what an
    // empty file says, and an empty file is the failure this arm would otherwise
    // read as a pass.
    assert!(
        held.contains(&durable.as_str()),
        "the durable custody row did not reach the file, so its absence proves \
         nothing about the other one; file held {held:?}"
    );
    assert!(
        !held.contains(&session.as_str()),
        "a SESSION-ONLY custody row crossed the save boundary. The loader does \
         not put a rider back on a mount, so this row is a claim the file cannot \
         answer; file held {held:?}"
    );
}
