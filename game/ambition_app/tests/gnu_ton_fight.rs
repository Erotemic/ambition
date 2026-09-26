//! GNU-ton's fight, played headlessly in the real arena: the authored room, the
//! authored pair, the scripted pattern, and the content conductor that performs
//! it (`ambition_content::bosses::gnu_ton`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::gnu_ton::choreography::Move;
use ambition_content::bosses::gnu_ton::conductor::back_platform;
use ambition_content::bosses::gnu_ton::GnuTonConductor;
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability, Limb, LimbRig, LimbSlot};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::sim_view::ActorAnimIndex;
use bevy::prelude::*;

const ARENA: &str = "gnu_ton_arena";

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no GNU-ton, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the gnu-ton arena builds headlessly");
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

fn untouchable_player(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
}

#[derive(Debug)]
struct Scene {
    scholar: ae::BodyKinematics,
    scholar_hp: i32,
    performing: Option<(Move, bool)>,
    floor: f32,
    giant: ae::BodyKinematics,
    giant_unmirrored: bool,
    /// The side the gnu is drawn toward, from the presentation read-model.
    giant_drawn_side: f32,
    /// Left fist first: body, health, max health.
    fists: Vec<(ae::BodyKinematics, i32, i32)>,
}

fn scene(sim: &mut Platformer2dSimHarness) -> Scene {
    let world = sim.world_mut();
    let (scholar, scholar_hp, performing, floor) = world
        .query::<(&BossConfig, &ae::BodyKinematics, &BodyHealth, &GnuTonConductor)>()
        .iter(world)
        .find(|(config, ..)| config.behavior.id == "gnu_ton_rider")
        .map(|(_, kin, health, conductor)| {
            (
                kin.clone(),
                health.current(),
                conductor.performing(),
                conductor.hall().expect("the conductor measured the hall").floor,
            )
        })
        .expect("GNU-ton is in the arena, conducted");
    let (giant, rig, giant_unmirrored, giant_id) = world
        .query_filtered::<(&ae::BodyKinematics, &LimbRig, Has<ae::Unmirrored>, &FeatureId), Without<Limb>>()
        .iter(world)
        .next()
        .map(|(kin, rig, unmirrored, id)| (kin.clone(), rig.clone(), unmirrored, id.clone()))
        .expect("the giant gnu is in the arena");
    let giant_drawn_side = world
        .resource::<ActorAnimIndex>()
        .get(giant_id.as_str())
        .expect("the gnu has a pose row: it is drawn")
        .facing;
    let mut fists: Vec<_> = rig
        .limbs
        .values()
        .map(|&fist| {
            let kin = world.get::<ae::BodyKinematics>(fist).expect("a fist body").clone();
            let health = *world.get::<BodyHealth>(fist).expect("a fist's health");
            (kin, health.current(), health.max())
        })
        .collect();
    fists.sort_by(|a, b| a.0.pos.x.total_cmp(&b.0.pos.x));
    Scene { scholar, scholar_hp, performing, floor, giant, giant_unmirrored, giant_drawn_side, fists }
}

/// The side each fist is DRAWN toward, left hand then right, from the
/// presentation read-model.
fn fist_drawn_sides(sim: &mut Platformer2dSimHarness) -> [f32; 2] {
    let world = sim.world_mut();
    let rig = world
        .query_filtered::<&LimbRig, Without<Limb>>()
        .iter(world)
        .next()
        .expect("the giant gnu is in the arena")
        .clone();
    [LimbSlot::HAND_LEFT, LimbSlot::HAND_RIGHT].map(|slot| {
        let fist = rig.get(slot).expect("a fist in each hand slot");
        let id = world.get::<FeatureId>(fist).expect("a fist has an id").clone();
        world.resource::<ActorAnimIndex>().get(id.as_str()).expect("a fist is drawn").facing
    })
}

/// The scholar's seat from the gnu's centre, as the gnu is drawn.
const SEAT: ae::Vec2 = ae::Vec2::new(50.0, -65.0);

fn saddle(s: &Scene) -> ae::Vec2 {
    s.giant.pos + ae::Vec2::new(SEAT.x * ae::mirror_side(s.giant.facing, s.giant_unmirrored), SEAT.y)
}

