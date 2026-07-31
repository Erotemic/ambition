//! **What the select screen LOOKS like.**
//!
//! One panel per offered seat, each saying what that seat has decided. The panel
//! text is the seat's state read back verbatim — "press A to join", a character
//! name with arrows around it while browsing, the name alone once locked — so
//! anything the screen believes is visible on it, and a state nobody can reach
//! shows up as a panel nobody can change.
//!
//! ⚠ this exists because the screen worked and could not be SEEN: its route is
//! the composition's launcher route, so without this it rendered the launcher's
//! experience list while four people pressed buttons at an invisible menu.

use crate::select::{SELECTABLE, SeatSelection, SmashSelect};
use bevy::prelude::*;

/// The screen's UI root. One marker, so teardown is `despawn` on a query filtered
/// by THIS owner rather than a sweep of every node — a shared marker's teardown
/// clobbers whatever else happened to carry it.
#[derive(Component)]
pub struct SmashSelectUiRoot;

/// Which seat a panel shows. Carried on the entity so the update system does not
/// depend on child order.
#[derive(Component, Clone, Copy)]
pub struct SmashSeatPanel(pub usize);

/// What a seat's panel says. Public because the test asserts on it: a UI test
/// that checks entities EXIST is a test that passes over an empty box.
pub fn panel_text(seat: usize, selection: SeatSelection) -> String {
    match selection {
        SeatSelection::Empty => format!("P{} — press confirm to join", seat + 1),
        SeatSelection::Browsing { cursor } => {
            format!("P{} — < {} >", seat + 1, SELECTABLE[cursor])
        }
        SeatSelection::LockedIn { character } => {
            format!("P{} — {} READY", seat + 1, SELECTABLE[character])
        }
    }
}

/// The line under the panels: what the screen is waiting for.
pub fn prompt_text(select: &SmashSelect) -> String {
    if select.ready() {
        "Starting…".to_string()
    } else if select.joined() < 2 {
        "Two players needed".to_string()
    } else {
        "Waiting for everyone to lock in".to_string()
    }
}

#[derive(Component)]
pub struct SmashSelectPrompt;

pub fn spawn_select_ui(mut commands: Commands, existing: Query<(), With<SmashSelectUiRoot>>) {
    if !existing.is_empty() {
        return;
    }
    commands
        .spawn((
            SmashSelectUiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Text::new("CHOOSE YOUR FIGHTER"));
            for seat in 0..crate::select::MAX_SMASH_SEATS {
                root.spawn((
                    SmashSeatPanel(seat),
                    Text::new(panel_text(seat, SeatSelection::Empty)),
                ));
            }
            root.spawn((SmashSelectPrompt, Text::new("Two players needed")));
        });
}

pub fn despawn_select_ui(mut commands: Commands, roots: Query<Entity, With<SmashSelectUiRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

pub fn update_select_ui(
    select: Res<SmashSelect>,
    devices: Option<Res<ambition::input::LocalDeviceOrder>>,
    mut panels: Query<(&SmashSeatPanel, &mut Text, &mut Node), Without<SmashSelectPrompt>>,
    mut prompt: Query<&mut Text, With<SmashSelectPrompt>>,
) {
    // **A SEAT WITH NO CONTROLLER IS NOT SHOWN.** Its panel used to read "press
    // confirm to join" at a chair nobody could sit in — an invitation the screen
    // could not honour, because `drive_the_select_screen` only walks the seats
    // the pads actually offer. Hidden rather than removed: pads appear and
    // disappear, and a select screen is exactly where that happens.
    let offered = devices
        .as_deref()
        .map(crate::select::seats_offered)
        .unwrap_or(1);
    for (panel, mut text, mut node) in &mut panels {
        let shown = panel.0 < offered;
        let want = if shown { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
        let next = panel_text(panel.0, select.seat(panel.0));
        if text.0 != next {
            text.0 = next;
        }
    }
    for mut text in &mut prompt {
        let next = prompt_text(&select);
        if text.0 != next {
            text.0 = next;
        }
    }
}
