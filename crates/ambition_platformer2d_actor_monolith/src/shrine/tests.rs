//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::BodyBaseSize;
use crate::actor::{PlayerEntity, PrimaryPlayer};

#[test]
fn interacting_at_the_shrine_heals_to_full() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d_world::rooms::RoomTransitionRequested>();
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

/// **A save point that actually saves, and a session that resumes there.**
///
/// The shrine claimed both halves and delivered neither: it called
/// `save.set_changed()` on a value it never modified — which the value-comparing
/// autosave correctly ignores — and there was no checkpoint field to write into
/// even if the marker had worked (GPT 5.6, 2026-07-27). It healed, logged
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

    fn room_set(room_id: &str) -> crate::rooms::RoomSet {
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
        crate::rooms::RoomSet::from_parts(
            room_id,
            vec![crate::rooms::RoomSpec::new(room_id, world)],
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
        .checkpoint
        .clone()
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
    next.add_message::<ambition_platformer2d_world::rooms::RoomTransitionRequested>();
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

/// A checkpoint recorded in ANOTHER room must not be applied here. Teleporting a
/// body to coordinates from a different room is the exact failure the room id
/// exists to prevent, and it is the failure mode a position-only checkpoint has.
#[test]
fn a_checkpoint_from_another_room_leaves_the_body_where_it_spawned() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.checkpoint = Some(ambition_persistence::save_data::PersistedCheckpoint::new(
        "somewhere_else",
        999,
        999,
    ));
    app.add_message::<ambition_platformer2d_world::rooms::RoomTransitionRequested>();
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
        crate::rooms::RoomSet::from_parts(
            "here",
            vec![crate::rooms::RoomSpec::new("here", world)],
            Vec::new(),
        ),
    );
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

/// **A checkpoint in ANOTHER room of this world routes the session to it.**
///
/// This is what "resume where you last rested" means, and until 2026-07-27 it
/// did not happen: the saved room id was only COMPARED against whatever room
/// the session opened, and a mismatch returned. Rest in B, quit, start a
/// session that opens in A, and the checkpoint was silently ignored — and the
/// handled latch was set BEFORE the comparison, so walking into B later in that
/// same session did not apply it either.
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
    save.checkpoint = Some(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.add_message::<ambition_platformer2d_world::rooms::RoomTransitionRequested>();
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        crate::rooms::RoomSpec::new(
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
        crate::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    // ⚠ **the resume needs a body to name.** A transition states WHICH body is
    // crossing (D71), and the resume's answer is the avatar the save is about —
    // so a fixture with no avatar at all models nothing the game can do. The
    // `SimId` is what `ensure_sim_id` files a `PrimaryPlayer` under on every host.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    app.add_systems(Update, restore_checkpoint_on_session_start);
    app.update();

    let requests: Vec<_> = app
        .world()
        .resource::<bevy::prelude::Messages<ambition_platformer2d_world::rooms::RoomTransitionRequested>>()
        .iter_current_update_messages()
        .cloned()
        .collect();
    assert_eq!(
        requests.len(),
        1,
        "the session opened in `entry` while the checkpoint is in `rest_room` \
         and no transition was requested — the player does not resume where \
         they rested"
    );
    assert_eq!(requests[0].transition.target_room, 1);
    assert_eq!(
        requests[0].subject,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
        "the resume asked for a room without saying whose resume it is, so the \
         commit would transit whoever happens to be controlled several frames later"
    );
    assert_eq!(
        (
            requests[0].transition.arrival.x,
            requests[0].transition.arrival.y
        ),
        (512.0, 300.0),
        "the transition must arrive AT the checkpoint, not at the room's own spawn"
    );

    // Once per session: a transition takes several frames to commit, and
    // re-requesting every frame would restart it forever.
    app.update();
    app.update();
    let repeats = app
        .world()
        .resource::<bevy::prelude::Messages<ambition_platformer2d_world::rooms::RoomTransitionRequested>>()
        .iter_current_update_messages()
        .count();
    assert_eq!(repeats, 0, "the resume transition was requested repeatedly");
}
