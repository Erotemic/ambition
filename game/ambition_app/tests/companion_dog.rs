#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::{BodyAbilities, BodyKinematics};
use ambition_platformer2d::vfx::vfx::VfxMessage;
use bevy::prelude::{Entity, Messages};

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
    let dog_id = sim
        .world()
        .get::<FeatureId>(dog)
        .expect("the dog has a feature id")
        .as_str()
        .to_string();
    let mut barked = false;
    for _ in 0..2400 {
        sim.step(base());
        // The bark pose is presentation: the sim asks for it with a message
        // naming the dog, and keeps no gesture state on the body.
        if let Some(mut messages) = sim.world_mut().get_resource_mut::<Messages<VfxMessage>>() {
            barked |= messages.drain().any(|message| {
                matches!(message, VfxMessage::BarkGesture { ref feature_id, .. } if *feature_id == dog_id)
            });
        }
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

/// Interact beside the dog pets it: both bodies hold still for the whole pet,
/// the player is drawn petting and the dog petted, and both let go when it ends.
/// The shipped app, the real input road and the authored catalog row.
#[test]
fn interact_beside_the_dog_pets_it_and_both_hold_still_until_it_ends() {
    use ambition_platformer2d::characters::actor::BodyAnimFacts;
    use ambition_platformer2d::characters::control::{ControlHold, ControlHolds};

    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);
    let dog = {
        let world = sim.world_mut();
        let mut query = world.query::<(Entity, &WornCharacter)>();
        query
            .iter(world)
            .find(|(_, worn)| worn.id() == "npc_companion_dog")
            .map(|(entity, _)| entity)
            .expect("the basement stages the authored dog")
    };
    let player = {
        let world = sim.world_mut();
        let mut query = world.query_filtered::<
            Entity,
            bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        query.single(world).expect("one primary player")
    };
    let pos = |sim: &ambition_app::Platformer2dSimHarness, body: Entity| {
        sim.world()
            .get::<BodyKinematics>(body)
            .expect("a live body")
            .pos
    };
    // Standing ON the dog, which roams in front of the basement's doors: the
    // dog is nearer than any door, so the press pets it.
    let here = pos(&sim, dog);
    sim.teleport_player((here.x, here.y));
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });

    // The press is buffered on one tick and acted on by the interaction phase.
    sim.step(base());
    let petting = |sim: &ambition_app::Platformer2dSimHarness, body: Entity| {
        sim.world()
            .get::<BodyAnimFacts>(body)
            .expect("an animated body")
            .clone()
    };
    assert!(
        petting(&sim, player).pet_anim_timer > 0.0,
        "the press pets the dog"
    );
    assert!(
        petting(&sim, dog).petted_anim_timer > 0.0,
        "and the dog is petted"
    );
    // A tick for the hold projection to claim both bodies.
    sim.step(base());
    let started = (pos(&sim, player), pos(&sim, dog));
    for _ in 0..90 {
        sim.step(ambition_app::AgentAction {
            move_x: 1.0,
            ..base()
        });
        for body in [player, dog] {
            assert!(
                sim.world()
                    .get::<ControlHolds>(body)
                    .is_some_and(|holds| holds.holds(ControlHold::Gesture)),
                "both bodies are held for the pet"
            );
        }
    }
    let player_now = pos(&sim, player);
    assert!(
        (player_now.x - started.0.x).abs() < 0.5,
        "the player held still through the pet despite pushing right: {started:?} -> {player_now:?}"
    );
    assert!(
        (pos(&sim, dog).x - started.1.x).abs() < 0.5,
        "the dog stopped roaming while it was petted: {:?} -> {:?}",
        started.1,
        pos(&sim, dog)
    );
    // Past the pet's two seconds, both are free again.
    sim.step_n(base(), 60);
    for body in [player, dog] {
        assert!(
            !sim.world()
                .get::<ControlHolds>(body)
                .is_some_and(|holds| holds.holds(ControlHold::Gesture)),
            "the pet let go when it ended"
        );
    }
}
