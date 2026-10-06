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

/// Stand in front of him (on the floor, just beyond his snout) every tick,
/// until `until` says so: the jaw grab is a near move.
fn stand_in_front_until(
    sim: &mut Platformer2dSimHarness,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        let r = rex(sim);
        let hall = r.view.hall.expect("hall");
        let (kin, _) = player(sim);
        let x = r.kin.pos.x + r.side * (module::ram_front() - 10.0);
        place_player(sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", rex(sim));
}

fn held(sim: &mut Platformer2dSimHarness) -> bool {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition_platformer2d::combat::capture::CapturedBy, PrimaryPlayerOnly>();
    q.iter(world).next().is_some()
}

/// Wounded, he grabs you in his jaws, thrashes you, and flings you across the
/// hall.
#[test]
fn wounded_he_grabs_you_thrashes_you_and_flings_you() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    stand_in_front_until(&mut sim, 60 * 40, "him to lunge for a grab", |sim| {
        matches!(rex(sim).view.performing, Some((Move::JawGrab, false)))
    });
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    stand_in_front_until(&mut sim, 60 * 2, "his jaws to close on the player", |sim| held(sim));
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    let r = rex(&mut sim);
    assert!(r.view.thrashing, "the player is held but he is not thrashing: {r:?}");
    // Held between his jaws: in front of him, up off the floor.
    let (kin, _) = player(&mut sim);
    let ahead = (kin.pos.x - r.kin.pos.x) * r.side;
    assert!(ahead > module::ram_front() * 0.6, "the held player is {ahead} ahead of him, not in his jaws");
    // The thrash ends in the fling.
    let mut flung = None;
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
        if !held(&mut sim) {
            flung = Some(player(&mut sim).0);
            break;
        }
    }
    let flung = flung.expect("he never let go");
    assert!(
        flung.vel.x * r.side > 300.0 && flung.vel.y < -200.0,
        "the player left his jaws at {:?}, not flung forward and up",
        flung.vel
    );
    let (_, after) = player(&mut sim);
    assert!(after <= before - 3, "the grab took {} health, not its bites and its throw", before - after);
}

/// Mash, and you break out of his jaws before the fling.
#[test]
fn mashing_breaks_his_jaw_grab() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    stand_in_front_until(&mut sim, 60 * 40, "him to lunge for a grab", |sim| {
        matches!(rex(sim).view.performing, Some((Move::JawGrab, false)))
    });
    untouchable_player(&mut sim, false);
    stand_in_front_until(&mut sim, 60 * 2, "his jaws to close on the player", |sim| held(sim));
    let mut freed_after = None;
    for tick in 0..60 * 3 {
        // Every button, every other tick: a mash.
        let mash = tick % 2 == 0;
        let action = AgentAction { jump: mash, attack: mash, dash: mash, ..AgentAction::default() };
        sim.step(action);
        if !held(&mut sim) {
            freed_after = Some(tick);
            break;
        }
    }
    let freed_after = freed_after.expect("mashing never freed the player");
    let thrash_ticks = (1.3 * 60.0) as usize;
    assert!(freed_after < thrash_ticks, "mashing freed the player after {freed_after} ticks; the thrash is {thrash_ticks}");
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    assert!(!rex(&mut sim).view.thrashing, "the player broke free and he thrashes on");
}

/// The bodies wearing catalog character `id`, alive: where each is.
fn kin(sim: &mut Platformer2dSimHarness, id: &str) -> Vec<ae::Vec2> {
    let world = sim.world_mut();
    let mut q = world.query::<(&ambition_platformer2d::characters::actor::WornCharacter, &BodyHealth, &ae::BodyKinematics)>();
    q.iter(world)
        .filter(|(worn, health, _)| worn.id() == id && health.alive())
        .map(|(_, _, kin)| kin.pos)
        .collect()
}

/// Wounded, he calls his kin: stochastic parrots come down from the high
/// corners of the hall.
#[test]
fn wounded_he_calls_parrots_from_the_high_corners() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    assert!(kin(&mut sim, "stochastic_parrot").is_empty(), "premise: no parrot before he calls");
    step_until(&mut sim, 60 * 40, "him to call", |sim| matches!(rex(sim).view.performing, Some((Move::Call, true))));
    for _ in 0..4 {
        sim.step(AgentAction::default());
    }
    let hall = rex(&mut sim).view.hall.expect("hall");
    let parrots = kin(&mut sim, "stochastic_parrot");
    assert_eq!(parrots.len(), 2, "his call brought {parrots:?}");
    for p in &parrots {
        assert!(p.y < hall.floor - 300.0, "a parrot came in at {p:?}, not high");
    }
    assert!(kin(&mut sim, "npc_raptor_stalker").is_empty(), "raptors came before he was enraged");
}

