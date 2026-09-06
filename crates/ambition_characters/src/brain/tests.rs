use super::*;
use crate::brain::action_set::RangedStyle;

/// Tick a `Brain` through the NINE simple arms, for tests that build one.
///
/// ⭐ TEST-LOCAL ON PURPOSE. `Brain::tick` was deleted when the dispatcher split
/// (D168): the outer match over all twelve variants now lives in the actor
/// monolith, because three of the arms are destined for crates above this one.
/// Nothing in production needed the inherent method — it had zero callers here —
/// so this is a convenience for fixtures, not the road back.
fn tick_simple(
    brain: &mut Brain,
    snapshot: &BrainSnapshot,
    out: &mut crate::actor::control::ActorControlFrame,
) -> bool {
    let Brain::StateMachine(cfg) = brain;
    crate::brain::state_machine::tick_simple_state_machine(cfg, snapshot, out)
}

#[test]
fn brain_display_contains_label() {
    // Display impl for state-machine brains embeds the label —
    // a future label rename should automatically reflect in
    // Display output. Pin the relationship.
    for template in [
        StateMachineCfg::StandStill,
        StateMachineCfg::Patrol {
            cfg: PatrolCfg::NPC_DEFAULT,
            state: PatrolState::default(),
        },
        StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
        },
    ] {
        let b = Brain::StateMachine(template);
        let display = format!("{}", b);
        let label = b.label();
        assert!(
            display.contains(label),
            "Display '{}' should contain label '{}'",
            display,
            label,
        );
    }
}

#[test]
fn emit_brain_action_messages_skips_entities_missing_components() {
    // Resolver queries Brain + ActionSet + crate::control::ActorControl +
    // ActorPose. Entities missing any one are skipped silently
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
            crate::control::ActorControl::default(),
            crate::actor::ActorPose::default(),
        ))
        .id();
    // Entity 2: missing ActorPose.
    let _e2 = app
        .world_mut()
        .spawn((
            Brain::stand_still(),
            crate::control::ActorControl::default(),
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
    // Stress: 50 actors with Brain + ActionSet + ActorPose all
    // wanting to attack this tick. The resolver should emit
    // 50 messages in one update with no panic or quadratic
    // slowdown.
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, emit_brain_action_messages);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    for i in 0..50 {
        app.world_mut().spawn((
            Brain::stand_still(),
            crate::control::ActorControl(frame),
            actions.clone(),
            crate::actor::ActorPose {
                center: ae::Vec2::new(i as f32 * 10.0, 0.0),
                feet: ae::Vec2::new(i as f32 * 10.0, 24.0),
                facing: 1.0,
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

#[test]
fn actor_control_default_is_neutral_frame() {
    // crate::control::ActorControl Default = frame.neutral. Pins the
    // "fresh-spawn crate::control::ActorControl has zero intent" baseline so
    // the EFFECTS consumer that reads it before any
    // brain tick has run won't spuriously fire actions.
    let ac = crate::control::ActorControl::default();
    assert_eq!(ac.0, crate::actor::control::ActorControlFrame::neutral());
    assert!(!ac.0.wants_any_action());
}

#[test]
fn brain_plugin_registers_message_and_counter_resource() {
    // Pins the BrainPlugin contract: installs ActorActionMessage
    // + BrainActionCounter resource. A future refactor that
    // splits the plugin or accidentally drops a registration
    // trips this test.
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_plugins(BrainPlugin);
    // Message resource present.
    let _msg = app
        .world()
        .get_resource::<bevy::ecs::message::Messages<ActorActionMessage>>()
        .expect("ActorActionMessage registered");
    // Counter resource present + default-initialized.
    let counter = app
        .world()
        .get_resource::<BrainActionCounter>()
        .expect("BrainActionCounter registered");
    assert_eq!(counter.total, 0);
    assert_eq!(counter.last_frame, 0);
}

#[test]
fn brain_swap_via_commands_replaces_existing_component() {
    // Pins the runtime brain-swap contract — Bevy's
    // commands.entity(e).insert(Brain) replaces the existing
    // Brain component in place rather than producing a
    // duplicate-component panic or silently ignoring the
    // insert. This is the path damage.rs hostile-flip uses.
    use bevy::prelude::*;
    let mut app = App::new();
    let entity = app
        .world_mut()
        .spawn((
            Brain::stand_still(),
            crate::control::ActorControl::default(),
        ))
        .id();
    // Initially StandStill.
    let world = app.world();
    let brain = world.get::<Brain>(entity).expect("Brain attached");
    assert!(matches!(
        brain,
        Brain::StateMachine(StateMachineCfg::StandStill)
    ));
    // Swap to MeleeBrute via the same commands.insert path.
    app.world_mut()
        .entity_mut(entity)
        .insert(Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        }));
    let brain = app
        .world()
        .get::<Brain>(entity)
        .expect("Brain still attached");
    assert!(matches!(
        brain,
        Brain::StateMachine(StateMachineCfg::MeleeBrute { .. })
    ));
}

#[test]
fn brain_tick_survives_100_ticks_for_every_template() {
    // Smoke test: tick each brain template 100 times with a
    // moving target and verify no panic / NaN propagation /
    // state corruption. Pins that the brain dispatch is safe
    // for a sustained game-length tick run, not just one
    // tick.
    let templates: Vec<StateMachineCfg> = vec![
        StateMachineCfg::StandStill,
        StateMachineCfg::Patrol {
            cfg: PatrolCfg::NPC_DEFAULT,
            state: PatrolState::default(),
        },
        StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
        },
        StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        },
        StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg::RANGER_DEFAULT,
            state: SkirmisherState::default(),
        },
        StateMachineCfg::Sniper {
            cfg: SniperCfg::DEFAULT,
            state: SniperState::default(),
        },
    ];
    for template in templates {
        let mut brain = Brain::StateMachine(template);
        for i in 0..100 {
            let mut snap = BrainSnapshot::idle();
            snap.actor_pos = ae::Vec2::new((i as f32) * 0.5, 0.0);
            snap.target_pos = ae::Vec2::new(100.0 + (i as f32) * 0.5, 0.0);
            snap.sim_time = (i as f32) / 60.0;
            snap.dt = 1.0 / 60.0;
            let mut frame = crate::actor::control::ActorControlFrame::neutral();
            tick_simple(&mut brain, &snap, &mut frame);
            // No NaN propagation.
            assert!(frame.locomotion.x.is_finite());
            assert!(frame.locomotion.y.is_finite());
            assert!(frame.facing.is_finite());
        }
    }
}

