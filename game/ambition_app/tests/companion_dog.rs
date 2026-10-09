#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::{BodyAbilities, BodyKinematics};
use ambition_platformer2d::vfx::vfx::VfxMessage;
use ambition_platformer2d::vfx::vfx::VfxInRoom;
use bevy::prelude::{Entity, Messages};

/// ⭐ THE DOG GOES TO PLACES BY THE ROOM'S SURFACES.
///
/// Jon, 2026-10-09: the dog "should navigate to random waypoints, and not jump
/// in a fixed pattern". The dog has the `Roam` brain. The navigation advisor
/// gives it places it can reach and the legs to them, from the surface graph
/// of the room for the dog's own tuning. The shipped app, the authored room
/// and the authored catalog row.
///
/// The graph is not trusted here: the dog's feet are. A surface counts when
/// the dog STANDS on it.
#[test]
fn the_basement_dog_is_peaceful_and_goes_to_places_on_the_rooms_surfaces() {
    use ambition_platformer2d::actors::features::ecs::navigation::RoomNavigation;
    use ambition_platformer2d::engine_core::BodyGroundState;
    use std::collections::BTreeSet;

    let mut sim = fixed_60hz_room_sim("central_hub_complex");
    sim.step_n(base(), 10);

    let dog: Entity = {
        let world = sim.world_mut();
        let mut query =
            world.query::<(Entity, &WornCharacter, &ActorDisposition, &BodyAbilities)>();
        let (entity, _, disposition, abilities) = query
            .iter(world)
            .find(|(_, worn, ..)| worn.id() == "npc_companion_dog")
            .expect("the basement stages the authored dog")
        ;
        assert_eq!(*disposition, ActorDisposition::Peaceful);
        assert!(abilities.abilities.move_horizontal && abilities.abilities.jump);
        assert!(!abilities.abilities.attack);
        entity
    };
    let dog_id = sim
        .world()
        .get::<FeatureId>(dog)
        .expect("the dog has a feature id")
        .as_str()
        .to_string();
    // One graph: the dog's body, with the dog's own tuning. A body is born
    // with a motion model that is not its own, and the advisor does not build
    // a graph for it before the kernel has stepped it.
    let navigation = sim.world().resource::<RoomNavigation>();
    assert_eq!(navigation.len(), 1, "the advisor built {} graphs for one dog", navigation.len());
    let graph = navigation.graphs().next().cloned().expect("the advisor built a graph for the dog");
    assert!(
        (graph.run_speed - 120.0).abs() < 1.0,
        "the graph is for a body that runs at {}, and the dog runs at 120",
        graph.run_speed
    );
    let feet = |sim: &ambition_app::Platformer2dSimHarness| {
        let kin = sim.world().get::<BodyKinematics>(dog).expect("the dog stays in the room");
        kin.pos + bevy::math::Vec2::Y * kin.size.y * 0.5
    };
    let home = graph.surface_at(feet(&sim)).expect("the dog starts on a standing surface");
    let reachable: BTreeSet<usize> = graph.reachable_from(home).into_iter().collect();
    // ⛔ THE PREMISES: the basement gives the dog places to go, and the room
    // has places it cannot go (the hall above it).
    assert!(reachable.len() >= 6, "the dog can reach only {} surfaces", reachable.len());
    assert!(reachable.len() < graph.surfaces.len(), "control: each surface of the room is in reach");

    let mut stood_on = BTreeSet::new();
    let mut take_offs = Vec::new();
    let mut was_on_ground = true;
    let mut barked = false;
    for tick in 0..3600 {
        sim.step(base());
        // The bark pose is presentation: the sim asks for it with a message
        // naming the dog, and keeps no gesture state on the body.
        if let Some(mut messages) = sim.world_mut().get_resource_mut::<Messages<VfxInRoom>>() {
            barked |= messages.drain().map(|m| m.vfx).any(|message| {
                matches!(message, VfxMessage::BarkGesture { ref feature_id, .. } if *feature_id == dog_id)
            });
        }
        let on_ground = sim.world().get::<BodyGroundState>(dog).expect("a live body").on_ground;
        if on_ground {
            if let Some(surface) = graph.surface_at(feet(&sim)) {
                stood_on.insert(surface);
            }
        } else if was_on_ground {
            take_offs.push(tick);
        }
        was_on_ground = on_ground;
    }
    assert!(
        stood_on.len() >= 4,
        "in a minute the dog stood on {} surface(s): {stood_on:?}",
        stood_on.len()
    );
    assert!(
        stood_on.is_subset(&reachable),
        "the dog stood where the graph says it cannot: {:?}",
        stood_on.difference(&reachable).collect::<Vec<_>>()
    );
    // Not on a clock: the times between the dog's take-offs are not one time.
    let waits: Vec<i32> = take_offs.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let (shortest, longest) = (waits.iter().min().copied(), waits.iter().max().copied());
    assert!(
        take_offs.len() >= 4 && longest.zip(shortest).is_some_and(|(long, short)| long - short > 30),
        "the dog left the ground at ticks {take_offs:?}"
    );
    assert!(barked, "the dog did not give an ambient bark");
}

