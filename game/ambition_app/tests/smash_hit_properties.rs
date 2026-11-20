//! Authored hit interactions in the shipped Smash match.

use ambition_platformer2d::{
    characters::{actor::BodyHealth, brain::Brain},
    combat::{clank::AttacksClanked, moveset::{ActorMoveset, MovePlayback}},
    engine_core as ae,
};
use bevy::prelude::*;

fn two_georges() -> (App, [Entity; 2]) {
    two_fighters([ambition_demo_smash::SMASH_GEORGE_BOOUL; 2])
}

fn two_fighters(characters: [&str; 2]) -> (App, [Entity; 2]) {
    let roster = ambition_demo_smash::smash_roster(characters);
    let mut app = crate::common::smash::live_round(roster);
    let bodies = [0, 1].map(|seat| crate::common::smash::seat(&mut app, seat));
    for body in bodies {
        app.world_mut().entity_mut(body).insert(Brain::stand_still());
    }
    for _ in 0..60 { app.update(); }
    let center = {
        let world = app.world_mut();
        world.query::<&ae::RoomGeometry>().single(world).unwrap().0.size.x * 0.5
    };
    for (n, body) in bodies.into_iter().enumerate() {
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap();
        kin.pos.x = center + if n == 0 { -20.0 } else { 20.0 };
        kin.vel = ae::Vec2::ZERO;
        kin.facing = if n == 0 { 1.0 } else { -1.0 };
    }
    for _ in 0..120 {
        app.update();
        if bodies.iter().all(|body| app.world().get::<ae::BodyGroundState>(*body).is_some_and(|ground| ground.on_ground)) { break; }
    }
    assert!(bodies.iter().all(|body| app.world().get::<ae::BodyGroundState>(*body).unwrap().on_ground));
    for body in bodies { app.world_mut().get_mut::<ambition_platformer2d::characters::actor::BodyCombat>(body).unwrap().damage_invuln_timer = 0.0; }
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

#[test]
fn a_transcendent_volume_reaches_a_body_without_clashing() {
    use ambition_platformer2d::entity_catalog::ClashPolicy;
    let (mut app, bodies) = two_georges();
    for (n, body) in bodies.into_iter().enumerate() {
        let moves = app.world().get::<ActorMoveset>(body).unwrap();
        let mut jab = moves.0.moves.iter().find(|spec| spec.id == "jab").unwrap().clone();
        if n == 0 {
            for volume in jab.windows.iter_mut().flat_map(|window| &mut window.volumes) {
                volume.properties.clash = ClashPolicy::Transcendent;
            }
        }
        app.world_mut().entity_mut(body).insert(MovePlayback::new(std::sync::Arc::new(jab), if n == 0 { 1.0 } else { -1.0 }));
    }
    let mut cursor = bevy::ecs::message::MessageCursor::<AttacksClanked>::default();
    let mut clanked = false;
    for _ in 0..90 {
        app.update();
        clanked |= cursor.read(app.world().resource::<Messages<AttacksClanked>>()).next().is_some();
    }
    assert!(!clanked);
    assert!(bodies.iter().any(|body| app.world().get::<BodyHealth>(*body).unwrap().damage_taken() > 0));
}

#[test]
fn the_shipped_down_air_chooses_its_floor_pop_or_air_spike_at_contact() {
    use ambition_platformer2d::combat::{HitEvent, HitTarget};
    for airborne in [false, true] {
        let (mut app, [attacker, victim]) = two_fighters(["projectile_polygon", ambition_demo_smash::SMASH_GEORGE_BOOUL]);
        let target = app.world().get::<ae::BodyKinematics>(victim).unwrap().pos
            - if airborne { ae::Vec2::new(0.0, 220.0) } else { ae::Vec2::ZERO };
        for (body, pos) in [(victim, target), (attacker, target - ae::Vec2::new(0.0, 50.0))] {
            let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap();
            kin.pos = pos;
            kin.vel = ae::Vec2::ZERO;
        }
        let spec = app.world().get::<ActorMoveset>(attacker).unwrap().0.moves.iter()
            .find(|spec| spec.id == "polygon_projectile_air_down").unwrap().clone();
        app.world_mut().entity_mut(attacker).insert(MovePlayback::new(std::sync::Arc::new(spec), 1.0).started_in_stance(false));
        let mut cursor = bevy::ecs::message::MessageCursor::<HitEvent>::default();
        let mut landed = None;
        for _ in 0..90 {
            app.update();
            if let Some(hit) = cursor.read(app.world().resource::<Messages<HitEvent>>())
                .find(|hit| hit.attacker == Some(attacker) && hit.target == HitTarget::Body(victim)) {
                landed = Some(hit.clone());
                break;
            }
        }
        let hit = landed.expect("the authored down-air must reach the staged victim");
        assert_eq!(hit.properties.targets.airborne, airborne);
        assert_eq!(hit.properties.targets.grounded, !airborne);
        assert_eq!(hit.knockback.unwrap().launch_dir.unwrap().world().y.is_sign_positive(), airborne);
    }
}

#[test]
fn an_airborne_grab_press_plays_the_authored_tether_hit_without_capture() {
    use bevy::ecs::system::RunSystemOnce;
    use ambition_platformer2d::characters::control::ActorControl;
    let (mut app, [attacker, victim]) = two_fighters(["projectile_polygon", ambition_demo_smash::SMASH_GEORGE_BOOUL]);
    {
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(attacker).unwrap();
        kin.pos.y -= 20.0;
    }
    app.world_mut().get_mut::<ae::BodyGroundState>(attacker).unwrap().on_ground = false;
    app.world_mut().get_mut::<ActorControl>(attacker).unwrap().0.grab_pressed = true;
    app.world_mut().run_system_once(ambition_platformer2d::combat::moveset::trigger_moveset_moves).unwrap();
    let playing = app.world().get::<MovePlayback>(attacker).expect("the airborne press starts a move");
    assert_eq!(playing.spec.id, "polygon_projectile_tether_air");
    assert!(!playing.spec.frame_data().captures);
    let mut captured = false;
    for _ in 0..90 {
        app.update();
        captured |= app.world().get::<ambition_platformer2d::combat::capture::CapturedBy>(victim).is_some();
    }
    assert!(!captured);
    assert!(app.world().get::<BodyHealth>(victim).unwrap().damage_taken() > 0);
}

/// S0.8: the Patent Clerk's authored light argument paralyzes the body it
/// catches. The victim is held where it was struck and cannot act; the hit's
/// launch and stun wait for the end, and then it is launched.
#[test]
fn the_light_argument_holds_its_victim_and_the_launch_waits_for_the_end() {
    use ambition_platformer2d::characters::actor::BodyCombat;
    let (mut app, [clerk, victim]) = two_fighters(["special_patent_clerk", ambition_demo_smash::SMASH_GEORGE_BOOUL]);
    let spec = app.world().get::<ActorMoveset>(clerk).unwrap().0.moves.iter()
        .find(|spec| spec.id == "light_argument").unwrap().clone();
    app.world_mut().entity_mut(clerk).insert(MovePlayback::new(std::sync::Arc::new(spec), 1.0));
    let mut held_at = None;
    for _ in 0..90 {
        app.update();
        if app.world().get::<BodyCombat>(victim).unwrap().is_paralyzed() {
            held_at = Some(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos);
            break;
        }
    }
    let held_at = held_at.expect("the authored light argument must paralyze the body it reaches");
    let combat = *app.world().get::<BodyCombat>(victim).unwrap();
    assert!(combat.hard_lock_timer() > 0.0, "a paralyzed body cannot act");
    let stun = combat.hitstun_timer;
    assert!(stun > 0.0, "the hit charged its stun");
    let mut ticks = 0;
    while app.world().get::<BodyCombat>(victim).unwrap().is_paralyzed() {
        assert!(ticks < 60, "the paralysis outlasted its authored 0.6 s");
        assert_eq!(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos, held_at, "the body moved while held");
        app.update();
        ticks += 1;
    }
    assert!(ticks >= 30, "held only {ticks} ticks of an authored 0.6 s");
    assert!(app.world().get::<BodyCombat>(victim).unwrap().hitstun_timer > 0.0, "the stun waited for the end of the hold");
    for _ in 0..10 {
        app.update();
    }
    assert!(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos.x > held_at.x + 1.0, "the launch is spent after the hold");
}

/// S0.8: a real hit ends a paralysis, as it wakes a sleeper: being struck is
/// the attacker's counterplay to the hold, and the hit's own stun replaces it.
#[test]
fn a_real_hit_ends_a_paralysis() {
    use ambition_platformer2d::characters::actor::BodyCombat;
    let (mut app, [attacker, victim]) = two_georges();
    app.world_mut().get_mut::<BodyCombat>(victim).unwrap().paralysis_timer = 1.5;
    let jab = app.world().get::<ActorMoveset>(attacker).unwrap().0.moves.iter().find(|spec| spec.id == "jab").unwrap().clone();
    app.world_mut().entity_mut(attacker).insert(MovePlayback::new(std::sync::Arc::new(jab), 1.0));
    let before = app.world().get::<BodyHealth>(victim).unwrap().damage_taken();
    for _ in 0..40 {
        app.update();
        if app.world().get::<BodyHealth>(victim).unwrap().damage_taken() > before {
            break;
        }
    }
    assert!(app.world().get::<BodyHealth>(victim).unwrap().damage_taken() > before, "the jab must land");
    assert!(!app.world().get::<BodyCombat>(victim).unwrap().is_paralyzed(), "the hit ended the paralysis");
}

/// S0.8: Bob's authored pile driver buries a standing body. The victim is
/// held where it stood (its box stays on the floor), cannot act, and is not
/// launched: the burial replaced the spike. It stands where it was when the
/// burial ends.
#[test]
fn the_pile_driver_buries_a_standing_body_instead_of_spiking_it() {
    use ambition_platformer2d::characters::actor::BodyCombat;
    let (mut app, [bob, victim]) = two_fighters(["npc_bob", ambition_demo_smash::SMASH_GEORGE_BOOUL]);
    let stood_at = app.world().get::<ae::BodyKinematics>(victim).unwrap().pos;
    {
        let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(bob).unwrap();
        kin.pos = stood_at - ae::Vec2::new(0.0, 50.0);
        kin.vel = ae::Vec2::ZERO;
    }
    app.world_mut().get_mut::<ae::BodyGroundState>(bob).unwrap().on_ground = false;
    let spec = app.world().get::<ActorMoveset>(bob).unwrap().0.moves.iter()
        .find(|spec| spec.id == "pile_driver").unwrap().clone();
    app.world_mut().entity_mut(bob).insert(MovePlayback::new(std::sync::Arc::new(spec), 1.0).started_in_stance(false));
    let mut buried = false;
    for _ in 0..60 {
        app.update();
        if app.world().get::<BodyCombat>(victim).unwrap().is_buried() {
            buried = true;
            break;
        }
    }
    assert!(buried, "the authored pile driver must bury the standing body it lands on");
    let held_at = app.world().get::<ae::BodyKinematics>(victim).unwrap().pos;
    assert!((held_at.y - stood_at.y).abs() < 1.0, "the burial embedded the box: {held_at:?} from {stood_at:?}");
    while app.world().get::<BodyCombat>(victim).unwrap().is_buried() {
        assert_eq!(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos, held_at, "the buried body moved");
        app.update();
    }
    for _ in 0..10 {
        app.update();
    }
    let after = app.world().get::<ae::BodyKinematics>(victim).unwrap().pos;
    assert!((after - held_at).length() < 2.0, "the burial's hit still launched the body: {after:?} from {held_at:?}");
}

/// S0.8: the Perfect Cellular Automaton's authored pulse freezes the body it
/// catches into a still life: it cannot act, and it is not held, so the
/// pulse's launch still carries it. That is the difference from hitlag.
#[test]
fn the_cellular_pulse_freezes_its_victim_and_the_block_still_flies() {
    use ambition_platformer2d::characters::actor::BodyCombat;
    let (mut app, [automaton, victim]) = two_fighters(["perfect_cellular_automaton", ambition_demo_smash::SMASH_GEORGE_BOOUL]);
    let spec = app.world().get::<ActorMoveset>(automaton).unwrap().0.moves.iter()
        .find(|spec| spec.id == "cellular_pulse").unwrap().clone();
    app.world_mut().entity_mut(automaton).insert(MovePlayback::new(std::sync::Arc::new(spec), 1.0));
    let mut frozen_at = None;
    for _ in 0..90 {
        app.update();
        if app.world().get::<BodyCombat>(victim).unwrap().is_frozen() {
            frozen_at = Some(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos);
            break;
        }
    }
    let frozen_at = frozen_at.expect("the authored cellular pulse must freeze the body it reaches");
    assert!(app.world().get::<BodyCombat>(victim).unwrap().hard_lock_timer() > 0.0, "a frozen body cannot act");
    for _ in 0..20 {
        app.update();
    }
    assert!(app.world().get::<BodyCombat>(victim).unwrap().is_frozen(), "the freeze ended early");
    let moved = (app.world().get::<ae::BodyKinematics>(victim).unwrap().pos - frozen_at).length();
    assert!(moved > 2.0, "the frozen block was held where it was struck ({moved}px), as hitlag would");
}

/// S0.8: Alice's authored side channel crumples a standing body: it is not
/// launched, it sags where it stood, and then it lies prone in the floor game.
#[test]
fn alices_side_channel_crumples_a_standing_body_into_the_floor_game() {
    use ambition_platformer2d::characters::actor::BodyCombat;
    let (mut app, [alice, victim]) = two_fighters(["npc_alice", ambition_demo_smash::SMASH_GEORGE_BOOUL]);
    let spec = app.world().get::<ActorMoveset>(alice).unwrap().0.moves.iter()
        .find(|spec| spec.id == "side_channel").unwrap().clone();
    app.world_mut().entity_mut(alice).insert(MovePlayback::new(std::sync::Arc::new(spec), 1.0));
    let mut sagged_at = None;
    for _ in 0..90 {
        app.update();
        if app.world().get::<BodyCombat>(victim).unwrap().is_crumpling() {
            sagged_at = Some(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos);
            break;
        }
    }
    let sagged_at = sagged_at.expect("the authored side channel must crumple the standing body it reaches");
    while app.world().get::<BodyCombat>(victim).unwrap().is_crumpling() {
        assert_eq!(app.world().get::<ae::BodyKinematics>(victim).unwrap().pos, sagged_at, "the sagging body moved");
        app.update();
    }
    app.update();
    let facts = app.world().get::<ae::BodyMotionFacts>(victim).unwrap();
    assert!(facts.knocked_down, "the crumple did not end prone in the floor game");
}