#[test]
fn brain_tick_is_deterministic_given_same_snapshot() {
    // The brain interface is pure(-ish): same brain + same
    // snapshot → same output (modulo internal state mutation).
    // Pin determinism so RL training + trace replay can rely
    // on reproducibility.
    let snap = BrainSnapshot::idle();
    let mut a = Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg::STRIKER_DEFAULT,
        state: MeleeBruteState::default(),
    });
    let mut b = a.clone();
    let mut frame_a = crate::actor::control::ActorControlFrame::neutral();
    let mut frame_b = crate::actor::control::ActorControlFrame::neutral();
    tick_simple(&mut a, &snap, &mut frame_a);
    tick_simple(&mut b, &snap, &mut frame_b);
    assert_eq!(frame_a, frame_b, "same brain + same snapshot → same frame");
}

// Note: `shadow_tick_brain*` helpers + the `CombatTimers` struct
// were removed when the hostile/boss runtimes became the
// single-producer-of-intent path; the tests that pinned their
// behavior went with them.

#[test]
fn brain_display_includes_the_label_for_a_state_machine() {
    let sm = Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg::STRIKER_DEFAULT,
        state: MeleeBruteState::default(),
    });
    assert_eq!(format!("{}", sm), "StateMachine(melee_brute)");

    let stand = Brain::stand_still();
    assert_eq!(format!("{}", stand), "StateMachine(stand_still)");
}

#[test]
fn brain_stand_still_ctor_matches_variant() {
    let b = Brain::stand_still();
    assert!(matches!(
        b,
        Brain::StateMachine(StateMachineCfg::StandStill)
    ));
    assert!(!b.is_hostile());
    assert_eq!(b.label(), "stand_still");
}

#[test]
fn brain_npc_patrol_ctor_inherits_spawn_and_radius() {
    let b = Brain::npc_patrol(120.0, 40.0);
    match &b {
        Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
            assert_eq!(cfg.lane.center_x, 120.0);
            assert_eq!(cfg.lane.radius_px, 40.0);
            assert_eq!(cfg.aggressiveness, 0.0);
        }
        other => panic!("expected Patrol, got {:?}", other),
    }
    assert!(!b.is_hostile());
}

/// Driver identity is a body component and does not replace the actor's brain.
#[test]
fn the_driving_participant_is_a_component_on_the_body_not_a_brain_variant() {
    let mut world = bevy::prelude::World::new();
    let driven = world
        .spawn((
            Brain::stand_still(),
            crate::control::DrivingParticipant(crate::control::PlayerSlot(2)),
        ))
        .id();
    let cpu = world.spawn(Brain::stand_still()).id();

    assert_eq!(
        world
            .get::<crate::control::DrivingParticipant>(driven)
            .map(|d| d.0),
        Some(crate::control::PlayerSlot(2))
    );
    assert_eq!(
        world
            .get::<crate::control::DrivingParticipant>(cpu)
            .map(|d| d.0),
        None
    );
    assert!(
        matches!(world.get::<Brain>(driven), Some(Brain::StateMachine(_))),
        "the driven body lost its own policy — nothing may swap a brain to say \
         who is driving"
    );
}