/// What a system from outside the timeline does to the dog's room under the
/// sync test. Each counts its own runs, and a count does not rewind.
#[derive(bevy::prelude::Resource, Clone, Copy, PartialEq, Eq)]
enum FromOutsideTheTimeline {
    /// Move the dog's body a little: rollback state. The control.
    NudgeTheDog,
    /// Drop the surface graphs: a derived cache, built again when asked for.
    DropTheGraphs,
}

fn act_from_outside_the_timeline(
    what: bevy::prelude::Res<FromOutsideTheTimeline>,
    mut runs: bevy::prelude::Local<u32>,
    mut navigation: bevy::prelude::ResMut<
        ambition_platformer2d::actors::features::ecs::navigation::RoomNavigation,
    >,
    mut bodies: bevy::prelude::Query<(&mut BodyKinematics, &WornCharacter)>,
) {
    *runs += 1;
    match *what {
        FromOutsideTheTimeline::NudgeTheDog => {
            for (mut kin, worn) in &mut bodies {
                if worn.id() == "npc_companion_dog" {
                    kin.pos.x += (*runs % 7) as f32 * 0.01;
                }
            }
        }
        // Not each run: a build of the basement graph is tens of milliseconds.
        FromOutsideTheTimeline::DropTheGraphs => {
            if *runs % 400 == 0 {
                *navigation = Default::default();
            }
        }
    }
}

/// (surfaces the dog stood on, its take-offs, the session ran, its health)
/// after 40 seconds of the basement under a sync test that rewinds and
/// replays each frame.
fn the_dog_under_a_sync_test(
    what: FromOutsideTheTimeline,
) -> (usize, usize, bool, Result<(), String>) {
    use crate::common::fixed_60hz_room_options;
    use ambition_platformer2d::actors::features::ecs::navigation::RoomNavigation;
    use ambition_platformer2d::engine_core::BodyGroundState;
    use ambition_platformer2d::sim::SimScheduleExt;

    let options =
        fixed_60hz_room_options("central_hub_complex").with_sync_test_rollback_settings(4, 10);
    let mut sim = ambition_app::Platformer2dSimHarness::build(options, |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.insert_resource(what);
        let label = app.sim_schedule();
        app.add_systems(label, act_from_outside_the_timeline);
        Ok(())
    })
    .expect("the hub boots under a sync test");
    let mut stood_on = std::collections::BTreeSet::new();
    let mut take_offs = 0;
    let mut was_on_ground = true;
    for _ in 0..2400 {
        // `try_step`: an unhealthy session refuses the step, and that refusal
        // is the reading, so the loop ends at it.
        if sim.try_step(base()).is_err() {
            break;
        }
        let world = sim.world_mut();
        let mut dogs = world.query::<(&WornCharacter, &BodyKinematics, &BodyGroundState)>();
        let Some((_, kin, ground)) =
            dogs.iter(world).find(|(worn, ..)| worn.id() == "npc_companion_dog")
        else {
            continue;
        };
        let (feet, on_ground) = (kin.pos + bevy::math::Vec2::Y * kin.size.y * 0.5, ground.on_ground);
        if on_ground {
            let navigation = world.resource::<RoomNavigation>();
            if let Some(surface) = navigation.graphs().next().and_then(|graph| graph.surface_at(feet)) {
                stood_on.insert(surface);
            }
        } else if was_on_ground {
            take_offs += 1;
        }
        was_on_ground = on_ground;
    }
    (
        stood_on.len(),
        take_offs,
        ambition_platformer2d::rollback::session_is_active(sim.world()),
        ambition_platformer2d::rollback::session_health(sim.world()),
    )
}

