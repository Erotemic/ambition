//! The Tyrant King's fight, played headlessly in the real arena: the authored
//! room, the scripted pattern, and the conducted module that performs it
//! (`ambition_content_modules::trex`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::trex::{conductor_of, Move, TrexView, TREX_ID};
use ambition_content::bosses::trex::conductor_module as module;
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ARENA: &str = "trex_arena";

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no T-rex, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the T-rex arena builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current())
}

fn untouchable_player(sim: &mut Platformer2dSimHarness, untouchable: bool) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, untouchable);
    }
}

#[derive(Debug)]
struct Rex {
    kin: ae::BodyKinematics,
    side: f32,
    view: TrexView,
}

fn rex(sim: &mut Platformer2dSimHarness) -> Rex {
    let world = sim.world_mut();
    let (entity, kin, side) = world
        .query::<(Entity, &BossConfig, &ae::BodyKinematics, &ambition_platformer2d::boss_encounter::conduct::ConductedFacing)>()
        .iter(world)
        .find(|(_, config, ..)| config.behavior.id == TREX_ID)
        .map(|(entity, _, kin, side)| (entity, kin.clone(), side.0))
        .expect("the T-rex is in the arena, conducted");
    let view = conductor_of(world, entity).expect("his conductor has measured the hall");
    Rex { kin, side, view }
}

/// Wound him to `fraction` of his health: past phase 1 at a half, enraged
/// near the end.
fn wound(sim: &mut Platformer2dSimHarness, fraction: f32) {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
    let (_, mut health) = q
        .iter_mut(world)
        .find(|(config, _)| config.behavior.id == TREX_ID)
        .expect("the T-rex");
    health.health.current = (health.max() as f32 * fraction).ceil() as i32;
}

fn rocks(sim: &mut Platformer2dSimHarness) -> Vec<ae::Vec2> {
    let world = sim.world_mut();
    let mut q = world.query::<(&ambition_platformer2d::projectiles::ProjectileVisualId, &ae::BodyKinematics)>();
    q.iter(world).filter(|(id, _)| id.0 == "trex_rock").map(|(_, kin)| kin.pos).collect()
}

fn step_until(
    sim: &mut Platformer2dSimHarness,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", rex(sim));
}

/// He stands on the floor: his drawn feet on it, every tick, walking or not.
#[test]
fn the_tyrant_stands_on_the_floor() {
    let mut sim = arena();
    let floor = rex(&mut sim).view.hall.expect("he measured his hall").floor;
    for _ in 0..120 {
        sim.step(AgentAction::default());
        let r = rex(&mut sim);
        let feet = r.kin.pos.y + module::feet_below();
        assert!((feet - floor).abs() < 0.5, "his feet are {feet}, the floor {floor}: {r:?}");
    }
}

/// Between moves he turns to you and stalks toward you.
#[test]
fn he_stalks_you_between_moves() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let r = rex(&mut sim);
    let hall = r.view.hall.expect("hall");
    // Far from him, across the hall.
    let far = if r.kin.pos.x > (hall.left + hall.right) * 0.5 { hall.left + 60.0 } else { hall.right - 60.0 };
    place_player(&mut sim, ae::Vec2::new(far, hall.floor - 30.0));
    let toward = (far - r.kin.pos.x).signum();
    let mut walked = 0.0f32;
    let mut last = r.kin.pos.x;
    for _ in 0..180 {
        sim.step(AgentAction::default());
        let r = rex(&mut sim);
        if r.view.performing.is_none() && !r.view.charging && !r.view.stunned {
            walked += (r.kin.pos.x - last) * toward;
        }
        last = r.kin.pos.x;
    }
    assert!(walked > 20.0, "he walked {walked} toward a player across the hall between moves");
}

/// The charge runs the hall and cannot stop: his snout meets the wall, and he
/// lies stunned there — the long punish window.
#[test]
fn a_charge_runs_into_the_wall_and_leaves_him_stunned() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 30, "him to charge", |sim| rex(sim).view.charging);
    step_until(&mut sim, 60 * 4, "the charge to hit the wall", |sim| rex(sim).view.stunned);
    let r = rex(&mut sim);
    let hall = r.view.hall.expect("hall");
    let wall = if r.side > 0.0 { hall.right } else { hall.left };
    let snout = r.kin.pos.x + r.side * (module::ram_front() + module::CRASH_RECOIL);
    assert!((snout - wall).abs() < 1.0, "he crashed with his snout at {snout}, the wall is {wall}: {r:?}");
    // He lies there: a window, not a flinch.
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    assert!(rex(&mut sim).view.stunned, "the stun lasted under a second");
}

