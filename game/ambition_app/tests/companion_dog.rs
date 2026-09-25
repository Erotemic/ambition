#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::engine_core::{BodyAbilities, BodyKinematics};
use bevy::prelude::Entity;

#[test]
fn the_basement_dog_is_peaceful_and_moves_and_hops() {
    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);

    let dog: Entity = {
        let world = sim.world_mut();
        let mut query =
            world.query::<(Entity, &WornCharacter, &ActorDisposition, &BodyAbilities)>();
        let (entity, _, disposition, abilities) = query
            .iter(world)
            .find(|(_, worn, ..)| worn.id() == "npc_companion_dog")
            .expect("the basement stages the authored dog");
        assert_eq!(*disposition, ActorDisposition::Peaceful);
        assert!(abilities.abilities.move_horizontal && abilities.abilities.jump);
        assert!(!abilities.abilities.attack);
        entity
    };

    let start = sim
        .world()
        .get::<BodyKinematics>(dog)
        .expect("the dog has a live body")
        .pos;
    let mut min_x = start.x;
    let mut max_x = start.x;
    let mut min_y = start.y;
    for _ in 0..360 {
        sim.step(base());
        let pos = sim
            .world()
            .get::<BodyKinematics>(dog)
            .expect("the dog stays in the room")
            .pos;
        min_x = min_x.min(pos.x);
        max_x = max_x.max(pos.x);
        min_y = min_y.min(pos.y);
    }
    assert!(
        max_x - min_x > 20.0,
        "the dog did not roam: {min_x}..{max_x}"
    );
    assert!(
        start.y - min_y > 5.0,
        "the dog did not hop: {start:?}, {min_y}"
    );
}
