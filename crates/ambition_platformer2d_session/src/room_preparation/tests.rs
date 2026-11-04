use super::*;
use crate::checkpoint::{AcceptedRestore, CheckpointOperationKey};
use crate::lifecycle_commit::{LifecycleIntent, RoomReconstitutionIntent};
use ambition_platformer2d_shared_tangle::lifecycle::{
    CheckpointRestoreInputs, OccurrenceBaseline, insert_session_world_component,
    session_world_component, session_world_component_mut,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::ecs::system::RunSystemOnce;

fn fixture() -> World {
    let mut world = World::new();
    insert_session_world_component(
        &mut world,
        (
            CapturedRoomPreparation::default(),
            AcceptedCheckpointRestore::default(),
        ),
    );
    world.insert_resource(PendingLifecycleCommit::default());
    world.insert_resource(AuthoredOccurrences::default());
    admit(&mut world, 4);
    world
}

fn admit(world: &mut World, frame: i32) {
    assert!(
        world
            .resource_mut::<PendingLifecycleCommit>()
            .record(
                frame,
                LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent {
                    target_room: "east".into()
                })
            )
            .admitted()
    );
}

fn capture(world: &mut World) -> RoomPreparation {
    world
        .run_system_once(capture_room_preparation_inputs)
        .unwrap();
    let operation = world.resource::<PendingLifecycleCommit>().peek().unwrap();
    session_world_component::<CapturedRoomPreparation>(world)
        .unwrap()
        .for_operation(operation)
        .unwrap()
        .clone()
}

#[test]
fn delayed_readiness_keeps_the_admission_population_and_withdrawal_allows_a_new_capture() {
    let mut world = fixture();
    world
        .resource_mut::<AuthoredOccurrences>()
        .consume([SimId::placement("first")]);
    let expected = world.resource::<AuthoredOccurrences>().clone();
    let first = capture(&mut world);
    world
        .resource_mut::<AuthoredOccurrences>()
        .consume([SimId::placement("later")]);
    let later = capture(&mut world);
    assert!(Arc::ptr_eq(&first.0, &later.0));
    assert_eq!(
        later.continuity().unwrap().occurrences.as_ref(),
        Some(&expected)
    );
    world.resource_mut::<PendingLifecycleCommit>().take();
    world
        .run_system_once(capture_room_preparation_inputs)
        .unwrap();
    assert_eq!(
        session_world_component::<CapturedRoomPreparation>(&world)
            .unwrap()
            .checksum(),
        0
    );
    admit(&mut world, 8);
    let next = capture(&mut world);
    assert_ne!(next, first);
    assert_eq!(
        next.continuity().unwrap().occurrences.as_ref(),
        Some(world.resource::<AuthoredOccurrences>())
    );
}

#[test]
fn a_restore_uses_its_exact_key_and_never_falls_back_to_live_inputs() {
    let mut world = fixture();
    let key = CheckpointOperationKey {
        scope: None,
        sequence: 3,
    };
    world
        .resource_mut::<PendingLifecycleCommit>()
        .pending
        .as_mut()
        .unwrap()
        .checkpoint_operation = Some(key);
    let missing = capture(&mut world);
    assert_eq!(
        missing.continuity(),
        Err(RoomPreparationError::MissingRestoreInputs)
    );
    let mut baseline = OccurrenceBaseline::default();
    let mut old = AuthoredOccurrences::default();
    old.consume([SimId::placement("at-checkpoint")]);
    baseline.adopt(old.clone());
    let operation = world
        .resource::<PendingLifecycleCommit>()
        .peek()
        .unwrap()
        .clone();
    session_world_component_mut::<AcceptedCheckpointRestore>(&mut world)
        .unwrap()
        .accept(AcceptedRestore {
            key,
            frame: operation.frame,
            intent: operation.kind,
            lifecycle: Some(CheckpointRestoreInputs {
                occurrences: baseline,
                custody: Default::default(),
            }),
            item: None,
            fresh: false,
            replay: None,
        });
    // A failed selection is immutable until the simulation admits another operation.
    assert_eq!(capture(&mut world), missing);
    world
        .resource_mut::<PendingLifecycleCommit>()
        .pending
        .as_mut()
        .unwrap()
        .frame += 1;
    assert_eq!(
        capture(&mut world)
            .continuity()
            .unwrap()
            .occurrences
            .as_ref(),
        Some(&old)
    );
    world
        .resource_mut::<PendingLifecycleCommit>()
        .pending
        .as_mut()
        .unwrap()
        .checkpoint_operation = Some(CheckpointOperationKey {
        scope: None,
        sequence: 4,
    });
    assert_eq!(
        capture(&mut world).continuity(),
        Err(RoomPreparationError::MissingRestoreInputs)
    );
}

