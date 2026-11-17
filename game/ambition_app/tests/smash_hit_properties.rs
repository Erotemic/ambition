//! Authored hit interactions in the shipped Smash match.

use ambition_platformer2d::{
    characters::{actor::BodyHealth, brain::Brain},
    combat::{clank::AttacksClanked, moveset::{ActorMoveset, MovePlayback}},
    engine_core as ae,
};
use bevy::prelude::*;

fn two_georges() -> (App, [Entity; 2]) {
    let roster = ambition_demo_smash::smash_roster([
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ]);
    let mut app = crate::common::smash::live_round(roster);
    let bodies = [0, 1].map(|seat| crate::common::smash::seat(&mut app, seat));
    for body in bodies {
        app.world_mut().entity_mut(body).insert(Brain::stand_still());
    }
    for _ in 0..60 { app.update(); }
    let y = app.world().get::<ae::BodyKinematics>(bodies[0]).unwrap().pos.y;
    let center = {
        let world = app.world_mut();
        world.query::<&ae::RoomGeometry>().single(world).unwrap().0.size.x * 0.5
    };
    for (n, body) in bodies.into_iter().enumerate() {
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap();
        kin.pos = ae::Vec2::new(center + if n == 0 { -30.0 } else { 30.0 }, y);
        kin.vel = ae::Vec2::ZERO;
        kin.facing = if n == 0 { 1.0 } else { -1.0 };
    }
    for _ in 0..3 { app.update(); }
    (app, bodies)
}

#[test]
fn the_shipped_rules_clash_two_authored_ground_jabs_before_damage() {
    let (mut app, bodies) = two_georges();
    let before = bodies.map(|body| app.world().get::<BodyHealth>(body).unwrap().damage_taken());
    for (n, body) in bodies.into_iter().enumerate() {
        let moves = app.world().get::<ActorMoveset>(body).unwrap();
        let jab = moves.0.moves.iter().find(|spec| spec.id == "jab").unwrap().clone();
        app.world_mut().entity_mut(body).insert(MovePlayback::new(std::sync::Arc::new(jab), if n == 0 { 1.0 } else { -1.0 }));
    }
    let mut cursor = bevy::ecs::message::MessageCursor::<AttacksClanked>::default();
    let mut clanked = false;
    for _ in 0..90 {
        app.update();
        clanked |= cursor.read(app.world().resource::<Messages<AttacksClanked>>()).any(|event|
            bodies.contains(&event.owners.0) && bodies.contains(&event.owners.1));
        if clanked { break; }
    }
    assert!(clanked, "the match's authored rules must admit a real clash");
    let after = bodies.map(|body| app.world().get::<BodyHealth>(body).unwrap().damage_taken());
    assert_eq!(before, after, "the attacks clash before either damages a body");
    assert!(bodies.iter().all(|body| app.world().get::<MovePlayback>(*body).is_none()), "both moves end through their normal interruption path");
}