fn on_floor(kin: &ae::BodyKinematics, floor: f32) -> bool {
    (kin.pos.y + kin.size.y * 0.5 - floor).abs() < 1.0
}

/// Step until `until` holds, or panic naming what it was waiting for.
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
    panic!("waited {frames} frames for {what}; last scene: {:?}", scene(sim));
}

/// The giant stands on the floor at its drawn size, and the scholar sits on its
/// shoulder at his: two of the complaints the rework started from.
#[test]
fn the_gnu_stands_on_the_floor_and_carries_a_person_sized_scholar() {
    let mut sim = arena();
    let s = scene(&mut sim);
    assert!(on_floor(&s.giant, s.floor), "the gnu's feet are on the floor {}: {:?}", s.floor, s.giant);
    assert!(
        s.giant.size.x > 400.0 && s.giant.size.y > 380.0,
        "the gnu is its drawn size, not a 220 box: {:?}",
        s.giant.size
    );
    assert!(
        // His v2 sheet draws him 60x110: a person on a 440-wide giant.
        (50.0..80.0).contains(&s.scholar.size.x) && (90.0..125.0).contains(&s.scholar.size.y),
        "the scholar is a person, not a speck: {:?}",
        s.scholar.size
    );
    assert!(
        (s.scholar.pos - saddle(&s)).length() < 1.0,
        "he sits in the saddle {:?}, not wherever his own brain steers: {:?}",
        saddle(&s),
        s.scholar.pos
    );
    // Standing ON the gnu's shoulders: his soles on the back the player stands
    // on, not in the air above it (he stood 11.7 wu up, on the art's shoulder
    // point, and read as floating in front of the neck).
    let back = back_platform(&s.giant, s.giant_unmirrored);
    let soles = s.scholar.pos.y + s.scholar.size.y * 0.5;
    assert!(
        (soles - back.min.y).abs() < 1.0 && (back.min.x..back.max.x).contains(&s.scholar.pos.x),
        "his soles at y {soles} stand on the gnu's back ({:?})",
        back
    );
    assert_eq!(s.fists.len(), 2);
    for (fist, ..) in &s.fists {
        assert!(fist.size.x > 100.0, "a fist is a giant's fist: {:?}", fist.size);
    }
}

/// A demonstration hovers over the player, falls where they stood, hurts them,
/// and stays in the floor; a blow to the stuck fist lands on the scholar.
#[test]
fn a_slam_lands_where_you_stood_and_the_stuck_fist_carries_your_blow_to_him() {
    let mut sim = arena();
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    let (_, hp_before) = player(&mut sim);
    step_until(&mut sim, 900, "a fist stuck in the floor after a demonstration", |sim| {
        let s = scene(sim);
        s.performing == Some((Move::Demonstrate, true)) && s.fists.iter().any(|(fist, ..)| on_floor(fist, s.floor))
    });
    let s = scene(&mut sim);
    let (stuck, ..) = s.fists.iter().find(|(fist, ..)| on_floor(fist, s.floor)).cloned().expect("a stuck fist");
    assert!((stuck.pos.x - 400.0).abs() < 40.0, "it fell where the player stood: {:?}", stuck.pos);
    // Far from the gnu, it is still drawn bound to it: its view carries the
    // point on the gnu's body its trail runs from.
    {
        let world = sim.world_mut();
        let ids: Vec<FeatureId> =
            world.query_filtered::<&FeatureId, With<Limb>>().iter(world).cloned().collect();
        let views = world.resource::<ambition_platformer2d::sim_view::FeatureViewIndex>();
        assert_eq!(ids.len(), 2);
        for id in ids {
            let bond = views.get(id.as_str()).and_then(|view| view.limb_host);
            let bond = bond.unwrap_or_else(|| panic!("fist {id:?} is drawn with no bond to the gnu"));
            let off = bond - s.giant.pos;
            assert!(
                off.x.abs() < s.giant.size.x * 0.5 && off.y.abs() < s.giant.size.y * 0.5,
                "fist {id:?}'s bond {bond:?} is off the gnu's body at {:?}",
                s.giant.pos
            );
        }
    }
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }
    let (_, hp_after) = player(&mut sim);
    assert!(hp_after < hp_before, "and it hurt them: {hp_before} -> {hp_after}");

    // Stand beside the stuck fist, facing it, and hit it — twice, while it is
    // still stuck.
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(stuck.pos.x + stuck.size.x * 0.5 + 18.0, 1200.0));
    // A body turns once it has landed: lean toward the fist until it does.
    for _ in 0..30 {
        sim.step(AgentAction { move_x: -1.0, ..Default::default() });
        if player(&mut sim).0.facing < 0.0 {
            break;
        }
    }
    sim.step(AgentAction::default());
    let (me, _) = player(&mut sim);
    assert_eq!(me.facing, -1.0, "the premise: the player faces the stuck fist");
    let scholar_before = scene(&mut sim).scholar_hp;
    let swing = |frame: usize, at: usize| AgentAction {
        attack: frame == at,
        attack_held: (at..at + 4).contains(&frame),
        ..Default::default()
    };
    for frame in 0..12 {
        sim.step(swing(frame, 1));
    }
    let s = scene(&mut sim);
    let after_first = s.scholar_hp;
    assert!(
        after_first < scholar_before,
        "a blow to his stuck fist lands on him: {scholar_before} -> {after_first}"
    );
    for (_, hp, max) in &s.fists {
        assert_eq!(hp, max, "and the fist itself is whole again");
    }
    // One blow per stick: the second reaches nobody. A stuck fist carried every
    // blow, and one pair slam took most of a phase.
    for frame in 0..12 {
        sim.step(swing(frame, 1));
    }
    let s = scene(&mut sim);
    assert!(
        s.fists.iter().any(|(fist, ..)| on_floor(fist, s.floor) && (fist.pos.x - stuck.pos.x).abs() < 1.0),
        "the premise: the fist is still stuck for the second blow"
    );
    assert_eq!(s.scholar_hp, after_first, "a second blow to the same stuck fist does not land on him");
}

