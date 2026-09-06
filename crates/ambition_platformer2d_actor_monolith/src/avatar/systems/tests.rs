// The device→slot bridge lives in `crate::control` (R6c); these end-to-end
// tests drive the player tick THROUGH it so they exercise the same slot-owned
// authority as production rather than stamping body-local input state.
use super::*;
use crate::schedule::publish_seat_controls_when_nobody_else_does;
use ambition_characters::control::ActorControl;
use ambition_characters::control::{PlayerSlot, SeatRawFrames};

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
            ambition_platformer2d_core::BodyMana::default(),
        ))
        .id();
    // Drain it, then let it tick back up.
    app.world_mut()
        .get_mut::<ambition_platformer2d_core::BodyMana>(player)
        .unwrap()
        .meter
        .try_spend(60.0);
    let before = app
        .world()
        .get::<ambition_platformer2d_core::BodyMana>(player)
        .unwrap()
        .meter
        .current;
    app.update();
    let after = app
        .world()
        .get::<ambition_platformer2d_core::BodyMana>(player)
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
        .get::<ambition_platformer2d_core::BodyMana>(player)
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
        action_set::RangedStyle, ActionSet, MeleeActionSpec, RangedActionSpec, SpecialActionSpec,
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
        Some(RangedActionSpec {
            style: RangedStyle::Bolt,
            ..
        })
    ));
    assert!(matches!(
        action_set.special,
        Some(SpecialActionSpec::Special(ref key)) if key == "bubble_shield"
    ));
}

/// End-to-end: player releases the projectile charge →
/// tick_controlled_brains fills frame.fire → resolver emits a
/// Ranged action message with the player's Bolt spec. Pins
/// the ranged side of the seam alongside the melee test below.
#[test]
fn player_projectile_release_emits_ranged_bolt_action_message_end_to_end() {
    use ambition_characters::brain::{
        emit_brain_action_messages, ActionRequest, ActorActionMessage,
    };
    use bevy::transform::components::Transform;
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_message::<ActorActionMessage>();
    let mut player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(40.0, 60.0),
        ae::AbilitySet::sandbox_all(),
    );
    ae::refresh_movement_resources_clusters(
        &player.abilities,
        &mut player.dash,
        &mut player.jump,
        &mut player.dodge,
        ae::DEFAULT_TUNING.air_jumps,
        ae::RecoveryRefresh::Answered,
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
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
            emit_brain_action_messages,
        )
            .chain(),
    );
    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.projectile_released = true;
            // aim diagonally up-right; brain reads aim when present
            cf.aim_x = 0.8;
            cf.aim_y = -0.6;
        });
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
            spec:
                ambition_characters::brain::RangedActionSpec {
                    style: ambition_characters::brain::action_set::RangedStyle::Bolt,
                    speed,
                    ..
                },
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

/// End-to-end: player presses attack → tick_controlled_brains fills
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
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_message::<ActorActionMessage>();
    let mut player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(40.0, 60.0),
        ae::AbilitySet::sandbox_all(),
    );
    ae::refresh_movement_resources_clusters(
        &player.abilities,
        &mut player.dash,
        &mut player.jump,
        &mut player.dodge,
        ae::DEFAULT_TUNING.air_jumps,
        ae::RecoveryRefresh::Answered,
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
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
            emit_brain_action_messages,
        )
            .chain(),
    );

    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.attack_pressed = true;
            cf.axis_x = 1.0;
        });
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
/// populate ControlFrame into the primary slot, run tick_controlled_brains,
/// and assert ActorControl reflects that slot input.
/// Pins the universal-brain seam on the player side.
#[test]
fn player_brain_seam_translates_control_frame_to_actor_control() {
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    let mut player = crate::avatar::primary_player_scratch(
        ae::Vec2::new(100.0, 100.0),
        ae::AbilitySet::sandbox_all(),
    );
    ae::refresh_movement_resources_clusters(
        &player.abilities,
        &mut player.dash,
        &mut player.jump,
        &mut player.dodge,
        ae::DEFAULT_TUNING.air_jumps,
        ae::RecoveryRefresh::Answered,
    );
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    app.world_mut().spawn(bundle);
    app.add_systems(
        Update,
        (
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
        )
            .chain(),
    );

    // Stamp the control frame with a known input.
    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
            cf.attack_pressed = true;
            cf.shield_held = true;
        });
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

