//! Bevy message adapter for the load coordinator.

use bevy::prelude::{
    App, IntoScheduleConfigs, Message, MessageReader, MessageWriter, Plugin, Update,
};

use crate::{
    AmbitionLoadSet, DiscoveryForecast, LoadBarrierId, LoadBarrierSpec, LoadCoordinator, LoadId,
    LoadPlanSpec, LoadPriority, LoadWorkId, LoadWorkSpec, LoadWorkState,
};

#[derive(Message, Clone, Debug, PartialEq)]
pub enum LoadCommand {
    Begin(LoadPlanSpec),
    DeclareBarrier {
        load_id: LoadId,
        spec: LoadBarrierSpec,
    },
    SetDiscovery {
        load_id: LoadId,
        barrier_id: LoadBarrierId,
        open: bool,
        forecast: Option<DiscoveryForecast>,
    },
    UpsertWork {
        load_id: LoadId,
        spec: LoadWorkSpec,
    },
    SetWorkState {
        load_id: LoadId,
        work_id: LoadWorkId,
        state: LoadWorkState,
    },
    RemoveWork {
        load_id: LoadId,
        work_id: LoadWorkId,
    },
    SetWorkPriority {
        load_id: LoadId,
        work_id: LoadWorkId,
        priority: LoadPriority,
    },
    PromoteWork {
        load_id: LoadId,
        work_id: LoadWorkId,
        barrier_id: LoadBarrierId,
    },
    Cancel {
        load_id: LoadId,
    },
    RequestCommit {
        load_id: LoadId,
        barrier_id: LoadBarrierId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadCommitRejection {
    UnknownBarrier,
    BarrierNotReady(crate::BarrierReadiness),
    AlreadyAuthorized,
}

#[derive(Message, Clone, Debug, PartialEq)]
pub enum LoadEvent {
    PlanChanged {
        load_id: LoadId,
    },
    PlanCancelled {
        load_id: LoadId,
    },
    PlanSuperseded {
        load_id: LoadId,
        replacement: LoadId,
    },
    CommitAuthorized {
        load_id: LoadId,
        barrier_id: LoadBarrierId,
    },
    CommitRejected {
        load_id: LoadId,
        barrier_id: LoadBarrierId,
        reason: LoadCommitRejection,
    },
}

#[derive(Default)]
pub struct AmbitionLoadPlugin;

impl Plugin for AmbitionLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadCoordinator>()
            .add_message::<LoadCommand>()
            .add_message::<LoadEvent>()
            .configure_sets(
                Update,
                (AmbitionLoadSet::Contributors, AmbitionLoadSet::Commands).chain(),
            )
            .add_systems(
                Update,
                apply_load_commands.in_set(AmbitionLoadSet::Commands),
            );
    }
}

fn apply_load_commands(
    mut commands: MessageReader<LoadCommand>,
    mut coordinator: bevy::prelude::ResMut<LoadCoordinator>,
    mut events: MessageWriter<LoadEvent>,
) {
    for command in commands.read() {
        for event in coordinator.apply(command.clone()) {
            events.write(event);
        }
    }
}
