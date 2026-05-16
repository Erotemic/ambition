//! Player ECS systems.

use bevy::prelude::*;

use super::components::{
    PlayerBody, PlayerCombatState, PlayerEntity, PlayerHealth, PlayerMovementAuthority,
};
use super::events::PlayerHealRequested;

/// Write `PlayerBody` and `PlayerCombatState::attacking` from the authoritative
/// sources each frame.
///
/// `PlayerBody` is a snapshot of `PlayerMovementAuthority::player`.
/// `attacking` mirrors whether `CurrentPlayerAttack` has an active swing.
pub fn write_player_ecs_components(
    attack_res: Res<crate::CurrentPlayerAttack>,
    mut players: Query<
        (
            &PlayerMovementAuthority,
            &mut PlayerBody,
            &mut PlayerCombatState,
        ),
        With<PlayerEntity>,
    >,
) {
    let Ok((authority, mut body, mut combat)) = players.single_mut() else {
        return;
    };
    *body = PlayerBody::from_player(&authority.player);
    combat.attacking = attack_res.0.is_some();
}

/// Apply heal messages to the authoritative `PlayerHealth` ECS component.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut PlayerHealth, With<PlayerEntity>>,
) {
    let Ok(mut health) = players.single_mut() else {
        // No player entity yet (startup or headless): drain the queue.
        for _ in heals.read() {}
        return;
    };
    for heal in heals.read() {
        if heal.amount > 0 {
            health.heal(heal.amount);
        }
    }
}
