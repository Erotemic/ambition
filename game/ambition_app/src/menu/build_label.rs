//! Build identity at the bottom of the open pause menu.

use ambition_platformer2d::inventory_ui::InventoryUiState;
use bevy::prelude::*;
use bevy::text::LineBreak;

pub(super) struct PauseBuildLabelPlugin;

impl Plugin for PauseBuildLabelPlugin {
    fn build(&self, app: &mut App) {
        if !matches!(option_env!("AMBITION_HIDE_BUILD_ID"), Some("1" | "true")) {
            app.add_systems(Update, sync_build_label);
        }
    }
}

#[derive(Component)]
struct PauseBuildLabel;

fn sync_build_label(
    mut commands: Commands,
    menus: Query<&InventoryUiState>,
    cameras: Query<Entity, With<IsDefaultUiCamera>>,
    mut label: Query<&mut Node, With<PauseBuildLabel>>,
) {
    let visible = menus.iter().any(|menu| menu.visible);
    let display = if visible {
        Display::Flex
    } else {
        Display::None
    };
    if let Ok(mut node) = label.single_mut() {
        if node.display != display {
            node.display = display;
        }
        return;
    }
    if !visible {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    commands
        .spawn((
            PauseBuildLabel,
            Name::new("Pause menu build identity"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(8.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            UiTargetCamera(camera),
            GlobalZIndex(1100),
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(build_label()),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgba(0.76, 0.80, 0.86, 0.90)),
                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                TextShadow {
                    offset: Vec2::new(1.0, 1.0),
                    color: Color::BLACK,
                },
                Pickable::IGNORE,
            ));
        });
}

/// The label: the one compiled in (an `AMBITION_BUILD_ID`, a browser or
/// Android build), else the executable's own build time and the working
/// tree's revision, read when the menu first opens. The tree is where the
/// game runs from, not necessarily what the binary was built from, so it is
/// named as the tree's.
fn build_label() -> String {
    if let Some(label) = option_env!("AMBITION_BUILD_LABEL") {
        return label.to_owned();
    }
    let built = std::env::current_exe()
        .and_then(std::fs::metadata)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or_else(|| "?".to_owned(), |at| at.as_secs().to_string());
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).output().ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    match git(&["rev-parse", "--short=10", "HEAD"]) {
        Some(revision) => {
            let dirty = git(&["status", "--porcelain", "--untracked-files=no", "--ignore-submodules=dirty"])
                .is_some_and(|status| !status.is_empty());
            format!("Build {built} / tree {revision}{}", if dirty { "+" } else { "" })
        }
        None => format!("Build {built}"),
    }
}