/// His trunk hurts to touch: standing in him is not a hiding place.
#[test]
fn his_trunk_hurts_to_touch() {
    let mut sim = arena();
    let r = rex(&mut sim);
    let (trunk, _) = module::trunk();
    place_player(&mut sim, r.kin.pos + ae::Vec2::new(trunk.x * r.side, trunk.y + 30.0));
    let (_, before) = player(&mut sim);
    step_until(&mut sim, 60, "his trunk to hurt the player", |sim| player(sim).1 < before);
}

/// The bite comes down under his chin: the place below his head is not safe.
#[test]
fn the_bite_reaches_under_his_chin() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 30, "a bite's tell", |sim| {
        matches!(rex(sim).view.performing, Some((Move::Bite, false)))
    });
    // Stand where his chin is, on the floor, as he rears.
    let r = rex(&mut sim);
    let hall = r.view.hall.expect("hall");
    let (bite, _) = module::bite_box();
    place_player(&mut sim, ae::Vec2::new(r.kin.pos.x + r.side * bite.x, hall.floor - 24.0));
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    // The tell (up to 1.1 s) runs out, then the strike lands.
    step_until(&mut sim, 120, "the bite to land", |sim| player(sim).1 < before);
}

/// The arena's left ledge (`trex_arena_area.ron`): its top, and its middle.
const LEDGE_TOP: f32 = 464.0;
const LEDGE_X: f32 = 112.0;

fn stand_on_the_ledge(sim: &mut Platformer2dSimHarness) {
    let (kin, _) = player(sim);
    place_player(sim, ae::Vec2::new(LEDGE_X, LEDGE_TOP - kin.size.y * 0.5 - 1.0));
}

/// The ledge is a refuge from the charge: his lowered head runs under it.
#[test]
fn a_ledge_is_a_refuge_from_the_charge() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 30, "him to charge", |sim| rex(sim).view.charging);
    stand_on_the_ledge(&mut sim);
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    step_until(&mut sim, 60 * 4, "the charge to end in the wall", |sim| rex(sim).view.stunned);
    let (kin, after) = player(&mut sim);
    assert_eq!(after, before, "the charge reached a player on the ledge at {:?}", kin.pos);
}

/// Standing on a ledge is answered: he snaps up at it.
#[test]
fn standing_on_a_ledge_is_answered() {
    let mut sim = arena();
    stand_on_the_ledge(&mut sim);
    let (_, before) = player(&mut sim);
    let mut snapped = false;
    for _ in 0..60 * 20 {
        sim.step(AgentAction::default());
        if matches!(rex(&mut sim).view.performing, Some((Move::SnapUp, true))) {
            snapped = true;
        }
        if snapped && player(&mut sim).1 < before {
            return;
        }
        // Keep the player on the ledge whatever knocks them about.
        let (kin, _) = player(&mut sim);
        if (kin.pos.x - LEDGE_X).abs() > 80.0 || kin.pos.y > LEDGE_TOP {
            stand_on_the_ledge(&mut sim);
        }
    }
    panic!("twenty seconds on the ledge and no snap landed (snapped: {snapped}); last: {:?}", rex(&mut sim));
}

/// Wounded, he stomps: a shock runs the floor both ways from his foot, and
/// the ceiling sheds rocks over where you stand.
#[test]
fn wounded_he_stomps_a_shock_along_the_floor_and_the_ceiling_comes_down() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    step_until(&mut sim, 60 * 30, "him to stomp", |sim| {
        matches!(rex(sim).view.performing, Some((Move::Stomp, true)))
    });
    step_until(&mut sim, 10, "the shock to leave his foot", |sim| rex(sim).view.shock_rolling);
    let r = rex(&mut sim);
    let hall = r.view.hall.expect("hall");
    // A player standing in its path, clear of him, is caught by it.
    let x = if r.kin.pos.x > (hall.left + hall.right) * 0.5 { hall.left + 80.0 } else { hall.right - 80.0 };
    let (kin, _) = player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    let mut fell = 0;
    let mut hit = false;
    for _ in 0..150 {
        sim.step(AgentAction::default());
        fell = fell.max(rocks(&mut sim).len());
        hit |= player(&mut sim).1 < before;
    }
    assert!(hit, "a shock rolled the floor and never reached a player standing in its path at x {x}");
    assert!(fell > 0, "the stomp shook no rocks from the ceiling");
}

