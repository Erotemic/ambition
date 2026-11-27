//! A video page with the options that the host selects.

use ambition_game_shell::{
    PublishShellPausePages, ShellPauseMenu, ShellPauseOption, ShellPausePages, ShellPauseRow,
};
use ambition_input::ParticipantId;
use ambition_persistence::settings::UserSettings;
pub use ambition_settings_menu::settings::VideoMenuOption;
use bevy::prelude::*;

const PAGE: &str = "video";

/// Adds the selected video options to the basic pause menu.
pub struct VideoPausePage(pub Vec<VideoMenuOption>);

impl Plugin for VideoPausePage {
    fn build(&self, app: &mut App) {
        if self.0.is_empty() {
            return;
        }
        app.init_resource::<ShellPausePages>()
            .world_mut()
            .resource_mut::<ShellPausePages>()
            .register(PAGE, "Video");
        app.insert_resource(VideoOptions(self.0.clone()))
            .add_systems(Update, publish.in_set(PublishShellPausePages));
    }
}

#[derive(Resource)]
struct VideoOptions(Vec<VideoMenuOption>);

fn publish(
    menu: Res<ShellPauseMenu>,
    options: Res<VideoOptions>,
    mut actions: MessageReader<ShellPauseOption>,
    mut settings: ResMut<UserSettings>,
    mut pages: ResMut<ShellPausePages>,
) {
    let owner = menu
        .owner()
        .filter(|_| menu.page() == Some(PAGE))
        .map(ParticipantId);
    for action in actions
        .read()
        .filter(|action| action.page == PAGE && Some(action.owner) == owner)
    {
        if let Some(option) = options.0.iter().find(|option| option.key() == action.row) {
            option.adjust(&mut settings.video, action.direction);
        }
    }
    let Some(owner) = owner else {
        return;
    };
    let rows = options
        .0
        .iter()
        .map(|option| {
            let row = option.row(&settings.video);
            ShellPauseRow {
                id: option.key().into(),
                label: row.label,
                value: row.value_label,
            }
        })
        .collect();
    pages.publish(PAGE, owner, rows, None);
}