/// ⭐ A DOG THAT NAVIGATES RESIMULATES TO THE SAME WORLD.
///
/// What a navigating body adds to a rewind: its brain holds a leg and a phase
/// (`NavFollower`), the advice it reads is written again each tick, and the
/// graph the advice comes from is a cache that no snapshot holds. A sync test
/// rewinds and replays each frame and compares checksums.
///
/// The graph is dropped from outside the timeline while the dog is on its way.
/// If the graph were state, a replay with a graph built at a different moment
/// would be a different world.
///
/// The control is a nudge of the dog's body from outside the timeline: it must
/// be a mismatch, or the checksum does not see the dog.
#[test]
fn a_dog_that_navigates_resimulates_to_the_same_world_with_its_graph_dropped() {
    let (_, _, active, health) = the_dog_under_a_sync_test(FromOutsideTheTimeline::NudgeTheDog);
    assert!(
        active && health.is_err(),
        "control: a nudge of the dog from outside the timeline must be a mismatch, or this \
         sync test does not see the dog (active {active}, health {health:?})"
    );
    let (stood_on, take_offs, active, health) =
        the_dog_under_a_sync_test(FromOutsideTheTimeline::DropTheGraphs);
    // ⛔ THE PREMISE: the dog went by legs in these frames. A dog that rests
    // for 40 seconds holds under any sync test.
    assert!(
        stood_on >= 2 && take_offs >= 2,
        "premise: the dog stood on {stood_on} surface(s) and left the ground {take_offs} time(s) \
         (active {active}, health {health:?})"
    );
    assert_eq!((active, health), (true, Ok(())));
}

/// ⭐ THE DOG COMES TO A PLAYER WHO IS FAR FROM IT, UP THE PLATFORMS.
///
/// The dog keeps near the player (`stay_within` in its catalog row). The
/// player stands on the upper deck of the basement, at the far end from the
/// dog. No straight walk gets there: the dog must climb. The advisor gives the
/// dog a place beside the player and the legs to it.
#[test]
fn the_dog_climbs_to_a_player_who_stands_far_away_on_the_upper_deck() {
    let (mut sim, dog, player) = the_dog_and_the_player();
    let at = |sim: &ambition_app::Platformer2dSimHarness, body: Entity| {
        sim.world().get::<BodyKinematics>(body).expect("a live body").pos
    };
    let floor = at(&sim, dog).y;
    // The upper deck of the basement, near its left end.
    sim.teleport_player((420.0, 1024.0 + 608.0 - 60.0));
    sim.step_n(base(), 30);
    let stands = at(&sim, player);
    // ⛔ THE PREMISES: the player is far from the dog and well above its floor.
    assert!(
        (stands.x - at(&sim, dog).x).abs() > 900.0 && floor - stands.y > 250.0,
        "the fixture did not stand the player far up the deck: player {stands:?}, dog {:?}",
        at(&sim, dog)
    );
    // ⛔ THE MECHANISM, not only the outcome. A dog that roams by chance
    // gets near the player in time too (this test passed that way before the
    // dog knew where the player was). So: the FIRST place the dog chooses,
    // with the player far away, is beside the player.
    let mut first_goal = None;
    let mut arrived = None;
    for tick in 0..3000 {
        sim.step(base());
        let goal = sim
            .world()
            .get::<ambition_platformer2d::characters::brain::Brain>(dog)
            .and_then(|brain| brain.navigation_request())
            .and_then(|request| request.goal);
        first_goal = first_goal.or(goal);
        let gap = at(&sim, dog) - at(&sim, player);
        if gap.x.abs() < 200.0 && gap.y.abs() < 40.0 {
            arrived = Some(tick);
            break;
        }
    }
    let beside = first_goal.map(|goal| goal - stands);
    assert!(
        beside.is_some_and(|offset| offset.x.abs() < 160.0 && offset.y.abs() < 80.0),
        "the dog's first place was {first_goal:?}, not beside the player at {stands:?}"
    );
    assert!(
        arrived.is_some(),
        "in 50 s the dog did not come to the player: dog {:?}, player {:?}, brain {:?}",
        at(&sim, dog),
        at(&sim, player),
        sim.world().get::<ambition_platformer2d::characters::brain::Brain>(dog),
    );
}

