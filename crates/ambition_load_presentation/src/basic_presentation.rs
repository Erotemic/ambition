//! Show load status and the controls needed to continue or recover.

use ambition_game_shell::{FrontendOwnedEntity, FrontendPresentationKind};
use ambition_platformer2d_shared_tangle::developer_hotkeys::{
    DeveloperAction, DeveloperHotkeyBindings,
};
use bevy::prelude::*;

use crate::{
    LoadForegroundPhase, LoadPresentations, LoadPresentationAction, LoadPresentationModel, LoadPresentationOwnerId,
    LoadPresentationSet,
};

#[derive(Component)]
pub struct BasicLoadRoot;

#[derive(Component)]
struct BasicLoadOwner(LoadPresentationOwnerId);

/// The loading screen's sign of life: a glyph that turns while the game
/// works, so a slow machine is never mistaken for a frozen one (Jon,
/// 2026-10-04: a loading screen on fast hardware never appears; on a slower
/// machine it lets the player know the game is still working).
#[derive(Component)]
pub struct BasicLoadSpinner;

/// The spinner's frames: ASCII, so every font draws them.
const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

/// The spinner frame `seconds` into the load, turning eight times a second.
pub fn spinner_frame(seconds: f32) -> &'static str {
    SPINNER_FRAMES[(seconds.max(0.0) * 8.0) as usize % SPINNER_FRAMES.len()]
}

#[derive(Default)]
pub struct BasicLoadPresentationPlugin;

impl Plugin for BasicLoadPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeveloperAction>()
            .add_systems(
                Update,
                basic_load_keyboard.in_set(ambition_input::InputSet::Consume),
            )
            .add_systems(
                Update,
                (render_basic_load, turn_basic_load_spinner)
                    .chain()
                    .in_set(LoadPresentationSet::Render),
            );
    }
}

fn basic_load_keyboard(
    input: crate::input::LoadInput,
    presentations: Res<LoadPresentations>,
    mut developer_actions: MessageReader<DeveloperAction>,
    mut actions: MessageWriter<LoadPresentationAction>,
) {
    let quit_to_home = developer_actions.read().any(|action| *action == DeveloperAction::QuitToHome);
    for entry in presentations.iter() {
        let active = &entry.foreground;
        let controls = input.read(active.spec.input_owner);
        if !controls.back && controls.select {
            let action = if active.phase == LoadForegroundPhase::ReadyHold {
                Some(LoadPresentationAction::Continue { owner: active.owner.clone() })
            } else if active.phase == LoadForegroundPhase::Failed && entry.model.failures.iter().any(|failure| failure.retryable) {
                Some(LoadPresentationAction::Retry { owner: active.owner.clone() })
            } else { None };
            if let Some(action) = action { actions.write(action); }
        }
        if quit_to_home {
            actions.write(LoadPresentationAction::Quit { owner: active.owner.clone() });
        } else if controls.back {
            actions.write(LoadPresentationAction::Cancel { owner: active.owner.clone() });
        }
    }
}

fn render_basic_load(
    mut commands: Commands,
    presentations: Res<LoadPresentations>,
    hotkeys: Option<Res<DeveloperHotkeyBindings>>,
    roots: Query<(Entity, &BasicLoadOwner), With<BasicLoadRoot>>,
    mut prior: Local<std::collections::BTreeMap<LoadPresentationOwnerId, String>>,
) {
    let quit_label = hotkeys.as_deref()
        .and_then(|bindings| bindings.label_for(DeveloperAction::QuitToHome))
        .unwrap_or_else(|| "Quit to home".to_owned());
    for (entity, owner) in &roots {
        if presentations.get(&owner.0).is_none() { commands.entity(entity).despawn(); }
    }
    prior.retain(|owner, _| presentations.get(owner).is_some());
    for entry in presentations.iter() {
        let active = &entry.foreground;
        let text = format_model(&entry.model, &quit_label);
        let key = format!("{}:{text}", active.barrier.load_id.as_str());
        if prior.get(&active.owner) == Some(&key) { continue; }
        prior.insert(active.owner.clone(), key);
        for (entity, owner) in &roots {
            if owner.0 == active.owner { commands.entity(entity).despawn(); }
        }
        if text.is_empty() { continue; }
    commands
        .spawn((
            BasicLoadRoot,
            BasicLoadOwner(active.owner.clone()),
            ambition_platformer2d_shared_tangle::gameplay_presentation::PresentationCover,
            FrontendOwnedEntity::load(
                active.barrier.load_id.clone(),
                FrontendPresentationKind::LoadingRoot,
            ),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.015, 0.02, 0.035)),
            GlobalZIndex(1000),
            Name::new("basic load presentation"),
        ))
        .with_children(|root| {
            root.spawn((
                BasicLoadSpinner,
                Text::new(spinner_frame(0.0)),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.95, 1.0)),
            ));
            root.spawn((
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.95, 1.0)),
                TextLayout::justify(Justify::Center),
            ));
        });
    }
}

