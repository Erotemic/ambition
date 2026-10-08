use super::*;
use crate::avatar::PlayerHealRequested;
use ambition_combat::components::FeatureId;
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use bevy::prelude::{App, Update};

fn player_at(app: &mut App, pos: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
            BodyKinematics {
                pos,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
        ))
        .id()
}

fn health_pickup_at(app: &mut App, id: &str, pos: ae::Vec2) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            FeatureName::new("Health"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(12.0, 12.0)),
            PickupFeature::new(ambition_interaction::Pickup::new(
                id,
                ambition_interaction::PickupKind::Health { amount: 1 },
            )),
        ))
        .id()
}

#[test]
fn collect_marks_only_the_overlapping_pickup() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxInRoom>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    player_at(&mut app, center);
    let overlapping = health_pickup_at(&mut app, "hp_near", center);
    let distant = health_pickup_at(&mut app, "hp_far", ae::Vec2::new(1000.0, 1000.0));

    app.update();

    assert!(
        app.world().get::<Collected>(overlapping).is_some(),
        "a pickup the player overlaps should be Collected"
    );
    assert!(
        app.world().get::<Collected>(distant).is_none(),
        "a distant pickup should be left uncollected"
    );
}

#[test]
fn currency_pickup_credits_the_player_wallet() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxInRoom>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            ambition_characters::actor::BodyWallet::default(),
            BodyKinematics {
                pos: center,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
        ))
        .id();
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("coin"),
        FeatureName::new("Coin"),
        CenteredAabb::from_center_size(center, ae::Vec2::new(12.0, 12.0)),
        PickupFeature::new(ambition_interaction::Pickup::new(
            "coin",
            ambition_interaction::PickupKind::Currency { amount: 25 },
        )),
    ));

    app.update();
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyWallet>(player)
            .unwrap()
            .balance,
        25,
        "collecting a currency pickup should credit the wallet"
    );
}

#[test]
fn collecting_an_ability_pickup_grants_it_to_the_catalog() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.insert_resource(ambition_items::OwnedItems::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxInRoom>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    app.world_mut().spawn((
        PlayerEntity,
        ambition_characters::actor::BodyWallet::default(),
        BodyKinematics {
            pos: center,
            size: ae::Vec2::new(28.0, 46.0),
            facing: 1.0,
            ..Default::default()
        },
        BodyBaseSize {
            base_size: ae::Vec2::new(28.0, 46.0),
        },
    ));
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureId::new("ability_drop"),
        FeatureName::new("Blink"),
        CenteredAabb::from_center_size(center, ae::Vec2::new(16.0, 16.0)),
        PickupFeature::new(ambition_interaction::Pickup::new(
            "ability_drop",
            ambition_interaction::PickupKind::Ability {
                ability_id: "blink".to_string(),
            },
        )),
    ));

    app.update();
    assert!(
        app.world()
            .resource::<ambition_items::OwnedItems>()
            .has(ambition_items::Item::Blink),
        "collecting an ability pickup should grant it to the catalog",
    );
}

#[test]
fn collect_is_a_noop_with_no_player() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxInRoom>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let pickup = health_pickup_at(&mut app, "hp", ae::Vec2::new(64.0, 64.0));
    app.update();
    assert!(
        app.world().get::<Collected>(pickup).is_none(),
        "with no player, nothing is collected"
    );
}

#[test]
fn a_pickup_that_declares_no_magnet_stays_where_it_landed() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(100.0, 100.0));
    // Well inside the CLASSIC range (dist 100 < 130), and carrying no magnet.
    let sitting = health_pickup_at(&mut app, "sitting", ae::Vec2::new(200.0, 100.0));
    app.update();
    assert_eq!(
        app.world().get::<CenteredAabb>(sitting).unwrap().center.x,
        200.0,
        "a pickup with no PickupMagnet must not drift — Mary-O's coins and \
         Sanic's rings are exactly this case",
    );
}

