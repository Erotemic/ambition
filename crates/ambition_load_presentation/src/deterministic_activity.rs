//! Optional deterministic loading activity acceptance fixture.

use ambition_game_shell::{FrontendOwnedEntity, FrontendPresentationKind};
use bevy::prelude::*;

use crate::{
    LoadActivityOutcome, LoadActivityScopedEntity, LoadActivitySignal, LoadActivityState,
    LoadPresentationSet, DETERMINISTIC_LOADING_ACTIVITY_ID,
};

const TARGET_EDGES: u64 = 4;

#[derive(Component)]
struct DeterministicActivityView {
    activation_id: u64,
    edges: u64,
    engaged: bool,
}

#[derive(Default)]
pub struct DeterministicLoadingActivityPlugin;

impl Plugin for DeterministicLoadingActivityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_activity.in_set(LoadPresentationSet::Activity))
            .add_systems(Update, drive_activity.in_set(ambition_input::InputSet::Consume));
    }
}

fn spawn_activity(
    mut commands: Commands,
    activity: Res<LoadActivityState>,
    existing: Query<&DeterministicActivityView>,
) {
    let Some(active) = activity.active.as_ref() else {
        return;
    };
    if active.activity_id.as_str() != DETERMINISTIC_LOADING_ACTIVITY_ID
        || existing
            .iter()
            .any(|view| view.activation_id == active.activation_id)
    {
        return;
    }
    commands.spawn((
        LoadActivityScopedEntity { activation_id: active.activation_id },
        FrontendOwnedEntity::load(active.barrier.load_id.clone(), FrontendPresentationKind::LoadingActivity),
        Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0),
            overflow: Overflow::clip(), ..default() },
        GlobalZIndex(1001),
        Name::new("deterministic loading activity"),
    )).with_children(|children| {
        children.spawn((
            DeterministicActivityView { activation_id: active.activation_id, edges: 0, engaged: false },
            Text::new("Loading practice: tap up/down four times (0/4)"),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::srgb(0.78, 0.84, 1.0)),
            Node { position_type: PositionType::Absolute, bottom: Val::Percent(12.0),
                width: Val::Percent(100.0), justify_content: JustifyContent::Center, ..default() },
        ));
    });
}

fn drive_activity(
    input: crate::input::LoadInput,
    activity: Res<LoadActivityState>,
    mut views: Query<(&mut DeterministicActivityView, &mut Text)>,
    mut signals: MessageWriter<LoadActivitySignal>,
) {
    let Some(active) = activity.active.as_ref() else {
        return;
    };
    if active.activity_id.as_str() != DETERMINISTIC_LOADING_ACTIVITY_ID {
        return;
    }
    let actions = input.read(active.input_owner);
    if !actions.up && !actions.down {
        return;
    }

    for (mut view, mut text) in &mut views {
        if view.activation_id != active.activation_id || view.edges >= TARGET_EDGES {
            continue;
        }
        if !view.engaged {
            view.engaged = true;
            signals.write(LoadActivitySignal::Engaged {
                activation_id: view.activation_id,
            });
        }
        view.edges = view.edges.saturating_add(1);
        if view.edges >= TARGET_EDGES {
            **text = "Loading practice complete — press confirm when ready".to_owned();
            let mut outcome = LoadActivityOutcome {
                score: Some(view.edges),
                completed: true,
                ..default()
            };
            outcome.telemetry.insert(
                "neutral_navigation_edges".to_owned(),
                view.edges.to_string(),
            );
            signals.write(LoadActivitySignal::Finished {
                activation_id: view.activation_id,
                outcome,
            });
        } else {
            **text = format!(
                "Loading practice: tap up/down four times ({}/4)",
                view.edges
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LoadPresentationOwnerId;
    use crate::{ActiveLoadActivity, LoadActivityId};
    use ambition_game_shell::{FrontendEntityOwner, FrontendPresentationKind};
    use ambition_input::MenuControlFrame;
    use ambition_load::{LoadBarrierId, LoadBarrierRef, LoadId};

    #[derive(Resource, Default, Debug, Eq, PartialEq)]
    struct DestinationFixture(u64);

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<LoadActivitySignal>();
        app.init_resource::<LoadActivityState>();
        app.init_resource::<MenuControlFrame>();
        app.init_resource::<DestinationFixture>();
        app.add_plugins(DeterministicLoadingActivityPlugin);
        app.world_mut().resource_mut::<LoadActivityState>().active = Some(ActiveLoadActivity {
            input_owner: Some(ambition_input::InputOwner::AllParticipants),
            activation_id: 7,
            activity_id: LoadActivityId::new(DETERMINISTIC_LOADING_ACTIVITY_ID),
            owner: LoadPresentationOwnerId::new("fixture-owner"),
            barrier: LoadBarrierRef {
                load_id: LoadId::new("load-7"),
                barrier_id: LoadBarrierId::new("ready"),
            },
        });
        app
    }

    /// One semantic nav edge for one frame — the shape keyboard arrows, the
    /// D-pad, and the touch stick all reduce to upstream of the shell.
    fn nav(app: &mut App, up: bool) {
        {
            let mut frame = app.world_mut().resource_mut::<MenuControlFrame>();
            *frame = MenuControlFrame {
                up,
                down: !up,
                ..Default::default()
            };
        }
        app.update();
        *app.world_mut().resource_mut::<MenuControlFrame>() = MenuControlFrame::default();
    }

    fn drain(app: &mut App) -> Vec<LoadActivitySignal> {
        app.world_mut()
            .resource_mut::<Messages<LoadActivitySignal>>()
            .drain()
            .collect()
    }

    #[test]
    fn activity_navigation_follows_the_loading_input_owner() {
        let mut app = app();
        app.init_resource::<ambition_input::SeatMenuFrames>();
        app.world_mut().resource_mut::<LoadActivityState>().active.as_mut().unwrap().input_owner =
            Some(ambition_input::InputOwner::Participant(ambition_input::ParticipantId::SECONDARY));
        app.update();
        nav(&mut app, true);
        assert!(drain(&mut app).is_empty());
        app.world_mut().resource_mut::<ambition_input::SeatMenuFrames>().set(1,
            ambition_input::MenuControlFrame { down: true, ..default() });
        app.update();
        assert!(matches!(drain(&mut app).as_slice(), [LoadActivitySignal::Engaged { activation_id: 7 }]));
    }

    #[test]
    fn semantic_nav_is_scoped_records_an_optional_result_and_never_mutates_destination() {
        let mut app = app();
        app.update();
        let mut scoped = app.world_mut().query::<&LoadActivityScopedEntity>();
        assert_eq!(scoped.iter(app.world()).count(), 1);
        let mut owned = app.world_mut().query::<&FrontendOwnedEntity>();
        let owner = owned
            .iter(app.world())
            .find(|owned| owned.kind == FrontendPresentationKind::LoadingActivity)
            .expect("activity has explicit load ownership");
        assert_eq!(
            owner.owner,
            FrontendEntityOwner::Load(LoadId::new("load-7")),
        );

        let mut signals = Vec::new();
        for up in [true, false, true, false] {
            nav(&mut app, up);
            signals.extend(drain(&mut app));
        }
        assert!(matches!(
            signals.first(),
            Some(LoadActivitySignal::Engaged { activation_id: 7 })
        ));
        assert!(matches!(
            signals.last(),
            Some(LoadActivitySignal::Finished { activation_id: 7, outcome })
                if outcome.completed && outcome.score == Some(4)
        ));
        assert_eq!(
            *app.world().resource::<DestinationFixture>(),
            DestinationFixture(0)
        );
    }
}
