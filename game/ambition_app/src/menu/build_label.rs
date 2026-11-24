//! Build identity at the bottom of the launcher and pause menu.

use ambition_platformer2d::game_shell::ShellLauncherState;
use ambition_platformer2d::inventory_ui::InventoryUiState;
use bevy::prelude::*;
use bevy::text::LineBreak;

pub(crate) struct BuildLabelPlugin;

impl Plugin for BuildLabelPlugin {
    fn build(&self, app: &mut App) {
        if !matches!(option_env!("AMBITION_HIDE_BUILD_ID"), Some("1" | "true")) {
            app.add_systems(Update, sync_build_label);
        }
    }
}

#[derive(Component)]
struct BuildLabel;

fn sync_build_label(
    mut commands: Commands,
    launcher: Option<Res<ShellLauncherState>>,
    menus: Query<&InventoryUiState>,
    cameras: Query<Entity, With<IsDefaultUiCamera>>,
    mut label: Query<&mut Node, With<BuildLabel>>,
) {
    let visible = launcher.is_some_and(|launcher| launcher.active)
        || menus.iter().any(|menu| menu.visible);
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
            BuildLabel,
            Name::new("Build identity"),
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

fn build_label() -> String {
    if let (Some(stamp), Some(identity)) = (
        option_env!("AMBITION_BUILD_EPOCH"),
        option_env!("AMBITION_BUILD_LABEL"),
    ) {
        let seconds = stamp.parse().expect("build epoch in seconds");
        return format!("Build {} / {identity}", utc_build_date(seconds));
    }
    let built = std::env::current_exe()
        .and_then(std::fs::metadata)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or_else(|| "?".to_owned(), |at| utc_build_date(at.as_secs()));
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

fn utc_build_date(seconds: u64) -> String {
    const DAY: u64 = 86_400;
    const CYCLE: u64 = 146_097;
    let leap = |year: u64| year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    // Each 400-year cycle in the Gregorian calendar has 146,097 days.
    let days = seconds / DAY;
    let mut year = 1970 + 400 * (days / CYCLE);
    let mut day = days % CYCLE;
    loop {
        let year_days = 365 + u64::from(leap(year));
        if day < year_days {
            break;
        }
        day -= year_days;
        year += 1;
    }
    let mut month = 1;
    for month_days in [31, 28 + u64::from(leap(year)), 31, 30, 31, 30, 31, 31, 30, 31, 30, 31] {
        if day < month_days {
            break;
        }
        day -= month_days;
        month += 1;
    }
    let day = day + 1;
    let hour = seconds % DAY / 3600;
    let minute = seconds / 60 % 60;
    let second = seconds % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_dates_use_the_utc_calendar() {
        for (seconds, expected) in [
            (0, "1970-01-01 00:00:00 UTC"),
            (951_782_400, "2000-02-29 00:00:00 UTC"),
            (1_795_531_786, "2026-11-24 14:49:46 UTC"),
            (4_107_542_400, "2100-03-01 00:00:00 UTC"),
        ] {
            assert_eq!(utc_build_date(seconds), expected);
        }
    }

    #[test]
    fn the_launcher_and_pause_menu_share_one_build_label() {
        let mut app = App::new();
        app.add_systems(Update, sync_build_label);
        app.world_mut().spawn(IsDefaultUiCamera);
        app.insert_resource(ShellLauncherState {
            active: true,
            ..default()
        });

        app.update();
        let mut labels = app.world_mut().query_filtered::<(Entity, &Node, &Children), With<BuildLabel>>();
        let (entity, node, children) = labels.single(app.world()).unwrap();
        assert_eq!(node.display, Display::Flex);
        let text = app.world().get::<Text>(children[0]).unwrap();
        assert!(text.0.starts_with("Build ") && text.0.contains(" UTC"));

        app.world_mut().resource_mut::<ShellLauncherState>().active = false;
        app.update();
        assert_eq!(labels.single(app.world()).unwrap().1.display, Display::None);

        app.world_mut().spawn(InventoryUiState {
            visible: true,
            ..default()
        });
        app.update();
        let (pause_label, node, _) = labels.single(app.world()).unwrap();
        assert_eq!(pause_label, entity);
        assert_eq!(node.display, Display::Flex);
    }
}
