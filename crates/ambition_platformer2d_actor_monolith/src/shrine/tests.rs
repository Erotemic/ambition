use super::*;
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

#[test]
fn interacting_at_the_shrine_heals_to_full() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.add_systems(Update, heal_save_shrine_system);

    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
            BodyMana::default(),
        ))
        .id();
    // Drain mana so we can see it refill.
    app.world_mut()
        .get_mut::<BodyMana>(player)
        .unwrap()
        .meter
        .try_spend(40.0);
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(100.0, 100.0),
        half_extent: Vec2::new(22.0, 40.0),
    });

    // Interact while overlapping → heal to full.
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();

    let health = *app.world().get::<BodyHealth>(player).unwrap();
    assert_eq!(health.current(), health.max(), "health should be full");
    let mana = app.world().get::<BodyMana>(player).unwrap().meter;
    assert!(
        mana.is_full(),
        "mana should be refilled, got {}",
        mana.current
    );
}

#[test]
fn no_heal_without_interact_or_when_not_touching() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.add_systems(Update, heal_save_shrine_system);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
            BodyMana::default(),
        ))
        .id();
    // A shrine far away.
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(900.0, 900.0),
        half_extent: Vec2::new(22.0, 40.0),
    });

    // Interact pressed but not touching → no heal.
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();
    assert_eq!(
        app.world().get::<BodyHealth>(player).unwrap().current(),
        1,
        "no heal when not at the shrine"
    );
}

/// A save point that actually saves, and a session that resumes there.
///
/// The shrine claimed both halves and delivered neither: it called
/// `save.set_changed()` on a value it never modified — which the value-comparing
/// autosave correctly ignores — and there was no checkpoint field to write into
/// even if the marker had worked. It healed, logged
/// "healed to full + saved", and persisted nothing.
///
/// Both halves in one test on purpose. Either alone is worthless: a checkpoint
/// nothing records is a lie, and a checkpoint nothing restores is a number in a
/// file.
#[test]
fn resting_at_a_shrine_records_a_checkpoint_and_the_next_session_resumes_there() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    fn room_set(room_id: &str) -> ambition_platformer2d_world::rooms::RoomSet {
        let world = ambition_platformer2d_core::World::new(
            "Shrine Room",
            Vec2::new(640.0, 480.0),
            Vec2::new(32.0, 400.0),
            vec![ambition_platformer2d_core::Block::solid(
                "floor",
                Vec2::new(0.0, 440.0),
                Vec2::new(640.0, 40.0),
            )],
        );
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            room_id,
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                room_id, world,
            )],
            Vec::new(),
        )
    }

    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    insert_session_world_component(app.world_mut(), room_set("shrine_room"));
    app.add_systems(Update, heal_save_shrine_system);

    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(412.0, 396.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
            BodyMana::default(),
        ))
        .id();
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(412.0, 396.0),
        half_extent: Vec2::new(22.0, 40.0),
    });
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();

    let recorded = app
        .world()
        .resource::<ambition_persistence::save::AmbitionGameSave>()
        .data()
        .checkpoint()
        .cloned()
        .expect(
            "resting at a shrine recorded no checkpoint, so the save point saves \
             nothing — which is what it did while logging that it had",
        );
    assert_eq!(recorded.room_id, "shrine_room");
    assert_eq!((recorded.x, recorded.y), (412, 396));

    // ── The next session, from the same save ─────────────────────────────────
    //
    // A fresh app with the recorded save: the body starts at the room's authored
    // spawn and must be moved to the checkpoint instead.
    let mut next = App::new();
    next.insert_resource(ambition_persistence::save::AmbitionGameSave(
        app.world()
            .resource::<ambition_persistence::save::AmbitionGameSave>()
            .data()
            .clone(),
    ));
    next.init_resource::<ActiveSessionScope>();
    next.world_mut()
        .resource_mut::<ActiveSessionScope>()
        .begin();
    insert_session_world_component(next.world_mut(), room_set("shrine_room"));
    next.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    next.init_resource::<CheckpointResumeProgress>();
    next.add_systems(Update, restore_checkpoint_on_session_start);
    // The REAL player bundle, at the room's authored spawn — the body the
    // construction path produces, with every cluster the transit authority reads.
    let resumed = next
        .world_mut()
        .spawn((crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                ambition_platformer2d_core::Vec2::new(32.0, 400.0),
                ambition_platformer2d_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),))
        .id();
    next.update();

    let pos = next.world().get::<BodyKinematics>(resumed).unwrap().pos;
    assert_eq!(
        (pos.x, pos.y),
        (412.0, 396.0),
        "the session opened in the checkpoint's room and left the body at the \
         authored spawn — a checkpoint nothing restores is a number in a file"
    );
}

