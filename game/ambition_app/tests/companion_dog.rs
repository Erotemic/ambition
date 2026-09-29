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

/// Interact beside the dog talks to it, and does not pet it: the pet is a
/// choice in that conversation (`<<pet>>` in `hall_npc_companion_dog`). Picking
/// it pets the dog, both bodies hold still for the whole pet, and both let go
/// when it ends. The shipped app, the real input road and the authored catalog
/// row.
///
/// ⚠ A headless harness has no dialog box to confirm a choice in, so this runs
/// the command the choice runs, through the shipped Yarn vocabulary, while the
/// real conversation is live. `dialogue_lint` holds the authored option's
/// command name and arity.
#[test]
fn talking_to_the_dog_offers_a_pet_that_holds_both_still_until_it_ends() {
    use ambition_platformer2d::characters::actor::BodyAnimFacts;
    use ambition_platformer2d::characters::control::{ControlHold, ControlHolds};
    use ambition_platformer2d::conversation::ActiveConversation;

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
    let anim = |sim: &ambition_app::Platformer2dSimHarness, body: Entity| {
        sim.world()
            .get::<BodyAnimFacts>(body)
            .expect("an animated body")
            .clone()
    };
    let talking_to_the_dog = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world()
            .get_resource::<ActiveConversation>()
            .is_some_and(|conversation| conversation.talker() == Some(dog))
    };

    // Standing ON the dog, in front of one of the basement's doors: the dog is
    // nearer than the door, so the press talks to it.
    let here = pos(&sim, dog);
    sim.teleport_player((here.x, here.y));
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });
    for _ in 0..10 {
        if talking_to_the_dog(&sim) {
            break;
        }
        sim.step(base());
    }
    assert!(talking_to_the_dog(&sim), "Interact beside the dog talks to it");
    assert_eq!(
        (anim(&sim, player).pet_anim_timer, anim(&sim, dog).petted_anim_timer),
        (0.0, 0.0),
        "Interact alone does not pet the dog"
    );

    // THE CHOICE.
    sim.world_mut()
        .run_system_cached(ambition_content::yarn_vocabulary::cmd_pet)
        .expect("the `<<pet>>` command runs");
    // The ledger releases the pet on the tick after the command, and the
    // effects phase applies it.
    for _ in 0..4 {
        if anim(&sim, player).pet_anim_timer > 0.0 {
            break;
        }
        sim.step(base());
    }
    assert!(
        anim(&sim, player).pet_anim_timer > 0.0 && anim(&sim, dog).petted_anim_timer > 0.0,
        "the choice pets the dog"
    );
    // The choice ends the node, and the box closes.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::dialog::DialogState>()
        .close();
    for _ in 0..10 {
        if !talking_to_the_dog(&sim) {
            break;
        }
        sim.step(base());
    }
    assert!(!talking_to_the_dog(&sim), "the conversation ended");

    let started = (pos(&sim, player), pos(&sim, dog));
    for _ in 0..60 {
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
    sim.step_n(base(), 120);
    for body in [player, dog] {
        assert!(
            !sim.world()
                .get::<ControlHolds>(body)
                .is_some_and(|holds| holds.holds(ControlHold::Gesture)),
            "the pet let go when it ended"
        );
    }
}