/// His hands are his: nothing he throws lands on him.
#[test]
fn his_own_fists_never_hurt_him() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(900.0, 1200.0));
    let before = scene(&mut sim).scholar_hp;
    for _ in 0..1500 {
        sim.step(AgentAction::default());
    }
    assert_eq!(scene(&mut sim).scholar_hp, before, "a fist that passed him took nothing");
}

/// The gnu's back is ground — the way to reach him in the saddle — and when he
/// bucks it throws whoever is standing there.
#[test]
fn you_can_stand_on_the_giants_shoulders_until_it_bucks() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    let s = scene(&mut sim);
    let back = back_platform(&s.giant, s.giant_unmirrored);
    let spot = ae::Vec2::new((back.min.x + back.max.x) * 0.5 - 60.0, back.min.y - 40.0);
    place_player(&mut sim, spot);
    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    let (kin, _) = player(&mut sim);
    let feet = kin.pos.y + kin.size.y * 0.5;
    assert!((feet - back.min.y).abs() < 2.0, "standing on the gnu's back ({}): feet at {feet}", back.min.y);

    step_until(&mut sim, 1500, "the buck to throw the player off the gnu's back", |sim| {
        let (kin, _) = player(sim);
        kin.pos.y + kin.size.y * 0.5 < back.min.y - 120.0
    });
}

/// After the apple rain a golden apple knocks him off the gnu: he sits dazed on
/// the floor, then climbs back to the saddle.
#[test]
fn eureka_puts_him_on_the_floor_and_he_climbs_back() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(300.0, 1200.0));
    step_until(&mut sim, 3000, "him dazed on the floor after the apple rain", |sim| {
        let s = scene(sim);
        s.performing == Some((Move::Eureka, true)) && on_floor(&s.scholar, s.floor)
    });
    step_until(&mut sim, 400, "him back in the saddle", |sim| {
        let s = scene(sim);
        s.performing.is_none() && (s.scholar.pos - saddle(&s)).length() < 1.0
    });
}

