//! Destructible state, accepted damage, respawn, and collision publication.

use ambition_characters::control::DrivingParticipant;
use ambition_combat::components::{CenteredAabb, FeatureId, FeatureName};
use ambition_combat::events::GameplayBanner;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{
    FeatureSimEntity, SessionWorldMut, SessionWorldRef,
};
use ambition_time::{GameplayElapsed, WorldTime};
use ambition_vfx::vfx::{DebrisBurstMessage, ParticleKind, VfxMessage, VfxWriter};
use ambition_world_returns::{RespawnTimer, WorldTimeSchedule, owners_beside};
use bevy::prelude::*;

const BREAK_ON_STAND_SECONDS: f32 = 0.85;

/// ECS-native breakable payload.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BreakableFeature {
    breakable: ambition_interaction::Breakable,
}

impl BreakableFeature {
    pub fn new(breakable: ambition_interaction::Breakable) -> Self {
        Self { breakable }
    }

    /// The current destructible state.
    pub fn state(&self) -> &ambition_interaction::Breakable {
        &self.breakable
    }

    pub fn broken(&self) -> bool {
        self.breakable.state == ambition_interaction::BreakableState::Broken
    }
}

/// Stand-to-crumble timer for breakables with an `OnStand` trigger.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StandTimer(pub f32);

mod simulation;
pub use simulation::{emit_breakable_destroyed, strike, update_ecs_breakables};
mod returns;
pub use returns::mirror_breakable_respawns;

pub mod construction;

mod rollback_registration;
pub use rollback_registration::register_rollback_state;

mod volumes;
pub use volumes::refresh_breakable_damageable_volumes;

/// ECS breakable-state lookup for sprite swapping.
pub fn ecs_breakable_state(
    id: &str,
    breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<ambition_interaction::BreakableState> {
    breakables
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, breakable)| breakable.state().state)
}

/// Coins a smashed crate/pot drops — smaller than an enemy kill, so combat still
/// pays best, but the environment is worth poking.
pub const BREAKABLE_BOUNTY: i32 = 2;

/// Drop the value of a destroyed breakable in its current room.
pub fn drop_destroyed_loot(
    commands: &mut Commands,
    scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
    parent: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    id: &str,
    pos: ae::Vec2,
) {
    ambition_platformer2d_items::loot::drop_currency_coin(
        commands,
        scope,
        parent,
        id,
        pos,
        BREAKABLE_BOUNTY,
    );
}
