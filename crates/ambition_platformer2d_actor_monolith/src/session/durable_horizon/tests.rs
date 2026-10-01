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

/// The session's chain plus the item domain's adapters, as the runtime composes them.
fn install_horizon(app: &mut App) {
    install_durable_save_horizon(app);
    crate::items::persist::install_item_durable_horizon(app);
}

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
    install_horizon(&mut app);
    app.update();

    let written = app.world().resource::<AmbitionGameSave>().data().clone();
    assert_eq!(written.occurrences().len(), 3, "every row reaches the file");

    let mut reloaded = horizon_app();
    reloaded.world_mut().resource_mut::<AmbitionGameSave>().0 = written;
    install_horizon(&mut reloaded);
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
    install_horizon(&mut app);
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
    install_horizon(&mut app);
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
    install_horizon(&mut app);
    app.update();
    let data = app.world().resource::<AmbitionGameSave>().data();
    assert!(data.occurrences().is_empty());
    assert!(data.custody().is_empty());
    assert!(data.minted_items().is_empty());
}

#[test]
fn a_load_with_nothing_remembered_asks_for_no_resume() {
    let mut app = horizon_app();
    install_horizon(&mut app);
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

    install_horizon(&mut app);
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

#[test]
fn a_population_the_restore_cannot_complete_on_is_written_to_by_nobody() {
    // The seeded save and every assertion mirror
    // `a_load_seeds_every_domain_baseline_and_requests_the_resume`; the ONE
    // difference is a second `PrimaryPlayer` body, which is the population
    // `complete_durable_restore` cannot finish on.
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
        app.world_mut().resource_mut::<AmbitionGameSave>().0 = data;
    }
    app.world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, BodyWallet { balance: 0 }));

    // ⭐ THE PREMISE IS HALF THE TEST. The session-start gate reads this same
    // population and reports NOT pending, so a rollback session is free to
    // start here — which is what makes an `Update` write to rollback state on
    // this road a live-timeline write rather than a pre-timeline one.
    assert!(
        !durable_hydration_is_pending(app.world_mut()),
        "the gate must let this population through, or the arm is measuring a \
         world that never starts a timeline",
    );

    install_horizon(&mut app);
    app.update();

    assert_eq!(
        app.world()
            .resource::<OccurrenceBaseline>()
            .remembered()
            .whereabouts(&SimId::placement("carried")),
        None,
        "the occurrence baseline is rollback-registered and checksummed; \
         adopting it here writes it from `Update` on every frame of a live \
         timeline, because the latch that would stop the repeat never rises",
    );
    assert_eq!(
        app.world()
            .resource::<CustodyBaseline>()
            .custodian_of(&SimId::placement("carried")),
        None,
    );
    assert!(
        !app.world().resource::<SaveRestored>().0,
        "the latch cannot rise on this population -- that is the reason the \
         adoption must not happen, so it is asserted rather than assumed",
    );

    let mut resets = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ResetToCheckpoint>>();
    assert_eq!(resets.drain().count(), 0);
}

/// FI9: the durable mirrors and the custody projection, with `n` dormant
/// mints lying in a room that is not live. Each is a ledger `Placed` row and a
/// minted row of the save.
fn dormant_world(n: usize) -> App {
    use ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts;
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;
    install_horizon(&mut app);
    app.add_systems(
        Update,
        ambition_platformer2d_shared_tangle::lifecycle::project_custody_onto_authored_occurrences,
    );
    let rows = (0..n)
        .map(|i| {
            (
                dormant(i),
                OccurrenceWhereabouts::Placed { room: "elsewhere".into(), at: Vec2::new(i as f32, 0.0) },
            )
        })
        .collect();
    app.world_mut().resource_mut::<AuthoredOccurrences>().adopt_rows(rows);
    let minted = (0..n)
        .map(|i| PersistedMintedItem {
            occurrence: dormant(i).as_str().to_string(),
            parent: "boss".into(),
            sequence: i as u64,
            held_item: Default::default(),
        })
        .collect();
    app.world_mut().resource_mut::<AmbitionGameSave>().data_mut().set_minted_items(minted);
    for _ in 0..3 {
        app.update();
    }
    let data = app.world().resource::<AmbitionGameSave>().data();
    assert_eq!(
        (data.occurrences().len(), data.minted_items().len()),
        (n, n),
        "precondition: the save does not hold the {n} dormant rows"
    );
    app
}