/// Enraged, his call brings raptors along the floor too, and he calls no more
/// than four of his kin alive at once.
#[test]
fn enraged_his_call_brings_raptors_and_no_more_than_four() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.15);
    // Past phase 2 (which calls parrots only): the roar opens his enrage, and
    // his call follows it.
    step_until(&mut sim, 60 * 40, "his enrage roar", |sim| matches!(rex(sim).view.performing, Some((Move::Roar, _))));
    step_until(&mut sim, 60 * 6, "him to call", |sim| matches!(rex(sim).view.performing, Some((Move::Call, true))));
    for _ in 0..4 {
        sim.step(AgentAction::default());
    }
    let raptors = kin(&mut sim, "npc_raptor_stalker");
    assert_eq!(raptors.len(), 2, "enraged, his call brought raptors {raptors:?}");
    // His next call, with all four alive, brings none.
    step_until(&mut sim, 60 * 40, "his next call", |sim| matches!(rex(sim).view.performing, Some((Move::Call, false))));
    step_until(&mut sim, 60 * 3, "the call's shriek", |sim| matches!(rex(sim).view.performing, Some((Move::Call, true))));
    for _ in 0..4 {
        sim.step(AgentAction::default());
    }
    let all = kin(&mut sim, "stochastic_parrot").len() + kin(&mut sim, "npc_raptor_stalker").len();
    assert!(all <= 4, "{all} of his kin are alive at once");
}

/// His kin hunt you: a parrot called from a high corner comes down for a
/// player in the open, far across the hall (a summon knows where the fight is,
/// `SummonedToTheFight`; the perception window alone would never show it the
/// floor 440 below).
#[test]
fn his_parrots_come_for_you() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    step_until(&mut sim, 60 * 40, "him to call", |sim| matches!(rex(sim).view.performing, Some((Move::Call, true))));
    let hall = rex(&mut sim).view.hall.expect("hall");
    let mut closest = f32::MAX;
    for _ in 0..60 * 8 {
        let (pk, _) = player(&mut sim);
        // Out in the open, mid-hall, away from him.
        let r = rex(&mut sim);
        let x = if r.kin.pos.x > (hall.left + hall.right) * 0.5 { hall.left + 400.0 } else { hall.right - 400.0 };
        place_player(&mut sim, ae::Vec2::new(x, hall.floor - pk.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        let p = player(&mut sim).0.pos;
        for parrot in kin(&mut sim, "stochastic_parrot") {
            closest = closest.min(parrot.distance(p));
        }
    }
    assert!(closest < 90.0, "his parrots came no nearer than {closest} to a player in the open");
}

/// What he has said since `cursor` last listened: the ids of the open-ended
/// cues played, as their names.
fn heard(sim: &mut Platformer2dSimHarness, cursor: &mut bevy::ecs::message::MessageCursor<ambition_platformer2d::sfx::OwnedSfxMessage>) -> Vec<&'static str> {
    use ambition_platformer2d::sfx::SfxMessage;
    let mut every: Vec<&'static str> = module::VOICE.to_vec();
    every.extend(EVERY_TELL_CUE);
    every.push("boss.trex.scream");
    let names: Vec<(ambition_platformer2d::sfx::SfxId, &'static str)> =
        every.into_iter().map(|name| (ambition_platformer2d::sfx::SfxId::new(name), name)).collect();
    let messages = sim.world().resource::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
    cursor
        .read(messages)
        .filter_map(|m| match m.request {
            SfxMessage::Play { id, .. } => names.iter().find(|(known, _)| *known == id).map(|(_, name)| *name),
            _ => None,
        })
        .collect()
}

/// Every cue his pattern's tells name (`boss_profiles.ron`).
const EVERY_TELL_CUE: [&str; 12] = [
    "boss.trex.growl_snarl_a",
    "boss.trex.growl_snarl_b",
    "boss.trex.growl_low_a",
    "boss.trex.growl_low_b",
    "boss.trex.growl_huff_a",
    "boss.trex.growl_huff_b",
    "boss.trex.growl_grunt_a",
    "boss.trex.growl_grunt_b",
    "boss.trex.growl_rise_a",
    "boss.trex.growl_rise_b",
    "boss.trex.growl_chuff_a",
    "boss.trex.roar",
];

/// Into phase 2 he rears and SCREAMS, holding his ground through the beat
/// between the phases (the encounter's transition lock).
#[test]
fn wounded_he_screams_into_phase_two() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let mut cursor = Default::default();
    heard(&mut sim, &mut cursor);
    wound(&mut sim, 0.5);
    let mut said = Vec::new();
    step_until(&mut sim, 60 * 10, "him to scream", |sim| {
        said.extend(heard(sim, &mut cursor));
        said.contains(&"boss.trex.scream")
    });
    let r = rex(&mut sim);
    assert!(r.view.rearing, "he screams but is not rearing: {r:?}");
    let x = r.kin.pos.x;
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    let r = rex(&mut sim);
    assert!((r.kin.pos.x - x).abs() < 1.0, "he walked {} while screaming", r.kin.pos.x - x);
}

