use crate::control::tick_controlled_brains;
use ambition_platformer2d_core as ae;
// The device→slot bridge lives in `crate::control` (R6c); these end-to-end
// tests drive the player tick THROUGH it so they exercise the same slot-owned
// authority as production rather than stamping body-local input state.
use super::*;
use crate::control::publish_seat_controls_when_nobody_else_does;
use ambition_characters::control::{PlayerSlot, SeatRawFrames, SlotControls};

fn mana_of(app: &App, body: Entity) -> f32 {
    ambition_abilities::mana::level(app.world().get(body))
        .expect("the fixture body holds Mana")
        .current
}

fn drain_mana(app: &mut App, body: Entity, amount: f32) {
    assert!(ambition_abilities::mana::spend(
        app.world_mut()
            .get_mut::<ambition_platformer2d_core::resources::ActorResources>(body)
            .as_deref_mut(),
        amount
    ));
}

fn mana_regen_app(dt: f32, policy: Option<f32>) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(dt, dt));
    if let Some(per_second) = policy {
        app.insert_resource(PlayerManaRegen(per_second));
    }
    app.add_systems(Update, regen_player_mana);
    app
}

#[test]
fn mana_regenerates_over_time_but_clamps_to_max() {
    let mut app = mana_regen_app(1.0, Some(ambition_abilities::mana::REGEN_PER_SEC));
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, ambition_abilities::mana::bank()))
        .id();
    // Drain it, then let it tick back up.
    drain_mana(&mut app, player, 60.0);
    let before = mana_of(&app, player);
    app.update();
    let after = mana_of(&app, player);
    assert!(
        after > before,
        "mana should regenerate ({before} -> {after})"
    );

    // Many ticks can't exceed max.
    for _ in 0..20 {
        app.update();
    }
    let m = ambition_abilities::mana::level(app.world().get(player)).expect("held");
    assert!(m.current <= m.max + 1e-3, "mana clamps to max");
}

/// ⛔ THE RATE IS THE COMPOSITION'S, AND ITS ABSENCE IS AN ANSWER. A game that
/// states no regen refills nothing, and a body that holds no Mana is not given
/// any by the refill. The control is the same drained body refilling under a
/// stated rate.
#[test]
fn no_stated_rate_refills_nothing_and_no_pool_gains_mana() {
    let mut app = mana_regen_app(1.0, None);
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, ambition_abilities::mana::bank()))
        .id();
    drain_mana(&mut app, player, 60.0);
    app.update();
    assert_eq!(mana_of(&app, player), 40.0, "a game with no stated rate refilled Mana");

    let mut app = mana_regen_app(1.0, Some(ambition_abilities::mana::REGEN_PER_SEC));
    let bare = app.world_mut().spawn((PlayerEntity, PrimaryPlayer)).id();
    app.update();
    assert!(
        app.world()
            .get::<ambition_platformer2d_core::resources::ActorResources>(bare)
            .is_none(),
        "the refill handed a body a pool it was never declared"
    );
    app.world_mut()
        .entity_mut(bare)
        .insert(ambition_abilities::mana::bank());
    drain_mana(&mut app, bare, 60.0);
    app.update();
    assert!(mana_of(&app, bare) > 40.0, "the control: a held pool does refill");
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
    let mut player = crate::avatar::home_body_scratch(
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
    let action_set: &ActionSet = &bundle.0.action_set;
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
    let mut player = crate::avatar::home_body_scratch(
        ae::Vec2::new(0.0, 0.0),
        ae::AbilitySet::sandbox_all(),
    );
    player.abilities.abilities.shield = false;
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    let action_set: &ActionSet = &bundle.0.action_set;
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
    let player = crate::avatar::home_body_scratch(
        ae::Vec2::new(0.0, 0.0),
        ae::AbilitySet::sandbox_all(),
    );
    let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
        player,
        ambition_characters::actor::Health::new(10),
    );
    let action_set: &ActionSet = &bundle.0.action_set;
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