/// The magnet pulls toward the NEAREST collector, not toward "the player".
///
///  the old rule queried `With<PrimaryPlayer>` and `.single()`, so on a couch
/// every coin in the room flew at seat one — and with two players present it
/// would not have run at all.
#[test]
fn a_magnetized_pickup_goes_to_the_nearest_collector_of_several() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(0.0, 100.0));
    player_at(&mut app, ae::Vec2::new(260.0, 100.0));

    let pickup = health_pickup_at(&mut app, "contested", ae::Vec2::new(200.0, 100.0));
    app.world_mut()
        .entity_mut(pickup)
        .insert(super::PickupMagnet::classic());
    app.update();

    let x = app.world().get::<CenteredAabb>(pickup).unwrap().center.x;
    assert!(
        x > 200.0,
        "the pickup must move toward the NEARER body at x=260 (went to x={x})",
    );
}

/// ⛔⛔ TWO EQUIDISTANT COLLECTORS IS A COIN FLIP UNTIL SOMETHING BREAKS THE TIE.
///
/// `min_by` on distance alone keeps whichever candidate the query yields first,
/// which is archetype order — not a promise, and not what a rollback
/// resimulation reproduces. Which body a contested pickup flies to is
/// authoritative gameplay state, so deciding it by iteration order is
/// deterministically wrong.
///
/// ⭐ THE ARM THAT CATCHES IT IS SPAWN ORDER, exactly as the projectile-victim
/// tie-break's is: the same two bodies are spawned left-then-right and
/// right-then-left, and the pickup must go the SAME way both times. A single
/// arrangement agrees with the bug whenever the archetype happens to list the
/// winner first.
#[test]
fn a_pickup_between_two_equidistant_collectors_goes_the_same_way_whichever_spawned_first() {
    fn drift(left_first: bool) -> f32 {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
        app.add_systems(Update, magnetize_pickups);
        // EXACTLY equidistant, and both inside the classic 130px range.
        let left = ae::Vec2::new(100.0, 100.0);
        let right = ae::Vec2::new(300.0, 100.0);
        let (first, second) = if left_first {
            (left, right)
        } else {
            (right, left)
        };
        let a = player_at(&mut app, first);
        let b = player_at(&mut app, second);
        // ⭐ IDENTITIES, or the tie-break has nothing to break the tie WITH and
        // this test measures encounter order twice. The ids are fixed to the
        // POSITION, not to the spawn order, so "the same winner" means the same
        // body and not the same slot.
        for (entity, at) in [(a, first), (b, second)] {
            let id = if at.x < 200.0 { "left" } else { "right" };
            app.world_mut()
                .entity_mut(entity)
                .insert(ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id));
        }

        let pickup = health_pickup_at(&mut app, "contested", ae::Vec2::new(200.0, 100.0));
        app.world_mut()
            .entity_mut(pickup)
            .insert(super::PickupMagnet::classic());
        app.update();
        app.world().get::<CenteredAabb>(pickup).unwrap().center.x
    }

    let a = drift(true);
    let b = drift(false);
    assert!(
        (a - 200.0).abs() > 1.0,
        "the pickup did not move at all, so this arm cannot tell one winner \
         from the other (x={a})"
    );
    assert_eq!(
        a, b,
        "the contested pickup went one way when the left collector was spawned \
         first and the other way when the right one was — the winner is \
         archetype order, which a resimulation does not reproduce"
    );
}

#[test]
fn nearby_pickups_drift_toward_the_player() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
    app.add_systems(Update, magnetize_pickups);
    player_at(&mut app, ae::Vec2::new(100.0, 100.0));
    // In range (dist 100 < 130) -> drifts toward the collector (leftward).
    let near = health_pickup_at(&mut app, "near", ae::Vec2::new(200.0, 100.0));
    // Out of range (dist 400) -> unmoved.
    let far = health_pickup_at(&mut app, "far", ae::Vec2::new(500.0, 100.0));
    // Both DECLARE the classic magnet now — attraction is a pickup's policy.
    for pickup in [near, far] {
        app.world_mut()
            .entity_mut(pickup)
            .insert(super::PickupMagnet::classic());
    }
    app.update();
    let near_x = app.world().get::<CenteredAabb>(near).unwrap().center.x;
    let far_x = app.world().get::<CenteredAabb>(far).unwrap().center.x;
    assert!(
        near_x < 200.0,
        "the nearby pickup drifted toward the player (x={near_x})"
    );
    assert_eq!(far_x, 500.0, "the far pickup is out of magnet range");
}

