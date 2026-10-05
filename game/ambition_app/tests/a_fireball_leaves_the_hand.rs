//! The player's shot is born at the hand that fires it.
//!
//! The shot was born off the edge of the body's box in the aim direction, a
//! fifth of the body's height above its centre: in front of the robot's face.
//! The robot's `shoot` row holds the orb in its near hand at knee height and
//! draws the bolt leaving from there. The shot is now born with its rear edge
//! at that hand, from the landmark query (ruling Q41).
//!
//! ⚠ The expected hand does NOT come from that query. It comes from the draw
//! table the renderer draws the `shoot` row from, at the scale the body states.

#![cfg(feature = "rl_sim")]

use crate::a_pet_hand_meets_the_contact_point::{art_point_in_world, drawn_at_the_middle};
use crate::common::{base, fixed_60hz_sim};
use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::projectiles::ProjectileSpawnRequest;
use ambition_platformer2d::sprite_sheet::character::SpritePosedBody;
use bevy::math::Vec2;
use bevy::prelude::{Entity, Messages, With};

/// The primary body, standing, and the shot one tap of its ranged button asks
/// for: where it is born and its half extent.
struct Tap {
    body: BodyKinematics,
    sheet: SpritePosedBody,
    born: Vec2,
    half: Vec2,
}

fn requests(sim: &Platformer2dSimHarness, owner: Entity) -> Vec<(Vec2, Vec2)> {
    sim.world()
        .resource::<Messages<ProjectileSpawnRequest>>()
        .iter_current_update_messages()
        .filter(|request| request.owner == owner)
        .map(|request| {
            let kin = &request.projectile.body.kin;
            (kin.pos, kin.size * 0.5)
        })
        .collect()
}

fn tap_the_ranged_button(sim: &mut Platformer2dSimHarness) -> Tap {
    sim.step_n(base(), 120);
    let player = {
        let world = sim.world_mut();
        let mut query = world
            .query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
        query.single(world).expect("one primary body")
    };
    let body = sim.world().get::<BodyKinematics>(player).expect("a live body").clone();
    let sheet = sim
        .world()
        .get::<SpritePosedBody>(player)
        .expect("the protagonist is a posed body, which states its own art scale")
        .clone();
    let mut born = Vec::new();
    sim.step(ambition_app::AgentAction {
        projectile: true,
        projectile_held: true,
        ..base()
    });
    born.extend(requests(sim, player));
    sim.step(ambition_app::AgentAction {
        projectile_released: true,
        ..base()
    });
    born.extend(requests(sim, player));
    assert_eq!(born.len(), 1, "one tap asks for one shot: {born:?}");
    let still = sim.world().get::<BodyKinematics>(player).expect("a live body").pos;
    assert!(
        (still - body.pos).length() < 0.01,
        "control: the body stood still through the tap ({:?} -> {still:?})",
        body.pos,
    );
    Tap {
        body,
        sheet,
        born: born[0].0,
        half: born[0].1,
    }
}

#[test]
fn a_shot_is_born_with_its_rear_edge_at_the_hand_that_fires_it() {
    let mut sim = fixed_60hz_sim();
    let tap = tap_the_ranged_button(&mut sim);
    let facing = tap.body.facing.signum();
    let hand = art_point_in_world(
        &tap.body,
        drawn_at_the_middle(&tap.sheet.target, &["shoot", "idle"], "near_hand"),
        tap.sheet.world_per_pixel,
    );
    let feet_y = tap.body.pos.y + tap.body.size.y * 0.5;
    // A shot born touching the floor dies, so a shot whose hand is too near
    // the feet is born just clear of them.
    let expected = Vec2::new(hand.x + facing * tap.half.x, hand.y.min(feet_y - tap.half.y - 1.0));
    // The place the two boxes gave: off the front of the body's box, a fifth
    // of its height above its centre.
    let box_muzzle = tap.body.pos + Vec2::new(facing * (tap.body.size.x * 0.5 + 4.0), -0.2 * tap.body.size.y);
    eprintln!(
        "shot: born {:?} (from the centre {:?}), half {:?}; drawn hand {:?}; expected {expected:?}; \
         the box muzzle was {box_muzzle:?}",
        tap.born,
        tap.born - tap.body.pos,
        tap.half,
        hand,
    );
    // A measurement, not an assertion: how far the shot flies before it first
    // meets the floor. The place a shot is born changes this, and it is what
    // a player feels.
    let mut path = Vec::new();
    for _ in 0..90 {
        sim.step(base());
        let world = sim.world_mut();
        let mut shots = world.query_filtered::<&BodyKinematics, With<ambition_platformer2d::projectiles::ProjectileGameplay>>();
        match shots.iter(world).next() {
            Some(shot) => path.push((shot.pos, shot.vel)),
            None => break,
        }
    }
    let first_bounce = path.windows(2).position(|pair| pair[0].1.y > 0.0 && pair[1].1.y < 0.0);
    eprintln!(
        "shot flight: {} ticks alive, {:.0} travelled; first bounce after {:?} ticks, {:?} ahead of \
         where it was born",
        path.len(),
        path.last().map_or(0.0, |last| (last.0.x - tap.born.x).abs()),
        first_bounce,
        first_bounce.map(|tick| (path[tick].0.x - tap.born.x).abs()),
    );

    assert!(
        (tap.born - expected).length() < 0.5,
        "the shot is born at {:?}; with its rear edge at the hand ({hand:?}) it is born at {expected:?}",
        tap.born,
    );
    // Control: the two places are far apart for this body, so this arm tells
    // them apart.
    assert!(
        (box_muzzle - expected).length() > 15.0,
        "control: the box muzzle ({box_muzzle:?}) and the hand ({expected:?}) are too near to \
         witness which one the shot used",
    );
}

