//! Plain Bevy UI reference presentation for launchers and shell sequences.
//!
//! Launcher content is translated into `ambition_menu`'s renderer-independent
//! page model and drawn by its flat Bevy-UI renderer. The shell keeps only the
//! host-relative route catalog and cursor; it does not introduce a competing
//! menu content or rendering model.

use ambition_menu::render::bevy_ui::{
    spawn_bevy_ui_menu_with_assets, BevyUiMenuRoot, BevyUiMenuTabSpec, BevyUiMenuView,
};
use ambition_menu::{MenuColor, MenuControlKind, MenuPageModel, MenuRect, MenuTextAlign};
use bevy::prelude::*;

use crate::{
    ActiveShellSequence, ShellLaunchCatalog, ShellLauncherCommand, ShellLauncherPresentation,
    ShellLauncherState, ShellRouteId, ShellSegmentPresentation, ShellSequenceCommand,
};

#[derive(Component)]
struct BasicSequenceRoot;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BasicLauncherPage {
    Home,
}

#[derive(Default)]
pub struct BasicShellPresentationPlugin;

impl Plugin for BasicShellPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (basic_shell_keyboard, render_basic_shell).chain());
    }
}

fn basic_shell_keyboard(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    launcher: Res<ShellLauncherState>,
    sequence: Res<ActiveShellSequence>,
    mut launcher_commands: MessageWriter<ShellLauncherCommand>,
    mut sequence_commands: MessageWriter<ShellSequenceCommand>,
) {
    let Some(keys) = keys else {
        return;
    };
    if launcher.active {
        if keys.just_pressed(KeyCode::ArrowUp) {
            launcher_commands.write(ShellLauncherCommand::Previous);
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            launcher_commands.write(ShellLauncherCommand::Next);
        }
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
            launcher_commands.write(ShellLauncherCommand::LaunchSelected);
        }
    } else if let (Some(activation_id), Some(runtime)) =
        (sequence.activation_id, sequence.runtime.as_ref())
    {
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
            if runtime
                .current()
                .is_some_and(|segment| segment.policy.requires_acknowledgement)
            {
                sequence_commands.write(ShellSequenceCommand::Acknowledge { activation_id });
            } else {
                sequence_commands.write(ShellSequenceCommand::Skip { activation_id });
            }
        }
    }
}

#[derive(Default)]
struct BasicSequenceFrame {
    key: String,
    text: String,
    image_path: Option<String>,
}

fn render_basic_shell(
    mut commands: Commands,
    launcher: Res<ShellLauncherState>,
    catalog: Res<ShellLaunchCatalog>,
    launcher_presentation: Res<ShellLauncherPresentation>,
    sequence: Res<ActiveShellSequence>,
    asset_server: Option<Res<AssetServer>>,
    sequence_roots: Query<Entity, With<BasicSequenceRoot>>,
    launcher_roots: Query<Entity, With<BevyUiMenuRoot>>,
    mut prior_key: Local<String>,
) {
    let frame_key = shell_frame_key(&launcher, &catalog, &launcher_presentation, &sequence);
    if *prior_key == frame_key {
        return;
    }
    *prior_key = frame_key;

    for entity in &sequence_roots {
        commands.entity(entity).despawn();
    }
    for entity in &launcher_roots {
        commands.entity(entity).despawn();
    }

    if launcher.active {
        spawn_launcher_menu(
            &mut commands,
            &launcher,
            &catalog,
            &launcher_presentation,
            asset_server.as_deref(),
        );
        return;
    }

    let frame = sequence_frame(&sequence);
    if frame.text.is_empty() && frame.image_path.is_none() {
        return;
    }
    commands
        .spawn((
            BasicSequenceRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(24.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.025, 0.03, 0.05)),
            GlobalZIndex(900),
            Name::new("basic shell sequence presentation"),
        ))
        .with_children(|root| {
            if let Some(handle) = frame
                .image_path
                .as_ref()
                .zip(asset_server.as_deref())
                .map(|(path, server)| server.load::<Image>(path.clone()))
            {
                root.spawn((
                    ImageNode::new(handle),
                    Node {
                        width: Val::Percent(70.0),
                        height: Val::Percent(60.0),
                        ..default()
                    },
                    Name::new("basic shell sequence image"),
                ));
            }
            if !frame.text.is_empty() {
                root.spawn((
                    Text::new(frame.text),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.94, 1.0)),
                    TextLayout::new_with_justify(Justify::Center),
                ));
            }
        });
}