/// The rocks fall where the dust trickled: from under the ceiling, one over
/// where you stood when he stamped.
#[test]
fn the_rocks_fall_from_the_ceiling_over_where_you_stood() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    step_until(&mut sim, 60 * 30, "the ceiling to shake", |sim| rex(sim).view.rocks_falling);
    let (kin, _) = player(&mut sim);
    let stood = kin.pos.x;
    let mut first = Vec::new();
    step_until(&mut sim, 120, "a rock to fall", |sim| {
        first = rocks(sim);
        !first.is_empty()
    });
    let hall = rex(&mut sim).view.hall.expect("hall");
    let rock = first[0];
    assert!(rock.y < hall.floor - 500.0, "a rock appeared at {rock:?}, not under the ceiling");
    assert!((rock.x - stood).abs() < 40.0, "the first rock fell at x {}, the player stood at {stood}", rock.x);
}

/// Enraged, his charge turns once at the wall and comes back before it ends
/// in the other wall.
#[test]
fn enraged_his_charge_turns_once_and_comes_back() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.15);
    step_until(&mut sim, 60 * 40, "him to charge", |sim| rex(sim).view.charging);
    let set_off = rex(&mut sim).side;
    step_until(&mut sim, 60 * 4, "him to turn at the wall", |sim| {
        let r = rex(sim);
        r.view.charging && r.side != set_off
    });
    step_until(&mut sim, 60 * 4, "the charge back to end in the other wall", |sim| rex(sim).view.stunned);
    assert_ne!(rex(&mut sim).side, set_off, "he crashed facing the way he set off: he never came back");
}

/// Enraged, he leaps at where you stood and lands there, with a quake.
#[test]
fn enraged_he_leaps_at_where_you_stood() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.15);
    step_until(&mut sim, 60 * 40, "him to leap", |sim| matches!(rex(sim).view.performing, Some((Move::Leap, true))));
    let (kin, _) = player(&mut sim);
    let stood = kin.pos.x;
    // Out of the way, so where he lands is where you stood, not where you are.
    let hall = rex(&mut sim).view.hall.expect("hall");
    let away = if stood > (hall.left + hall.right) * 0.5 { hall.left + 60.0 } else { hall.right - 60.0 };
    place_player(&mut sim, ae::Vec2::new(away, hall.floor - kin.size.y * 0.5 - 1.0));
    let floor = hall.floor;
    let feet = |sim: &mut Platformer2dSimHarness| rex(sim).kin.pos.y + module::feet_below();
    step_until(&mut sim, 60, "him to leave the floor", |sim| floor - feet(sim) > 150.0);
    step_until(&mut sim, 60, "him to land", |sim| (feet(sim) - floor).abs() < 0.5);
    for _ in 0..2 {
        sim.step(AgentAction::default());
    }
    let r = rex(&mut sim);
    assert!(r.view.shock_rolling, "he landed without a quake: {r:?}");
    // He lands where you stood, as far as his body fits between the walls.
    let reach = module::ram_front().max(200.0);
    assert!((r.kin.pos.x - stood).abs() < reach, "he landed at x {}, you stood at {stood}", r.kin.pos.x);
    let feet = r.kin.pos.y + module::feet_below();
    assert!((feet - floor).abs() < 0.5, "he landed with his feet at {feet}, the floor at {floor}");
}

/// Enraged, he opens with a roar that blows you off him.
#[test]
fn enraged_he_roars_you_off_him() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.15);
    step_until(&mut sim, 60 * 40, "him to roar", |sim| matches!(rex(sim).view.performing, Some((Move::Roar, false))));
    let r = rex(&mut sim);
    // In front of his mouth, on the floor.
    let hall = r.view.hall.expect("hall");
    let (kin, _) = player(&mut sim);
    let x = r.kin.pos.x + r.side * (module::ram_front() + 40.0);
    place_player(&mut sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    step_until(&mut sim, 60 * 2, "the roar to land", |sim| player(sim).1 < before);
    let away = (player(&mut sim).0.pos.x - x) * r.side;
    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    let away_after = (player(&mut sim).0.pos.x - x) * r.side;
    assert!(away_after > away + 20.0, "the roar did not blow the player back: {away} then {away_after}");
}

