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
    menu: Single<&InventoryUiState>,
    cameras: Query<Entity, With<IsDefaultUiCamera>>,
    mut label: Query<&mut Node, With<PauseBuildLabel>>,
) {
    let display = if menu.visible { Display::Flex } else { Display::None };
    if let Ok(mut node) = label.single_mut() {
        if node.display != display {
            node.display = display;
        }
        return;
    }
    if !menu.visible { return; }
    let Ok(camera) = cameras.single() else { return; };
    commands.spawn((
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
    )).with_children(|root| {
        root.spawn((
            Text::new(env!("AMBITION_BUILD_LABEL")),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgba(0.76, 0.80, 0.86, 0.90)),
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::BLACK },
            Pickable::IGNORE,
        ));
    });
}