/// ⭐ NAVIGATION IS A CAPABILITY OF A BRAIN, NOT OF THE DOG'S BRAIN.
///
/// The dog's body with another policy: a `MeleeBrute` whose profile says it
/// navigates. That brain shares one thing with `Roam`, the route follower.
/// It chases its target as each brute does, and when the target stands on
/// another surface it takes the room's routes to it. The control is the same
/// brute with `navigates` off: it runs along the floor under the deck and
/// does not come up.
///
/// No shipped character has this policy yet: the rooms are sized for the
/// player's jump, and a brute cannot reach their platforms. This test is the
/// proof that the policy works where the room allows it.
#[test]
fn a_brute_brain_in_the_dogs_body_climbs_to_its_target_and_one_that_does_not_navigate_stays_below() {
    use ambition_platformer2d::characters::brain::state_machine::{MeleeBruteCfg, MeleeBruteState};
    use ambition_platformer2d::combat::components::{ActorAggression, Grudge};
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};

    let climbs = |navigates: bool| {
        let (mut sim, dog, player) = the_dog_and_the_player();
        let at = |sim: &ambition_app::Platformer2dSimHarness, body: Entity| {
            sim.world().get::<BodyKinematics>(body).expect("a live body").pos
        };
        *sim.world_mut().get_mut::<Brain>(dog).expect("the dog has a brain") =
            Brain::StateMachine(StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg {
                    aggressiveness: 1.0,
                    aggro_radius: 4000.0,
                    attack_range: 36.0,
                    chase_speed: 110.0,
                    navigates,
                },
                state: MeleeBruteState::default(),
            });
        // A brute chases a foe: the body has a grudge against the player, so
        // the player is its combat target.
        *sim.world_mut().get_mut::<ActorAggression>(dog).expect("an aggression") =
            ActorAggression { grudge: Some(Grudge::Body(player)), ..ActorAggression::hostile() };
        let floor = at(&sim, dog).y;
        // On the upper deck, above the dog and in its sight: a brute chases a
        // foe it knows of, and a foe across the room is not one.
        sim.teleport_player((1480.0, 1024.0 + 608.0 - 60.0));
        sim.step_n(base(), 30);
        assert!(floor - at(&sim, player).y > 250.0, "premise: the player stands well above the dog's floor");
        assert!(
            sim.world()
                .get::<ambition_platformer2d::combat::components::ActorTarget>(dog)
                .is_some_and(|target| target.entity == Some(player)),
            "premise: the brute's target is the player"
        );
        let mut highest = floor;
        for _ in 0..3000 {
            sim.step(base());
            let gap = at(&sim, dog) - at(&sim, player);
            highest = highest.min(at(&sim, dog).y);
            if gap.x.abs() < 200.0 && gap.y.abs() < 40.0 {
                return (true, floor - highest);
            }
        }
        (false, floor - highest)
    };
    let (arrived, rose) = climbs(true);
    assert!(arrived, "the brute that navigates did not come to its target; it rose {rose:.0} px");
    let (arrived, rose) = climbs(false);
    assert!(
        !arrived && rose < 100.0,
        "control: the brute that does not navigate came up (arrived {arrived}, rose {rose:.0} px)"
    );
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

/// The basement dog and the primary player, in a hub that has run 10 ticks.
fn the_dog_and_the_player() -> (ambition_app::Platformer2dSimHarness, Entity, Entity) {
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
    (sim, dog, player)
}

/// The gap between the player's box and the dog's footprint on world x
/// (the hub's side axis): negative when they overlap.
fn gap_to_the_dog(sim: &ambition_app::Platformer2dSimHarness, player: Entity, dog: Entity) -> f32 {
    let kin = sim.world().get::<BodyKinematics>(player).expect("a live player");
    let footprint = sim
        .world()
        .get::<ambition_platformer2d::combat::components::CenteredAabb>(dog)
        .expect("the dog has a footprint");
    (kin.pos.x - footprint.center.x).abs() - footprint.half_size.x - kin.size.x * 0.5
}

/// Press Interact and wait for a conversation with the dog to open.
fn talk_to_the_dog(sim: &mut ambition_app::Platformer2dSimHarness, dog: Entity) -> bool {
    let talking = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world()
            .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
            .is_some_and(|conversation| conversation.talker() == Some(dog))
    };
    sim.step(ambition_app::AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    });
    for _ in 0..10 {
        if talking(sim) {
            return true;
        }
        sim.step(base());
    }
    talking(sim)
}

