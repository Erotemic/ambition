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

/// What one seat's panel says. Public because the test asserts on it: a UI test
/// that checks entities EXIST is a test that passes over an empty box.
///
/// `manned` is whether a controller actually offers this seat: an unmanned one
/// cannot be joined, and telling somebody to "press confirm" at a chair the
/// device order does not reach is an invitation the screen cannot honour. What
/// it CAN be is a CPU, so that is what its panel offers.
pub fn panel_text(seat: usize, selection: SeatSelection, manned: bool) -> String {
    match selection {
        SeatSelection::Empty if manned => format!("P{} — press confirm to join", seat + 1),
        SeatSelection::Empty => format!("P{} — empty · Down adds a CPU", seat + 1),
        SeatSelection::Browsing { cursor } => {
            format!("P{} — < {} >", seat + 1, SELECTABLE[cursor])
        }
        SeatSelection::LockedIn { character } => {
            format!("P{} — {} READY", seat + 1, SELECTABLE[character])
        }
        SeatSelection::Cpu { character } => {
            format!("P{} — CPU · {} READY", seat + 1, SELECTABLE[character])
        }
    }
}

/// The line under the panels: what the screen is waiting for, or what to press
/// to get past it.
///
/// ⚠ it used to read "Two players needed" and stop there, which was true and
/// useless: on a keyboard there was no second player available and no press that
/// produced one. A prompt that names a requirement without naming the button
/// that satisfies it is a dead end with punctuation.
pub fn prompt_text(select: &SmashSelect) -> String {
    if select.ready() {
        "Starting…".to_string()
    } else if select.joined() < 2 {
        "Press Down to add a CPU opponent (Up removes one)".to_string()
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
                    // Spawned as an unmanned empty seat; `update_select_ui`
                    // corrects it against the live device order on the same
                    // frame, before anything is presented.
                    Text::new(panel_text(seat, SeatSelection::Empty, false)),
                ));
            }
            root.spawn((
                SmashSelectPrompt,
                Text::new(prompt_text(&SmashSelect::default())),
            ));
        });
}

pub fn despawn_select_ui(mut commands: Commands, roots: Query<Entity, With<SmashSelectUiRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

pub fn update_select_ui(
    select: Res<SmashSelect>,
    devices: Option<Res<ambition_platformer2d::input::LocalDeviceOrder>>,
    mut panels: Query<(&SmashSeatPanel, &mut Text, &mut Node), Without<SmashSelectPrompt>>,
    mut prompt: Query<&mut Text, With<SmashSelectPrompt>>,
) {
    // **EVERY SEAT IS SHOWN, and what it OFFERS depends on whether a controller
    // reaches it.** (Jon, 2026-07-31: *"a seat without a controller should be
    // able to become a CPU or be left empty."*)
    //
    // The panels used to be hidden past the pad count, and that was the right
    // fix for the wrong problem: the complaint was a chair nobody could sit in
    // saying "press confirm to join", and hiding it also hid the only other
    // thing that chair could be. On a keyboard it hid three quarters of the
    // screen and left one seat that could never reach the two a match needs.
    let offered = devices
        .as_deref()
        .map(crate::select::seats_offered)
        .unwrap_or(1);
    for (panel, mut text, mut node) in &mut panels {
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
        let next = panel_text(panel.0, select.seat(panel.0), panel.0 < offered);
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
