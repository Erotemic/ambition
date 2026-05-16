//! Player ECS spawn bundles.

use ambition_engine as ae;
use bevy::prelude::*;

use super::components::{
    PlayerAnimState, PlayerBlinkCameraState, PlayerBody, PlayerCombatState, PlayerEntity,
    PlayerHealth, PlayerInteractionState, PlayerMovementAuthority,
};

/// All simulation components required on the player entity.
///
/// Use this bundle in `commands.spawn()` together with presentation-side
/// components (`Transform`, `PlayerVisual`) so the spawn call documents
/// what simulation state the player entity carries. The bundle does not
/// include `Transform` or `Sprite` — those are presentation concerns.
#[derive(Bundle)]
pub struct PlayerSimulationBundle {
    pub marker: PlayerEntity,
    pub authority: PlayerMovementAuthority,
    pub body: PlayerBody,
    pub health: PlayerHealth,
    pub combat: PlayerCombatState,
    pub interaction: PlayerInteractionState,
    pub anim: PlayerAnimState,
    pub blink_cam: PlayerBlinkCameraState,
    pub name: Name,
}

impl PlayerSimulationBundle {
    /// Build the bundle from an engine `Player` and initial `Health`.
    pub fn new(player: ae::Player, health: ae::Health) -> Self {
        let authority = PlayerMovementAuthority::new(player);
        let body = authority.body();
        Self {
            marker: PlayerEntity,
            authority,
            body,
            health: PlayerHealth::new(health),
            combat: PlayerCombatState::default(),
            interaction: PlayerInteractionState::default(),
            anim: PlayerAnimState::default(),
            blink_cam: PlayerBlinkCameraState::default(),
            name: Name::new("Player"),
        }
    }
}
