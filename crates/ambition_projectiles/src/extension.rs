//! The projectile domain's extension request adapter.
//!
//! A module submits a [`ProjectileSpawnPort`] value; this adapter lowers it
//! through [`ProjectileSpawnRequest::open`], the one spawn-request seam, with
//! the invocation's body as owner and the invocation's move use as credit.
//! The module cannot name another owner or another move use.

use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION, WIELDED_USE};
use ambition_projectile_spec::ProjectileSpawnPort;
use bevy::prelude::*;

use crate::spawn_request::{ProjectileSpawnRequest, ProjectileStart};

/// Install the request port in `technique_execution`, where a spawn
/// materializes before this tick's projectile step.
pub fn install(app: &mut App) {
    app.install_extension_request::<ProjectileSpawnPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_projectiles",
        lower_projectile_spawns::<InTechniqueExecution>,
    );
}

/// Install the same request port in `wielded_use` (a held item's use), which
/// runs before this tick's projectile step too.
pub fn install_for_wielded_use(app: &mut App) {
    app.install_extension_request::<ProjectileSpawnPort, _>(
        WIELDED_USE,
        "ambition_projectiles",
        lower_projectile_spawns::<InWieldedUse>,
    );
}

/// Install the same request port in `module_entity_tick` (a module-owned
/// entity fires: the entity owns the projectile).
pub fn install_for_module_entity_tick(app: &mut App) {
    app.install_extension_request::<ProjectileSpawnPort, _>(
        MODULE_ENTITY_TICK,
        "ambition_projectiles",
        lower_projectile_spawns::<InModuleEntityTick>,
    );
}

/// The phase a request adapter instance lowers for: one port offered in two
/// phases has two named adapter systems, not one system registered twice.
pub struct InTechniqueExecution;
/// See [`InTechniqueExecution`].
pub struct InWieldedUse;
/// See [`InTechniqueExecution`].
pub struct InModuleEntityTick;

fn lower_projectile_spawns<Phase: Send + Sync + 'static>(
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
