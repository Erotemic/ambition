//! An AI Slop of Mary-O is squashed by each player of its own live room, and
//! by no player of another. It read one global `PrimaryPlayer` with
//! `single_mut()`.

use bevy::prelude::*;

use ambition_demo_mary_o::ai_slop::is_ai_slop_brain;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

fn her(app: &mut App) -> Entity {
    let world = app.world_mut();
    world.query_filtered::<Entity, With<PrimaryPlayer>>().iter(world).next().expect("her body")
}

/// 1-1 with her seated, and a live AI Slop.
fn a_mob() -> (App, Entity) {
    let mut app = ambition_demo_mary_o_app::build_demo_app();
    let mob = (0..600)
        .find_map(|_| {
            app.update();
            let world = app.world_mut();
            let seated = world.query_filtered::<(), With<PrimaryPlayer>>().iter(world).next().is_some();
            let mut q = world.query::<(Entity, &ActorConfig, &BodyHealth)>();
            let mob = q
                .iter(world)
                .filter(|(_, config, health)| is_ai_slop_brain(&config.brain) && health.alive())
                .map(|(entity, ..)| entity)
                .min();
            mob.filter(|_| seated)
        })
        .expect("1-1 builds an AI Slop and seats Mary-O");
    (app, mob)
}

fn a_player_body(app: &mut App, room: LiveRoomInstance) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 32.0),
                facing: 1.0,
            },
            PlayerEntity,
            InRoomInstance(room),
        ))
        .id()
}

/// Stand `stomper` on the mob's head for some ticks, with Mary-O far away
/// unless she is the stomper. Is the mob still there after?
fn still_there_after_a_stomp(app: &mut App, mob: Entity, stomper: Entity) -> bool {
    let mary_o = her(app);
    for _ in 0..20 {
        let Some(mob_box) = app.world().get::<ae::BodyKinematics>(mob).map(|kin| kin.aabb()) else {
            return false;
        };
        let size = app.world().get::<ae::BodyKinematics>(stomper).unwrap().size;
        let on_head = Vec2::new(mob_box.center().x, mob_box.min.y + 1.0 - size.y * 0.5);
        if stomper != mary_o {
            let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(mary_o).unwrap();
            kin.pos = on_head + Vec2::new(1500.0, -400.0);
            kin.vel = Vec2::ZERO;
        }
        if let Some(mut health) = app.world_mut().get_mut::<BodyHealth>(mary_o) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(stomper).unwrap();
        kin.pos = on_head;
        kin.vel = Vec2::ZERO;
        app.update();
        if app.world().get::<BodyHealth>(mob).is_none_or(|health| !health.alive()) {
            return false;
        }
    }
    true
}

/// ⭐ Each player of the mob's live room squashes it; a player of another
/// live room at the same place does not.
#[test]
fn an_ai_slop_is_squashed_by_each_player_of_its_room_and_by_no_other() {
    // The control: Mary-O squashes it.
    let (mut app, mob) = a_mob();
    let mary_o = her(&mut app);
    assert!(!still_there_after_a_stomp(&mut app, mob, mary_o), "the premise: her stomp squashes it");

    let (mut app, mob) = a_mob();
    let room = app.world().get::<InRoomInstance>(mob).expect("the mob has its live room").0;
    let second = a_player_body(&mut app, room);
    assert!(!still_there_after_a_stomp(&mut app, mob, second), "a second player of its room squashes it");

    let (mut app, mob) = a_mob();
    let other = LiveRoomInstance::ACTIVATION.next().next().next();
    app.world_mut().spawn((RoomInstanceRoot, other));
    let elsewhere = a_player_body(&mut app, other);
    assert!(still_there_after_a_stomp(&mut app, mob, elsewhere), "a player of another live room does not");
}

/// The squash is a module, and its bounce and stomp band are her pack's
/// (`stomp` in `mary_o_rules.ron`), read through the observation
/// `mary_o.rules.stomp` at each invocation, not module constants.
#[test]
fn the_squash_bounces_her_at_the_speed_her_pack_states() {
    let (mut app, mob) = a_mob();
    let mary_o = her(&mut app);
    let authority = ambition_demo_mary_o::rules::mary_o_rules().stomp.bounce_speed;
    let mut bounced = None;
    for _ in 0..20 {
        let Some(mob_box) = app.world().get::<ae::BodyKinematics>(mob).map(|kin| kin.aabb()) else {
            break;
        };
        if let Some(mut health) = app.world_mut().get_mut::<BodyHealth>(mary_o) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
        let size = app.world().get::<ae::BodyKinematics>(mary_o).unwrap().size;
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(mary_o).unwrap();
        kin.pos = Vec2::new(mob_box.center().x, mob_box.min.y + 1.0 - size.y * 0.5);
        kin.vel = Vec2::ZERO;
        app.update();
        if app.world().get::<BodyHealth>(mob).is_none_or(|health| !health.alive()) {
            bounced = app.world().get::<ae::BodyKinematics>(mary_o).map(|kin| -kin.vel.y);
            break;
        }
    }
    let bounced = bounced.expect("the premise: her stomp squashes it");
    // The bounce is set in the contact phase; the rest of the tick's gravity
    // takes off at most one tick of fall.
    let tick_of_fall = 2000.0 / 60.0;
    assert!(
        bounced <= authority && bounced > authority - tick_of_fall,
        "she left the squash at {bounced} up, and her pack states {authority}"
    );
}

/// The squash plays the dust her rules file states (`stomp.dust`), the
/// snake's dust too, read by the module through `mary_o.rules.stomp`.
#[test]
fn the_squash_plays_the_dust_its_rules_state() {
    let (mut app, mob) = a_mob();
    crate::the_switch_ends_the_castle::record_effects(&mut app);
    let mary_o = her(&mut app);
    assert!(!still_there_after_a_stomp(&mut app, mob, mary_o), "the premise: her stomp squashes it");
    let stated = crate::the_switch_ends_the_castle::shape_of(&ambition_demo_mary_o::rules::mary_o_rules().stomp.dust);
    let played = &app.world().resource::<crate::the_switch_ends_the_castle::Played>().bursts;
    assert!(played.contains(&stated), "the squash did not play the file's dust {stated:?}: played {played:?}");
}
