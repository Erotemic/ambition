//! Player ECS systems.

use bevy::prelude::*;

use super::components::{
    ActivePlayerAttack, LocalPlayer, PlayerCombatState, PlayerEntity, PlayerHealth,
    PlayerInputFrame, PlayerSlot, PrimaryPlayer,
};
use super::events::PlayerHealRequested;
use super::movement_components::{PlayerGroundState, PlayerKinematics};
use crate::brain::{ActorControl, Brain, BrainSnapshot};
#[cfg(test)]
use crate::engine_core as ae;
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
/// frame via `crate::brain::tick_player_brain_from_input`.
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
        &PlayerKinematics,
        &PlayerGroundState,
        &mut Brain,
        &mut ActorControl,
    )>,
) {
    for (slot, input, kin, ground, mut brain, mut control) in &mut players {
        // Build the snapshot from the player's cluster components plus
        // the per-tick input frame. The input is what makes
        // Brain::Player's translation deterministic: same input +
        // same body snapshot → same ActorControlFrame.
        let snapshot = BrainSnapshot {
            actor_pos: kin.pos,
            actor_vel: kin.vel,
            actor_facing: kin.facing,
            actor_on_ground: ground.on_ground,
            alive: true,
            target_pos: kin.pos,
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
            // Player brain doesn't consult these fields; leave them
            // None so the snapshot builder doesn't pay for queries
            // the brain ignores.
            crowding: None,
            terrain: None,
            // Player brain reads its own air-jump state via the
            // PlayerInputFrame / engine path, not via the snapshot.
            air_jumps_remaining: 0,
        };
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        control.0 = frame;
        // Silence unused-var: slot is part of the multi-player seam.
        let _ = slot;
    }
}

/// Mirror `ActivePlayerAttack::is_active()` onto
/// `PlayerCombatState::attacking` so rendering systems can branch on
/// attack state without a separate query.
///
/// All Phase 2 work has landed: `PlayerMovementAuthority` and
/// `PlayerBody` are gone, and the cluster components in
/// [`super::movement_components`] are the authoritative simulation
/// state. This system is the residue of the previous mirror — it's
/// kept as a separate function so the scheduler ordering hooked into
/// `PresentationSync` doesn't change shape.
pub fn write_player_ecs_components(
    mut players: Query<(&ActivePlayerAttack, &mut PlayerCombatState), With<PlayerEntity>>,
) {
    for (attack, mut combat) in &mut players {
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
    use crate::brain::ActorControl;

    /// Default player ActionSet derives from AbilitySet — when
    /// `attack` is on, the ActionSet has a Swipe melee; when off,
    /// melee is None and the resolver emits nothing for melee
    /// presses. Pins the ability-gated capability invariant.
    #[test]
    fn player_action_set_melee_disabled_when_attack_ability_off() {
        use crate::brain::ActionSet;
        let mut player =
            crate::player::primary_player_scratch(ae::Vec2::new(0.0, 0.0), ae::AbilitySet::sandbox_all());
        // Force-disable the attack ability.
        player.abilities.abilities.attack = false;
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(player, crate::actor::Health::new(10));
        // ActionSet on the bundle reflects the disabled ability.
        let action_set: &ActionSet = &bundle.action_set;
        assert!(
            action_set.melee.is_none(),
            "ActionSet.melee should be None when AbilitySet.attack is off"
        );
    }

    /// Similarly: with shield ability off, special slot is None.
    /// Pins the same gating discipline for special-ability slots.
    #[test]
    fn player_action_set_special_disabled_when_shield_ability_off() {
        use crate::brain::ActionSet;
        let mut player =
            crate::player::primary_player_scratch(ae::Vec2::new(0.0, 0.0), ae::AbilitySet::sandbox_all());
        player.abilities.abilities.shield = false;
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(player, crate::actor::Health::new(10));
        let action_set: &ActionSet = &bundle.action_set;
        assert!(
            action_set.special.is_none(),
            "ActionSet.special should be None when AbilitySet.shield is off"
        );
    }

    /// Default player ActionSet has a Swipe melee + Bolt ranged +
    /// BubbleShield special when the player has all abilities. Pins
    /// the sandbox_all() default — EFFECTS consumers
    /// can rely on these slots being filled.
    #[test]
    fn player_action_set_has_full_moveset_with_sandbox_all_abilities() {
        use crate::brain::{ActionSet, MeleeActionSpec, RangedActionSpec, SpecialActionSpec};
        let player =
            crate::player::primary_player_scratch(ae::Vec2::new(0.0, 0.0), ae::AbilitySet::sandbox_all());
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(player, crate::actor::Health::new(10));
        let action_set: &ActionSet = &bundle.action_set;
        assert!(matches!(action_set.melee, Some(MeleeActionSpec::Swipe(_))));
        assert!(matches!(
            action_set.ranged,
            Some(RangedActionSpec::Bolt { .. })
        ));
        assert!(matches!(
            action_set.special,
            Some(SpecialActionSpec::BubbleShield)
        ));
    }

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
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(40.0, 60.0),
            ae::AbilitySet::sandbox_all(),
        );
        ae::refresh_movement_resources_clusters(
            &player.abilities,
            &mut player.dash,
            &mut player.jump,
            ae::DEFAULT_TUNING,
        );
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(player, crate::actor::Health::new(10));
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
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(40.0, 60.0),
            ae::AbilitySet::sandbox_all(),
        );
        ae::refresh_movement_resources_clusters(
            &player.abilities,
            &mut player.dash,
            &mut player.jump,
            ae::DEFAULT_TUNING,
        );
        // Use the canonical bundle so the player's ActionSet is the
        // production default (Swipe melee + Bolt ranged). Bundle
        // already includes a PlayerBody synced off the authority.
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(player, crate::actor::Health::new(10));
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
        let mut player = crate::player::primary_player_scratch(
            ae::Vec2::new(100.0, 100.0),
            ae::AbilitySet::sandbox_all(),
        );
        ae::refresh_movement_resources_clusters(
            &player.abilities,
            &mut player.dash,
            &mut player.jump,
            ae::DEFAULT_TUNING,
        );
        // `PlayerSimulationBundle` carries the same cluster components
        // that `PlayerMovementAuthority` + `PlayerBody` used to be
        // synthesized from. `Brain` / `ActorControl` are bundle fields
        // too, so no extra spawn-tuple state is needed.
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(player, crate::actor::Health::new(10));
        app.world_mut().spawn(bundle);
        app.add_systems(
            Update,
            (sync_local_player_input_frame, tick_player_brains).chain(),
        );

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
