//! Plain Bevy UI reference presentation for load evidence and ready-hold.

use ambition_game_shell::{shell_action_edges, FrontendOwnedEntity, FrontendPresentationKind};
use ambition_platformer_primitives::developer_hotkeys::{DeveloperAction, DeveloperHotkeyBindings};
use bevy::prelude::*;

use crate::{
    LoadForegroundPhase, LoadForegroundState, LoadPresentationAction, LoadPresentationModel,
    LoadPresentationSet,
};

#[derive(Component)]
pub struct BasicLoadRoot;

#[derive(Default)]
pub struct BasicLoadPresentationPlugin;

impl Plugin for BasicLoadPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeveloperAction>()
            .add_systems(
                Update,
                basic_load_keyboard
                    .in_set(LoadPresentationSet::Input)
                    // A consumer of the routed input semantics: after every
                    // producer (participant populate, touch folds), same frame.
                    .in_set(ambition_input::InputSet::Consume),
            )
            .add_systems(
                Update,
                render_basic_load.in_set(LoadPresentationSet::Render),
            );
    }
}

fn basic_load_keyboard(
    // Retry is the loading context's one remaining raw-device read (R / pad
    // West): the loading surface is not part of the migrated startup/launcher
    // contexts yet and no semantic retry intent exists. Everything else below
    // consumes the participant-populated `MenuControlFrame`.
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<&bevy::input::gamepad::Gamepad>,
    menu_frame: Option<Res<ambition_input::MenuControlFrame>>,
    foreground: Res<LoadForegroundState>,
    model: Res<LoadPresentationModel>,
    mut developer_actions: MessageReader<DeveloperAction>,
    mut actions: MessageWriter<LoadPresentationAction>,
) {
    let Some(active) = foreground.active.as_ref() else {
        return;
    };
    let shell_actions = shell_action_edges(menu_frame.as_deref());
    if active.phase == LoadForegroundPhase::ReadyHold && shell_actions.loading_continue {
        actions.write(LoadPresentationAction::Continue {
            owner: active.owner.clone(),
        });
    }
    let retry_pressed = keys
        .as_deref()
        .is_some_and(|input| input.just_pressed(KeyCode::KeyR))
        || pads
            .iter()
            .any(|gamepad| gamepad.just_pressed(bevy::input::gamepad::GamepadButton::West));
    if active.phase == LoadForegroundPhase::Failed
        && model.failures.iter().any(|failure| failure.retryable)
        && retry_pressed
    {
        actions.write(LoadPresentationAction::Retry {
            owner: active.owner.clone(),
        });
    }
    let quit_to_home = developer_actions
        .read()
        .any(|action| *action == DeveloperAction::QuitToHome);
    if quit_to_home {
        actions.write(LoadPresentationAction::Quit {
            owner: active.owner.clone(),
        });
    } else if shell_actions.back {
        actions.write(LoadPresentationAction::Cancel {
            owner: active.owner.clone(),
        });
    }
}

fn render_basic_load(
    mut commands: Commands,
    model: Res<LoadPresentationModel>,
    foreground: Res<LoadForegroundState>,
    hotkeys: Option<Res<DeveloperHotkeyBindings>>,
    roots: Query<Entity, With<BasicLoadRoot>>,
    mut prior: Local<String>,
) {
    let quit_label = hotkeys
        .as_deref()
        .and_then(|bindings| bindings.label_for(DeveloperAction::QuitToHome))
        .unwrap_or_else(|| "Quit to home".to_owned());
    let text = format_model(&model, &quit_label);
    let key = format!(
        "{}:{text}",
        foreground
            .active
            .as_ref()
            .map(|active| active.barrier.load_id.as_str())
            .unwrap_or("none")
    );
    if *prior == key {
        return;
    }
    *prior = key;
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    if text.is_empty() {
        return;
    }
    let active = foreground
        .active
        .as_ref()
        .expect("visible load presentation has an exact active foreground");
    commands
        .spawn((
            BasicLoadRoot,
            FrontendOwnedEntity::load(
                active.barrier.load_id.clone(),
                FrontendPresentationKind::LoadingRoot,
            ),
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

fn format_model(model: &LoadPresentationModel, quit_label: &str) -> String {
    if !model.visible {
        return String::new();
    }
    if let Some(failure) = model.failures.first() {
        let controls = if failure.retryable {
            format!("R: retry · Escape: cancel · {quit_label}: quit")
        } else {
            format!("Escape: cancel · {quit_label}: quit")
        };
        return format!("Load failed\n\n{}\n\n{controls}", failure.player_message);
    }
    match model.readiness {
        Some(ambition_load::BarrierReadiness::Failed) => {
            return "Load failed\n\nEscape: cancel".to_owned();
        }
        Some(ambition_load::BarrierReadiness::Cancelled) => {
            return "Load cancelled\n\nEscape: cancel".to_owned();
        }
        Some(ambition_load::BarrierReadiness::Superseded) => {
            return "Load replaced by a newer request\n\nEscape: cancel".to_owned();
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
    append_work_section(&mut lines, "Working now", &model.active_labels);
    append_work_section(&mut lines, "Completed", &model.completed_labels);
    append_work_section(&mut lines, "Required next", &model.remaining_labels);
    append_work_section(
        &mut lines,
        "Streaming after launch",
        &model.streamable_labels,
    );
    append_work_section(
        &mut lines,
        "Optional background work",
        &model.speculative_labels,
    );
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

fn append_work_section(lines: &mut Vec<String>, heading: &str, labels: &[String]) {
    if labels.is_empty() {
        return;
    }
    lines.push(format!("{heading}:\n{}", labels.join("\n")));
}