/// The scholar turns to face you; the gnu under him has no left/right variant,
/// so nothing of it moves when he does.
///
/// He steers the gnu's facing (a rider's mount takes it), so every turn of his
/// spun the 440 px gnu, the back you stand on and his own saddle. The gnu is
/// `Unmirrored`: its facing is still a fact, and mirrors nothing. With nothing
/// to spin he turns by his own box, and watches a player standing under him.
#[test]
fn the_scholar_turns_and_the_gnu_under_him_does_not() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    let s = scene(&mut sim);
    assert!(s.giant_unmirrored, "the gnu declares no left/right variant");
    // And it is drawn behind him: he stands on its shoulders, in front of it
    // (Jon: the scholar was drawn behind the gnu and could not be seen).
    let planes = {
        let world = sim.world_mut();
        let mut ids = world.query_filtered::<&FeatureId, (With<LimbRig>, Without<Limb>)>();
        let giant = ids.iter(world).next().expect("the giant").clone();
        let mut ids = world.query_filtered::<&FeatureId, With<GnuTonConductor>>();
        let scholar = ids.iter(world).next().expect("the conducted scholar").clone();
        let views = world.resource::<ambition_platformer2d::sim_view::FeatureViewIndex>();
        let plane = |id: &FeatureId| views.get(id.as_str()).expect("a drawn body").depth_plane;
        (plane(&scholar), plane(&giant))
    };
    assert_eq!(planes, (ae::DepthPlane::PLAYABLE, ae::DepthPlane::BEHIND), "(scholar, gnu) as drawn");
    let (player_kin, _) = player(&mut sim);
    let stand = |x: f32| ae::Vec2::new(x, s.floor - player_kin.size.y * 0.5 - 1.0);
    let half = s.giant.size.x * 0.5;
    assert!(half > 150.0, "the premise is a giant: its body is {} wide", s.giant.size.x);
    // What a turn could move, relative to the gnu: the side it is drawn toward,
    // its back, and the saddle while he sits in it.
    let drawn = |s: &Scene| {
        let back = back_platform(&s.giant, s.giant_unmirrored);
        (s.giant_drawn_side, back.min.x - s.giant.pos.x, back.max.x - s.giant.pos.x)
    };
    let before = drawn(&s);
    assert_eq!(before.0, 1.0, "the gnu is drawn as authored, facing right");

    let scholar_x = s.scholar.pos.x;
    let mut seen = Vec::new();
    for x in [s.giant.pos.x - half - 120.0, scholar_x + 60.0, scholar_x - 60.0, s.giant.pos.x + half + 120.0] {
        place_player(&mut sim, stand(x));
        for frame in 0..40 {
            sim.step(AgentAction::default());
            let s = scene(&mut sim);
            assert_eq!(drawn(&s), before, "frame {frame} with the player at {x}: a turn moved the gnu");
            // The sheet draws the right hand (wrist left, knuckles right): the
            // left is its mirror, wherever a move carries it and whichever way
            // he faces.
            assert_eq!(fist_drawn_sides(&mut sim), [-1.0, 1.0], "frame {frame} with the player at {x}");
            if s.performing.is_none() {
                let seat = s.scholar.pos - s.giant.pos;
                assert!((seat - SEAT).length() < 1.0, "frame {frame}: he sits at {seat:?}, the saddle is {SEAT:?}");
            }
        }
        let s = scene(&mut sim);
        seen.push((x - scholar_x, s.scholar.facing, s.giant.facing));
    }
    // The premise the stillness is measured against: he turned toward the
    // player each time, under the gnu too, and the gnu's facing went with him.
    for (dx, scholar, giant) in &seen {
        assert!(dx.signum() == scholar.signum(), "player {dx:+} from him, he faces {scholar}: {seen:?}");
        assert_eq!(scholar, giant, "the gnu takes his facing: {seen:?}");
    }
}

