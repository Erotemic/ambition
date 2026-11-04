//! Local restore keys survive the domain's registered GGRS snapshot path.

use bevy::prelude::*;
use bevy_ggrs::{ChecksumPlugin, LoadWorld, RollbackFrameCount, SaveWorld, SnapshotPlugin};

use ambition_characters::control::PlayerSlot;
use ambition_platformer2d_runtime::SimulationHost;
use ambition_platformer2d_session::checkpoint::SessionCheckpointOperations;
use ambition_platformer2d_session::lifecycle_commit::{
    LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodyId, SessionScopeId};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

#[test]
fn pending_withdrawn_and_applied_restore_keys_survive_the_registered_snapshot_path() {
    for state in [0, 1, 2] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SnapshotPlugin, ChecksumPlugin));
        app.insert_resource(SimulationHost::Rollback);
        ambition_platformer2d_session::register_rollback_state(
            &mut crate::GgrsRollbackRegistrar::new(&mut app),
        );
        let mut pending = PendingLifecycleCommit::default();
        let key = pending.record_restore(
            4,
            LifecycleIntent::Transition(RoomTransitionIntent {
                disposition: Default::default(),
                source_zone: None,
                initiator: ambition_platformer2d_session::lifecycle_commit::RoomBodyTransfer {
                    subject: LiveBodyId::new(SimId::placement("traveller"), None),
                    arrival: ambition_platformer2d_session::lifecycle_commit::RoomArrival::At(Vec2::new(20.0, 30.0)),
                    participant: Some(PlayerSlot(2)),
                },
                companions: Vec::new(),
                target_room: "east".into(),
                edge_exit: false,
                zone_sfx: None,
            }),
            &mut SessionCheckpointOperations::default(),
            Some(SessionScopeId(123)),
        ).unwrap();
        if state == 1 {
            assert!(pending.retract_recorded_on(4, 5));
        } else if state == 2 {
            assert!(pending.complete(5, 91, Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION)).is_some());
        }
        let expected = pending.clone();
        app.insert_resource(pending);
        app.world_mut().resource_mut::<RollbackFrameCount>().0 = 6;
        app.world_mut().run_schedule(SaveWorld);
        app.world_mut().remove_resource::<PendingLifecycleCommit>();
        app.world_mut().run_schedule(LoadWorld);
        let restored = app.world().resource::<PendingLifecycleCommit>();
        assert_eq!(restored, &expected);
        let restored_key = match state {
            0 => restored.peek().unwrap().checkpoint_operation,
            1 => restored.withdrawn.as_ref().unwrap().checkpoint_operation,
            _ => restored.applied.as_ref().unwrap().operation.checkpoint_operation,
        };
        assert_eq!(restored_key, Some(key));
        assert_eq!(restored_key.unwrap().scope, Some(SessionScopeId(123)));
    }
}

#[test]
fn actor_terminal_uses_and_replies_survive_the_registered_snapshot_path() {
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    use ambition_platformer2d_world::rooms::{DialRefusal, GateNetworkState};
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SnapshotPlugin, ChecksumPlugin));
    app.insert_resource(SimulationHost::Rollback);
    ambition_platformer2d_world::rooms::register_gate_network_rollback_state(
        &mut crate::GgrsRollbackRegistrar::new(&mut app),
    );
    let first = LiveBodyId::new(SimId::placement("traveller"), Some(LiveRoomInstance::from_ordinal(1)));
    let second = LiveBodyId::new(SimId::placement("traveller"), Some(LiveRoomInstance::from_ordinal(2)));
    let mut expected = GateNetworkState::default();
    expected.terminals.insert(first.clone(), "west".into());
    expected.terminals.insert(second.clone(), "east".into());
    expected.report(&first, "west", vec![1], Ok("east".into()));
    expected.report(&second, "east", vec![2], Err(DialRefusal::Unknown));
    app.insert_resource(expected.clone());
    app.world_mut().resource_mut::<RollbackFrameCount>().0 = 4;
    app.world_mut().run_schedule(SaveWorld);
    app.world_mut().remove_resource::<GateNetworkState>();
    app.world_mut().run_schedule(LoadWorld);
    assert_eq!(app.world().resource::<GateNetworkState>(), &expected);
}

#[test]
fn room_preparation_inputs_survive_the_registered_snapshot_path() {
    use bevy::ecs::system::RunSystemOnce;
    use ambition_platformer2d_session::checkpoint::AcceptedCheckpointRestore;
    use ambition_platformer2d_session::room_preparation::{
        capture_room_preparation_inputs, CapturedRoomPreparation,
    };
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, AuthoredOccurrences,
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SnapshotPlugin, ChecksumPlugin));
    app.insert_resource(SimulationHost::Rollback);
    ambition_platformer2d_session::register_rollback_state(
        &mut crate::GgrsRollbackRegistrar::new(&mut app),
    );
    let root = insert_session_world_component(app.world_mut(), (
        bevy_ggrs::Rollback, CapturedRoomPreparation::default(), AcceptedCheckpointRestore::default(),
    ));
    let mut pending = PendingLifecycleCommit::default();
    assert!(pending.record(4, LifecycleIntent::Transition(RoomTransitionIntent {
        disposition: Default::default(),
        source_zone: None,
                initiator: ambition_platformer2d_session::lifecycle_commit::RoomBodyTransfer {
                    subject: LiveBodyId::new(SimId::placement("traveller"), None),
                    arrival: ambition_platformer2d_session::lifecycle_commit::RoomArrival::At(Vec2::new(20.0, 30.0)),
                    participant: Some(PlayerSlot(0)),
                },
                companions: Vec::new(),
        target_room: "east".into(),
        edge_exit: false, zone_sfx: None,
    })).admitted());
    app.insert_resource(pending.clone());
    let mut ledger = AuthoredOccurrences::default();
    ledger.consume([SimId::placement("collected-before-admission")]);
    app.insert_resource(ledger);
    app.world_mut().run_system_once(capture_room_preparation_inputs).unwrap();
    let expected = app.world().get::<CapturedRoomPreparation>(root).unwrap().clone();
    app.world_mut().resource_mut::<RollbackFrameCount>().0 = 4;
    app.world_mut().run_schedule(SaveWorld);
    app.world_mut().entity_mut(root).insert(CapturedRoomPreparation::default());
    app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
    app.world_mut().resource_mut::<AuthoredOccurrences>().consume([SimId::placement("later")]);
    app.world_mut().run_schedule(LoadWorld);
    let restored = app.world().get::<CapturedRoomPreparation>(root).unwrap();
    assert_eq!(restored, &expected);
    assert_eq!(restored.checksum(), expected.checksum());
    assert!(restored.for_operation(pending.peek().unwrap()).is_some());
    app.world_mut().run_system_once(capture_room_preparation_inputs).unwrap();
    assert_eq!(app.world().get::<CapturedRoomPreparation>(root).unwrap(), &expected);
}