/// ⛔⛔ WHO GETS THE RING, WHEN TWO PLAYERS ARE STANDING ON IT.
///
/// `collect_ecs_pickups` resolved this with `collectors.iter().find(..)`, and
/// the comment above it said "find the first overlapping collector" — which
/// reads like a rule and is not one. "First" is Bevy query order, i.e. archetype
/// order, and a resimulated tick can present the same two bodies in the other
/// one. Depending on the pickup that decides who heals, who banks the currency,
/// and who takes the flag.
mod who_gets_it {
    use super::*;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    fn identified_player_at(app: &mut App, slot: u8, pos: ae::Vec2) -> bevy::prelude::Entity {
        let entity = player_at(app, pos);
        app.world_mut()
            .entity_mut(entity)
            .insert(SimId::player_slot(slot));
        entity
    }

    /// Run one collection with the two collectors spawned in `order`, and report
    /// the winner's stable identity.
    fn winner_with_spawn_order(order: [u8; 2]) -> SimId {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<PlayerHealRequested>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxInRoom>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
        app.add_systems(Update, collect_ecs_pickups);

        let ring = ae::Vec2::new(64.0, 64.0);
        // EQUIDISTANT ON PURPOSE. The metric cannot separate them, so the answer
        // is entirely the tie-break — which is the half that was missing.
        let places = [
            ring + ae::Vec2::new(-6.0, 0.0),
            ring + ae::Vec2::new(6.0, 0.0),
        ];
        for (slot, place) in order.into_iter().zip(places) {
            identified_player_at(&mut app, slot, place);
        }
        let pickup = health_pickup_at(&mut app, "hp_contested", ring);

        app.update();

        assert!(
            app.world().get::<Collected>(pickup).is_some(),
            "nobody collected a pickup both bodies overlap, so this arm proves \
             nothing about who won"
        );
        let world = app.world_mut();
        let heals = world.resource_mut::<bevy::prelude::Messages<PlayerHealRequested>>();
        let mut cursor = heals.get_cursor();
        let target = cursor
            .read(&heals)
            .next()
            .and_then(|heal| heal.target)
            .expect("the heal is routed to the specific body that collected it");
        let world = app.world();
        world
            .get::<SimId>(target)
            .cloned()
            .expect("the winner carries the identity the tie-break used")
    }

    /// ⭐ THE PROPERTY. Reversing the order the two bodies were spawned in must
    /// not change who collects. Under `.iter().find(..)` it does — the winner
    /// follows archetype order, so this arm reads `slot:0` one way and `slot:1`
    /// the other.
    #[test]
    fn the_same_body_collects_whichever_order_the_two_were_spawned_in() {
        let forward = winner_with_spawn_order([0, 1]);
        let reversed = winner_with_spawn_order([1, 0]);
        assert_eq!(
            forward, reversed,
            "which of two equidistant bodies collected the pickup changed with \
             the order they were spawned in, so a resimulated tick can hand it \
             to the other player"
        );
        assert_eq!(
            forward,
            SimId::player_slot(0),
            "the tie-break is stable SimId, so the lower slot wins an exact tie"
        );
    }

