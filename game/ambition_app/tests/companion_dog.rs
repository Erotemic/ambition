#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::{BodyAbilities, BodyKinematics};
use ambition_platformer2d::vfx::vfx::VfxMessage;
use ambition_platformer2d::vfx::vfx::VfxInRoom;
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
        if let Some(mut messages) = sim.world_mut().get_resource_mut::<Messages<VfxInRoom>>() {
            barked |= messages.drain().map(|m| m.vfx).any(|message| {
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
/// real conversation is live.
/// `ambition_content`'s `the_dogs_pet_is_a_conversation_choice` selects the
/// authored option in the real interpreter and sees it run `<<pet>>`.
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
        (anim(&sim, player).petting, anim(&sim, dog).petted),
        (false, false),
        "Interact alone does not pet the dog"
    );

    // THE CHOICE.
    sim.world_mut()
        .run_system_cached(ambition_content::yarn_vocabulary::cmd_pet)
        .expect("the `<<pet>>` command runs");
    // The ledger releases the pet on the tick after the command. The player
    // then WALKS to the dog's front, and the gesture starts on arrival.
    //
    // ⭐ THE WALK IS THE SUBJECT. The pet used to write the player's position
    // to the mark in one tick, which read as a teleport. Every tick of the walk
    // must move the player no farther than the walk's own speed allows.
    let per_tick = ambition_platformer2d::actors::features::ecs::PET_WALK_SPEED / 60.0;
    let mut path = vec![pos(&sim, player).x];
    for _ in 0..180 {
        if anim(&sim, player).petting {
            break;
        }
        sim.step(base());
        path.push(pos(&sim, player).x);
    }
    assert!(
        anim(&sim, player).petting && anim(&sim, dog).petted,
        "the choice pets the dog: beat {:?}, walk {:?}, dog at {:?}, player at {:?}",
        sim.world().get::<ambition_platformer2d::actors::features::ecs::PetBeat>(player),
        sim.world().get::<ambition_platformer2d::characters::control::CommandedMove>(player),
        pos(&sim, dog),
        pos(&sim, player),
    );
    let longest = path
        .windows(2)
        .map(|step| (step[1] - step[0]).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        longest <= per_tick * 1.5,
        "the player jumped {longest:.1}px in one tick to reach the dog (a walk moves at most \
         {per_tick:.1}px): {path:?}"
    );
    assert!(
        path.len() > 3 && (path[path.len() - 1] - path[0]).abs() > per_tick,
        "control: the player did not walk to the dog's front at all: {path:?}"
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

/// The pet's holds follow the pet on the tick it changes, in the composed
/// schedule.
///
/// The control gate runs early in a tick and the pet's script late. The holds
/// were projected in the feature phase, between the two, from the beats as
/// the tick before left them. So on the tick a pet began, nothing held either
/// body, and the next tick's control was the stick's with the walk on top of
/// it. And on the tick a hit ended the pet, both bodies stayed held, and the
/// next tick's control was blanked for a pet that was over.
///
/// Arm 1: at the end of the tick the beat appears on, both bodies hold
/// `Gesture`, so the next control frame is the script's. Arm 2: a hit on the
/// dog during the gesture; at the end of that tick, neither body holds
/// `Gesture` and neither is posed. The control for arm 2 is the tick before
/// the hit, when both are held.
#[test]
fn a_pet_holds_both_bodies_from_its_first_tick_and_lets_go_on_the_tick_it_breaks() {
    use ambition_platformer2d::characters::actor::{BodyAnimFacts, BodyCombat};
    use ambition_platformer2d::characters::control::{ControlHold, ControlHolds};
    use ambition_platformer2d::actors::features::ecs::PetBeat;

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
    let held = |sim: &ambition_app::Platformer2dSimHarness, body: Entity| {
        sim.world()
            .get::<ControlHolds>(body)
            .is_some_and(|holds| holds.holds(ControlHold::Gesture))
    };
    let posed = |sim: &ambition_app::Platformer2dSimHarness| {
        let anim = |body| sim.world().get::<BodyAnimFacts>(body).cloned().unwrap_or_default();
        (anim(player).petting, anim(dog).petted)
    };

    let here = sim.world().get::<BodyKinematics>(dog).expect("the dog").pos;
    sim.teleport_player((here.x, here.y));
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });
    sim.step_n(base(), 5);
    sim.world_mut()
        .run_system_cached(ambition_content::yarn_vocabulary::cmd_pet)
        .expect("the `<<pet>>` command runs");

    // Arm 1: the tick the beat appears on.
    let mut began = false;
    for _ in 0..10 {
        sim.step(base());
        if sim.world().get::<PetBeat>(player).is_some() {
            began = true;
            break;
        }
    }
    assert!(began, "setup: the pet never began");
    assert!(
        held(&sim, player) && held(&sim, dog),
        "the pet began this tick and a body is not held, so the next tick's control is \
         the stick's (player held: {}, dog held: {})",
        held(&sim, player),
        held(&sim, dog),
    );

    // Arm 2: into the gesture, then a hit on the dog.
    for _ in 0..180 {
        if posed(&sim) == (true, true) {
            break;
        }
        sim.step(base());
    }
    assert_eq!(posed(&sim), (true, true), "setup: the gesture never played");
    assert!(held(&sim, player) && held(&sim, dog), "control: the gesture holds both");
    sim.world_mut()
        .entity_mut(dog)
        .entry::<BodyCombat>()
        .or_default()
        .get_mut()
        .recoil_lock_timer = 0.3;
    sim.step(base());
    assert!(sim.world().get::<PetBeat>(player).is_none(), "the hit did not end the pet");
    assert_eq!(posed(&sim), (false, false), "the pet ended and a body is still posed");
    assert!(
        !held(&sim, player) && !held(&sim, dog),
        "the pet ended this tick and a body is still held, so the next tick's control is \
         blanked for a pet that is over (player held: {}, dog held: {})",
        held(&sim, player),
        held(&sim, dog),
    );
}


/// Pet the basement dog with the gravity of the hub complex at `down`, and
/// answer, for the petter and for the dog, the world x of the way each looks
/// when the gesture starts, and the world x from each to the other.
///
/// `facing` is a sign on the side axis of the body's own frame, so the way a
/// body looks in the world is that axis times its facing.
fn pet_the_dog_under(down: (f32, f32)) -> [(f32, f32); 2] {
    use ambition_platformer2d::characters::actor::BodyAnimFacts;
    use ambition_platformer2d::conversation::ActiveConversation;
    use ambition_platformer2d::platformer::frame_env::ResolvedMotionFrame;

    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);
    sim.set_base_gravity_dir(down);
    // Each body falls to the floor of this gravity.
    sim.step_n(base(), 300);
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
        sim.world().get::<BodyKinematics>(body).expect("a live body").pos
    };
    for body in [player, dog] {
        let frame = sim.world().get::<ResolvedMotionFrame>(body).expect("a frame");
        assert_eq!(
            (frame.down().x, frame.down().y),
            down,
            "premise: the gravity of the hub complex is the gravity of each body"
        );
    }
    let here = pos(&sim, dog);
    sim.teleport_player((here.x, here.y));
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });
    let talking = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world()
            .get_resource::<ActiveConversation>()
            .is_some_and(|conversation| conversation.talker() == Some(dog))
    };
    for _ in 0..10 {
        if talking(&sim) {
            break;
        }
        sim.step(base());
    }
    assert!(talking(&sim), "premise: Interact beside the dog talks to it under gravity {down:?}");
    sim.world_mut()
        .run_system_cached(ambition_content::yarn_vocabulary::cmd_pet)
        .expect("the `<<pet>>` command runs");
    let petting = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world().get::<BodyAnimFacts>(player).expect("an animated body").petting
    };
    for _ in 0..300 {
        if petting(&sim) {
            break;
        }
        sim.step(base());
    }
    assert!(petting(&sim), "premise: the walk arrives and the gesture starts under gravity {down:?}");
    let looks = |body: Entity, other: Entity| {
        let kin = sim.world().get::<BodyKinematics>(body).expect("a live body");
        let side = sim.world().get::<ResolvedMotionFrame>(body).expect("a frame").basis().side;
        ((side * kin.facing).x, pos(&sim, other).x - kin.pos.x)
    };
    [looks(player, dog), looks(dog, player)]
}

/// The petter and the dog look at each other. `central_hub_complex` is one
/// room with the hub's Flip Gravity switch and the basement's dog, so a
/// player can pet the dog on the ceiling. There each body looked AWAY from
/// the other: the pet wrote each facing as a sign on world x, and facing is a
/// sign on the side axis of the body, which a flip turns over.
#[test]
fn the_petter_and_the_dog_look_at_each_other_under_flipped_gravity() {
    for (name, down) in [("normal gravity (the control)", (0.0, 1.0)), ("flipped gravity", (0.0, -1.0))] {
        let [petter, dog] = pet_the_dog_under(down);
        assert!(petter.1.abs() > 1.0, "premise, {name}: the petter stands beside the dog: {petter:?}");
        assert_eq!(
            petter.0.signum(),
            petter.1.signum(),
            "{name}: the petter looks along world x {} and the dog is at {} from it",
            petter.0,
            petter.1
        );
        assert_eq!(
            dog.0.signum(),
            dog.1.signum(),
            "{name}: the dog looks along world x {} and the petter is at {} from it",
            dog.0,
            dog.1
        );
    }
}
