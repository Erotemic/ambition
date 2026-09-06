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
    registrar.rollback_component_clone::<crate::character::SpritePosedBody>(
        OWNER,
        "actor.sprite_posed_body",
    );
}
