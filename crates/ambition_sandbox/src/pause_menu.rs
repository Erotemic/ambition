//! Pause menu overlay.
//!
//! The existing `GameMode::Paused` state already gates gameplay (input, sim,
//! and feature updates short-circuit when not in `Playing`). This module
//! adds the visible side: a translucent overlay with a small action menu
//! (Resume / Inventory / Quit) and a focused selection that responds to
//! both keyboard and gamepad navigation through the existing
//! `SandboxAction` input map.

use bevy::app::AppExit;
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::game_mode::GameMode;
use crate::input::SandboxAction;
use crate::inventory::InventoryUiState;

/// Top-level entity tagging for the pause overlay.
#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseMenuItem {
    Resume,
    Inventory,
    Quit,
}

impl PauseMenuItem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Resume => "Resume",
            Self::Inventory => "Inventory",
            Self::Quit => "Quit to Desktop",
        }
    }

    pub const ALL: [Self; 3] = [Self::Resume, Self::Inventory, Self::Quit];
}

#[derive(Resource, Default)]
pub struct PauseMenuState {
    pub selected: usize,
}

/// `MenuToggle` input opens/closes the pause menu by toggling `GameMode`.
/// Runs before `sandbox_update` consumes the start press so the gameplay
/// loop's existing toggle path stays disabled while the menu is the
/// authoritative driver of pause/resume.
pub fn pause_menu_toggle(
    action_state: Query<&ActionState<SandboxAction>>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut inventory: ResMut<InventoryUiState>,
) {
    let Ok(actions) = action_state.single() else {
        return;
    };
    let toggle = actions.just_pressed(&SandboxAction::Start);
    if !toggle {
        return;
    }
    match mode.get() {
        GameMode::Playing => {
            state.selected = 0;
            next_mode.set(GameMode::Paused);
        }
        GameMode::Paused => {
            // Pressing pause again resumes immediately and closes the
            // inventory if it was open from the menu.
            inventory.visible = false;
            next_mode.set(GameMode::Playing);
        }
        _ => {}
    }
}

pub fn pause_menu_navigate(
    action_state: Query<&ActionState<SandboxAction>>,
    mode: Res<State<GameMode>>,
    mut state: ResMut<PauseMenuState>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<InventoryUiState>,
    mut exit: MessageWriter<AppExit>,
) {
    if !matches!(mode.get(), GameMode::Paused) {
        return;
    }
    // The inventory overlay covers the pause menu and reads the same nav
    // actions for its own selection. Without this guard the pause menu
    // silently scrolls (and confirms!) in the background while the
    // inventory is open — e.g. using a Spare Battery while pause's
    // selected index has rolled onto "Quit" would fire AppExit.
    if inventory.visible {
        return;
    }
    let Ok(actions) = action_state.single() else {
        return;
    };

    let items = PauseMenuItem::ALL;
    if actions.just_pressed(&SandboxAction::MoveUp) {
        state.selected = (state.selected + items.len() - 1) % items.len();
    }
    if actions.just_pressed(&SandboxAction::MoveDown) {
        state.selected = (state.selected + 1) % items.len();
    }

    if actions.just_pressed(&SandboxAction::Jump) {
        let item = items[state.selected];
        match item {
            PauseMenuItem::Resume => {
                inventory.visible = false;
                next_mode.set(GameMode::Playing);
            }
            PauseMenuItem::Inventory => {
                inventory.visible = true;
                inventory.selected = 0;
                inventory.opened_from_pause = true;
            }
            PauseMenuItem::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

pub fn spawn_pause_menu(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.78)),
            ZIndex(50),
            Visibility::Hidden,
            PauseMenuRoot,
            Name::new("Pause menu"),
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(28.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.94)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.85)),
            Name::new("Pause panel"),
        ))
        .id();
    commands.entity(root).add_child(panel);

    let title = commands
        .spawn((
            Text::new("Paused"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            Name::new("Pause title"),
        ))
        .id();
    commands.entity(panel).add_child(title);

    for item in PauseMenuItem::ALL {
        let entity = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(item.label()),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                item,
                Name::new(format!("Pause item: {}", item.label())),
            ))
            .id();
        commands.entity(panel).add_child(entity);
    }
}

/// Show/hide the pause overlay based on `GameMode` and update item highlights.
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut items: Query<(&PauseMenuItem, &mut TextColor, &mut BackgroundColor)>,
) {
    // Hide while the inventory is open so it doesn't double-stack with the
    // inventory panel; the inventory has its own dismiss handling and
    // returns control to the pause menu when closed.
    let visible = matches!(mode.get(), GameMode::Paused) && !inventory.visible;
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }
    let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
    for (item, mut color, mut bg) in &mut items {
        let is_selected = Some(*item) == selected_item;
        *color = if is_selected {
            TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
        } else {
            TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96))
        };
        *bg = if is_selected {
            BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
        } else {
            BackgroundColor(Color::NONE)
        };
    }
}