/// ⭐ A CONVERSATION STEPS ITS BODIES APART, SO THEY LOOK LIKE THEY TALK.
///
/// Jon, 2026-10-07: *"When you engage in dialog, the dialog box should still
/// come up, but if it is safe for the player to move to a spot where there is
/// separation between the characters so it really looks like they are talking
/// (e.g. similar to how the dog pet works) it should do so."*
///
/// The player talks to the dog from on top of it. The conversation opens, the
/// player walks out to `TALK_GAP` from the dog's side, and the two face each
/// other. The conversation is still open at the end: the step stays inside
/// the reach a conversation breaks outside of.
#[test]
fn talking_from_on_top_of_the_dog_steps_the_player_out_to_a_talking_distance() {
    use ambition_platformer2d::actors::features::ecs::{TalkSpacing, TALK_GAP};

    let (mut sim, dog, player) = the_dog_and_the_player();
    let here = sim.world().get::<BodyKinematics>(dog).expect("a live dog").pos;
    sim.teleport_player((here.x + 4.0, here.y));
    // Land, so the step is asked of a body on its floor.
    sim.step_n(base(), 15);
    // ⛔ THE PREMISE: the two overlap when the conversation opens.
    assert!(
        gap_to_the_dog(&sim, player, dog) < 0.0,
        "the fixture did not stand the player on the dog: gap {}",
        gap_to_the_dog(&sim, player, dog)
    );
    assert!(talk_to_the_dog(&mut sim, dog), "Interact on the dog talks to it");

    let walking = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world().get::<TalkSpacing>(player).map(|spacing| spacing.walking)
    };
    assert!(
        walking(&sim).is_some_and(|walking| walking > 0.0),
        "the conversation did not start a step: {:?}",
        sim.world().get::<TalkSpacing>(player)
    );
    for _ in 0..180 {
        if walking(&sim) == Some(0.0) {
            break;
        }
        sim.step(base());
    }
    assert_eq!(walking(&sim), Some(0.0), "the step never ended");
    let gap = gap_to_the_dog(&sim, player, dog);
    assert!(
        (gap - TALK_GAP).abs() <= 3.0,
        "the player stopped {gap:.1}px from the dog, not at a talking distance of {TALK_GAP}px"
    );
    let at = |body: Entity| sim.world().get::<BodyKinematics>(body).expect("a live body");
    let toward_the_dog = (at(dog).pos.x - at(player).pos.x).signum();
    assert_eq!(
        (at(player).facing, at(dog).facing),
        (toward_the_dog, -toward_the_dog),
        "the two do not face each other"
    );
    assert!(
        sim.world()
            .get_resource::<ambition_platformer2d::conversation::ActiveConversation>()
            .is_some_and(|conversation| conversation.talker() == Some(dog)),
        "the step broke the conversation it was for"
    );
}

/// ⭐ INTERACT TALKS FROM A LITTLE WAY OFF, NOT ONLY FROM ON TOP.
///
/// Jon, 2026-10-07: *"We may also want a bit of a buffer so you can a talk
/// interaction can be triggered when you are close enough to the other
/// character."* A conversation used to open only when the two boxes
/// overlapped. The player stands clear of the dog, inside the talk reach, and
/// Interact opens the conversation.
#[test]
fn interact_talks_to_the_dog_from_a_step_away() {
    let (mut sim, dog, player) = the_dog_and_the_player();
    let footprint = *sim
        .world()
        .get::<ambition_platformer2d::combat::components::CenteredAabb>(dog)
        .expect("the dog has a footprint");
    let width = sim.world().get::<BodyKinematics>(player).expect("a live player").size.x;
    let reach = ambition_platformer2d::interaction::TALK_REACH;
    sim.teleport_player((
        footprint.center.x + footprint.half_size.x + width * 0.5 + reach * 0.5,
        footprint.center.y,
    ));
    sim.step_n(base(), 15);
    // ⛔ THE PREMISE: clear of the dog, and inside the reach. The dog roams,
    // so this is measured on the tick of the press.
    let gap = gap_to_the_dog(&sim, player, dog);
    assert!(
        gap > 2.0 && gap < reach - 2.0,
        "the fixture did not stand the player a step from the dog: gap {gap:.1}"
    );
    assert!(
        talk_to_the_dog(&mut sim, dog),
        "Interact {gap:.1}px from the dog did not talk to it"
    );
}
