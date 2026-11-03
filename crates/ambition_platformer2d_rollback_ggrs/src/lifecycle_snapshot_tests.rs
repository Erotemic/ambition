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
fn pending_and_withdrawn_restore_keys_survive_the_registered_snapshot_path() {
    for withdrawn in [false, true] {
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
                subject: LiveBodyId::new(SimId::placement("traveller"), None),
                target_room: "east".into(),
                arrival: Vec2::new(20.0, 30.0),
                edge_exit: false,
                zone_sfx: None,
                participant: Some(PlayerSlot(2)),
            }),
            &mut SessionCheckpointOperations::default(),
            Some(SessionScopeId(123)),
        ).unwrap();
        if withdrawn {
            assert!(pending.retract_recorded_on(4, 5));
        }
        let expected = pending.clone();
        app.insert_resource(pending);
        app.world_mut().resource_mut::<RollbackFrameCount>().0 = 6;
        app.world_mut().run_schedule(SaveWorld);
        app.world_mut().remove_resource::<PendingLifecycleCommit>();
        app.world_mut().run_schedule(LoadWorld);
        let restored = app.world().resource::<PendingLifecycleCommit>();
        assert_eq!(restored, &expected);
        let restored_key = if withdrawn {
            restored.withdrawn.as_ref().unwrap().checkpoint_operation
        } else {
            restored.peek().unwrap().checkpoint_operation
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