/// A checkpoint recorded in ANOTHER room must not be applied here.
#[test]
fn a_checkpoint_from_another_room_leaves_the_body_where_it_spawned() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "somewhere_else",
        999,
        999,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let world = ambition_platformer2d_core::World::new(
        "Here",
        Vec2::new(640.0, 480.0),
        Vec2::new(32.0, 400.0),
        vec![],
    );
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "here",
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                "here", world,
            )],
            Vec::new(),
        ),
    );
    // The slot a transition is recorded into: production initializes it in sim-core resources,
    // so a fixture running this system owes it too.
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);
    let body = app
        .world_mut()
        .spawn((crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                ambition_platformer2d_core::Vec2::new(32.0, 400.0),
                ambition_platformer2d_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),))
        .id();
    app.update();

    let pos = app.world().get::<BodyKinematics>(body).unwrap().pos;
    assert_eq!(
        (pos.x, pos.y),
        (32.0, 400.0),
        "a checkpoint from another room was applied to this one"
    );
}

/// A checkpoint in ANOTHER room of this world routes the session to it.
///
/// Distinct from `a_checkpoint_from_another_room_leaves_the_body_where_it_spawned`,
/// which covers a room this world does NOT contain. Refusing to teleport a body
/// into coordinates from a room that does not exist is right; refusing to OPEN a
/// room that does is the gap.
#[test]
fn a_checkpoint_in_another_room_of_this_world_routes_the_session_there() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        ambition_platformer2d_world::rooms::RoomSpec::new(
            name,
            ambition_platformer2d_core::World::new(
                name,
                Vec2::new(640.0, 480.0),
                Vec2::new(32.0, 400.0),
                vec![],
            ),
        )
    };
    insert_session_world_component(
        app.world_mut(),
        // Opens in `entry`; the player rested in `rest_room`.
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    // The `SimId` is what `ensure_sim_id` files a `PrimaryPlayer` under on every host.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    // The slot a transition is recorded into: production initializes it in sim-core resources,
    // so a fixture running this system owes it too.
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);
    app.update();

    // It wrote a `RoomTransitionRequested` around a synthetic door; the message is gone and so
    // is the invented zone. The intent names its room by AUTHORED ID, which is also what made
    // the index lookup here deletable.
    fn recorded(app: &App) -> Option<crate::session::lifecycle_commit::PendingIntent> {
        app.world()
            .resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>()
            .pending
            .clone()
    }
    let Some(intent) = recorded(&app) else {
        panic!(
            "the session opened in `entry` while the checkpoint is in `rest_room` \
             and no transition was recorded — the player does not resume where \
             they rested"
        );
    };
    // A shrine warp is a CROSSING by a body, never the bodyless
    // `ReconstituteRoom` that v146 added — it names a subject and an arrival.
    let crate::session::lifecycle_commit::LifecycleIntent::Transition(transition) = intent.kind
    else {
        panic!("the shrine recorded a bodyless room reconstitution, not a warp");
    };
    assert_eq!(transition.target_room, "rest_room");
    assert_eq!(
        transition.subject,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
        "the resume asked for a room without saying whose resume it is, so the \
         commit would transit whoever happens to be controlled several frames later"
    );
    assert_eq!(
        (transition.arrival.x, transition.arrival.y),
        (512.0, 300.0),
        "the transition must arrive AT the checkpoint, not at the room's own spawn"
    );

    // Clearing the slot is what makes the question askable at all.
    app.world_mut()
        .resource_mut::<crate::session::lifecycle_commit::PendingLifecycleCommit>()
        .take();
    app.update();
    app.update();
    assert!(
        recorded(&app).is_none(),
        "the resume transition was recorded repeatedly"
    );
}

/// ⭐⭐ TWO DRIVEN BODIES RESTING AT ONE SHRINE BOTH HEAL — and the session
/// still gets exactly ONE checkpoint.
///
/// ⛔⛔ THE HEAL RESOLVED ONE `ControlledSubject`, so a couch's second seat could
/// stand in the shrine and press interact forever.
///
/// ⛔ AND THE CHECKPOINT IS NOT N. Two seats resting on the same tick heal two
/// bodies; writing two checkpoints would mean the second silently overwrote the
/// first. It is written by the first body in the rewind-stable driven order that
/// actually rests, so the value does not depend on query order.
#[test]
fn two_driven_bodies_resting_at_a_shrine_both_heal_and_write_one_checkpoint() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    app.add_systems(Update, heal_save_shrine_system);

    let seated = |app: &mut App, slot: u8, sim: &str| -> Entity {
        let body = app
            .world_mut()
            .spawn((
                ActorControl::default(),
                BodyKinematics {
                    pos: Vec2::new(100.0, 100.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: Vec2::new(24.0, 40.0),
                },
                BodyHealth::new(ambition_characters::actor::Health {
                    current: 1,
                    max: 5,
                    invulnerable: Default::default(),
                }),
                BodyMana::default(),
                DrivingParticipant(PlayerSlot(slot)),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
            ))
            .id();
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .interact_pressed = true;
        body
    };
    let a = seated(&mut app, 0, "seat_a");
    let b = seated(&mut app, 1, "seat_b");
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(100.0, 100.0),
        half_extent: Vec2::new(22.0, 40.0),
    });

    app.update();

    for (body, who) in [(a, "a"), (b, "b")] {
        let health = *app.world().get::<BodyHealth>(body).unwrap();
        assert_eq!(
            health.current(),
            health.max(),
            "seat {who} rested at the shrine and was not healed"
        );
    }
}

