//! Player ECS spawn bundles.

use ambition_engine as ae;
use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, LocalPlayer, PlayerAnimState, PlayerBlinkCameraState, PlayerBody,
    PlayerCombatState, PlayerEntity, PlayerHealth, PlayerInputFrame, PlayerInteractionState,
    PlayerMovementAuthority, PlayerPlatformRideState, PlayerSafetyState, PlayerSlot, PrimaryPlayer,
};
use crate::brain::{ActionSet, ActorControl, Brain};
use crate::features::ActorFaction;

/// All simulation components required on the player entity.
///
/// Use this bundle in `commands.spawn()` together with presentation-side
/// components (`Transform`, `PlayerVisual`) so the spawn call documents
/// what simulation state the player entity carries. The bundle does not
/// include `Transform` or `Sprite` — those are presentation concerns.
/// Identity tag bundle: every player entity carries exactly these
/// components. Useful as a building block in tests that want to spawn
/// an additional player without rebuilding the full simulation bundle.
#[derive(Bundle)]
pub struct PlayerIdentityBundle {
    pub marker: PlayerEntity,
    pub slot: PlayerSlot,
}

impl PlayerIdentityBundle {
    pub fn new(slot: PlayerSlot) -> Self {
        Self {
            marker: PlayerEntity,
            slot,
        }
    }
}

#[derive(Bundle)]
pub struct PlayerSimulationBundle {
    pub identity: PlayerIdentityBundle,
    pub primary: PrimaryPlayer,
    pub local: LocalPlayer,
    pub authority: PlayerMovementAuthority,
    pub body: PlayerBody,
    pub health: PlayerHealth,
    pub combat: PlayerCombatState,
    pub interaction: PlayerInteractionState,
    pub anim: PlayerAnimState,
    pub blink_cam: PlayerBlinkCameraState,
    pub ride: PlayerPlatformRideState,
    pub attack: ActivePlayerAttack,
    pub safety: PlayerSafetyState,
    pub input: PlayerInputFrame,
    pub faction: ActorFaction,
    pub name: Name,
    /// Universal-brain seam. The player entity carries a
    /// `Brain::Player(slot)`, an `ActionSet` (its full moveset), and
    /// an `ActorControl` that the brain-driver system fills each
    /// frame from `PlayerInputFrame`. Until Chunk 4d/e wires the
    /// authority to consume the frame, the brain and control
    /// component are *parallel* state — they're built but nothing
    /// reads them yet.
    pub brain: Brain,
    pub action_set: ActionSet,
    pub actor_control: ActorControl,
}

impl PlayerSimulationBundle {
    /// Build the canonical local-primary player bundle from an engine
    /// `Player` and initial `Health`. The result spawns with
    /// `PlayerSlot(0)`, `PrimaryPlayer`, and `LocalPlayer` — the
    /// single-player default.
    ///
    /// Future code that needs to spawn a second / guest / remote
    /// player should compose `PlayerIdentityBundle::new(PlayerSlot(n))`
    /// with the simulation components manually rather than calling
    /// this helper, since the second player should not inherit
    /// `PrimaryPlayer` and may not be `LocalPlayer`.
    pub fn new(player: ae::Player, health: ae::Health) -> Self {
        let authority = PlayerMovementAuthority::new(player);
        let body = authority.body();
        let initial_safe_pos = authority.player.pos;
        Self {
            identity: PlayerIdentityBundle::new(PlayerSlot::PRIMARY),
            primary: PrimaryPlayer,
            local: LocalPlayer,
            authority,
            body,
            health: PlayerHealth::new(health),
            combat: PlayerCombatState::default(),
            interaction: PlayerInteractionState::default(),
            anim: PlayerAnimState::default(),
            blink_cam: PlayerBlinkCameraState::default(),
            ride: PlayerPlatformRideState::default(),
            attack: ActivePlayerAttack::default(),
            safety: PlayerSafetyState::new(initial_safe_pos),
            input: PlayerInputFrame::default(),
            faction: ActorFaction::Player,
            name: Name::new("Player"),
            brain: Brain::Player(PlayerSlot::PRIMARY),
            // Player ActionSet today is a placeholder; the player
            // brain drives the existing update_player path, not
            // ActionSet-resolved effects. When Chunk 4e+ pushes the
            // player onto the unified effect-resolve path, this
            // ActionSet will carry the player's full moveset.
            action_set: ActionSet::peaceful(),
            actor_control: ActorControl::default(),
        }
    }
}
