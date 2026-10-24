//! The robot's fireball flies as Mario's does (Jon, 2026-10-24: "The fireball
//! should feel like a mario fireball in terms of how it bounces"): it leaves the
//! hand angled down, hops along the floor at one low height and one forward
//! speed, does not sail over a standing body, and dies on a wall or at the end
//! of its range. Played in a flat room with rollback on.

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::projectiles::ProjectileGameplay;
use bevy::math::Vec2;
use bevy::prelude::{Entity, With};

/// A room with a long flat floor in front of where the robot starts.
const ROOM: &str = "switch_lab";

fn lab() -> Platformer2dSimHarness {
    // Rollback on, as the game plays it: GGRS drives the schedule.
    let opts = fixed_60hz_room_options(ROOM).with_sync_test_rollback_settings(0, 12);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the lab builds headlessly");
    sim.step_n(base(), 90);
    sim
}

fn me(sim: &mut Platformer2dSimHarness) -> BodyKinematics {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&BodyKinematics, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
    q.single(world).expect("one primary body").clone()
}

fn tap(sim: &mut Platformer2dSimHarness) {
    sim.step(AgentAction { projectile: true, projectile_held: true, ..base() });
    sim.step(AgentAction { projectile_released: true, ..base() });
}

/// One shot's flight: its centre and velocity each tick it lived.
struct Flight {
    path: Vec<(Vec2, Vec2)>,
    half: Vec2,
    floor_y: f32,
    from_x: f32,
}

impl Flight {
    /// How high the shot's bottom rises above the floor at the top of each
    /// hop after its first bounce.
    fn hop_heights(&self) -> Vec<f32> {
        let mut peaks = Vec::new();
        let mut bounced = false;
        for pair in self.path.windows(2) {
            let ((_, v0), (p1, v1)) = (pair[0], pair[1]);
            if v0.y > 0.0 && v1.y < 0.0 {
                bounced = true;
            }
            if bounced && v0.y < 0.0 && v1.y >= 0.0 {
                peaks.push(self.floor_y - (p1.y + self.half.y));
            }
        }
        peaks
    }

    fn bounces(&self) -> usize {
        self.path.windows(2).filter(|pair| pair[0].1.y > 0.0 && pair[1].1.y < 0.0).count()
    }

    fn first_bounce_x(&self) -> Option<f32> {
        let tick = self.path.windows(2).position(|pair| pair[0].1.y > 0.0 && pair[1].1.y < 0.0)?;
        Some((self.path[tick].0.x - self.from_x).abs())
    }

    fn range(&self) -> f32 {
        self.path.last().map_or(0.0, |last| (last.0.x - self.from_x).abs())
    }
}

fn my_entity(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
    q.single(world).expect("one primary body")
}

fn fly_one(sim: &mut Platformer2dSimHarness) -> Flight {
    let body = me(sim);
    let owner = my_entity(sim);
    let floor_y = body.pos.y + body.size.y * 0.5;
    tap(sim);
    let mut path = Vec::new();
    let mut half = Vec2::ZERO;
    for _ in 0..240 {
        let world = sim.world_mut();
        let mut shots = world.query_filtered::<(&BodyKinematics, &ambition_platformer2d::projectiles::ProjectileOwner), With<ProjectileGameplay>>();
        match shots.iter(world).find(|(_, by)| by.0 == owner).map(|(kin, _)| kin) {
            Some(shot) => {
                half = shot.size * 0.5;
                path.push((shot.pos, shot.vel));
            }
            None if !path.is_empty() => break,
            None => {}
        }
        sim.step(base());
    }
    Flight { path, half, floor_y, from_x: body.pos.x }
}

/// The robot's tap, on flat floor: where it first lands, how high it hops,
/// how far it goes.
fn measured_flight() -> Flight {
    let mut sim = lab();
    let flight = fly_one(&mut sim);
    eprintln!(
        "fireball: {} ticks, range {:.0} px, {} bounces, first bounce {:?} px out, hops {:?} px, \
         born {:?} above the floor (half {:?})",
        flight.path.len(),
        flight.range(),
        flight.bounces(),
        flight.first_bounce_x(),
        flight.hop_heights().iter().map(|h| h.round()).collect::<Vec<_>>(),
        flight.path.first().map(|(p, _)| flight.floor_y - p.y),
        flight.half,
    );
    flight
}

/// A tap at a standing body `distance` px ahead of the robot: did it hurt it?
fn tap_hits_a_standing_body(distance: f32) -> bool {
    let mut sim = lab();
    let body = me(&mut sim);
    let facing = body.facing.signum();
    sim.spawn_enemy_character_at(
        "standing_target",
        "Robot",
        (body.pos.x + facing * distance, body.pos.y),
        (14.0, 23.0),
        CharacterBrain::Passive,
        "player_robot_v3",
    );
    sim.step_n(base(), 30);
    let target = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        q.iter(world).find(|(_, id)| id.as_str() == "standing_target").map(|(e, _)| e).expect("the target")
    };
    let hp = |sim: &Platformer2dSimHarness| sim.world().get::<BodyHealth>(target).expect("its health").current();
    let before = hp(&sim);
    tap(&mut sim);
    sim.step_n(base(), 120);
    hp(&sim) < before
}

