//! Optional deterministic loading activity acceptance fixture.

use ambition_game_shell::{shell_action_edges, ShellAnalogLatch};
use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;

use crate::{
    LoadActivityOutcome, LoadActivityScopedEntity, LoadActivitySignal, LoadActivityState,
    LoadPresentationSet,
};

pub const DETERMINISTIC_LOADING_ACTIVITY_ID: &str = "ambition.loading.edge-practice";
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
        app.add_systems(
            Update,
            spawn_activity.in_set(LoadPresentationSet::Activity),
        )
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
            outcome
                .telemetry
                .insert("neutral_navigation_edges".to_owned(), view.edges.to_string());
            signals.write(LoadActivitySignal::Finished {
                activation_id: view.activation_id,
                outcome,
            });
        } else {
            **text = format!("Loading practice: tap up/down four times ({}/4)", view.edges);
        }
    }
}
