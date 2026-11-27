//! A rebind page for the foreground participant.

use ambition_game_shell::{
    PublishShellPausePages, ShellPauseMenu, ShellPauseOption, ShellPausePages, ShellPauseRow,
};
use ambition_input::{
    action_name, action_named, participant::context_priority, rebind_rows, ContextClaim,
    ParticipantContextAccess, ParticipantId, RebindCapture, RebindInputs, SeatActiveDevices,
    SeatBindings, PAUSE_CONTEXT,
};
use ambition_persistence::settings::UserSettings;
use bevy::prelude::*;

const PAGE: &str = "rebind";

/// Adds a keyboard and controller rebind page to the basic pause menu.
pub struct RebindPausePage;

impl Plugin for RebindPausePage {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShellPausePages>()
            .world_mut()
            .resource_mut::<ShellPausePages>()
            .register(PAGE, "Rebind Controls");
        app.add_systems(Update, publish.in_set(PublishShellPausePages));
    }
}

#[derive(Default)]
struct Capture {
    owner: Option<ParticipantId>,
    input: RebindCapture,
}

fn publish(
    menu: Res<ShellPauseMenu>,
    access: ParticipantContextAccess,
    mut actions: MessageReader<ShellPauseOption>,
    mut settings: ResMut<UserSettings>,
    mut pages: ResMut<ShellPausePages>,
    mut capture: Local<Capture>,
    mut inputs: RebindInputs,
    bindings: Option<Res<SeatBindings>>,
    devices: Option<Res<SeatActiveDevices>>,
) {
    let owner = menu
        .owner()
        .filter(|_| menu.page() == Some(PAGE))
        .map(ParticipantId);
    if capture.owner != owner {
        capture.owner = owner;
        capture.input.cancel();
    }
    for action in actions
        .read()
        .filter(|action| action.page == PAGE && Some(action.owner) == owner)
    {
        if action.direction == 0 {
            if let Some(action) = action_named(&action.row) {
                capture.input.arm(action);
            }
        }
    }
    let Some(owner) = owner else {
        return;
    };
    if access.permits(
        owner,
        ContextClaim::capturing(PAUSE_CONTEXT, context_priority::PAUSE),
    ) {
        inputs.poll(
            owner,
            &mut capture.input,
            settings
                .reborrow()
                .map_unchanged(|settings| &mut settings.controls),
        );
    }
    let device = devices
        .as_deref()
        .map(|devices| devices.for_seat(owner.slot()))
        .unwrap_or_default();
    let rows = bindings
        .as_deref()
        .into_iter()
        .flat_map(|bindings| rebind_rows(bindings.for_seat(owner.slot()), device))
        .map(|row| {
            let action = action_name(&row.action);
            let value = if capture.input.armed() == Some(row.action) {
                "Waiting for input".into()
            } else if row.also.is_empty() {
                row.binding
            } else {
                format!("{} · Also {}", row.binding, row.also.join(", "))
            };
            ShellPauseRow {
                id: action.clone(),
                label: action,
                value,
            }
        })
        .collect();
    let hint = capture
        .input
        .armed()
        .map(|_| "Press the control to assign. Use the Back row to cancel.");
    pages.publish(PAGE, owner, rows, hint);
}
