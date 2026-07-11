//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

// The device→slot and slot→body bridges left for `crate::control` (R6c);
// these end-to-end tests drive the player tick THROUGH them, so they import
// them as fixtures rather than owning them.
use super::*;
use crate::control::{populate_slot_controls, sync_local_player_input_frame};
use ambition_characters::brain::ActorControl;
use ambition_input::ControlFrame;

#[test]
fn mana_regenerates_over_time_but_clamps_to_max() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0,
        scaled_dt: 1.0,
    });
    app.add_systems(Update, regen_player_mana);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            crate::actor::BodyMana::default(),
        ))
        .id();
    // Drain it, then let it tick back up.
    app.world_mut()
        .get_mut::<crate::actor::BodyMana>(player)
        .unwrap()
        .meter
        .try_spend(60.0);
    let before = app
        .world()
        .get::<crate::actor::BodyMana>(player)
        .unwrap()
        .meter
        .current;
    app.update();
    let after = app
        .world()
        .get::<crate::actor::BodyMana>(player)
        .unwrap()
        .meter
        .current;
    assert!(
        after > before,
        "mana should regenerate ({before} -> {after})"
    );

    // Many ticks can't exceed max.
    for _ in 0..20 {
        app.update();
    }
    let m = app
        .world()
        .get::<crate::actor::BodyMana>(player)
        .unwrap()
        .meter;
    assert!(m.current <= m.max + 1e-3, "mana clamps to max");
}

#[test]
fn wallet_add_clamps_and_spend_respects_balance() {
    let mut wallet = ambition_characters::actor::BodyWallet::default();
    assert_eq!(wallet.balance, 0);
    wallet.add(50);
    wallet.add(-100); // can't drive below zero
    assert_eq!(wallet.balance, 0);
    wallet.add(30);
    assert!(wallet.try_spend(20));
    assert_eq!(wallet.balance, 10);
    assert!(!wallet.try_spend(99), "can't overspend");
    assert_eq!(wallet.balance, 10);
}

/// Default player ActionSet derives from AbilitySet — when
/// `attack` is on, the ActionSet has a Swipe melee; when off,
/// melee is None and the resolver emits nothing for melee
/// presses. Pins the ability-gated capability invariant.
#[test]
fn player_action_set_melee_disabled_when_attack_ability_off() {
    use ambition_characters::brain::ActionSet;
    let mut player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(0.0, 0.0),
        ae::AbilitySet::sandbox_all(),
    );
    // Force-disable the attack ability.
    player.abilities.abilities.attack = false;
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
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
    use ambition_characters::brain::ActionSet;
    let mut player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(0.0, 0.0),
        ae::AbilitySet::sandbox_all(),
    );
    player.abilities.abilities.shield = false;
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    let action_set: &ActionSet = &bundle.action_set;
    assert!(
        action_set.special.is_none(),
        "ActionSet.special should be None when AbilitySet.shield is off"
    );
}

/// Default player ActionSet has a Swipe melee + Bolt ranged +
/// `bubble_shield` special when the player has all abilities. Pins
/// the sandbox_all() default — EFFECTS consumers
/// can rely on these slots being filled.
#[test]
fn player_action_set_has_full_moveset_with_sandbox_all_abilities() {
    use ambition_characters::brain::{
        ActionSet, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
    };
    let player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(0.0, 0.0),
        ae::AbilitySet::sandbox_all(),
    );
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    let action_set: &ActionSet = &bundle.action_set;
    assert!(matches!(action_set.melee, Some(MeleeActionSpec::Swipe(_))));
    assert!(matches!(
        action_set.ranged,
        Some(RangedActionSpec::Bolt { .. })
    ));
    assert!(matches!(
        action_set.special,
        Some(SpecialActionSpec::Special(ref key)) if key == "bubble_shield"
    ));
}

/// End-to-end: player releases the projectile charge →
/// tick_player_brains fills frame.fire → resolver emits a
/// Ranged action message with the player's Bolt spec. Pins
/// the ranged side of the seam alongside the melee test below.
#[test]
fn player_projectile_release_emits_ranged_bolt_action_message_end_to_end() {
    use ambition_characters::brain::{
        emit_brain_action_messages, ActionRequest, ActorActionMessage, RangedActionSpec,
    };
    use bevy::transform::components::Transform;
    let mut app = App::new();
    app.init_resource::<ControlFrame>();
    app.init_resource::<SlotControls>();
    app.add_message::<ActorActionMessage>();
    let mut player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(40.0, 60.0),
        ae::AbilitySet::sandbox_all(),
    );
    ae::refresh_movement_resources_clusters(
        &player.abilities,
        &mut player.dash,
        &mut player.jump,
        ae::DEFAULT_TUNING,
    );
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut()
        .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)));
    app.add_systems(
        Update,
        (
            populate_slot_controls,
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
    match ranged[0].request.clone() {
        ActionRequest::Ranged {
            spec: RangedActionSpec::Bolt { speed, .. },
            dir,
            dir_policy,
            ..
        } => {
            assert!(speed > 0.0, "Bolt has positive speed");
            // dir is the controlled-body-local aim vector normalized.
            assert!(dir.x > 0.0 && dir.y < 0.0, "aim diagonally up-right");
            assert_eq!(dir_policy, ae::GameplayFramePolicy::ControlledBodyLocal);
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
    use ambition_characters::brain::{
        emit_brain_action_messages, ActionRequest, ActorActionMessage, MeleeActionSpec,
    };
    use bevy::transform::components::Transform;
    let mut app = App::new();
    app.init_resource::<ControlFrame>();
    app.init_resource::<SlotControls>();
    app.add_message::<ActorActionMessage>();
    let mut player = crate::avatar::primary_player_scratch(
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
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut()
        .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)));
    app.add_systems(
        Update,
        (
            populate_slot_controls,
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
    match received[0].request.clone() {
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
    app.init_resource::<SlotControls>();
    let mut player = crate::avatar::primary_player_scratch(
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
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut().spawn(bundle);
    app.add_systems(
        Update,
        (
            populate_slot_controls,
            sync_local_player_input_frame,
            tick_player_brains,
        )
            .chain(),
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
    assert_eq!(control.0.locomotion.x, 1.0);
    assert!(control.0.jump_pressed);
    assert!(control.0.melee_pressed);
    assert!(control.0.shield_held);
    assert_eq!(control.0.facing, 1.0);
}
