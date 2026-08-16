//! Unit-level proof that the two directions of the durable horizon are inverses,
//! including for the variant no live producer can reach.
//!
//! ⚠ **these are about the TRANSLATION, not about reconstruction.** What a
//! restored ledger makes a room do is a behavioural question and is asserted by
//! `game/ambition_app/tests/a_save_remembers_where_you_left_things.rs`, against a
//! real world.

use super::*;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

fn horizon_app() -> App {
    let mut app = App::new();
    app.add_message::<ResetToCheckpoint>()
        .init_resource::<AmbitionGameSave>()
        .init_resource::<SaveRestored>()
        .init_resource::<AuthoredOccurrences>()
        .init_resource::<OccurrenceBaseline>()
        .init_resource::<CustodyBaseline>()
        .init_resource::<MintedItemBaseline>();
    app.world_mut().spawn((PlayerEntity, PrimaryPlayer));
    app
}

/// **A ledger goes to disk and comes back the same ledger — every variant.**
///
/// ⭐ **the `Consumed` row is the reason this is a round trip and not a write
/// test.** It has no live producer, so no world can be driven into the state that
/// produces one; the only way to find out whether the format keeps it is to put
/// one in and take it out again. A terminal disposition a save cannot express is
/// a terminal disposition a load silently undoes.
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
    app.add_systems(Update, persist_durable_horizon_to_save);
    app.update();

    let written = app.world().resource::<AmbitionGameSave>().data().clone();
    assert_eq!(written.occurrences.len(), 3, "every row reaches the file");

    // ── and back into a world that knows nothing ─────────────────────────────
    let mut reloaded = horizon_app();
    reloaded.world_mut().resource_mut::<AmbitionGameSave>().0 = written;
    reloaded.add_systems(Update, restore_durable_horizon);
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
            // ⚠ ROUNDED to whole pixels on the way out, and that is the format's
            // choice rather than a defect — see `PersistedWhereabouts::Placed`.
            at: Vec2::new(-48.0, 97.0),
        }),
    );
    assert_eq!(
        ledger.whereabouts(&SimId::placement("eaten")),
        Some(&OccurrenceWhereabouts::Consumed),
        "⛔ a terminal disposition the file drops is one a load undoes",
    );
}

/// **The load installs what it read as this process's BASELINE, and asks for the
/// resume that acts on it.**
///
/// ⭐ both terms observed. A load that adopts the ledger and never writes
/// `ResetToCheckpoint` installs a memory nothing acts on — no room is ever
/// rebuilt from it — and a load that resumes without adopting the baseline hands
/// the first death an empty one, which takes the whole file back off the player.
#[test]
fn a_load_seeds_the_baseline_and_requests_the_resume() {
    let mut app = horizon_app();
    {
        let data = app.world_mut().resource_mut::<AmbitionGameSave>().0.clone();
        let mut data = data;
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
    app.add_systems(Update, restore_durable_horizon);
    app.update();

    assert_eq!(
        app.world()
            .resource::<OccurrenceBaseline>()
            .remembered()
            .whereabouts(&SimId::placement("carried")),
        Some(&OccurrenceWhereabouts::InCustody),
        "the loaded state IS this process's baseline: it has no checkpoint history",
    );
    assert_eq!(
        app.world()
            .resource::<CustodyBaseline>()
            .custodian_of(&SimId::placement("carried")),
        Some(&SimId::player_slot(0)),
        "and the hand the file named is the hand the restore will look for",
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
        "⛔ the PROVENANCE comes back too — an instance rebuilt without it is \
         invisible to the next capture and survives exactly one load",
    );

    let mut resets = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ResetToCheckpoint>>();
    assert_eq!(
        resets.drain().count(),
        1,
        "a load is a checkpoint RESUME; without the request nothing acts on any \
         of the three values above",
    );
}

/// **A save written by a world that remembers nothing writes nothing**, which is
/// what keeps an untouched world's autosave from churning the file.
#[test]
fn an_untouched_world_writes_no_occurrence_rows() {
    let mut app = horizon_app();
    app.world_mut().resource_mut::<SaveRestored>().0 = true;
    app.add_systems(Update, persist_durable_horizon_to_save);
    app.update();
    let data = app.world().resource::<AmbitionGameSave>().data();
    assert!(data.occurrences.is_empty());
    assert!(data.custody.is_empty());
    assert!(data.minted_items.is_empty());
}

/// **A file with nothing to say asks for no resume**, and the latch still moves.
///
/// ⛔ the failure this pins is an unconditional `ResetToCheckpoint` on every
/// load: a resume rebuilds the active room and teleports the body, so a fresh
/// boot — which is every demo, every harness and every new game — would take one
/// for a file that describes nothing.
#[test]
fn a_load_with_nothing_remembered_asks_for_no_resume() {
    let mut app = horizon_app();
    app.add_systems(Update, restore_durable_horizon);
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
