//! Rollback declaration owned by `ambition_cutscene`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register the playback state a rewind has to reproduce.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    //  OPTIONAL-canonical, not canonical. `CutscenePlugin` is installed by
    // compositions that HAVE cutscenes; a bare oracle harness or a demo without
    // scripted beats carries no `ActiveCutscene`, and the plain canonical form
    // installs a checksum system taking `Res<T>` that panics on every frame the
    // resource is absent. The lifecycle domain next door learned that by turning
    // eight rollback-oracle tests red.
    registrar.rollback_resource_optional_canonical::<crate::ActiveCutscene>(
        OWNER,
        "cutscene.playback",
    );
    // The trigger's last-room state must rewind with the cutscene; otherwise a
    // resimulation can suppress a transition that should fire again.
    registrar.rollback_resource_optional_canonical::<crate::LastCutsceneRoom>(
        OWNER,
        "cutscene.last_room",
    );
}
