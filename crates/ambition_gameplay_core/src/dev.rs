//! Developer-facing tooling that stays in the machinery lib: the dev
//! STATE (`dev_tools`: DeveloperTools + editable profiles, read by
//! persistence + presentation), the gameplay `trace` recorder (written
//! by sim code), and the startup `profiling` marks (read by audio).

pub mod dev_tools;
pub mod profiling;
pub mod trace;

use bevy::prelude::*;

use crate::actor::{
    BodyAbilities, BodyBlinkState, BodyDashState, BodyFlightState, BodyJumpState, PrimaryPlayerOnly,
};
use dev_tools::{EditableAbilitySet, EditableMovementTuning};

/// Push live dev-tools ability/tuning edits onto the authoritative player.
///
/// Registered by the host to run even while gameplay is suspended so the F3
/// inspector stays responsive; the logic is body-state mutation and lives here
/// beside the dev STATE it reads.
pub fn sync_live_player_dev_edits_system(
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        (
            &mut BodyAbilities,
            &mut BodyFlightState,
            &mut BodyBlinkState,
            &mut BodyDashState,
            &mut BodyJumpState,
        ),
        PrimaryPlayerOnly,
    >,
) {
    let Ok((mut abilities, mut flight, mut blink, mut dash, mut jump)) = player_q.single_mut()
    else {
        return;
    };
    dev_tools::sync_live_ability_edits_clusters(
        &mut abilities,
        &mut flight,
        &mut blink,
        &mut dash,
        &mut jump,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
    );
}