    /// And the metric still comes first: a body that is genuinely nearer wins
    /// regardless of its identity, or the tie-break would have quietly become
    /// the whole rule.
    #[test]
    fn the_nearer_body_wins_even_with_the_higher_identity() {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<PlayerHealRequested>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxInRoom>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
        app.add_systems(Update, collect_ecs_pickups);

        let ring = ae::Vec2::new(64.0, 64.0);
        identified_player_at(&mut app, 0, ring + ae::Vec2::new(-12.0, 0.0));
        identified_player_at(&mut app, 1, ring);
        health_pickup_at(&mut app, "hp_contested", ring);

        app.update();

        let world = app.world_mut();
        let heals = world.resource::<bevy::prelude::Messages<PlayerHealRequested>>();
        let mut cursor = heals.get_cursor();
        let target = cursor
            .read(heals)
            .next()
            .and_then(|heal| heal.target)
            .expect("a heal was routed");
        assert_eq!(
            app.world().get::<SimId>(target).cloned(),
            Some(SimId::player_slot(1)),
            "the higher slot standing exactly on the pickup lost to a farther \
             body, so the identity tie-break is outranking the gameplay metric"
        );
    }
}

/// Q154: only a collected AUTHORED pickup whose policy is `Never` is written
/// `Consumed`. The controls, all collected too: an `OnRoomReload` pickup (the
/// unauthored policy), an `AfterSeconds` one (it regrows), a `Never` one that
/// is still lying there, and a dropped `Never` one (no record to build again).
#[test]
fn only_a_taken_authored_never_pickup_is_remembered_as_consumed() {
    use ambition_entity_catalog::placements::HazardRespawn;
    use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
    use ambition_platformer2d_shared_tangle::lifecycle::{AuthoredOccurrences, OccurrenceWhereabouts};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    let mut app = App::new();
    app.init_resource::<AuthoredOccurrences>().init_resource::<ConsumedSinceCheckpoint>();
    app.add_systems(Update, record_consumed_pickups);
    let pickup = |app: &mut App, id: &str, respawn: HazardRespawn, authored: bool, taken: bool| {
        let mut feature = ambition_interaction::Pickup::new(id, ambition_interaction::PickupKind::Health { amount: 1 });
        feature.respawn = respawn;
        let origin = if authored {
            SpawnOrigin::Authored { source: "room".into(), instance: id.into() }
        } else {
            SpawnOrigin::Dynamic { parent: SimId::placement("boss"), sequence: 0 }
        };
        let mut entity = app.world_mut().spawn((SimId::placement(id), PickupFeature::new(feature), origin));
        if taken {
            entity.insert(Collected);
        }
    };
    pickup(&mut app, "never", HazardRespawn::Never, true, true);
    pickup(&mut app, "reload", HazardRespawn::OnRoomReload, true, true);
    pickup(&mut app, "regrows", HazardRespawn::AfterSeconds(4.0), true, true);
    pickup(&mut app, "untaken", HazardRespawn::Never, true, false);
    pickup(&mut app, "dropped", HazardRespawn::Never, false, true);
    app.update();
    let ledger = app.world().resource::<AuthoredOccurrences>();
    let rows: Vec<(String, OccurrenceWhereabouts)> = ledger
        .rows()
        .map(|(sim_id, row)| (sim_id.to_string(), row.clone()))
        .collect();
    assert_eq!(
        rows,
        vec![(SimId::placement("never").to_string(), OccurrenceWhereabouts::Consumed)],
        "the ledger's rows after one tick"
    );
}