/// Art that reaches with its far hand fires from that hand.
///
/// The Robot V1 art (`robot`, `player_robot_v2`) is drawn three-quarter on:
/// the hand nearer the viewer trails the body, and the far hand is on the
/// side it faces. Its catalog row states `gesture_hand: Far`. A shot from the
/// near hand is born behind the body's back.
#[test]
fn art_that_reaches_with_its_far_hand_fires_from_that_hand() {
    use ambition_platformer2d::combat::components::FeatureId;
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
    const TARGET: &str = "player_robot_v2";

    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 60);
    let player = {
        let world = sim.world_mut();
        let mut query = world
            .query_filtered::<&BodyKinematics, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
        query.single(world).expect("one primary body").pos
    };
    // A hostile body with the player's own kit opens with its ranged move.
    sim.spawn_enemy_character_at(
        "far_hand_robot",
        "Robot",
        (player.x + 200.0, player.y),
        (14.0, 23.0),
        CharacterBrain::Custom("player_robot".to_string()),
        TARGET,
    );
    let robot = {
        let world = sim.world_mut();
        let mut query = world.query::<(Entity, &FeatureId)>();
        query
            .iter(world)
            .find(|(_, id)| id.as_str() == "far_hand_robot")
            .map(|(entity, _)| entity)
            .expect("the robot was built")
    };
    let sheet = sim
        .world()
        .get::<SpritePosedBody>(robot)
        .expect("the robot is a posed body, which states its own art scale")
        .clone();
    assert_eq!(sheet.target, TARGET);
    let shot = (0..600).find_map(|_| {
        sim.step(base());
        requests(&sim, robot).into_iter().next()
    });
    let (born, half) = shot.expect("the robot fired no shot in ten seconds");
    let body = sim.world().get::<BodyKinematics>(robot).expect("a live body").clone();
    let facing = body.facing.signum();
    let at_hand = |track: &str| {
        let hand = art_point_in_world(
            &body,
            drawn_at_the_middle(&sheet.target, &["shoot", "idle"], track),
            sheet.world_per_pixel,
        );
        let feet_y = body.pos.y + body.size.y * 0.5;
        Vec2::new(hand.x + facing * half.x, hand.y.min(feet_y - half.y - 1.0))
    };
    let (far, near) = (at_hand("back_hand"), at_hand("front_hand"));
    eprintln!(
        "far-hand shot: born {born:?} (from the centre {:?}, facing {facing}), half {half:?}; from the far \
         hand {far:?}; from the near hand {near:?}",
        born - body.pos,
    );
    assert!(
        (born - far).length() < 0.5,
        "the shot is born at {born:?}; with its rear edge at the far hand it is born at {far:?} (and at \
         {near:?} from the near hand)",
    );
    assert!(
        (born.x - body.pos.x) * facing > body.size.x * 0.5,
        "the shot is born at {born:?}, which is not in front of the body at {:?}",
        body.pos,
    );
    // Control: the two hands are far apart in this art, so this arm tells
    // them apart.
    assert!(
        (far - near).length() > 15.0,
        "control: the far hand ({far:?}) and the near hand ({near:?}) are too near to witness which \
         one the shot used",
    );
}
