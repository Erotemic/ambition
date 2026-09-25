#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::engine_core::{BodyAbilities, BodyKinematics};
use ambition_platformer2d::sprite_sheet::character::ActorBarkGesture;
use bevy::prelude::Entity;

#[test]
fn the_basement_dog_is_peaceful_and_roams_across_the_floor() {
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
    let mut barked = false;
    for _ in 0..2400 {
        sim.step(base());
        barked |= sim.world().get::<ActorBarkGesture>(dog).is_some();
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
        max_x - min_x > 1000.0,
        "the dog did not cross the basement: {min_x}..{max_x}"
    );
    assert!(
        start.y - min_y > 5.0,
        "the dog did not hop: {start:?}, {min_y}"
    );
    assert!(barked, "the dog did not give an ambient bark");
}