#[test]
fn actor_action_message_predicates_match_request_variant() {
    use bevy::prelude::*;
    // Use World::spawn() to get a real Entity since Bevy 0.18
    // removed Entity::from_raw from the public API.
    let mut world = World::new();
    let actor = world.spawn(()).id();
    let m_melee = ActorActionMessage {
        actor,
        request: ActionRequest::Melee {
            spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
            origin: ae::Vec2::ZERO,
            facing: 1.0,
            attack_axis: ae::LocalAxes::ZERO,
        },
    };
    assert!(m_melee.is_melee());
    assert!(!m_melee.is_ranged());
    assert!(!m_melee.is_special());

    let m_special = ActorActionMessage {
        actor,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special("bubble_shield".to_string()),
            params: Default::default(),
        },
    };
    assert!(m_special.is_special());
    assert!(!m_special.is_melee());
}

#[test]
fn brain_label_is_per_backend() {
    assert_eq!(
        Brain::StateMachine(StateMachineCfg::StandStill).label(),
        "stand_still"
    );
    assert_eq!(
        Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default()
        })
        .label(),
        "melee_brute"
    );
    assert_eq!(
        Brain::StateMachine(StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
        })
        .label(),
        "wanderer"
    );
}

#[test]
fn brain_statemachine_delegates_hostility() {
    let peaceful = Brain::StateMachine(StateMachineCfg::Patrol {
        cfg: PatrolCfg::NPC_DEFAULT,
        state: PatrolState::default(),
    });
    assert!(!peaceful.is_hostile());

    let hostile = Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg::STRIKER_DEFAULT,
        state: MeleeBruteState::default(),
    });
    assert!(hostile.is_hostile());
}

#[test]
fn brain_tick_dispatches_through_enum() {
    // A StandStill brain should produce a neutral frame.
    let mut b = Brain::StateMachine(StateMachineCfg::StandStill);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    out.melee_pressed = true; // pre-poisoned
    tick_simple(&mut b, &BrainSnapshot::idle(), &mut out);
    assert!(!out.melee_pressed);
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
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    app.world_mut().spawn((
        Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        }),
        crate::control::ActorControl(frame),
        actions,
        crate::actor::ActorPose::default(),
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
/// crate::control::ActorControl + ActorPose entity and writes a message per resolved
/// ActionRequest. Pins that the resolver system, scheduled in
/// PlayerInput, observes the brain output correctly.
#[test]
fn emit_brain_action_messages_writes_one_message_per_request() {
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, emit_brain_action_messages);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
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
            crate::control::ActorControl(frame),
            actions,
            crate::actor::ActorPose {
                center: ae::Vec2::new(50.0, 100.0),
                feet: ae::Vec2::new(50.0, 124.0),
                facing: 1.0,
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

/// End-to-end ranged path: a Skirmisher brain ticking past its fire cooldown inside aggro range
/// produces `frame.fire`, and the resolver translates that — gated by the actor's ranged
/// `ActionSet` — into a concrete `ActionRequest::Ranged`. Pins the seam shark-rider archetypes rely
/// on: without it, the `ranged: Some(Bolt(...))` row in `character_archetypes.ron` is silently
/// inert.
#[test]
fn skirmisher_brain_resolves_through_action_set_to_ranged_request() {
    // Inside aggro radius, past cooldown.
    let cfg = SkirmisherCfg::RANGER_DEFAULT;
    let mut brain = Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg,
        state: SkirmisherState::default(),
    });
    let mut snap = BrainSnapshot::idle();
    snap.actor_pos = ae::Vec2::ZERO;
    snap.target_pos = ae::Vec2::new(200.0, 0.0); // inside aggro 320
    snap.sim_time = 5.0; // past fire_cooldown_s 0.8

    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_simple(&mut brain, &snap, &mut frame);
    assert!(
        frame.fire.is_some(),
        "Skirmisher inside aggro + past cooldown must emit fire intent",
    );

    let kit = ActionSet {
        ranged: Some(RangedActionSpec::bolt(500.0, 2)),
        ..Default::default()
    };
    let req = resolve_action_requests(&kit, &frame, snap.actor_pos);
    assert_eq!(req.len(), 1, "exactly one ranged request");
    match req[0].clone() {
        ActionRequest::Ranged {
            spec,
            dir,
            dir_policy,
            ..
        } => {
            assert!(
                matches!(
                    spec,
                    RangedActionSpec {
                        style: RangedStyle::Bolt,
                        ..
                    }
                ),
                "spec should come from the Bolt kit",
            );
            assert!(dir.x > 0.0, "fire direction should point at target");
            assert_eq!(
                dir_policy,
                ae::GameplayFramePolicy::WorldSpace,
                "Skirmisher fires at a direct world-space target vector"
            );
        }
        other => panic!("expected ActionRequest::Ranged, got {:?}", other),
    }
}

