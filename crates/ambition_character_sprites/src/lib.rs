//! Gameplay-derived facts from character sheets.
//!
//! This crate owns animation selection, pose-derived body geometry, and authored
//! attack-volume mapping. Sheet vocabulary itself remains in
//! `ambition_sprite_sheet`. [`SpritePosedBodyPlugin`] owns registration of the
//! geometry synchronization pass so the system is installed with its owner.

mod anim;
mod attack_hitbox;
mod posed_body;

pub use anim::{
    ActorAnimState, FighterClipFacts, GuardBreakBeat, body_state_clip, pick_actor_anim,
    pick_player_anim,
};
pub use attack_hitbox::{
    actor_attack_hitbox_local, actor_attack_hitbox_world, authored_attack_volume_resolver,
    manifest_attack_hitbox_local, manifest_attack_hitbox_world, player_attack_hitbox_local,
    player_attack_hitbox_world, refused_file_roots, resolves_by_file_root,
};
pub use posed_body::{
    PosedBodyGeometry, authored_body_pixel_size, posed_body_geometry, sync_sprite_posed_bodies,
};

use bevy::prelude::{App, IntoScheduleConfigs, Plugin};

use ambition_platformer2d_shared_tangle::schedule::{SimScheduleExt, WorldPrepSet};

/// Installs [`sync_sprite_posed_bodies`] before movement integration. Bodies
/// opt in through `SpritePosedBody`; pose-pinning rules run after movement, so
/// geometry follows the pose on the next tick by design.
pub struct SpritePosedBodyPlugin;

impl Plugin for SpritePosedBodyPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            sync_sprite_posed_bodies.in_set(WorldPrepSet::BeforeIntegrate),
        );
        // Pay the file-root index at Startup instead of on the first punch. See
        // `attack_hitbox::warm_file_root_registry` for the 189ms frame this cost.
        app.add_systems(bevy::app::Startup, |_: bevy::ecs::system::Commands| {
            crate::attack_hitbox::warm_file_root_registry();
        });
    }
}