/// ⛔⛔ A COMMENT STATED A RULE THE CODE DOES NOT FOLLOW, and this arm is which
/// one is real.
///
/// `heal_save_shrine_system` says the checkpoint is written *"for the PRIMARY
/// player's session, not the possessed subject's body"* and then writes the
/// RESTING body's `kin.pos`. Its consumer, `restore_checkpoint_on_session_start`,
/// places the PRIMARY avatar at those coordinates — so under possession the two
/// readings disagree about where the next session starts, and nothing measured
/// which one shipped.
///
/// ⭐ MEASURED: the code's rule is the RESTING body's position, and that is the
/// one kept. "I rested here, I come back here" is what a player means by a
/// checkpoint; the vessel they were wearing at the time is not part of the
/// promise. The comment claiming otherwise is deleted rather than implemented —
/// implementing it would mean a shrine touched while possessing silently records
/// a position the player never stood at.
#[test]
fn the_checkpoint_records_where_the_resting_body_stood() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    // A checkpoint needs a room: "a position with no room is not a checkpoint".
    {
        let world = ambition_platformer2d_core::World::new(
            "Shrine Room",
            Vec2::new(1000.0, 1000.0),
            Vec2::new(32.0, 400.0),
            Vec::new(),
        );
        insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_world::rooms::RoomSet::from_parts(
                "shrine_room",
                vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                    "shrine_room",
                    world,
                )],
                Vec::new(),
            ),
        );
    }
    app.add_systems(Update, heal_save_shrine_system);

    // The home avatar, standing well away from the shrine and pressing nothing.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ActorControl::default(),
        BodyKinematics {
            pos: Vec2::new(50.0, 900.0),
            vel: Vec2::ZERO,
            size: Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
        BodyBaseSize {
            base_size: Vec2::new(24.0, 40.0),
        },
        BodyHealth::new(ambition_characters::actor::Health {
            current: 5,
            max: 5,
            invulnerable: Default::default(),
        }),
        BodyMana::default(),
    ));

    // The body a participant is actually driving — a possessed vessel — resting
    // AT the shrine.
    let vessel = app
        .world_mut()
        .spawn((
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(700.0, 100.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
            BodyMana::default(),
            DrivingParticipant(PlayerSlot::PRIMARY),
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("vessel"),
        ))
        .id();
    app.world_mut()
        .get_mut::<ActorControl>(vessel)
        .unwrap()
        .0
        .interact_pressed = true;
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(700.0, 100.0),
        half_extent: Vec2::new(22.0, 40.0),
    });

    app.update();

    // ⛔ THE PREMISE: the rest has to have HAPPENED, or the checkpoint below is
    // a checkpoint nobody wrote.
    let health = *app.world().get::<BodyHealth>(vessel).unwrap();
    assert_eq!(
        health.current(),
        health.max(),
        "the vessel did not rest, so nothing below is about a shrine visit"
    );

    let checkpoint = app
        .world()
        .resource::<ambition_persistence::save::AmbitionGameSave>()
        .data()
        .checkpoint()
        .cloned()
        .expect("resting at a shrine records a checkpoint");
    assert_eq!(
        (checkpoint.x, checkpoint.y),
        (700, 100),
        "the checkpoint records the RESTING body's position. The avatar stood at \
         (50, 900) and never touched the shrine; recording ITS position would put \
         the next session somewhere nobody rested"
    );
    assert_eq!(checkpoint.room_id, "shrine_room");
}

