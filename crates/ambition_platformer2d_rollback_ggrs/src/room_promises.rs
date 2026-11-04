//! Frozen peer preparation offers and their native rollback lifetime.

use std::sync::Arc;

use bevy::prelude::*;
use ambition_platformer2d_publication::{ActiveContentBinding, RoomConstructionPlan};
use ambition_platformer2d_session::{
    lifecycle_commit::{PendingIntent, PendingLifecycleCommit},
    room_preparation::{CapturedRoomPreparation, RoomPreparation},
};
use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, SessionRoot, session_world_component};

/// Facts selected by the readiness transaction, independent of its presentation.
#[derive(Clone)]
pub(crate) struct AuthorizedRoomPlan {
    pub plan: Arc<RoomConstructionPlan>,
    pub preparation: RoomPreparation,
    pub source_room: usize,
}

impl AuthorizedRoomPlan {
    pub fn operation(&self) -> &PendingIntent {
        self.preparation.admission()
    }

    pub fn peer_identity(&self) -> Option<crate::PeerPlanIdentity> {
        self.plan.peer_identity().map(crate::PeerPlanIdentity::from)
    }

    pub fn matches_world(&self, world: &mut World) -> bool {
        let session = world.get_resource::<ActiveSessionScope>().and_then(|scope| scope.current());
        if self.plan.session_scope().id() != session {
            return false;
        }
        let binding = session_world_component::<ActiveContentBinding>(world).map(|binding| binding.0);
        if binding != Some(self.plan.content_scope().expected_live()) {
            return false;
        }
        if session_world_component::<CapturedRoomPreparation>(world)
            .and_then(|captured| captured.for_operation(self.operation())) != Some(&self.preparation) {
            return false;
        }
        ambition_platformer2d_world::rooms::live_room_definition_left_by(world, self.operation().kind.subject())
            .is_some_and(|definition| definition.index() == self.source_room)
    }
}

struct RetainedRoomPlan {
    authorized: AuthorizedRoomPlan,
    last_input_frame: i32,
}

/// Host state. A rewind must keep the plans promised by the original inputs.
#[derive(Resource, Default)]
pub(crate) struct PromisedRoomPlans {
    plans: Vec<RetainedRoomPlan>,
    saved_floor: i32,
}

impl PromisedRoomPlans {
    pub fn get(&self, preparation: &RoomPreparation) -> Option<&AuthorizedRoomPlan> {
        self.plans.iter().map(|entry| &entry.authorized)
            .find(|plan| &plan.preparation == preparation)
    }

    pub fn retain(&mut self, authorized: AuthorizedRoomPlan) {
        if self.get(&authorized.preparation).is_none() {
            let last_input_frame = authorized.operation().frame;
            self.plans.push(RetainedRoomPlan { authorized, last_input_frame });
        }
    }

    pub fn advertised(&mut self, verdict: crate::PeerVerdict, frame: i32) {
        if verdict.said != crate::PreparationVerdict::Prepared {
            return;
        }
        for entry in &mut self.plans {
            if crate::PeerOperation::from(&entry.authorized.preparation) == verdict.operation
                && entry.authorized.peer_identity() == Some(verdict.plan) {
                entry.last_input_frame = entry.last_input_frame.max(frame);
            }
        }
    }

    fn collect(&mut self, floor: i32, pending: Option<&PendingIntent>, captured: Option<&CapturedRoomPreparation>) {
        self.saved_floor = self.saved_floor.max(floor);
        self.plans.retain(|entry| {
            let plan = &entry.authorized;
            let still_pending = pending == Some(plan.operation())
                && captured.and_then(|inputs| inputs.for_operation(plan.operation())) == Some(&plan.preparation);
            still_pending || entry.last_input_frame >= self.saved_floor
        });
    }
}

/// Run after real snapshots. Sparse saves can precede the confirmed boundary.
pub(crate) fn collect_after_save(
    plans: Option<ResMut<PromisedRoomPlans>>,
    frame: Res<crate::RollbackFrameCount>,
    confirmed: Option<Res<crate::ConfirmedFrameCount>>,
    pending: Option<Res<PendingLifecycleCommit>>,
    captured: Query<&CapturedRoomPreparation, With<SessionRoot>>,
) {
    let (Some(mut plans), Some(confirmed)) = (plans, confirmed) else { return };
    plans.collect(frame.0.min(confirmed.0), pending.as_deref().and_then(|pending| pending.peek()), captured.single().ok());
}

