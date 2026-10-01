//! The projectile domain's extension request adapter.
//!
//! A module submits a [`ProjectileSpawnPort`] value; this adapter lowers it
//! through [`ProjectileSpawnRequest::open`], the one spawn-request seam, with
//! the invocation's body as owner and the invocation's move use as credit.
//! The module cannot name another owner or another move use.

use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_projectile_spec::ProjectileSpawnPort;
use bevy::prelude::*;

use crate::spawn_request::{ProjectileSpawnRequest, ProjectileStart};

/// Install the request port in `technique_execution`, where a spawn
/// materializes before this tick's projectile step.
pub fn install(app: &mut App) {
    app.install_extension_request::<ProjectileSpawnPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_projectiles",
        lower_projectile_spawns,
    );
}

fn lower_projectile_spawns(
    mut outbox: ResMut<ExtensionOutbox>,
    mut spawns: MessageWriter<ProjectileSpawnRequest>,
) {
    for submitted in outbox.drain::<ProjectileSpawnPort>() {
        spawns.write(
            ProjectileSpawnRequest::open(
                submitted.scope,
                submitted.value,
                ProjectileStart::StepThisTick,
            )
            .fired_by_move_if_any(submitted.occurrence),
        );
    }
}
