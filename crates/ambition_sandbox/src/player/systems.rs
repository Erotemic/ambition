//! Player ECS systems.

use ambition_engine as ae;
use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, LocalPlayer, PlayerBody, PlayerCombatState, PlayerEntity, PlayerHealth,
    PlayerInputFrame, PlayerMovementAuthority, PlayerSlot, PrimaryPlayer,
};
use super::events::PlayerHealRequested;
use crate::brain::{tick_player_brain_from_input, ActorControl, Brain, BrainSnapshot};
use crate::input::ControlFrame;

/// Mirror the global [`ControlFrame`] resource onto the local primary
/// player's [`PlayerInputFrame`] component each frame.
///
/// This is the producer for the per-player input migration (OVERNIGHT-
/// TODO #17.5). The visible binary's input pipeline + the headless
/// driver both keep writing the global resource; this system snapshots
/// it onto the entity so simulation systems can move toward reading
/// `Query<&PlayerInputFrame>` without losing the "primary local player"
/// behavior. Future remote / co-op players would have their own
/// PlayerInputFrame populated by a network adapter, bypassing the
/// global resource entirely.
///
/// Runs once per frame after the input pipeline has finished writing
/// `Res<ControlFrame>` (registered in the `PlayerInput` set, after
/// `interaction_input_system`).
pub fn sync_local_player_input_frame(
    frame: Res<ControlFrame>,
    mut players: Query<&mut PlayerInputFrame, (With<PlayerEntity>, With<LocalPlayer>)>,
) {
    let snapshot = *frame;
    for mut player_input in &mut players {
        player_input.frame = snapshot;
    }
}

/// Translate each player's input frame into their `ActorControl`
/// frame via [`tick_player_brain_from_input`].
///
/// This is the producer for the universal-brain seam on the player
/// side. Today nothing reads `ActorControl` for the player —
/// `update_player` still drives the body via `PlayerInputFrame`
/// directly. The point of running it now is to:
///
/// - Prove the brain → ActorControl path executes every tick
///   cleanly (no per-frame allocations or panics in the hot path).
/// - Let presentation / debug systems observe the frame so the
///   downstream EFFECTS-stage migration can be verified by reading
///   the frame instead of inspecting `PlayerInputFrame` directly.
///
/// Runs after `sync_local_player_input_frame` so the input frame is
/// already current. Iterates every PlayerEntity with a Brain — the
/// system shape is multi-player ready even though only one player
/// exists today.
pub fn tick_player_brains(
    mut players: Query<(
        &PlayerSlot,
        &PlayerInputFrame,
        &PlayerBody,
        &mut Brain,
        &mut ActorControl,
    )>,
) {
    for (slot, input, body, mut brain, mut control) in &mut players {
        // Build a minimal snapshot from the player's read-model
        // PlayerBody. The brain only reads `actor_facing` today; the
        // rest is filled with sensible defaults so a future brain
        // variant that asks for more (e.g. wall_contact for a player
        // possessing a slug) doesn't see garbage.
        let snapshot = BrainSnapshot {
            actor_pos: body.pos,
            actor_vel: body.vel,
            actor_facing: body.facing,
            actor_on_ground: body.on_ground,
            alive: true,
            target_pos: body.pos,
            target_alive: true,
            sim_time: 0.0,
            dt: 0.0,
            attack_cooldown_remaining: 0.0,
            attack_windup_remaining: 0.0,
            attack_active_remaining: 0.0,
            attack_recover_remaining: 0.0,
            stun_remaining: 0.0,
            wall_contact: None,
        };
        // The Brain enum's tick() goes through `tick_player_brain`
        // which today emits a neutral frame + facing — it doesn't
        // have access to the input frame at the snapshot boundary
        // yet. We bypass the enum dispatch here and call the
        // input-aware variant directly so the frame actually
        // populates from PlayerInputFrame. Once 4d wires the
        // snapshot to carry the input frame, this can collapse to
        // `brain.tick(&snapshot, &mut control.0)`.
        let mut frame = ae::ActorControlFrame::neutral();
        match brain.as_mut() {
            Brain::Player(_) => {
                tick_player_brain_from_input(input, &snapshot, &mut frame);
            }
            // Non-player brains on a PlayerEntity (e.g. possession)
            // — tick through the normal dispatch.
            other => other.tick(&snapshot, &mut frame),
        }
        control.0 = frame;
        // Silence unused-var: slot is part of the multi-player seam.
        let _ = slot;
    }
}

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