/// His tells are voiced: a fight's worth of phase 1 has him growl, snarl,
/// huff and wind up, not one sound over and over.
#[test]
fn his_tells_and_his_stalking_are_voiced() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let mut cursor = Default::default();
    let mut said = std::collections::BTreeSet::new();
    for _ in 0..60 * 30 {
        sim.step(AgentAction::default());
        said.extend(heard(&mut sim, &mut cursor).into_iter().filter(|cue| cue.starts_with("boss.trex.growl_")));
    }
    assert!(said.len() >= 3, "thirty seconds of him voiced only {said:?}");
}

/// Kill him with a real hit, the road the game's defeats take (setting his HP
/// to 0 does not kill him: his encounter phases on to its enrage).
fn kill_him(sim: &mut Platformer2dSimHarness) {
    let placement = {
        let world = sim.world_mut();
        world.query::<&BossConfig>().iter(world).find(|c| c.behavior.id == TREX_ID).map(|c| c.id.clone()).expect("the T-rex")
    };
    crate::boss_lifecycle::kill_boss_with_a_real_hit(sim, &placement, 60 * 10);
    sim.step(AgentAction::default());
    let world = sim.world_mut();
    let dead = world.query::<(&BossConfig, &BodyHealth)>().iter(world).any(|(c, h)| c.behavior.id == TREX_ID && !h.alive());
    assert!(dead, "premise: the real hit killed him");
}

/// He dies wailing, once.
#[test]
fn he_dies_with_one_wail() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let mut cursor = Default::default();
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    heard(&mut sim, &mut cursor);
    kill_him(&mut sim);
    let mut wails = heard(&mut sim, &mut cursor).iter().filter(|cue| **cue == "boss.trex.death").count();
    for _ in 0..60 * 5 {
        sim.step(AgentAction::default());
        wails += heard(&mut sim, &mut cursor).iter().filter(|cue| **cue == "boss.trex.death").count();
    }
    assert_eq!(wails, 1, "he wailed {wails} times dying");
}




/// Enraged, his raptors run you down: called in at the walls, they come
/// along the floor for a player in the open (they hunt by a profile of their
/// own; with none, a body notices nobody and stands where it was put).
#[test]
fn his_raptors_run_you_down() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.15);
    step_until(&mut sim, 60 * 40, "his enrage roar", |sim| matches!(rex(sim).view.performing, Some((Move::Roar, _))));
    step_until(&mut sim, 60 * 6, "him to call", |sim| matches!(rex(sim).view.performing, Some((Move::Call, true))));
    let hall = rex(&mut sim).view.hall.expect("hall");
    let mut closest = f32::MAX;
    for _ in 0..60 * 6 {
        let (pk, _) = player(&mut sim);
        let r = rex(&mut sim);
        let x = if r.kin.pos.x > (hall.left + hall.right) * 0.5 { hall.left + 400.0 } else { hall.right - 400.0 };
        place_player(&mut sim, ae::Vec2::new(x, hall.floor - pk.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        let p = player(&mut sim).0.pos;
        for raptor in kin(&mut sim, "npc_raptor_stalker") {
            closest = closest.min((raptor.x - p.x).abs());
        }
    }
    assert!(closest < 80.0, "his raptors came no nearer than {closest} to a player in the open");
}

/// Walking back in on him dead is silent: the death wail sounds when he
/// dies, not each time you enter a room where he lies dead (Jon, 2026-10-06).
#[test]
fn walking_back_in_on_him_dead_is_silent() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    kill_him(&mut sim);
    // His death and its outro.
    for _ in 0..60 * 6 {
        sim.step(AgentAction::default());
    }
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "hall_of_bosses"), "hall_of_bosses");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let mut cursor = Default::default();
    heard(&mut sim, &mut cursor);
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, ARENA), ARENA);
    let mut wails = 0;
    let mut saw_him = false;
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
        wails += heard(&mut sim, &mut cursor).iter().filter(|cue| **cue == "boss.trex.death").count();
        let world = sim.world_mut();
        saw_him |= world.query::<(&BossConfig, &BodyHealth)>().iter(world).any(|(c, h)| c.behavior.id == TREX_ID && !h.alive());
    }
    assert!(saw_him, "premise: he lies dead in the arena when the player walks back in");
    assert_eq!(wails, 0, "walking back in on him dead, he wailed {wails} times");
}