/// A lockstep peer has no saved world and cannot replay an earlier frame.
pub(crate) fn collect_without_snapshots(
    session: Option<Res<crate::session::AmbitionGgrsSession>>,
    plans: ResMut<PromisedRoomPlans>,
    frame: Option<Res<crate::RollbackFrameCount>>,
    pending: Option<Res<PendingLifecycleCommit>>,
    captured: Query<&CapturedRoomPreparation, With<SessionRoot>>,
) {
    if matches!(session.as_deref(), Some(bevy_ggrs::Session::P2P(session)) if session.max_prediction() == 0) {
        let Some(frame) = frame else { return };
        let mut plans = plans;
        plans.collect(frame.0, pending.as_deref().and_then(|pending| pending.peek()), captured.single().ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy_ggrs::{RollbackApp, SnapshotPlugin};
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_session::{
        checkpoint::AcceptedCheckpointRestore,
        lifecycle_commit::{LifecycleIntent, RoomReconstitutionIntent},
        room_preparation::capture_room_preparation_inputs,
    };
    use ambition_platformer2d_shared_tangle::{
        construction::ContentBinding,
        lifecycle::{insert_session_world_component, SessionSpawnScope},
    };

    fn prepared() -> (App, AuthorizedRoomPlan) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SnapshotPlugin));
        app.rollback_resource_with_clone::<PendingLifecycleCommit>()
            .rollback_component_with_clone::<CapturedRoomPreparation>();
        let root = insert_session_world_component(app.world_mut(), (
            bevy_ggrs::Rollback, CapturedRoomPreparation::default(), AcceptedCheckpointRestore::default(),
        ));
        let mut pending = PendingLifecycleCommit::default();
        assert!(pending.record(10, LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent {
            target_room: "east".into(),
        })).admitted());
        let operation = pending.peek().unwrap().clone();
        app.insert_resource(pending);
        app.world_mut().run_system_once(capture_room_preparation_inputs).unwrap();
        let preparation = app.world().get::<CapturedRoomPreparation>(root).unwrap()
            .for_operation(&operation).unwrap().clone();
        let binding = ContentBinding::content(ae::ContentEpoch(1), ae::PeerContentIdentity::from_bytes([1; 32]));
        let plan = RoomConstructionPlan::prepare_spec(
            0, ambition_platformer2d_world::rooms::RoomSpec::new("east", ae::World::new(
                "east", ae::Vec2::new(640.0, 480.0), ae::Vec2::new(96.0, 96.0), Vec::new(),
            )),
            &Default::default(), &Default::default(), &Default::default(), SessionSpawnScope::UNSCOPED,
            ambition_platformer2d_publication::construction::ActorConstructionContext::new(
                &Default::default(), &Default::default(),
                &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
                &Default::default(), binding,
            ).with_prepared(&Default::default()),
        ).unwrap();
        app.init_resource::<PromisedRoomPlans>();
        app.add_systems(crate::SaveWorld, collect_after_save.after(crate::SaveWorldSystems::Snapshot));
        (app, AuthorizedRoomPlan { plan: Arc::new(plan), preparation,
            source_room: 0 })
    }

    fn save(app: &mut App, frame: i32, confirmed: i32) {
        app.insert_resource(crate::RollbackFrameCount(frame));
        app.insert_resource(crate::ConfirmedFrameCount(confirmed));
        app.world_mut().run_schedule(crate::SaveWorld);
    }

    #[test]
    fn a_promise_survives_replay_and_sparse_saves_until_its_last_input_is_unreachable() {
        let (mut app, authorization) = prepared();
        let operation = authorization.operation().clone();
        let preparation = authorization.preparation.clone();
        let weak = Arc::downgrade(&authorization.plan);
        let verdict = crate::PeerVerdict {
            operation: crate::PeerOperation::from(&authorization.preparation),
            plan: authorization.peer_identity().unwrap(), said: crate::PreparationVerdict::Prepared,
        };
        app.world_mut().resource_mut::<PromisedRoomPlans>().retain(authorization);
        // This input can name a later frame because the session has input delay.
        app.world_mut().resource_mut::<PromisedRoomPlans>().advertised(verdict, 20);
        save(&mut app, 10, 0);
        app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
        save(&mut app, 11, 0);
        app.insert_resource(crate::RollbackFrameCount(10));
        app.world_mut().run_schedule(crate::LoadWorld);
        assert_eq!(app.world().resource::<PendingLifecycleCommit>().peek(), Some(&operation));
        let retained = app.world().resource::<PromisedRoomPlans>().get(&preparation).unwrap();
        assert!(Arc::ptr_eq(&weak.upgrade().unwrap(), &retained.plan));
        app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
        // GGRS can save a sparse frame below its confirmed boundary.
        save(&mut app, 12, 30);
        assert!(weak.upgrade().is_some());
        save(&mut app, 20, 30);
        assert!(weak.upgrade().is_some());
        save(&mut app, 21, 30);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn a_pending_promise_is_immutable_and_outlives_the_saved_floor() {
        let (mut app, authorization) = prepared();
        let weak = Arc::downgrade(&authorization.plan);
        let mut replacement = authorization.clone();
        replacement.plan = Arc::new((*authorization.plan).clone());
        let replacement_weak = Arc::downgrade(&replacement.plan);
        app.world_mut().resource_mut::<PromisedRoomPlans>().retain(authorization);
        app.world_mut().resource_mut::<PromisedRoomPlans>().retain(replacement);
        assert!(replacement_weak.upgrade().is_none());
        save(&mut app, 100, 100);
        assert!(weak.upgrade().is_some());
        app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
        save(&mut app, 101, 101);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn the_same_intent_on_a_corrected_branch_needs_its_own_preparation() {
        let (mut app, authorization) = prepared();
        let operation = authorization.operation().clone();
        let old_preparation = authorization.preparation.clone();
        let old_key = crate::PeerOperation::from(&old_preparation);
        app.world_mut().resource_mut::<PromisedRoomPlans>().retain(authorization);
        *ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<CapturedRoomPreparation>(app.world_mut()).unwrap()
            = CapturedRoomPreparation::default();
        let mut ledger = ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences::default();
        ledger.consume([ambition_platformer2d_shared_tangle::sim_id::SimId::placement("collected-on-the-corrected-branch")]);
        app.insert_resource(ledger);
        app.world_mut().run_system_once(capture_room_preparation_inputs).unwrap();
        let current = session_world_component::<CapturedRoomPreparation>(app.world()).unwrap()
            .for_operation(&operation).unwrap();
        let current_key = crate::PeerOperation::from(current);
        assert_eq!(old_preparation.admission(), current.admission());
        assert_ne!(old_key, current_key);
        assert!(app.world().resource::<PromisedRoomPlans>().get(current).is_none());
        assert!(app.world().resource::<PromisedRoomPlans>().get(&old_preparation).is_some());
    }

}