/// Turn the spinner on real time, which advances while the game waits on
/// work (the simulation clock may be paused behind the screen).
fn turn_basic_load_spinner(time: Option<Res<Time<Real>>>, mut spinners: Query<&mut Text, With<BasicLoadSpinner>>) {
    let Some(time) = time else {
        return;
    };
    let frame = spinner_frame(time.elapsed_secs());
    for mut text in &mut spinners {
        if text.0 != frame {
            text.0 = frame.to_owned();
        }
    }
}

fn format_model(model: &LoadPresentationModel, quit_label: &str) -> String {
    if !model.visible {
        return String::new();
    }
    if let Some(failure) = model.failures.first() {
        let controls = if failure.retryable {
            format!("Confirm: retry · Back: cancel · {quit_label}: quit")
        } else {
            format!("Back: cancel · {quit_label}: quit")
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
    if model.ready_hold {
        "Ready to play\n\nPress Enter to continue".to_owned()
    } else {
        "Loading...\n\nPlease wait".to_owned()
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use ambition_input::participant::*;
    use ambition_input::{MenuControlFrame, SeatMenuFrames};

    #[test]
    fn recovery_and_continuation_ignore_another_participants_controls() {
        for phase in [LoadForegroundPhase::ReadyHold, LoadForegroundPhase::Failed] {
            let mut app = App::new();
            app.add_message::<DeveloperAction>();
            app.add_message::<LoadPresentationAction>();
            app.init_resource::<MenuControlFrame>();
            app.init_resource::<SeatMenuFrames>();
            app.init_resource::<SeatInputContexts>();
            app.init_resource::<LoadPresentations>();
            let owner = crate::LoadPresentationOwnerId::new("fixture");
            let mut spec = crate::LoadExperienceSpec::basic("fixture");
            spec.input_owner = Some(InputOwner::Participant(ParticipantId::SECONDARY));
            app.world_mut().resource_mut::<LoadPresentations>().begin(crate::ActiveLoadForeground {
                owner: owner.clone(), barrier: ambition_load::LoadBarrierRef::new("fixture", "ready"), spec,
                phase, elapsed: std::time::Duration::ZERO,
                activity_activation_id: None, engaged: false, ready_released: false,
            }).model.failures.push(ambition_load::LoadFailure::new("fixture", "fixture").retryable(true));
            for id in [ParticipantId::PRIMARY, ParticipantId::SECONDARY] {
                let mut contexts = ParticipantContexts::default();
                contexts.declare(ContextClaim::capturing(if id == ParticipantId::SECONDARY { LOADING_CONTEXT } else { GAMEPLAY_CONTEXT },
                    context_priority::LOADING));
                app.world_mut().spawn((InputParticipant::with_id(id), contexts));
            }
            app.add_systems(Update, (resolve_active_input_context, basic_load_keyboard).chain());
            app.world_mut().resource_mut::<MenuControlFrame>().select = true;
            app.world_mut().resource_mut::<SeatMenuFrames>().set(0, MenuControlFrame { select: true, back: true, ..default() });
            app.update();
            assert!(app.world().resource::<Messages<LoadPresentationAction>>().is_empty());
            app.world_mut().resource_mut::<SeatMenuFrames>().set(1, MenuControlFrame { select: true, ..default() });
            app.update();
            let actions: Vec<_> = app.world_mut().resource_mut::<Messages<LoadPresentationAction>>().drain().collect();
            assert_eq!(actions, vec![if phase == LoadForegroundPhase::ReadyHold {
                LoadPresentationAction::Continue { owner: owner.clone() }
            } else { LoadPresentationAction::Retry { owner: owner.clone() } }]);
            app.world_mut().resource_mut::<SeatMenuFrames>().set(1, MenuControlFrame { select: true, back: true, ..default() });
            app.update();
            assert_eq!(app.world_mut().resource_mut::<Messages<LoadPresentationAction>>().drain().collect::<Vec<_>>(),
                vec![LoadPresentationAction::Cancel { owner }]);
        }
    }
}