/// His crash into the wall shakes the camera: the conductor's shake reaches
/// the presentation's request channel the tick he hits.
#[test]
fn the_crash_shakes_the_camera() {
    use ambition_platformer2d::platformer::camera_ease::CameraShakeRequest;
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 30, "him to charge", |sim| rex(sim).view.charging);
    let mut strongest = 0.0f32;
    step_until(&mut sim, 60 * 4, "the charge to hit the wall", |sim| {
        let shakes = sim.world_mut().resource_mut::<Messages<CameraShakeRequest>>();
        strongest = shakes.iter_current_update_messages().fold(strongest, |m, s| m.max(s.amplitude_px));
        rex(sim).view.stunned
    });
    assert!(strongest >= 10.0, "he crashed into the wall and the camera shook at most {strongest} px");
}

/// You hit him through his PARTS: head, jaw, neck, torso, tail and legs, posed
/// from the row he is drawn with. The air over his back and under his chin,
/// inside the old single box, is not him.
#[test]
fn he_is_hit_through_his_parts_not_a_box() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let (pos, side, volumes) = world
        .query::<(
            &BossConfig,
            &ae::BodyKinematics,
            &ambition_platformer2d::boss_encounter::conduct::ConductedFacing,
            &ambition_platformer2d::combat::components::DamageableVolumes,
        )>()
        .iter(world)
        .find(|(config, ..)| config.behavior.id == TREX_ID)
        .map(|(_, kin, side, volumes)| (kin.pos, side.0, volumes.volumes.clone()))
        .expect("the T-rex");
    assert!(volumes.len() >= 10, "he is hit through {} volumes, not his parts", volumes.len());
    // A sheet pixel of his idle art, as a world point (his position is the
    // frame's centre; the art faces right and he faces `side`).
    let art = |x: f32, y: f32| {
        let off = (ae::Vec2::new(x, y) - ae::Vec2::new(228.0, 150.0)) * module::PX;
        pos + ae::Vec2::new(off.x * side, off.y)
    };
    let hit = |p: ae::Vec2| {
        let probe = ae::Aabb::new(p, ae::Vec2::splat(3.0));
        volumes.iter().any(|v| v.intersects_aabb(probe))
    };
    for (what, x, y) in [("his head", 360.0, 105.0), ("his torso", 230.0, 160.0), ("his tail", 110.0, 140.0)] {
        assert!(hit(art(x, y)), "{what} at sheet ({x}, {y}) cannot be hit");
    }
    for (what, x, y) in [("the air over his back", 80.0, 75.0), ("the air under his chin", 350.0, 262.0)] {
        assert!(!hit(art(x, y)), "{what} at sheet ({x}, {y}) still counts as his body");
    }
}

/// His parts are posed from the row he is drawn with: stunned against the
/// wall, his head hangs, and the part you hit it through hangs with it.
#[test]
fn his_parts_follow_the_row_he_is_drawn_with() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    // The bottom of his front-most part (his head), from his position.
    let head_bottom = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(&BossConfig, &ae::BodyKinematics, &ambition_platformer2d::combat::components::DamageableVolumes)>()
            .iter(world)
            .find(|(config, ..)| config.behavior.id == TREX_ID)
            .and_then(|(_, kin, volumes)| {
                volumes
                    .volumes
                    .iter()
                    .map(|v| v.bounds())
                    .max_by(|a, b| {
                        let front = |bb: &ae::Aabb| ((bb.min.x + bb.max.x) * 0.5 - kin.pos.x) * kin.facing.signum();
                        front(a).total_cmp(&front(b))
                    })
                    .map(|bb| bb.max.y - kin.pos.y)
            })
            .expect("the T-rex's parts")
    };
    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    let standing = head_bottom(&mut sim);
    step_until(&mut sim, 60 * 30, "him to charge", |sim| rex(sim).view.charging);
    step_until(&mut sim, 60 * 4, "the charge to end in the wall", |sim| rex(sim).view.stunned);
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let stunned = head_bottom(&mut sim);
    assert!(stunned > standing + 25.0, "stunned, his head's part bottoms out at {stunned}; standing, {standing}");
}
