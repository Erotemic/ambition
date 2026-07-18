//! Optional deterministic loading activity acceptance fixture.

use ambition_game_shell::{
    shell_action_edges, FrontendOwnedEntity, FrontendPresentationKind, ShellAnalogLatch,
};
use bevy::input::gamepad::Gamepad;
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
            .add_systems(Update, drive_activity.in_set(LoadPresentationSet::Input));
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
        DeterministicActivityView {
            activation_id: active.activation_id,
            edges: 0,
            engaged: false,
        },
        LoadActivityScopedEntity {
            activation_id: active.activation_id,
        },
        FrontendOwnedEntity::load(
            active.barrier.load_id.clone(),
            FrontendPresentationKind::LoadingActivity,
        ),
        Text::new("Loading practice: tap up/down four times (0/4)"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.78, 0.84, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(12.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(1001),
        Name::new("deterministic loading activity"),
    ));
}

fn drive_activity(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<&Gamepad>,
    activity: Res<LoadActivityState>,
    mut views: Query<(&mut DeterministicActivityView, &mut Text)>,
    mut signals: MessageWriter<LoadActivitySignal>,
    mut analog: Local<ShellAnalogLatch>,
) {
    let Some(active) = activity.active.as_ref() else {
        return;
    };
    if active.activity_id.as_str() != DETERMINISTIC_LOADING_ACTIVITY_ID {
        return;
    }
    let actions = shell_action_edges(keys.as_deref(), &pads, &mut analog);
    if !actions.previous && !actions.next {
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
    use ambition_load::{LoadBarrierId, LoadBarrierRef, LoadId};
    use bevy::input::gamepad::{Gamepad, GamepadButton};

    #[derive(Resource, Default, Debug, Eq, PartialEq)]
    struct DestinationFixture(u64);

    fn app() -> App {
        let mut app = App::new();
        app.add_message::<LoadActivitySignal>();
        app.init_resource::<LoadActivityState>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<DestinationFixture>();
        app.add_plugins(DeterministicLoadingActivityPlugin);
        app.world_mut().resource_mut::<LoadActivityState>().active = Some(ActiveLoadActivity {
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

    fn tap_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        // `clear` only clears the transient edge sets; it deliberately leaves
        // the key in `pressed`. Reset the key so a later tap of the same key
        // creates a fresh `just_pressed` edge.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(key);
    }

    fn drain(app: &mut App) -> Vec<LoadActivitySignal> {
        app.world_mut()
            .resource_mut::<Messages<LoadActivitySignal>>()
            .drain()
            .collect()
    }

    #[test]
    fn keyboard_activity_is_scoped_records_an_optional_result_and_never_mutates_destination() {
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
        for key in [
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ] {
            tap_key(&mut app, key);
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

    #[test]
    fn controller_dpad_drives_the_same_activity_actions() {
        let mut app = app();
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        let mut signals = Vec::new();
        for button in [
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
        ] {
            {
                let mut entity = app.world_mut().entity_mut(pad);
                entity
                    .get_mut::<Gamepad>()
                    .expect("fixture gamepad")
                    .digital_mut()
                    .press(button);
            }
            app.update();
            signals.extend(drain(&mut app));
            // As with keyboard `ButtonInput`, `clear` would leave the
            // D-pad button held. Reset this button so the repeated direction
            // below produces another edge.
            app.world_mut()
                .entity_mut(pad)
                .get_mut::<Gamepad>()
                .expect("fixture gamepad")
                .digital_mut()
                .reset(button);
        }
        assert!(signals.iter().any(|signal| matches!(
            signal,
            LoadActivitySignal::Finished { activation_id: 7, outcome }
                if outcome.completed && outcome.score == Some(4)
        )));
        assert_eq!(
            *app.world().resource::<DestinationFixture>(),
            DestinationFixture(0)
        );
    }
}
