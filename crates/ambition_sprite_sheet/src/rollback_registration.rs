//! Rollback declaration owned by `ambition_sprite_sheet`.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_component_clone::<crate::character::ActorAnimOverride>(
        OWNER,
        "actor.anim_override",
    );
    // The poses applications propose, by key: the override is their
    // projection, so a rewind restores each proposal with its owner.
    registrar.rollback_component_clone_probed::<crate::character::PoseClaims>(
        OWNER,
        "actor.pose_claims",
        crate::character::PoseClaims::checksum,
    );
    registrar.rollback_component_clone::<crate::character::SpritePosedBody>(
        OWNER,
        "actor.sprite_posed_body",
    );
}