/// Q151 for successive rewinds: a restore takes the dying participant out of
/// each record's owners, so a record owned only by them is forgotten and a
/// later restore of another participant does not keep it. The restore's
/// acceptance pins only the rows a spared participant owns, so today, where
/// only the primary participant's death restores, this is not observable in
/// play; it is the arithmetic a second participant's restore needs.
#[test]
fn a_restore_takes_the_dying_participant_out_of_each_consumed_record() {
    use ambition_characters::control::PlayerSlot;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    let mut app = App::new();
    app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
    app.init_resource::<ConsumedSinceCheckpoint>();
    app.add_systems(Update, disown_consumed_pickups_on_restore);
    {
        let mut since = app.world_mut().resource_mut::<ConsumedSinceCheckpoint>();
        since.record(SimId::placement("alices"), "x".into(), vec![PlayerSlot(0)]);
        since.record(SimId::placement("shared"), "x".into(), vec![PlayerSlot(0), PlayerSlot(1)]);
        since.record(SimId::placement("bobs"), "y".into(), vec![PlayerSlot(1)]);
    }
    // Control: a replay that is not a checkpoint restore keeps every owner.
    app.world_mut().write_message(
        ambition_combat::events::RoomReplayAdmitted::because(ambition_combat::RoomResetReason::PlayerDeath)
            .sparing_participants(vec![PlayerSlot(1)]),
    );
    app.update();
    assert_eq!(
        app.world().resource::<ConsumedSinceCheckpoint>().owners(&SimId::placement("alices")),
        Some(&[PlayerSlot(0)][..]),
        "control: a replay that does not rewind to the checkpoint disowned a record"
    );
    app.world_mut().write_message(
        ambition_combat::events::RoomReplayAdmitted::because(ambition_combat::RoomResetReason::PlayerDeath)
            .to_the_checkpoint()
            .sparing_participants(vec![PlayerSlot(1)]),
    );
    app.update();
    let since = app.world().resource::<ConsumedSinceCheckpoint>();
    assert_eq!(
        (
            since.owners(&SimId::placement("alices")),
            since.owners(&SimId::placement("shared")),
            since.owners(&SimId::placement("bobs")),
        ),
        (None, Some(&[PlayerSlot(1)][..]), Some(&[PlayerSlot(1)][..])),
        "(Alice's, shared, Bob's) owners after Alice's restore"
    );
    assert_eq!(
        since.owned_by(&[PlayerSlot(0)]).count(),
        0,
        "a later restore that spares only Alice keeps a row she no longer owns"
    );
}

/// A 28 by 46 body in sideways gravity lies along the gravity: its collision
/// box is 46 wide and 28 tall. A pickup past its end is in that box and not in
/// its level box; a pickup beside it is in its level box only. The collector
/// asks the box the body's last step turned (`BodyKinematics::collision_box`).
#[test]
fn a_body_in_sideways_gravity_collects_the_pickup_its_own_box_touches() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxInRoom>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    let player = player_at(&mut app, center);
    let body = *app.world().get::<BodyKinematics>(player).expect("the player has a body");
    app.world_mut()
        .entity_mut(player)
        .insert(ae::SweepSample::at_rest(body, ae::Vec2::new(1.0, 0.0)));
    let past_its_end = health_pickup_at(&mut app, "hp_past_its_end", center + ae::Vec2::new(26.0, 0.0));
    let beside = health_pickup_at(&mut app, "hp_beside", center + ae::Vec2::new(0.0, 26.0));

    app.update();

    assert!(
        app.world().get::<Collected>(past_its_end).is_some(),
        "the pickup past the end of the body is in its box, and the body must collect it"
    );
    assert!(
        app.world().get::<Collected>(beside).is_none(),
        "the pickup beside the body is not in its box, and the body must not collect it"
    );
}

/// A collected pickup reports `QuestAdvanceEvent::ItemCollected` with its
/// placement id, the id a quest step names and the content validator accepts.
/// A pickup the body does not touch reports nothing. Before this, no system
/// wrote `ItemCollected`, so a validated quest step on a pickup never advanced.
#[test]
fn a_collected_pickup_reports_its_id_to_the_quests() {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxInRoom>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_persistence::quest::QuestAdvanceRequested>();
    app.add_systems(Update, collect_ecs_pickups);

    let center = ae::Vec2::new(64.0, 64.0);
    player_at(&mut app, center);
    health_pickup_at(&mut app, "hp_near", center);
    health_pickup_at(&mut app, "hp_far", ae::Vec2::new(1000.0, 1000.0));

    app.update();

    let messages = app
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_persistence::quest::QuestAdvanceRequested>>();
    let reported: Vec<_> = messages.iter_current_update_messages().map(|request| request.0.clone()).collect();
    assert_eq!(
        reported,
        vec![ambition_persistence::quest::QuestAdvanceEvent::ItemCollected("hp_near".into())],
        "the quest events of one update: the collected pickup only"
    );
}