#[test]
#[ignore = "a measurement; run with --ignored"]
fn measure_the_fireballs_flight_and_its_hits_on_a_standing_body() {
    measured_flight();
    let distances = [60.0, 120.0, 180.0, 240.0, 300.0, 360.0];
    let hits: Vec<bool> = distances.iter().map(|d| tap_hits_a_standing_body(*d)).collect();
    eprintln!(
        "standing body: hit {} of {} ({:?})",
        hits.iter().filter(|h| **h).count(),
        hits.len(),
        distances.iter().zip(&hits).collect::<Vec<_>>(),
    );
}

/// On flat floor a tap falls to the floor near the hand and then hops: every
/// hop to one height, and the top of the shot under the head of a body the
/// robot's height. So it does not sail over a body that stands in its way:
/// a tap hurts a standing body anywhere along its range.
#[test]
fn a_fireball_hops_low_and_even_and_hurts_a_standing_body_along_its_range() {
    let flight = measured_flight();
    let hops = flight.hop_heights();
    assert!(hops.len() >= 2, "it hopped along the floor: {hops:?}");
    let (low, high) = hops.iter().fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(*h), hi.max(*h)));
    assert!(high - low < 1.5, "each hop rises to one height: {hops:?}");
    let robot_height = 48.0;
    assert!(
        high + 2.0 * flight.half.y < robot_height * 0.9,
        "a hop carries the shot's top to {} px, over the head of a body {robot_height} px tall",
        high + 2.0 * flight.half.y,
    );
    let range = flight.range();
    for fraction in [0.15, 0.5, 0.8] {
        let distance = range * fraction;
        assert!(tap_hits_a_standing_body(distance), "a tap flew past a body standing {distance:.0} px away");
    }
}

/// The place the charge hangs while the fire button is held and the place the
/// shot is born when it is let go are one place, in the palms of the hold:
/// the robot's charge is drawn where its shot appears.
#[test]
fn the_charge_hangs_where_the_shot_is_born() {
    let (charge, born, _) = charge_and_birth();
    assert!(
        (charge - born).length() < 0.5,
        "the charge hangs at {charge:?} from the body's centre; the shot is born at {born:?}",
    );
}

/// While the button is held: where the charge hangs, from the body's centre.
/// Let go: where the shot is born, from the body's centre. And the body.
fn charge_and_birth() -> (Vec2, Vec2, BodyKinematics) {
    let mut sim = lab();
    let owner = my_entity(&mut sim);
    sim.step(AgentAction { projectile: true, projectile_held: true, ..base() });
    sim.step_n(AgentAction { projectile_held: true, ..base() }, 4);
    let body = me(&mut sim);
    let charge = sim
        .world()
        .get::<ambition_platformer2d::actor::BodyPoseView>(owner)
        .and_then(|pose| pose.charge_offset)
        .expect("a held charge has a place");
    sim.step(AgentAction { projectile_released: true, ..base() });
    let born = sim
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_platformer2d::projectiles::ProjectileSpawnRequest>>()
        .iter_current_update_messages()
        .find(|request| request.owner == owner)
        .map(|request| request.projectile.body.kin.pos - body.pos)
        .expect("the release fired a shot");
    (charge, born, body)
}

/// A probe for the proof strip: the robot's near hand in each frame of `aim`
/// and `shoot`, and where the charge hangs and the shot is born, in sheet
/// pixels from the feet (the art's own space), as JSON on stderr.
#[test]
#[ignore = "a probe; run with --ignored"]
fn probe_the_hand_the_charge_and_the_birth_in_sheet_pixels() {
    use crate::a_pet_hand_meets_the_contact_point::drawn_at;
    let (charge, born, body) = charge_and_birth();
    let mut sim = lab();
    let owner = my_entity(&mut sim);
    let sheet = sim
        .world()
        .get::<ambition_platformer2d::sprite_sheet::character::SpritePosedBody>(owner)
        .expect("a posed body")
        .clone();
    let facing = body.facing.signum();
    // From the body's centre (world) to feet-relative sheet pixels (+x the way
    // it faces).
    let to_px = |offset: Vec2| {
        let from_feet = offset - Vec2::new(0.0, body.size.y * 0.5);
        Vec2::new(from_feet.x * facing, from_feet.y) / sheet.world_per_pixel
    };
    let rows = ["aim", "shoot"].map(|row| {
        let frames = (0..16)
            .map_while(|i| {
                std::panic::catch_unwind(|| drawn_at(&sheet.target, &[row], "near_hand", |n| if i < n { i } else { usize::MAX }))
                    .ok()
            })
            .map(|p| format!("[{:.2},{:.2}]", p.x, p.y))
            .collect::<Vec<_>>();
        format!("\"{row}\":[{}]", frames.join(","))
    });
    let charge = to_px(charge);
    let born = to_px(born);
    eprintln!(
        "PROOF_JSON {{\"target\":\"{}\",\"world_per_pixel\":{},{},\"charge\":[{:.2},{:.2}],\"born\":[{:.2},{:.2}]}}",
        sheet.target, sheet.world_per_pixel, rows.join(","), charge.x, charge.y, born.x, born.y,
    );
}