#[test]
fn absence_and_an_empty_installed_domain_have_different_peer_checksums() {
    let mut world = fixture();
    let installed = capture(&mut world);
    world.remove_resource::<AuthoredOccurrences>();
    session_world_component_mut::<CapturedRoomPreparation>(&mut world)
        .unwrap()
        .0 = None;
    let absent = capture(&mut world);
    assert_ne!(installed.0.checksum, absent.0.checksum);
    assert!(absent.continuity().unwrap().occurrences.is_none());
}

#[test]
fn capture_releases_retiring_custody_but_keeps_it_when_another_participant_stays() {
    use crate::lifecycle_commit::RoomTransitionIntent;
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d_shared_tangle::lifecycle::{
        AwayFromAuthoredRoom, InRoomInstance, LiveBodyId, LiveRoomInstance, RoomInstanceRoot,
    };
    for another_stays in [false, true] {
        let mut world = fixture();
        let room = LiveRoomInstance::from_ordinal(1);
        world.spawn((RoomInstanceRoot, room));
        let traveller = SimId::placement("traveller");
        let away = SimId::placement("carried-from-east");
        world.spawn((
            traveller.clone(),
            InRoomInstance(room),
            DrivingParticipant(PlayerSlot(0)),
        ));
        world.spawn((away.clone(), InRoomInstance(room)));
        if another_stays {
            world.spawn((
                SimId::placement("stays"),
                InRoomInstance(room),
                DrivingParticipant(PlayerSlot(1)),
            ));
        }
        world.insert_resource(AwayFromAuthoredRoom([away.clone()].into_iter().collect()));
        world
            .resource_mut::<AuthoredOccurrences>()
            .republish_custody([away.clone()].into_iter().collect());
        world
            .resource_mut::<PendingLifecycleCommit>()
            .pending
            .as_mut()
            .unwrap()
            .kind = LifecycleIntent::Transition(RoomTransitionIntent {
            disposition: Default::default(),
            source_zone: None,
                initiator: crate::lifecycle_commit::RoomBodyTransfer {
                    subject: LiveBodyId::new(traveller, Some(room)),
                    arrival: crate::lifecycle_commit::RoomArrival::At(Vec2::ZERO),
                    participant: Some(PlayerSlot(0)),
                },
                companions: Vec::new(),
            target_room: "east".into(),
            edge_exit: false,
            zone_sfx: None,
        });
        let selected = capture(&mut world);
        assert_eq!(
            selected
                .continuity()
                .unwrap()
                .occurrences
                .as_ref()
                .unwrap()
                .in_custody()
                .contains(&away),
            another_stays
        );
        assert!(
            world
                .resource::<AuthoredOccurrences>()
                .in_custody()
                .contains(&away)
        );
    }
}

#[test]
fn captured_restore_checksums_exclude_host_local_scope_values() {
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;
    let selected = |scope| {
        let mut world = fixture();
        let key = CheckpointOperationKey {
            scope: Some(SessionScopeId(scope)),
            sequence: 3,
        };
        world
            .resource_mut::<PendingLifecycleCommit>()
            .pending
            .as_mut()
            .unwrap()
            .checkpoint_operation = Some(key);
        let operation = world
            .resource::<PendingLifecycleCommit>()
            .peek()
            .unwrap()
            .clone();
        session_world_component_mut::<AcceptedCheckpointRestore>(&mut world)
            .unwrap()
            .accept(AcceptedRestore {
                key,
                frame: operation.frame,
                intent: operation.kind,
                lifecycle: None,
                item: None,
                fresh: false,
                replay: None,
            });
        capture(&mut world)
    };
    let first = selected(123);
    let second = selected(456);
    assert_ne!(first.0.admission, second.0.admission);
    assert_eq!(first.0.checksum, second.0.checksum);
}

#[test]
fn delayed_preparation_keeps_the_mint_descriptions_selected_at_admission() {
    use ambition_persistence::{save::AmbitionGameSave, save_data::PersistedMintedItem};
    let mut world = fixture();
    let mut save = AmbitionGameSave::default();
    let occurrence = SimId::placement("mint-description-fixture");
    save.data_mut().set_minted_items(vec![PersistedMintedItem {
        occurrence: occurrence.as_str().into(),
        parent: "fixture-maker".into(),
        sequence: 4,
        held_item: "original".into(),
    }]);
    world.insert_resource(save);
    let first = capture(&mut world);
    world
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_minted_items(Vec::new());
    let delayed = capture(&mut world);
    assert_eq!(delayed, first);
    assert_eq!(
        delayed
            .continuity()
            .unwrap()
            .minted
            .as_ref()
            .unwrap()
            .description_of(&occurrence)
            .unwrap()
            .held_item,
        "original"
    );
    world
        .resource_mut::<PendingLifecycleCommit>()
        .pending
        .as_mut()
        .unwrap()
        .frame += 1;
    assert!(
        capture(&mut world)
            .continuity()
            .unwrap()
            .minted
            .as_ref()
            .unwrap()
            .is_empty()
    );
}