fn spawn_launcher_menu(
    commands: &mut Commands,
    launcher: &ShellLauncherState,
    catalog: &ShellLaunchCatalog,
    presentation: &ShellLauncherPresentation,
    asset_server: Option<&AssetServer>,
) {
    let mut page = MenuPageModel::new(
        BasicLauncherPage::Home,
        presentation.title.clone(),
        MenuColor::rgba(0.015, 0.020, 0.055, 0.98),
    );
    page.text(
        50.0,
        8.0,
        5.2,
        presentation.title.clone(),
        MenuTextAlign::Center,
        MenuColor::WHITE,
    );
    if catalog.entries.is_empty() {
        page.text(
            50.0,
            48.0,
            3.6,
            presentation.empty_message.clone(),
            MenuTextAlign::Center,
            MenuColor::WHITE,
        );
    } else {
        // Every registered experience gets a row: available ones are selectable
        // Actions; unavailable ones are non-actionable Items showing the reason.
        // The navigation cursor addresses only available entries, so map that
        // cursor onto the full list when deciding what to highlight.
        let row_height = (60.0 / catalog.entries.len().max(1) as f32).min(12.0);
        let mut available_index = 0usize;
        for (index, entry) in catalog.entries.iter().enumerate() {
            let (kind, action, detail, selected) = if entry.available {
                let selected = available_index == launcher.selected;
                available_index += 1;
                (
                    MenuControlKind::Action,
                    Some(entry.route_id.clone()),
                    (!entry.description.is_empty()).then_some(entry.description.clone()),
                    selected,
                )
            } else {
                (
                    MenuControlKind::Item,
                    None,
                    Some(
                        entry
                            .unavailable_reason
                            .clone()
                            .unwrap_or_else(|| "Unavailable".to_owned()),
                    ),
                    false,
                )
            };
            page.control(
                MenuRect::new(
                    16.0,
                    18.0 + index as f32 * (row_height + 1.5),
                    68.0,
                    row_height,
                ),
                kind,
                entry.label.clone(),
                detail,
                selected,
                false,
                action,
            );
        }
        if !presentation.footer.is_empty() {
            page.text(
                50.0,
                92.0,
                2.6,
                presentation.footer.clone(),
                MenuTextAlign::Center,
                MenuColor::WHITE,
            );
        }
    }

    let tabs = [BevyUiMenuTabSpec::new(BasicLauncherPage::Home, "Play")];
    let view = BevyUiMenuView::<BasicLauncherPage, ShellRouteId> {
        tabs: &tabs,
        active_tab: 0,
        page: &page,
        focused: None,
        focused_tab: None,
    };
    spawn_bevy_ui_menu_with_assets(commands, &view, asset_server);
}

fn shell_frame_key(
    launcher: &ShellLauncherState,
    catalog: &ShellLaunchCatalog,
    presentation: &ShellLauncherPresentation,
    sequence: &ActiveShellSequence,
) -> String {
    if launcher.active {
        return format!(
            "launcher:{}:{}:{:?}",
            launcher.selected, presentation.title, catalog.entries
        );
    }
    sequence_frame(sequence).key
}

fn sequence_frame(sequence: &ActiveShellSequence) -> BasicSequenceFrame {
    let Some(runtime) = sequence.runtime.as_ref() else {
        return BasicSequenceFrame::default();
    };
    let Some(segment) = runtime.current() else {
        return BasicSequenceFrame::default();
    };
    match &segment.presentation {
        ShellSegmentPresentation::TextCard { title, subtitle } => {
            let text = format!(
                "{}{}",
                title,
                subtitle
                    .as_ref()
                    .map(|item| format!("\n\n{item}"))
                    .unwrap_or_default()
            );
            BasicSequenceFrame {
                key: format!("text:{}:{text}", segment.id),
                text,
                image_path: None,
            }
        }
        ShellSegmentPresentation::StaticImage {
            asset_path,
            alt_text,
        } => BasicSequenceFrame {
            key: format!("image:{}:{asset_path}", segment.id),
            text: alt_text.clone(),
            image_path: Some(asset_path.clone()),
        },
        ShellSegmentPresentation::ImageSequence {
            frames,
            frames_per_second,
            alt_text,
        } => {
            let frame_index = if frames.is_empty() || *frames_per_second <= 0.0 {
                0
            } else {
                ((runtime.elapsed.as_secs_f32() * *frames_per_second) as usize) % frames.len()
            };
            let image_path = frames.get(frame_index).cloned();
            BasicSequenceFrame {
                key: format!("sequence:{}:{frame_index}:{image_path:?}", segment.id),
                text: alt_text.clone(),
                image_path,
            }
        }
        ShellSegmentPresentation::Registered(_) => BasicSequenceFrame::default(),
    }
}