/// A possessed actor is controlled by the SAME producer as the home avatar.
///
///  this is the test that decides whether the cut is safe, and it can only
/// fail one way: silently. `tick_controlled_brains` dropped its
/// `With<PlayerEntity>` filter so a possessed body reaches it, and
/// `tick_actor_brains` now leaves player-brained bodies alone — so if this
/// query does not MATCH a body built the way production builds an actor, that
/// body has no control producer at all and simply stops responding. A missing
/// component is not a compile error; it is an empty iterator.
///
/// So the body here is constructed through `ActorClusterSeed::into_components`,
/// the production path, and the only thing added is the participant's brain —
/// which is exactly what possession does (ONE control seam: possession is brain
/// transfer, never an input-copy component).
///
///  and the speed proves whose body it is. `velocity_target` is an absolute world-space
/// command, so the translator scales the stick by the body's own
/// `MotionModel::commanded_top_speed` — 137 here, a number no other body in the test has.
#[test]
fn a_possessed_actor_is_driven_by_the_controlled_brain_producer() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    const POSSESSED_TOP_SPEED: f32 = 137.0;

    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_systems(
        Update,
        (
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
        )
            .chain(),
    );

    let pos = ae::Vec2::new(64.0, 32.0);
    let size = ae::Vec2::new(44.0, 78.0);
    let mut seed = ambition_body_seed::ActorClusterSeed::new(
        "possessed",
        "possessed",
        ae::Aabb::new(pos, size * 0.5),
        ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        &[],
    );
    seed.kin.size = size;
    seed.kin.pos = pos;
    seed.health.reset();
    let mut params = ae::DEFAULT_TUNING.axis_swept_params();
    params.locomotion.max_run_speed = POSSESSED_TOP_SPEED;
    let body = app
        .world_mut()
        .spawn((
            seed.into_components(),
            // What the production spawn sites add beside the cluster: an intent
            // frame and a seat. Possession moves only the second.
            ambition_characters::control::ActorControl::default(),
            DrivingParticipant(PlayerSlot::PRIMARY),
        ))
        .id();
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_platformer2d_core::movement::MotionModel::axis_swept(params));

    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
        });
    }
    app.update();

    let control = app
        .world()
        .entity(body)
        .get::<ActorControl>()
        .expect("an actor cluster carries ActorControl")
        .0;
    assert_eq!(
        control.locomotion.x, 1.0,
        "the possessed body did not receive the participant's stick — if this is \
         zero, the controlled producer's query does not match a production actor \
         body and possession moves nothing"
    );
    assert!(
        control.jump_pressed,
        "the possessed body did not receive the participant's jump edge"
    );
    assert_eq!(
        control.velocity_target,
        ae::WorldVec2::new(POSSESSED_TOP_SPEED, 0.0),
        "the direct velocity command must be scaled by THIS body's own movement \
         policy, not by another body's capability and not by actor configuration"
    );
}

/// A scripted sequence now silences a possessed body too.
///
/// A death beat or a flagpole slide driving a possessed body would blank nothing.
///
/// One producer in one phase is what fixes it, and the guard is the ORDER: the
/// two systems run in their real schedule relation, so a future move of either
/// one back across the other turns this red.
#[test]
fn a_scripted_sequence_silences_a_possessed_body() {
    use ambition_characters::control::ScriptedControl;
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_systems(
        Update,
        (
            publish_seat_controls_when_nobody_else_does,
            tick_controlled_brains,
            blank_scripted_control_frames,
        )
            .chain(),
    );

    let pos = ae::Vec2::new(0.0, 0.0);
    let size = ae::Vec2::new(44.0, 78.0);
    let mut seed = ambition_body_seed::ActorClusterSeed::new(
        "scripted",
        "scripted",
        ae::Aabb::new(pos, size * 0.5),
        ambition_entity_catalog::placements::CharacterBrain::Custom("pirate_raider".into()),
        &[],
    );
    seed.kin.size = size;
    seed.kin.pos = pos;
    seed.health.reset();
    let body = app
        .world_mut()
        .spawn((
            seed.into_components(),
            ambition_characters::control::ActorControl::default(),
            DrivingParticipant(PlayerSlot::PRIMARY),
            ScriptedControl,
        ))
        .id();

    {
        let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
        raw.shape(PlayerSlot::PRIMARY, |cf| {
            cf.axis_x = 1.0;
            cf.jump_pressed = true;
        });
    }
    app.update();

    let control = app.world().entity(body).get::<ActorControl>().unwrap().0;
    assert_eq!(
        control.locomotion.x, 0.0,
        "a scripted sequence must silence the body it is driving, whichever \
         population that body belongs to"
    );
    assert!(!control.jump_pressed, "the jump edge survived the blanking");
}

/// ⭐⭐ EVERY DRIVEN BODY REGENERATES, not just the possessed subject.
///
/// ⛔⛔ THIS REFILLED ONE `ControlledSubject`. Seat one spent mana on a gauntlet
/// it could never get back — a slow leak rather than a dead verb, which is why
/// it outlived the verbs' own fix.
#[test]
fn two_driven_bodies_each_regenerate_their_own_mana() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 0.5,
        scaled_dt: 0.5,
    });
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    app.add_systems(Update, regen_player_mana);

    let drained = |app: &mut App, slot: u8, sim: &str| -> Entity {
        let body = app
            .world_mut()
            .spawn((
                ambition_platformer2d_core::BodyMana::default(),
                DrivingParticipant(PlayerSlot(slot)),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
            ))
            .id();
        app.world_mut()
            .get_mut::<ambition_platformer2d_core::BodyMana>(body)
            .unwrap()
            .meter
            .try_spend(40.0);
        body
    };
    let a = drained(&mut app, 0, "seat_a");
    let b = drained(&mut app, 1, "seat_b");
    let before = |app: &App, body: Entity| {
        app.world()
            .get::<ambition_platformer2d_core::BodyMana>(body)
            .unwrap()
            .meter
            .current
    };
    let (a_before, b_before) = (before(&app, a), before(&app, b));
    // ⛔ THE PREMISE: a full meter cannot be seen to refill.
    assert!(
        a_before < 100.0 && b_before < 100.0,
        "the fixture spent no mana"
    );

    app.update();

    for (body, was, who) in [(a, a_before, "a"), (b, b_before, "b")] {
        assert!(
            before(&app, body) > was,
            "seat {who}'s mana did not regenerate: {was} -> {}",
            before(&app, body)
        );
    }
}
