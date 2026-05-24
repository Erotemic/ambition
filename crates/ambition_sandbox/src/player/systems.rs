//! Player ECS systems.

use ambition_engine as ae;
use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, LocalPlayer, PlayerBody, PlayerCombatState, PlayerEntity, PlayerHealth,
    PlayerInputFrame, PlayerMovementAuthority, PlayerSlot, PrimaryPlayer,
};
use super::events::PlayerHealRequested;
use crate::brain::{ActorControl, Brain, BrainSnapshot};
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
        // Build the snapshot from the player's read-model PlayerBody
        // plus the per-tick input frame. The input is what makes
        // Brain::Player's translation deterministic: same input +
        // same body snapshot → same ActorControlFrame.
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
            player_input: Some(input.frame),
        };
        let mut frame = ae::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::{ActorControl, Brain};
    use bevy::prelude::*;

    /// End-to-end: player releases the projectile charge →
    /// tick_player_brains fills frame.fire → resolver emits a
    /// Ranged action message with the player's Bolt spec. Pins
    /// the ranged side of the seam alongside the melee test below.
    #[test]
    fn player_projectile_release_emits_ranged_bolt_action_message_end_to_end() {
        use crate::brain::{
            emit_brain_action_messages, ActionRequest, ActorActionMessage, RangedActionSpec,
        };
        use bevy::transform::components::Transform;
        let mut app = App::new();
        app.init_resource::<ControlFrame>();
        app.add_message::<ActorActionMessage>();
        let mut player = ae::Player::new_with_abilities(
            ae::Vec2::new(40.0, 60.0),
            ae::AbilitySet::sandbox_all(),
        );
        player.refresh_movement_resources(ae::DEFAULT_TUNING);
        let bundle = crate::player::PlayerSimulationBundle::new(player, ae::Health::new(10));
        app.world_mut()
            .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)));
        app.add_systems(
            Update,
            (
                sync_local_player_input_frame,
                tick_player_brains,
                emit_brain_action_messages,
            )
                .chain(),
        );
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.projectile_released = true;
            // aim diagonally up-right; brain reads aim when present
            cf.aim_x = 0.8;
            cf.aim_y = -0.6;
        }
        app.update();
        let mut messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let received: Vec<_> = messages.drain().collect();
        let ranged: Vec<_> = received
            .into_iter()
            .filter(|m| matches!(m.request, ActionRequest::Ranged { .. }))
            .collect();
        assert_eq!(ranged.len(), 1, "expected exactly one Ranged message");
        match ranged[0].request {
            ActionRequest::Ranged {
                spec: RangedActionSpec::Bolt { speed, .. },
                dir,
                ..
            } => {
                assert!(speed > 0.0, "Bolt has positive speed");
                // dir is the aim vector normalized
                assert!(dir.x > 0.0 && dir.y < 0.0, "aim diagonally up-right");
            }
            other => panic!("expected Ranged::Bolt, got {:?}", other),
        }
    }

    /// End-to-end: player presses attack → tick_player_brains fills
    /// ActorControl → emit_brain_action_messages produces an
    /// ActorActionMessage with a Swipe request. Pins the full
    /// player-side universal-brain seam from input to resolved
    /// concrete action.
    #[test]
    fn player_attack_press_emits_swipe_action_message_end_to_end() {
        use crate::brain::{
            emit_brain_action_messages, ActionRequest, ActorActionMessage, MeleeActionSpec,
        };
        use bevy::transform::components::Transform;
        let mut app = App::new();
        app.init_resource::<ControlFrame>();
        app.add_message::<ActorActionMessage>();
        let mut player = ae::Player::new_with_abilities(
            ae::Vec2::new(40.0, 60.0),
            ae::AbilitySet::sandbox_all(),
        );
        player.refresh_movement_resources(ae::DEFAULT_TUNING);
        // Use the canonical bundle so the player's ActionSet is the
        // production default (Swipe melee + Bolt ranged). Bundle
        // already includes a PlayerBody synced off the authority.
        let bundle = crate::player::PlayerSimulationBundle::new(player, ae::Health::new(10));
        app.world_mut()
            .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)));
        app.add_systems(
            Update,
            (
                sync_local_player_input_frame,
                tick_player_brains,
                emit_brain_action_messages,
            )
                .chain(),
        );

        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.attack_pressed = true;
            cf.axis_x = 1.0;
        }
        app.update();
        let mut messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let received: Vec<_> = messages.drain().collect();
        assert_eq!(received.len(), 1, "expected one Swipe message");
        match received[0].request {
            ActionRequest::Melee {
                spec: MeleeActionSpec::Swipe(_),
                facing,
                origin,
                ..
            } => {
                assert!(facing > 0.0, "facing should be right (+1)");
                assert_eq!(origin, ae::Vec2::new(40.0, 60.0));
            }
            other => panic!("expected Melee::Swipe, got {:?}", other),
        }
    }

    /// End-to-end: spawn a player entity with the brain components,
    /// populate ControlFrame, run sync_local_player_input_frame +
    /// tick_player_brains, assert ActorControl reflects the input.
    /// Pins the universal-brain seam on the player side.
    #[test]
    fn player_brain_seam_translates_control_frame_to_actor_control() {
        let mut app = App::new();
        app.init_resource::<ControlFrame>();
        let mut player = ae::Player::new_with_abilities(
            ae::Vec2::new(100.0, 100.0),
            ae::AbilitySet::sandbox_all(),
        );
        player.refresh_movement_resources(ae::DEFAULT_TUNING);
        let body = PlayerBody::from_player(&player);
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            LocalPlayer,
            PlayerSlot::PRIMARY,
            PlayerInputFrame::default(),
            PlayerMovementAuthority::new(player),
            body,
            Brain::Player(PlayerSlot::PRIMARY),
            ActorControl::default(),
        ));
        app.add_systems(Update, (sync_local_player_input_frame, tick_player_brains).chain());

        // Stamp the control frame with a known input.
        {
            let mut cf = app.world_mut().resource_mut::<ControlFrame>();
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
            cf.attack_pressed = true;
            cf.shield_held = true;
        }
        app.update();

        let mut q = app
            .world_mut()
            .query_filtered::<&ActorControl, With<PlayerEntity>>();
        let control = q
            .iter(app.world())
            .next()
            .expect("player entity should have ActorControl");
        // axis_x → desired_vel.x, jump_pressed → jump_pressed, etc.
        assert_eq!(control.0.desired_vel.x, 1.0);
        assert!(control.0.jump_pressed);
        assert!(control.0.melee_pressed);
        assert!(control.0.shield_held);
        assert_eq!(control.0.facing, 1.0);
    }
}