/// End-to-end: a MeleeBrute brain ticks at attack range; its
/// emitted frame routes through the actor's ActionSet to a
/// concrete Melee request. Same brain + different ActionSet =
/// different concrete attack (Swipe vs Lunge). This is the
/// possession / multi-body invariant: brains are policy,
/// ActionSets are capability.
#[test]
fn melee_brute_brain_resolves_through_action_set() {
    let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
    let mut brain_a = Brain::StateMachine(StateMachineCfg::MeleeBrute {
        cfg,
        state: MeleeBruteState::default(),
    });
    let mut brain_b = brain_a.clone();
    let mut snap = BrainSnapshot::idle();
    snap.actor_pos = ae::Vec2::ZERO;
    snap.target_pos = ae::Vec2::new(20.0, 0.0); // in attack range

    let mut frame_a = crate::actor::control::ActorControlFrame::neutral();
    let mut frame_b = crate::actor::control::ActorControlFrame::neutral();
    tick_simple(&mut brain_a, &snap, &mut frame_a);
    tick_simple(&mut brain_b, &snap, &mut frame_b);
    assert!(frame_a.melee_pressed);
    assert!(frame_b.melee_pressed);

    let goblin_kit = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let brute_kit = ActionSet {
        melee: Some(MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT)),
        ..Default::default()
    };
    let goblin_req = resolve_action_requests(&goblin_kit, &frame_a, snap.actor_pos);
    let brute_req = resolve_action_requests(&brute_kit, &frame_b, snap.actor_pos);
    assert_eq!(goblin_req.len(), 1);
    assert_eq!(brute_req.len(), 1);
    match (goblin_req[0].clone(), brute_req[0].clone()) {
        (
            ActionRequest::Melee {
                spec: MeleeActionSpec::Swipe(_),
                ..
            },
            ActionRequest::Melee {
                spec: MeleeActionSpec::Lunge(_),
                ..
            },
        ) => {}
        (a, b) => panic!("expected Swipe vs Lunge, got {:?} vs {:?}", a, b),
    }
}

/// It is a FAIRNESS asymmetry, not a rounding error. Two people, two pads, one
/// couch match, and only one of them gets a press that lands between two ticks.
#[test]
fn a_secondary_seats_sub_tick_tap_is_not_swallowed() {
    use ambition_platformer2d_core::ControlFrame;

    let mut latches = crate::control::SlotControlLatches::default();
    let seat_two = crate::control::PlayerSlot(1);

    // Player two taps jump and is already back up by the next rendered frame.
    latches.accumulate(
        seat_two,
        ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..ControlFrame::default()
        },
    );
    latches.accumulate(seat_two, ControlFrame::default());

    let tick = latches.take(seat_two);
    assert!(
        tick.jump_pressed,
        "player two's press edge was swallowed between two ticks — the exact \
         asymmetry this latch exists to remove"
    );
    assert!(
        !tick.jump_held,
        "the LEVEL must be the latest sample; the button is up again"
    );
}

#[test]
fn each_seats_latch_is_its_own() {
    use ambition_platformer2d_core::ControlFrame;

    let mut latches = crate::control::SlotControlLatches::default();
    latches.accumulate(
        crate::control::PlayerSlot(1),
        ControlFrame {
            attack_pressed: true,
            ..ControlFrame::default()
        },
    );

    assert!(latches.peek(crate::control::PlayerSlot(1)).attack_pressed);
    assert!(
        !latches.peek(crate::control::PlayerSlot(2)).attack_pressed,
        "seat two's attack reached seat three"
    );
    assert!(
        !latches.peek(crate::control::PlayerSlot(0)).attack_pressed,
        "seat two's attack reached the PRIMARY seat, which drains a different \
         latch entirely — double-latching slot 0 would hold one press for two \
         ticks"
    );
}

/// The primary seat follows the same rule.
#[test]
fn resetting_a_seat_drops_its_held_levels_not_just_its_edges() {
    use ambition_platformer2d_core::ControlFrame;

    let mut latches = crate::control::SlotControlLatches::default();
    let seat = crate::control::PlayerSlot(1);
    latches.accumulate(
        seat,
        ControlFrame {
            axis_x: 1.0,
            jump_pressed: true,
            jump_held: true,
            ..ControlFrame::default()
        },
    );

    latches.reset(seat);

    let tick = latches.take(seat);
    assert_eq!(tick.axis_x, 0.0, "a held direction survived the pause");
    assert!(!tick.jump_held, "a held button survived the pause");
    assert!(!tick.jump_pressed, "an edge survived the pause");
}
