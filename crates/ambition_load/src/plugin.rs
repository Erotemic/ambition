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

/// Marker proving this plugin's `build` has already run.
///
/// Bevy's own duplicate-plugin check cannot serve here: it panics, and the panic
/// is the problem. See [`AmbitionLoadPlugin`] for why.
#[derive(bevy::prelude::Resource)]
struct AmbitionLoadInstalled;

impl Plugin for AmbitionLoadPlugin {
    /// Adding this twice is a no-op, not a crash.
    ///
    /// The room-transition transaction IS a load plan, so the ENGINE group needs
    /// this plugin; a shell host also needs it, and `MinimalLoadShellPlugins`
    /// adds it as a group member, which a `PluginGroupBuilder` cannot make
    /// conditional. Whether an app crashed therefore depended on which groups it
    /// composed and in what order — an unwritten rule enforced by a hard panic.
    ///
    /// Duplicate composition must not depend on plugin-group ordering or fail with
    /// Bevy's duplicate-plugin panic.
    ///
    /// So the plugin is not unique and `build` is idempotent. The guard is a
    /// marker resource rather than `is_plugin_added::<Self>()`, because Bevy has
    /// already registered the name by the time `build` runs.
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<AmbitionLoadInstalled>() {
            return;
        }
        app.insert_resource(AmbitionLoadInstalled);
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

#[cfg(test)]
mod composition_tests {
    use super::*;

    /// Adding the load coordinator twice, in either order, must not crash.
    ///
    /// This is the composition hazard the Phase-6 external fixture hit: the engine group needs the
    /// coordinator because a room transition IS a load plan, and a shell host needs it too, and
    /// `MinimalLoadShellPlugins` adds it as a group member — which a `PluginGroupBuilder` cannot
    /// make conditional.
    ///
    /// The external consumer, invisible to a repo grep, sat red until somebody read the panic.
    /// An engine a stranger composes cannot have rules discoverable only by crashing, so this
    /// is the test that says the rule no longer exists.
    #[test]
    fn adding_the_load_coordinator_twice_is_a_no_op() {
        let mut app = App::new();
        app.add_plugins(AmbitionLoadPlugin);
        app.add_plugins(AmbitionLoadPlugin);
        app.add_plugins(AmbitionLoadPlugin);
        // It is installed and usable, not merely non-crashing.
        assert!(app.world().contains_resource::<LoadCoordinator>());
        app.update();
    }

    /// The second add must not re-register the message channels either. A
    /// duplicate `add_message` installs a SECOND update system for the same
    /// channel, which drains it twice as fast — messages a reader has not seen
    /// yet vanish, and nothing reports it. Idempotence has to mean "did nothing",
    /// not "did not panic".
    #[test]
    fn the_second_add_installs_nothing() {
        let mut once = App::new();
        once.add_plugins(AmbitionLoadPlugin);
        let systems_after_one = once
            .get_schedule(Update)
            .map(|schedule| schedule.systems_len())
            .unwrap_or(0);

        let mut twice = App::new();
        twice.add_plugins(AmbitionLoadPlugin);
        twice.add_plugins(AmbitionLoadPlugin);
        let systems_after_two = twice
            .get_schedule(Update)
            .map(|schedule| schedule.systems_len())
            .unwrap_or(0);

        assert_eq!(
            systems_after_one, systems_after_two,
            "the second add installed systems; a duplicated message-update system \
             drains a channel twice per frame and silently eats unread messages"
        );
    }
}
