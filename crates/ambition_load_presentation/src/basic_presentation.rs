//! Plain Bevy UI reference presentation for load evidence and ready-hold.

use bevy::prelude::*;

use crate::{
    LoadForegroundPhase, LoadForegroundState, LoadPresentationAction, LoadPresentationModel,
    LoadPresentationSet,
};

#[derive(Component)]
struct BasicLoadRoot;

#[derive(Default)]
pub struct BasicLoadPresentationPlugin;

impl Plugin for BasicLoadPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            basic_load_keyboard.in_set(LoadPresentationSet::Input),
        )
        .add_systems(
            Update,
            render_basic_load.in_set(LoadPresentationSet::Render),
        );
    }
}

fn basic_load_keyboard(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    foreground: Res<LoadForegroundState>,
    model: Res<LoadPresentationModel>,
    mut actions: MessageWriter<LoadPresentationAction>,
) {
    let Some(keys) = keys else {
        return;
    };
    let Some(active) = foreground.active.as_ref() else {
        return;
    };
    if active.phase == LoadForegroundPhase::ReadyHold
        && (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
    {
        actions.write(LoadPresentationAction::Continue);
    }
    if active.phase == LoadForegroundPhase::Failed
        && model.failures.iter().any(|failure| failure.retryable)
        && keys.just_pressed(KeyCode::KeyR)
    {
        actions.write(LoadPresentationAction::Retry);
    }
    if keys.just_pressed(KeyCode::Escape) {
        actions.write(LoadPresentationAction::CancelToPrevious);
    }
}

fn render_basic_load(
    mut commands: Commands,
    model: Res<LoadPresentationModel>,
    roots: Query<Entity, With<BasicLoadRoot>>,
    mut prior: Local<String>,
) {
    let text = format_model(&model);
    if *prior == text {
        return;
    }
    *prior = text.clone();
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    if text.is_empty() {
        return;
    }
    commands
        .spawn((
            BasicLoadRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.02, 0.035, 0.96)),
            GlobalZIndex(1000),
            Name::new("basic load presentation"),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(text),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.95, 1.0)),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}

fn format_model(model: &LoadPresentationModel) -> String {
    if !model.visible {
        return String::new();
    }
    if let Some(failure) = model.failures.first() {
        let controls = if failure.retryable {
            "R: retry · Escape: return"
        } else {
            "Escape: return"
        };
        return format!("Load failed\n\n{}\n\n{controls}", failure.player_message);
    }
    match model.readiness {
        Some(ambition_load::BarrierReadiness::Failed) => {
            return "Load failed\n\nEscape: return".to_owned();
        }
        Some(ambition_load::BarrierReadiness::Cancelled) => {
            return "Load cancelled\n\nEscape: return".to_owned();
        }
        Some(ambition_load::BarrierReadiness::Superseded) => {
            return "Load replaced by a newer request\n\nEscape: return".to_owned();
        }
        Some(
            ambition_load::BarrierReadiness::Preparing | ambition_load::BarrierReadiness::Ready,
        )
        | None => {}
    }
    let mut lines = vec![model.stage.clone()];
    if let Some(estimate) = &model.estimate {
        let percent = (estimate.fraction * 100.0).floor() as u32;
        lines.push(format!("About {percent}%"));
    }
    lines.push(format!(
        "{} complete · {} active · {} known remaining",
        model.completed_steps, model.active_steps, model.known_remaining_steps
    ));
    if let Some(additional) = &model.estimated_additional_steps {
        lines.push(format!(
            "Approximately {}–{} additional steps may be discovered",
            additional.start(),
            additional.end()
        ));
    } else if model.discovery_open {
        lines.push("More work may still be discovered".to_owned());
    }
    if model.ready_hold {
        lines.push("Ready — press Enter to continue".to_owned());
    }
    lines.join("\n\n")
}