/// Entering the arena, the fists start where they stand and stay at the giant.
///
/// The conductor eased its fists from a defaulted latch at the world origin, so
/// two frames after the room loaded both jumped ~1300 px to the room's top-left
/// corner and slid back (Jon: "the hands start in the upper left corner, and
/// then very quickly move into the correct location").
#[test]
fn entering_the_arena_the_fists_start_where_they_stand() {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room("hall_of_bosses");
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the hall of bosses builds headlessly");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let door = {
        let world = sim.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let set = rooms.iter(world).next().expect("a room set");
        set.active_loading_zones()
            .iter()
            .find(|zone| zone.id == "hall_gnu_ton_portal")
            .cloned()
            .expect("the hall of bosses has a door to GNU-ton")
    };
    use ambition_platformer2d::engine_core::AabbExt as _;
    let at = door.aabb.center();
    sim.teleport_player((at.x, at.y));
    let hold = AgentAction { interact: true, interact_held: true, ..AgentAction::default() };
    let mut entered = false;
    for _ in 0..120 {
        if sim.step(hold.clone()).active_room == ARENA {
            entered = true;
            break;
        }
    }
    assert!(entered, "holding interact at the door never entered {ARENA}");

    // Each fist and the home its limb was built with, the giant, and whether
    // he has started a move.
    #[allow(clippy::type_complexity)]
    let fists = |sim: &mut Platformer2dSimHarness| -> (Vec<ae::Vec2>, Vec<ae::Vec2>, ae::BodyKinematics, bool) {
        let world = sim.world_mut();
        let (fists, homes) = world
            .query::<(&ae::BodyKinematics, &Limb)>()
            .iter(world)
            .map(|(kin, limb)| (kin.pos, limb.home_offset))
            .unzip();
        let giant = world
            .query_filtered::<&ae::BodyKinematics, (With<LimbRig>, Without<Limb>)>()
            .iter(world)
            .next()
            .cloned()
            .expect("the giant is in the arena");
        let performing = world.query::<&GnuTonConductor>().iter(world).any(|c| c.performing().is_some());
        (fists, homes, giant, performing)
    };
    let (mut last, ..) = fists(&mut sim);
    assert_eq!(last.len(), 2, "the premise: both fists are in the arena from its first frame");
    let mut resting = 0;
    for frame in 0..60 {
        sim.step(AgentAction::default());
        let (now, homes, giant, performing) = fists(&mut sim);
        // Resting before his first move, a fist bobs (10 px) about its limb's
        // home: the one the spawn built it with. A second statement of the
        // home would park it somewhere else. (It is built ~15 px low — spawn
        // places it from the placement's box, before the gnu's body takes its
        // sheet's height — and eases up over the first 0.6 s.)
        if !performing && frame >= 40 {
            resting += 1;
            for (at, home) in now.iter().zip(&homes) {
                let off = *at - (giant.pos + *home);
                assert!(off.length() <= 10.5, "frame {frame}: a resting fist is {off:?} from its home {home:?}");
            }
        }
        for (before, after) in last.iter().zip(&now) {
            assert!(
                (*after - *before).length() < 40.0,
                "frame {frame}: a fist jumped {before:?} -> {after:?}"
            );
            assert!(
                (*after - giant.pos).length() < giant.size.x,
                "frame {frame}: a fist at {after:?} is out of reach of the giant at {:?}",
                giant.pos
            );
        }
        last = now;
    }
    assert!(resting >= 15, "the premise: he rests before his first move ({resting} frames)");
}

/// The gnu is behind the playable plane: it publishes no volume a swing, a pogo
/// or a shot could reach, all fight long.
///
/// It was invulnerable instead, which refused the damage and still took the
/// swing and gave the pogo (MEASURED by `fight_discovery`: hittable 100% of the
/// fight). Jon: "prevent a swing that would otherwise cause a hit or a pogo".
#[test]
fn nothing_can_strike_the_gnu() {
    let mut sim = arena();
    for frame in 0..180 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let (plane, volumes) = world
            .query_filtered::<(Option<&ae::DepthPlane>, &ambition_platformer2d::combat::components::DamageableVolumes), (With<LimbRig>, Without<Limb>)>()
            .iter(world)
            .next()
            .map(|(plane, volumes)| (plane.copied(), volumes.volumes.len()))
            .expect("the giant is in the arena with a damageable record");
        assert_eq!(plane, Some(ae::DepthPlane::BEHIND), "frame {frame}: the gnu stands behind the fight");
        assert_eq!(volumes, 0, "frame {frame}: the gnu published {volumes} damageable volume(s)");
    }
    // The control: the fists and the scholar are still in the fight.
    let world = sim.world_mut();
    let reachable = world
        .query_filtered::<&ambition_platformer2d::combat::components::DamageableVolumes, With<Limb>>()
        .iter(world)
        .filter(|volumes| !volumes.volumes.is_empty())
        .count();
    assert!(reachable > 0, "the fists stay reachable; only the gnu stepped back");
}
