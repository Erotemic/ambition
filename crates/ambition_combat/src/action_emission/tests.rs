use ambition_characters::brain::action_set::{ActionRequest, MeleeActionSpec, SwipeSpec};
use ambition_characters::brain::{
    observe_brain_action_counter, ActionSet, ActorActionMessage, Brain, BrainActionCounter,
    MeleeBruteCfg, MeleeBruteState, StateMachineCfg,
};
use ambition_platformer2d_core as ae;

use super::emit_brain_action_messages;

#[test]
fn emit_brain_action_messages_skips_entities_missing_components() {
    // Resolver queries Brain + ActionSet + ambition_characters::control::ActorControl +
    // BodyKinematics. Entities missing any one are skipped silently
    // (Bevy query filter). Pins this behavior so a future
    // refactor that loosens the filter doesn't accidentally
    // process partially-spawned entities and panic on the
    // missing fields.
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, emit_brain_action_messages);
    // Entity 1: missing ActionSet.
    let _e1 = app
        .world_mut()
        .spawn((
            Brain::stand_still(),
            ambition_characters::control::ActorControl::default(),
            ambition_platformer2d_core::BodyKinematics::default(),
        ))
        .id();
    // Entity 2: missing BodyKinematics.
    let _e2 = app
        .world_mut()
        .spawn((
            Brain::stand_still(),
            ambition_characters::control::ActorControl::default(),
            ActionSet::peaceful(),
        ))
        .id();
    app.update();
    let messages = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
    assert_eq!(
        messages.iter_current_update_messages().count(),
        0,
        "partial entities should produce zero messages",
    );
}

#[test]
fn emit_brain_action_messages_handles_many_actors() {
    // Stress: 50 actors with Brain + ActionSet + BodyKinematics all
    // wanting to attack this tick. The resolver should emit
    // 50 messages in one update with no panic or quadratic
    // slowdown.
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, emit_brain_action_messages);
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    for i in 0..50 {
        app.world_mut().spawn((
            Brain::stand_still(),
            ambition_characters::control::ActorControl(frame),
            actions.clone(),
            ambition_platformer2d_core::BodyKinematics {
                pos: ae::Vec2::new(i as f32 * 10.0, 0.0),
                facing: 1.0,
                ..Default::default()
            },
        ));
    }
    app.update();
    let messages = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
    let count = messages.iter_current_update_messages().count();
    assert_eq!(count, 50, "expected 50 messages, got {count}");
}

/// observe_brain_action_counter sums per-frame messages into
/// the resource. Pins the counter system shape — sandbox wiring
/// or HUD readouts can rely on `last_frame` reflecting the
/// resolver's per-frame output count.
#[test]
fn observe_brain_action_counter_sums_per_frame_messages() {
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.init_resource::<BrainActionCounter>();
    app.add_systems(
        Update,
        (emit_brain_action_messages, observe_brain_action_counter).chain(),
    );
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    app.world_mut().spawn((
        Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        }),
        ambition_characters::control::ActorControl(frame),
        actions,
        ambition_platformer2d_core::BodyKinematics::default(),
    ));
    app.update();
    let counter = app.world().resource::<BrainActionCounter>();
    assert_eq!(counter.last_frame, 1);
    assert_eq!(counter.total, 1);
    // Run another tick — counter accumulates.
    app.update();
    let counter = app.world().resource::<BrainActionCounter>();
    assert_eq!(counter.last_frame, 1);
    assert_eq!(counter.total, 2);
}

/// emit_brain_action_messages walks every Brain/ActionSet/
/// ambition_characters::control::ActorControl + BodyKinematics entity and writes a message per resolved
/// ActionRequest. Pins that the resolver system, scheduled in
/// PlayerInput, observes the brain output correctly.
#[test]
fn emit_brain_action_messages_writes_one_message_per_request() {
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, emit_brain_action_messages);
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.facing = 1.0;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let entity = app
        .world_mut()
        .spawn((
            Brain::StateMachine(StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            }),
            ambition_characters::control::ActorControl(frame),
            actions,
            ambition_platformer2d_core::BodyKinematics {
                pos: ae::Vec2::new(50.0, 100.0),
                facing: 1.0,
                ..Default::default()
            },
        ))
        .id();
    app.update();
    let mut messages = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
    let received: Vec<_> = messages.drain().collect();
    assert_eq!(received.len(), 1, "expected one Melee message");
    assert_eq!(received[0].actor, entity);
    match received[0].request.clone() {
        ActionRequest::Melee {
            origin,
            facing,
            spec: MeleeActionSpec::Swipe(_),
            ..
        } => {
            assert_eq!(origin, ae::Vec2::new(50.0, 100.0));
            assert_eq!(facing, 1.0);
        }
        other => panic!("expected Melee::Swipe, got {:?}", other),
    }
}


/// One press, one owner: a body whose moveset authors `ranged` fires through the
/// move's timed event, so the flat `fire → Ranged` emission must stand aside or the
/// shot leaves twice. The control arm is the same body with a melee-only moveset.
#[test]
fn a_ranged_moveset_owns_the_fire_intent_and_the_flat_emitter_stands_aside() {
    use ambition_characters::brain::action_set::RangedActionSpec;
    use bevy::prelude::*;

    let kit = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ranged: Some(RangedActionSpec::rock(300.0, 1)),
        ..Default::default()
    };
    let routed = ambition_characters::moveset_prefabs::build_actor_moveset(
        None,
        kit.melee.as_ref(),
        kit.ranged.as_ref(),
        None,
    )
        .expect("melee + ranged → a moveset");
    let mut melee_only = routed.clone();
    melee_only.verbs.remove(crate::moveset::RANGED_VERB);

    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.fire = Some(
        ambition_characters::actor::control::ActorFireRequest::controlled_body_local(ae::Vec2::X),
    );

    let flat_shots = |moveset: ambition_entity_catalog::MovesetContract| {
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, emit_brain_action_messages);
        app.world_mut().spawn((
            ambition_characters::control::ActorControl(frame),
            kit.clone(),
            ae::BodyKinematics::default(),
            crate::moveset::ActorMoveset(moveset),
        ));
        app.update();
        app.world()
            .resource::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .iter_current_update_messages()
            .filter(|m| matches!(m.request, ActionRequest::Ranged { .. }))
            .count()
    };
    assert_eq!(
        flat_shots(melee_only),
        1,
        "control: a body with no ranged move fires through the flat emitter"
    );
    assert_eq!(
        flat_shots(routed),
        0,
        "the moveset's ranged move owns the press; a flat shot here is the second projectile"
    );
}
