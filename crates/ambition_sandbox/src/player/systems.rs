//! Player ECS systems.

use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, PlayerBody, PlayerCombatState, PlayerEntity, PlayerHealth,
    PlayerMovementAuthority, PrimaryPlayer,
};
use super::events::PlayerHealRequested;

/// Write `PlayerBody` and `PlayerCombatState::attacking` from the authoritative
/// sources each frame.
///
/// `PlayerBody` is a snapshot of `PlayerMovementAuthority::player`.
/// `attacking` mirrors whether the player's `ActivePlayerAttack` has an
/// active swing — iterates so a future second player gets its own
/// `attacking` flag without changing the call shape.
pub fn write_player_ecs_components(
    mut players: Query<
        (
            &PlayerMovementAuthority,
            &mut PlayerBody,
            &mut PlayerCombatState,
            &ActivePlayerAttack,
        ),
        With<PlayerEntity>,
    >,
) {
    for (authority, mut body, mut combat, attack) in &mut players {
        *body = PlayerBody::from_player(&authority.player);
        combat.attacking = attack.is_active();
    }
}

/// Apply heal messages to the authoritative `PlayerHealth` ECS component.
///
/// A heal targets either a specific player entity (`heal.target ==
/// Some(entity)`) or the primary player as a fallback (`None`). The
/// fallback path keeps existing call sites — cutscene heals, dev-tool
/// heals — working with no change. Per-player producers like pickup
/// collection should set the target explicitly so a non-primary
/// player who walked into the heart actually gets healed.
pub fn apply_player_heal_requests(
    mut heals: MessageReader<PlayerHealRequested>,
    mut players: Query<&mut PlayerHealth, With<PlayerEntity>>,
    primary_q: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let primary = primary_q.single().ok();
    for heal in heals.read() {
        if heal.amount <= 0 {
            continue;
        }
        let target = heal.target.or(primary);
        let Some(target) = target else {
            // No player entity yet (startup or headless): drop the
            // heal silently so the queue still drains.
            continue;
        };
        if let Ok(mut health) = players.get_mut(target) {
            health.heal(heal.amount);
        }
    }
}