/// End-to-end: player releases the projectile button →
/// tick_controlled_brains fills frame.fire → resolver emits a
/// Ranged action message with the player's Bolt spec. Pins
/// the ranged side of the seam alongside the melee test below.
///
/// The body does NOT charge: a charging body's ranged intent belongs to the
/// charge path, and the resolver emitting a bolt for it as well was the
/// protagonist's two-projectiles-per-tap defect.
#[test]
fn player_projectile_release_emits_ranged_bolt_action_message_end_to_end() {
    use ambition_characters::brain::{ActionRequest, ActorActionMessage};
    use ambition_combat::action_emission::emit_brain_action_messages;
    use bevy::transform::components::Transform;
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    // The seat's frame-mode policy, beside the seat table it interprets.
    app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_message::<ActorActionMessage>();
    let mut player = crate::avatar::home_body_scratch(
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
        .spawn((bundle, Transform::from_xyz(40.0, 60.0, 0.0)))
        .remove::<(
            ambition_characters::brain::ChargesProjectiles,
            ambition_projectiles::PlayerProjectileState,
        )>();
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
            fire: ambition_characters::actor::control::ActorFireRequest { dir, dir_policy, .. },
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
    use ambition_characters::brain::{ActionRequest, ActorActionMessage, MeleeActionSpec};
    use ambition_combat::action_emission::emit_brain_action_messages;
    use bevy::transform::components::Transform;
    let mut app = App::new();
    app.init_resource::<SeatRawFrames>();
    app.init_resource::<SlotControls>();
    // The seat's frame-mode policy, beside the seat table it interprets.
    app.init_resource::<ambition_characters::control::SeatControlFrameModes>();
    //  and the table it is committed FROM. These are one model:
    // `BrainPlugin` installs both, and a hand-built fixture that takes
    // only the destination is describing a composition that cannot exist.
    app.init_resource::<SeatRawFrames>();
    app.add_message::<ActorActionMessage>();
    let mut player = crate::avatar::home_body_scratch(
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

#[test]
fn two_driven_bodies_each_regenerate_their_own_mana() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = mana_regen_app(0.5, Some(ambition_abilities::mana::REGEN_PER_SEC));
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));

    let drained = |app: &mut App, slot: u8, sim: &str| -> Entity {
        let body = app
            .world_mut()
            .spawn((
                ambition_abilities::mana::bank(),
                DrivingParticipant(PlayerSlot(slot)),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
            ))
            .id();
        drain_mana(app, body, 40.0);
        body
    };
    let a = drained(&mut app, 0, "seat_a");
    let b = drained(&mut app, 1, "seat_b");
    let before = mana_of;
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

/// A walk to a mark is on the side axis of the walker, and so is the facing
/// the walk gives it.
///
/// The control is normal gravity. The subject is flipped gravity, where the
/// side axis points to world -x. A mark at world +x is then on the NEGATIVE
/// side of the body: its throttle and its facing are both -1. A facing that
/// was the sign of world x was +1 there, and the body walked to its mark
/// backward. The speed a free mover reads stays toward the mark in the world.
#[test]
fn a_commanded_walk_faces_its_mark_on_the_side_axis_of_the_walker() {
    use ambition_characters::control::CommandedMove;

    let walk = CommandedMove {
        target: ae::Vec2::new(200.0, 50.0),
        speed: 60.0,
        arrive_tolerance: 2.0,
    };
    for (down, throttle_and_facing) in [(ae::Vec2::new(0.0, 1.0), 1.0), (ae::Vec2::new(0.0, -1.0), -1.0)] {
        let mut control = ambition_characters::actor::control::ActorControlFrame::default();
        // The body looks away from the mark at the start.
        let looks_away = -throttle_and_facing;
        walk.steer(
            ae::Vec2::new(100.0, 50.0),
            looks_away,
            Some((120.0, ae::MotionFrame::from_direction(down, 900.0))),
            &mut control,
        );
        assert_eq!(
            (control.locomotion.x, control.facing),
            (0.5 * throttle_and_facing, throttle_and_facing),
            "down {down:?}: the throttle and the facing of the walk"
        );
        assert_eq!(control.velocity_target, ae::WorldVec2::new(60.0, 0.0), "down {down:?}");
    }
}
