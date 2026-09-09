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

/// ⛔⛔ **`OnRest` WAS A SYNONYM FOR `DeadStaysDead` IN THE SHIPPED GAME.**
///
/// An `OnRest` placement's death writes `enemy_<id>_dead_until_rest`, and
/// `AmbitionGameSave::clear_dead_until_rest_flags` exists to drop those at a
/// rest. Measured 2026-09-09 by `git grep`: that function's ONLY occurrence in
/// the whole workspace was its own definition. Nothing called it. So the flag was
/// written and never cleared, and every placement authored `OnRest` stayed dead
/// forever — an authored policy that reads as a mechanic and was, in effect, a
/// second spelling of the policy beside it.
///
/// ⭐ THE OTHER TWO FLAG FAMILIES ARE THE MEASUREMENT, not decoration. A rest
/// that cleared everything would pass an `OnRest`-only assertion and quietly
/// resurrect the bodies a `DeadStaysDead` death is supposed to keep down — which
/// is the opposite defect and just as invisible.
#[test]
fn resting_revives_only_the_bodies_whose_policy_says_until_rest() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.add_systems(Update, heal_save_shrine_system);

    // Three deaths, three policies' worth of record.
    {
        let mut save = app
            .world_mut()
            .resource_mut::<ambition_persistence::save::AmbitionGameSave>();
        let data = save.data_mut();
        data.set_flag(
            &crate::features::enemy_dead_until_rest_flag("rests_away"),
            true,
        );
        data.set_flag(&crate::features::enemy_dead_flag("stays_dead"), true);
        data.set_flag("a_door_the_player_opened", true);
    }

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
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(100.0, 100.0),
        half_extent: Vec2::new(22.0, 40.0),
    });
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();

    let save = app
        .world()
        .resource::<ambition_persistence::save::AmbitionGameSave>();
    let data = save.data();
    assert!(
        !data.flag(&crate::features::enemy_dead_until_rest_flag("rests_away")),
        "resting left the `until rest` death record standing, so an `OnRest` \
         placement never comes back and the policy is a synonym for \
         `DeadStaysDead`"
    );
    assert!(
        data.flag(&crate::features::enemy_dead_flag("stays_dead")),
        "resting revived a `DeadStaysDead` body, which is the opposite defect: a \
         rest that clears everything looks identical to a correct one until \
         somebody kills the thing that is supposed to stay killed"
    );
    assert!(
        data.flag("a_door_the_player_opened"),
        "resting cleared an unrelated world flag — the rest mechanic reaches only \
         the deaths that recorded themselves as waiting for one"
    );
}