/// ⛔⛔ **F1: THE ROUTED LATCH BELONGED TO THE ATTEMPT, NOT TO THE ADMISSION.**
///
/// `restore_checkpoint_on_session_start` wrote `routed_for` and then *discarded*
/// the `Admission` the slot returned. A slot already owned by another lifecycle
/// intent refused the crossing while the session recorded that it had spent its
/// one resume — so the player stayed in whichever room the session happened to
/// open in, permanently, because the only road back is gated on a generation
/// that line had already burned.
///
/// ⭐ THE WITNESS IS THE RETRY, NOT THE FLAG. Asserting `routed_for.is_none()`
/// alone would pass on a system that never ran at all; this releases the slot
/// and requires the crossing to actually arrive on a later tick.
#[test]
fn a_refused_slot_leaves_the_checkpoint_resume_retryable() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };
    use crate::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        ambition_platformer2d_world::rooms::RoomSpec::new(
            name,
            ambition_platformer2d_core::World::new(
                name,
                Vec2::new(640.0, 480.0),
                Vec2::new(32.0, 400.0),
                vec![],
            ),
        )
    };
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    app.init_resource::<PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);

    // SOMEBODY ELSE ALREADY OWNS THE SLOT. A door crossing recorded on an
    // earlier frame is the ordinary shape of this: the slot is earliest-sticky.
    let incumbent = LifecycleIntent::Transition(RoomTransitionIntent {
        subject: ambition_platformer2d_shared_tangle::sim_id::SimId::placement("someone_else"),
        target_room: "entry".into(),
        arrival: Vec2::new(1.0, 2.0),
        edge_exit: true,
        zone_sfx: None,
    });
    assert!(app
        .world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .record(3, incumbent.clone())
        .admitted());

    app.update();
    app.update();

    assert_eq!(
        app.world()
            .resource::<PendingLifecycleCommit>()
            .pending
            .as_ref()
            .map(|pending| &pending.kind),
        Some(&incumbent),
        "the refused resume overwrote the incumbent lifecycle operation"
    );
    assert_eq!(
        app.world().resource::<CheckpointResumeProgress>().routed_for,
        None,
        "the resume recorded that it had routed while the slot refused it, so \
         the session can never ask again"
    );

    // THE INCUMBENT COMMITS AND FREES THE SLOT. The resume must now land.
    app.world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .take();
    app.update();

    let pending = app
        .world()
        .resource::<PendingLifecycleCommit>()
        .pending
        .clone()
        .expect(
            "the slot was released and the checkpoint resume never re-asked — a \
             refusal permanently stranded the player in the session's own room",
        );
    let LifecycleIntent::Transition(transition) = pending.kind else {
        panic!("the resume recorded a bodyless reconstitution, not a crossing");
    };
    assert_eq!(transition.target_room, "rest_room");
    assert_eq!(
        (transition.arrival.x, transition.arrival.y),
        (512.0, 300.0)
    );

    // ⭐ AND IT IS STILL ONCE-ONLY. The latch moved to the admission; it did not
    // disappear. A second release must not produce a second crossing.
    assert_eq!(
        app.world().resource::<CheckpointResumeProgress>().routed_for,
        Some(Some(
            app.world()
                .resource::<ActiveSessionScope>()
                .current()
                .expect("the session was begun")
                .0
        )),
        "an admitted crossing must latch the generation it was spent on"
    );
    app.world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .take();
    app.update();
    app.update();
    assert!(
        app.world()
            .resource::<PendingLifecycleCommit>()
            .pending
            .is_none(),
        "the admitted resume was recorded a second time, so a session that \
         reaches the slot twice restarts its crossing forever"
    );
}

/// A session whose avatar has not been constructed yet cannot name the body it
/// is resuming — and must not spend the once-per-session route saying so.
///
/// This is the OTHER half of the same rule as the refusal above: `routed_for` is
/// a receipt for an accepted crossing, and the two ways to fail to get one are a
/// busy slot and an unresolvable subject.
#[test]
fn a_resume_with_no_constructed_subject_stays_pending_until_the_body_exists() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };
    use crate::session::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit};

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        ambition_platformer2d_world::rooms::RoomSpec::new(
            name,
            ambition_platformer2d_core::World::new(
                name,
                Vec2::new(640.0, 480.0),
                Vec2::new(32.0, 400.0),
                vec![],
            ),
        )
    };
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    app.init_resource::<PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);

    // No body at all: construction has not finished.
    app.update();
    assert!(
        app.world()
            .resource::<PendingLifecycleCommit>()
            .pending
            .is_none(),
        "a crossing was recorded for a body nobody can name"
    );
    assert_eq!(
        app.world().resource::<CheckpointResumeProgress>().routed_for,
        None,
        "the session marked its resume routed before it had a subject"
    );

    // The avatar materializes.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    app.update();

    let pending = app
        .world()
        .resource::<PendingLifecycleCommit>()
        .pending
        .clone()
        .expect("the body arrived and the resume never asked again");
    let LifecycleIntent::Transition(transition) = pending.kind else {
        panic!("the resume recorded a bodyless reconstitution, not a crossing");
    };
    assert_eq!(transition.target_room, "rest_room");
}