fn dormant(i: usize) -> SimId {
    SimId::placement(&format!("dormant/{i:05}"))
}

/// The median time of one tick, over `ticks` ticks.
fn median_tick(app: &mut App, ticks: usize) -> std::time::Duration {
    let mut times: Vec<std::time::Duration> = (0..ticks)
        .map(|_| {
            let start = std::time::Instant::now();
            app.update();
            start.elapsed()
        })
        .collect();
    times.sort();
    times[ticks / 2]
}

/// FI9: dormant records add no all-world walk to a tick that changes
/// nothing. 10,000 dormant mints in a room that is not live cost an idle
/// tick about what no dormant mint costs. Before the change gate, they cost
/// 4.7 ms a tick against 0.14 ms (the save mirror walked every ledger row and
/// the minted mirror every minted row, each tick).
#[test]
fn dormant_rows_add_no_walk_to_an_idle_tick() {
    let mut empty = dormant_world(0);
    let mut full = dormant_world(10_000);
    let (empty, full) = (median_tick(&mut empty, 200), median_tick(&mut full, 200));
    assert!(
        full < empty * 3 + std::time::Duration::from_micros(100),
        "an idle tick with 10,000 dormant rows took {full:?}, against {empty:?} with none"
    );
}

/// FI9: the gate skips only what did not change. With 10,000 dormant rows,
/// one dormant mint put down somewhere else reaches the save's row on the
/// next tick.
#[test]
fn a_changed_dormant_row_still_reaches_the_save() {
    let mut app = dormant_world(10_000);
    let moved = dormant(4321);
    let refused = app.world_mut().resource_mut::<AuthoredOccurrences>().republish_placements(
        "elsewhere",
        [(moved.clone(), Vec2::new(7.0, 9.0))].into_iter().collect(),
    );
    assert!(refused.is_empty(), "precondition: the ledger refused the move");
    app.update();
    let row = app
        .world()
        .resource::<AmbitionGameSave>()
        .data()
        .occurrences()
        .iter()
        .find(|row| row.id == moved.as_str())
        .cloned()
        .expect("the moved occurrence has a save row");
    assert_eq!(
        row.whereabouts,
        PersistedWhereabouts::Placed { room: "elsewhere".into(), x: 7, y: 9 },
        "the save did not follow the ledger"
    );
}

/// FI9: a save replaced from outside (a load, a restore) is mirrored again,
/// although the ledger and the live bodies did not change.
#[test]
fn a_replaced_save_is_mirrored_again() {
    let mut app = dormant_world(1_000);
    let mut replaced = app.world().resource::<AmbitionGameSave>().clone();
    replaced.data_mut().set_durable_horizon(Vec::new(), Vec::new());
    replaced.data_mut().set_minted_items(Vec::new());
    app.world_mut().insert_resource(replaced);
    app.update();
    let data = app.world().resource::<AmbitionGameSave>().data();
    assert_eq!(data.occurrences().len(), 1_000, "the replaced save's occurrence rows were not mirrored again");
}

/// FI9: a dormant mint taken into custody (its ledger row is no longer
/// `Placed`, and no live body describes it) leaves the save's minted rows on
/// the next tick.
#[test]
fn a_dormant_mint_taken_up_leaves_the_minted_rows() {
    let mut app = dormant_world(1_000);
    let taken = dormant(17);
    app.world_mut()
        .resource_mut::<AuthoredOccurrences>()
        .republish_custody([taken.clone()].into_iter().collect());
    app.update();
    let minted = app.world().resource::<AmbitionGameSave>().data().minted_items().to_vec();
    assert_eq!(minted.len(), 999, "the minted rows did not follow the ledger");
    assert!(!minted.iter().any(|row| row.occurrence == taken.as_str()));
}
