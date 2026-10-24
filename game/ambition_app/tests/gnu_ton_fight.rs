//! GNU-ton's fight, played headlessly in the real arena: the authored room, the
//! authored pair, the scripted pattern, and the conductor module that performs
//! it (`ambition_content_modules::gnu_ton::conductor`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::mount::BackLedge;
use ambition_content::bosses::gnu_ton::{conductor_of, GNU_TON_ID};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::brain::{BossAttackProfile, BossAttackState};
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

/// A fist of the giant: its two hands, not its hoof (a limb that is always
/// on its leg).
fn is_fist(limb: &Limb) -> bool {
    limb.slot == LimbSlot::HAND_LEFT || limb.slot == LimbSlot::HAND_RIGHT
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

/// Lift [`untouchable_player`]: the buck's throw is a hit, and an untouchable
/// body refuses it.
fn touchable_player(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, false);
    }
}

/// The buck THROWS: within a few frames of its strike the player flies away
/// from him, and comes down OFF the gnu's back — below its top or clear of its
/// ends, not popped up to land back where they stood.
fn thrown_off_the_back(sim: &mut Platformer2dSimHarness, back: ae::Aabb, within: usize) {
    let away = {
        let s = scene(sim);
        let (kin, _) = player(sim);
        (kin.pos.x - s.scholar.pos.x).signum()
    };
    step_until(sim, within, "the buck to throw the player away from him", |sim| player(sim).0.vel.x * away > 300.0);
    step_until(sim, 90, "the thrown player to come down off the gnu's back", |sim| {
        let (kin, _) = player(sim);
        let feet = kin.pos.y + kin.size.y * 0.5;
        feet > back.min.y + 40.0 || kin.pos.x < back.min.x - 20.0 || kin.pos.x > back.max.x + 20.0
    });
}

#[derive(Debug)]
struct Scene {
    scholar: ae::BodyKinematics,
    scholar_hp: i32,
    /// The move he performs, by the `Special` key his pattern names it with,
    /// and whether it strikes.
    performing: Option<(&'static str, bool)>,
    floor: f32,
    giant: ae::BodyKinematics,
    giant_unmirrored: bool,
    /// The gnu's back, as its character authors it.
    giant_back: BackLedge,
    /// The side the gnu is drawn toward, from the presentation read-model.
    giant_drawn_side: f32,
    /// Left fist first: body, health, max health.
    fists: Vec<(ae::BodyKinematics, i32, i32)>,
}

fn scene(sim: &mut Platformer2dSimHarness) -> Scene {
    let world = sim.world_mut();
    let (entity, scholar, scholar_hp) = world
        .query::<(Entity, &BossConfig, &ae::BodyKinematics, &BodyHealth)>()
        .iter(world)
        .find(|(_, config, ..)| config.behavior.id == GNU_TON_ID)
        .map(|(entity, _, kin, health)| (entity, kin.clone(), health.current()))
        .expect("GNU-ton is in the arena");
    // His conductor is the `gnu_ton` module; its memory is the module's record.
    let view = conductor_of(world, entity).expect("GNU-ton, conducted");
    let performing = view.performing.map(|(mv, striking)| (mv.key(), striking));
    let floor = view.hall.expect("the conductor measured the hall").floor;
    let (giant, rig, giant_unmirrored, giant_id, giant_back) = world
        .query_filtered::<(&ae::BodyKinematics, &LimbRig, Has<ae::Unmirrored>, &FeatureId, &BackLedge), Without<Limb>>()
        .iter(world)
        .next()
        .map(|(kin, rig, unmirrored, id, back)| (kin.clone(), rig.clone(), unmirrored, id.clone(), back.clone()))
        .expect("the giant gnu is in the arena, with its back");
    let giant_drawn_side = world
        .resource::<ActorAnimIndex>()
        .get(giant_id.as_str())
        .expect("the gnu has a pose row: it is drawn")
        .facing;
    let mut fists: Vec<_> = rig
        .limbs
        .iter()
        .filter(|(slot, _)| **slot == LimbSlot::HAND_LEFT || **slot == LimbSlot::HAND_RIGHT)
        .map(|(_, &fist)| {
            let kin = world.get::<ae::BodyKinematics>(fist).expect("a fist body").clone();
            let health = *world.get::<BodyHealth>(fist).expect("a fist's health");
            (kin, health.current(), health.max())
        })
        .collect();
    fists.sort_by(|a, b| a.0.pos.x.total_cmp(&b.0.pos.x));
    Scene {
        scholar,
        scholar_hp,
        performing,
        floor,
        giant,
        giant_unmirrored,
        giant_back,
        giant_drawn_side,
        fists,
    }
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

/// The scholar's seat — where his soles rest — from the gnu's centre, as the gnu
/// is drawn: its row's `saddle`.
const SEAT: ae::Vec2 = ae::Vec2::new(256.0, -232.05);

fn saddle(s: &Scene) -> ae::Vec2 {
    // He stands on the seat: his centre half his height above it.
    s.giant.pos + ae::Vec2::new(SEAT.x * ae::mirror_side(s.giant.facing, s.giant_unmirrored), SEAT.y)
        - ae::Vec2::new(0.0, s.scholar.size.y * 0.5)
}

/// The move his PATTERN has live (telegraph or strike), by key — what the
/// brain asked for, which the conductor may decline to perform.
fn pattern_move(sim: &mut Platformer2dSimHarness) -> Option<String> {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &BossAttackState)>();
    let (_, state) = q.iter(world).find(|(config, _)| config.behavior.id == "gnu_ton_rider")?;
    let key = |p: &BossAttackProfile| match p {
        BossAttackProfile::Strike(key) | BossAttackProfile::Special(key) => key.clone(),
    };
    state.active_profile.as_ref().or(state.telegraph_profile.as_ref()).map(key)
}

/// Stand beside `at`, facing it, and swing once; `true` when he lost health.
fn strike_him_from(sim: &mut Platformer2dSimHarness, stand: ae::Vec2, toward: f32) -> bool {
    place_player(sim, stand);
    for _ in 0..40 {
        sim.step(AgentAction { move_x: toward, ..Default::default() });
        let (me, _) = player(sim);
        if me.facing == toward && me.vel.y == 0.0 {
            break;
        }
    }
    let before = scene(sim).scholar_hp;
    for frame in 0..14 {
        sim.step(AgentAction { attack: frame == 1, attack_held: (1..5).contains(&frame), ..Default::default() });
        if scene(sim).scholar_hp < before {
            return true;
        }
    }
    false
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
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
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

/// The arena with the fight past its phase 1: the giant's arms are in their
/// two lines and its fists are bodies of their own. The encounter is put in
/// phase 2; no blow is dealt, so the scholar's health is whole.
fn arena_with_free_fists() -> Platformer2dSimHarness {
    use ambition_platformer2d::boss_encounter::{BossEncounter, BossEncounterPhase};
    let mut sim = arena();
    {
        let world = sim.world_mut();
        let mut bosses = world.query::<(&BossConfig, &mut BossEncounter)>();
        let (_, mut encounter) = bosses
            .iter_mut(world)
            .find(|(config, _)| config.behavior.id == GNU_TON_ID)
            .expect("GNU-ton is in the arena");
        encounter.encounter.as_mut().expect("his encounter has its phases").phase = BossEncounterPhase::Phase2;
    }
    sim
}

/// ⭐ HIS LOOK LINES ARE A PORT (IN1, debt 3). After phase 1 his conductor asks
/// the portal domain for the line at each shoulder and behind each wrist
/// (`ambition.portal.light_line`), and the scholar's view of this tick's
/// lines (`HeldLightLines`, which the presentation draws) is where his record
/// says each line is. A defeated scholar asks for none, and his view goes.
/// The authority: the conductor's record.
#[test]
fn his_look_lines_are_the_lines_his_conductor_asks_for() {
    use ambition_platformer2d::portal::extension::HeldLightLines;
    fn scholar(sim: &mut Platformer2dSimHarness) -> Entity {
        let world = sim.world_mut();
        world
            .query::<(Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.behavior.id == GNU_TON_ID)
            .map(|(entity, _)| entity)
            .expect("GNU-ton is in the arena")
    }
    /// Each line of his view, by slot, and each line of his record.
    fn lines(sim: &mut Platformer2dSimHarness) -> (Vec<(u32, ae::Vec2)>, Vec<(u32, ae::Vec2)>) {
        let scholar = scholar(sim);
        let world = sim.world_mut();
        let mut view: Vec<(u32, ae::Vec2)> = world
            .get::<HeldLightLines>(scholar)
            .map(|held| held.0.iter().map(|line| (line.slot, line.at)).collect())
            .unwrap_or_default();
        view.sort_by_key(|(slot, _)| *slot);
        let record = conductor_of(world, scholar)
            .map(|conductor| {
                conductor.look_lines.iter().enumerate().filter_map(|(slot, at)| at.map(|at| (slot as u32, at))).collect()
            })
            .unwrap_or_default();
        (view, record)
    }
    let mut sim = arena_with_free_fists();
    step_until(&mut sim, 600, "his four look lines", |sim| lines(sim).1.len() == 4);
    for _ in 0..30 {
        sim.step(AgentAction::default());
        let (view, record) = lines(&mut sim);
        assert_eq!(view, record, "the lines drawn are the lines his record has");
    }
    assert_eq!(lines(&mut sim).0.len(), 4, "premise: four lines are drawn");

    scholar_health_to(&mut sim, 0.0);
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let scholar = scholar(&mut sim);
    assert!(
        sim.world().get::<HeldLightLines>(scholar).is_none(),
        "a defeated scholar's lines are still drawn: {:?}",
        sim.world().get::<HeldLightLines>(scholar)
    );
}

/// The pair of the punch is the way up. From the floor, two jumps take the
/// player up into the far line; it comes up out of the near line, over the
/// giant's right shoulder, and goes to the side onto the giant's back.
#[test]
fn the_punchs_pair_is_the_way_up_to_the_giants_back() {
    use ambition_platformer2d::portal::extension::HeldPortalOf;
    use ambition_platformer2d::portal::PlacedPortal;
    /// An end of the pair his conductor holds: the near line faces up the
    /// screen (the fist goes down into it), the far line down.
    fn line(sim: &mut Platformer2dSimHarness, near: bool) -> Option<ae::Vec2> {
        let world = sim.world_mut();
        let mut portals = world.query_filtered::<&PlacedPortal, With<HeldPortalOf>>();
        portals.iter(world).find(|portal| (portal.normal.y < 0.0) == near).map(|portal| portal.pos)
    }
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    step_until(&mut sim, 900, "the strike of a punch through the pair", |sim| {
        scene(sim).performing == Some(("portal_punch", true))
    });
    // The blow is over in half a second; the fist stays down, and the pair
    // stays open.
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let far = line(&mut sim, false).expect("the far line of the pair is open");
    let near = line(&mut sim, true).expect("the near line of the pair is open");
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    assert!(near.y < back.min.y, "the premise: the near line {near:?} is over the giant's back {back:?}");
    assert!(far.x < s.giant.pos.x, "the premise: the far line {far:?} is on the player's side of the giant");

    // On the floor under the far line, by the fist.
    place_player(&mut sim, ae::Vec2::new(far.x - 40.0, 1200.0));
    for _ in 0..12 {
        sim.step(AgentAction::default());
    }
    // Two jumps, straight up.
    let held = AgentAction { jump_held: true, ..AgentAction::default() };
    let press = AgentAction { jump: true, jump_held: true, ..AgentAction::default() };
    let release = AgentAction { jump_released: true, ..AgentAction::default() };
    let mut top = f32::MAX;
    let mut through = false;
    'jumps: for jump in 0..2 {
        sim.step(press.clone());
        for _ in 0..(if jump == 0 { 15 } else { 40 }) {
            sim.step(held.clone());
            let (kin, _) = player(&mut sim);
            top = top.min(kin.pos.y);
            if kin.pos.x > s.giant.pos.x {
                through = true;
                break 'jumps;
            }
        }
        sim.step(release.clone());
    }
    assert!(
        through,
        "two jumps from the floor did not take the player up through the far line at {far:?}: it rose to y {top} \
         (the floor is {})",
        s.floor
    );
    // Out of the near line, and to the side: onto the back.
    let left = AgentAction { move_x: -1.0, jump_held: true, ..AgentAction::default() };
    let mut on_back = false;
    let mut trail = Vec::new();
    for _ in 0..90 {
        sim.step(left.clone());
        let (kin, _) = player(&mut sim);
        trail.push((kin.pos.x.round(), kin.pos.y.round()));
        let feet = kin.pos.y + kin.size.y * 0.5;
        if (feet - back.min.y).abs() <= 4.0 && (back.min.x..=back.max.x).contains(&kin.pos.x) {
            on_back = true;
            break;
        }
    }
    assert!(on_back, "out of the near line at {near:?} the player did not get onto the back {back:?}: {trail:?}");
}

/// Put the scholar's health at `fraction` of its full.
fn scholar_health_to(sim: &mut Platformer2dSimHarness, fraction: f32) {
    let world = sim.world_mut();
    let mut bosses = world.query::<(&BossConfig, &mut BodyHealth)>();
    let (_, mut health) =
        bosses.iter_mut(world).find(|(config, _)| config.behavior.id == GNU_TON_ID).expect("the scholar");
    health.health.current = ((health.max() as f32) * fraction).ceil() as i32;
}

/// A demonstration hovers over the player, falls where they stood, hurts them,
/// and stays in the floor; a blow to the stuck fist lands on the scholar.
#[test]
fn a_slam_lands_where_you_stood_and_the_stuck_fist_carries_your_blow_to_him() {
    let mut sim = arena_with_free_fists();
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    let (_, hp_before) = player(&mut sim);
    step_until(&mut sim, 900, "a fist stuck in the floor after a demonstration", |sim| {
        let s = scene(sim);
        s.performing == Some(("demonstrate", true)) && s.fists.iter().any(|(fist, ..)| on_floor(fist, s.floor))
    });
    let s = scene(&mut sim);
    let (stuck, ..) = s.fists.iter().find(|(fist, ..)| on_floor(fist, s.floor)).cloned().expect("a stuck fist");
    assert!((stuck.pos.x - 400.0).abs() < 40.0, "it fell where the player stood: {:?}", stuck.pos);
    // Far from the gnu, it is still drawn bound to it: its view carries the
    // point its trail runs from, the line at its shoulder (debt 6) or, with
    // no line, a point on the gnu's body.
    {
        let world = sim.world_mut();
        let scholar = world
            .query::<(Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.behavior.id == GNU_TON_ID)
            .map(|(entity, _)| entity)
            .expect("GNU-ton is in the arena");
        let lines = conductor_of(world, scholar).expect("his conductor").look_lines;
        let fists: Vec<(LimbSlot, FeatureId)> = world
            .query::<(&Limb, &FeatureId)>()
            .iter(world)
            .filter(|(limb, _)| is_fist(limb))
            .map(|(limb, id)| (limb.slot, id.clone()))
            .collect();
        let views = world.resource::<ambition_platformer2d::sim_view::FeatureViewIndex>();
        assert_eq!(fists.len(), 2);
        for (slot, id) in fists {
            let bond = views.get(id.as_str()).and_then(|view| view.limb_host);
            let bond = bond.unwrap_or_else(|| panic!("fist {id:?} is drawn with no bond to the gnu"));
            let shoulder = lines[usize::from(slot == LimbSlot::HAND_RIGHT)];
            let off = bond - s.giant.pos;
            match shoulder {
                Some(line) => assert_eq!(bond, line, "fist {id:?}'s trail runs from the line at its shoulder"),
                None => assert!(
                    off.x.abs() < s.giant.size.x * 0.5 && off.y.abs() < s.giant.size.y * 0.5,
                    "fist {id:?}'s bond {bond:?} is off the gnu's body at {:?}",
                    s.giant.pos
                ),
            }
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
    let mut sim = arena_with_free_fists();
    untouchable_player(&mut sim);
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    let spot = ae::Vec2::new((back.min.x + back.max.x) * 0.5 - 60.0, back.min.y - 40.0);
    // The fixture puts the fight in phase 2 with no clamp. Until a move of
    // the free fists starts, the arms are on the giant, and he would fight a
    // player on its back (the pattern has no move then).
    step_until(&mut sim, 600, "the giant's arms in their lines", |sim| {
        let world = sim.world_mut();
        let scholar = world.query::<(Entity, &BossConfig)>().iter(world).find(|(_, config)| config.behavior.id == GNU_TON_ID).map(|(e, _)| e);
        scholar.and_then(|e| conductor_of(world, e)).is_some_and(|view| view.look_lines.iter().flatten().count() == 4)
    });
    place_player(&mut sim, spot);
    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    let (kin, _) = player(&mut sim);
    let feet = kin.pos.y + kin.size.y * 0.5;
    assert!((feet - back.min.y).abs() < 2.0, "standing on the gnu's back ({}): feet at {feet}", back.min.y);

    step_until(&mut sim, 1500, "the gnu to buck", |sim| pattern_move(sim).as_deref() == Some("buck"));
    let (kin, _) = player(&mut sim);
    assert!(
        ((kin.pos.y + kin.size.y * 0.5) - back.min.y).abs() < 2.0,
        "the premise: still standing on the gnu's back when it bucks"
    );
    touchable_player(&mut sim);
    step_until(&mut sim, 120, "the buck to strike", |sim| scene(sim).performing == Some(("buck", true)));
    thrown_off_the_back(&mut sim, back, 8);
}

/// Where the giant's art has moss, a body can stand: the slope of its back
/// (one line, walked), its shoulders and the level part of each horn.
#[test]
fn a_body_stands_where_the_giant_has_moss() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    let s = scene(&mut sim);
    let ground: Vec<ae::Aabb> = s.giant_back.ground(&s.giant, s.giant_unmirrored).collect();
    assert_eq!(ground.len(), 3, "the giant has its shoulders and two horns");
    let slope = s.giant_back.slope_line(&s.giant, s.giant_unmirrored).expect("the giant's back has a slope");
    // The highest point of the slope under a span: where a body over it stands.
    let slope_under = |left: f32, right: f32| {
        slope
            .windows(2)
            .filter_map(|w| {
                let (a, b) = (w[0], w[1]);
                let (u0, u1) = (a.x.max(left), b.x.min(right));
                (u1 > u0).then(|| {
                    let at = |x: f32| a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x);
                    at(u0).min(at(u1))
                })
            })
            .reduce(f32::min)
    };
    let spots = ground
        .iter()
        .map(|ledge| ae::Vec2::new((ledge.min.x + ledge.max.x) * 0.5, ledge.min.y - 40.0))
        .chain(slope.windows(2).map(|w| (w[0] + w[1]) * 0.5 - ae::Vec2::new(0.0, 40.0)));
    for spot in spots {
        place_player(&mut sim, spot);
        for _ in 0..20 {
            sim.step(AgentAction::default());
        }
        let (kin, _) = player(&mut sim);
        let feet = kin.pos.y + kin.size.y * 0.5;
        let (left, right) = (kin.pos.x - kin.size.x * 0.5, kin.pos.x + kin.size.x * 0.5);
        let on_a_ledge = ground
            .iter()
            .any(|other| other.max.x > left && other.min.x < right && (feet - other.min.y).abs() < 2.0);
        let on_the_slope = slope_under(left, right).is_some_and(|top| (feet - top).abs() < 2.0);
        assert!(
            on_a_ledge || on_the_slope,
            "a body put at {spot:?} stands on the giant: feet at {feet}, at x {}",
            kin.pos.x
        );
    }
}

/// The slope of the giant's back is walked, not hopped: from the rump up to
/// the shoulders with no tick in the air (it was ten steps a body hopped).
#[test]
fn a_body_walks_up_the_giants_back_to_his_shoulders() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    let s = scene(&mut sim);
    let slope = s.giant_back.slope_line(&s.giant, s.giant_unmirrored).expect("the giant's back has a slope");
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    // The low end of the slope is the rump; walk toward the high end.
    let (low, high) = if slope[0].y > slope[slope.len() - 1].y { (slope[0], slope[slope.len() - 1]) } else { (slope[slope.len() - 1], slope[0]) };
    let toward = (high.x - low.x).signum();
    place_player(&mut sim, ae::Vec2::new(low.x + toward * 30.0, low.y - 60.0));
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let mut airborne = 0;
    for _ in 0..240 {
        sim.step(AgentAction { move_x: toward, ..AgentAction::default() });
        let (kin, _) = player(&mut sim);
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&ae::BodyGroundState, PrimaryPlayerOnly>();
        if !q.iter(world).next().expect("the player").on_ground {
            airborne += 1;
        }
        if (kin.pos.y + kin.size.y * 0.5 - back.min.y).abs() < 2.0 && kin.pos.x > back.min.x && kin.pos.x < back.max.x {
            assert_eq!(airborne, 0, "the walk up the slope left the ground {airborne} ticks");
            return;
        }
    }
    let (kin, _) = player(&mut sim);
    panic!("the body did not reach the giant's shoulders: it is at {:?}, the back at {back:?}", kin.pos);
}

/// Hit him from the gnu's back and the gnu answers at once: it throws you off.
///
/// The back is the way to reach him, and a buck came once a cycle, on a roll: a
/// scripted swinger stood there and took all 42 of his HP (`fight_discovery`:
/// every hit "from raised ground", 15.5 s). His pattern cannot answer faster —
/// a strike already live commits, so its buck waited out up to a whole strike
/// (0.55 s in the measured climb, which took 15 HP). The gnu's own REFLEX does:
/// hurt its rider while you stand on its back and it rears and throws you.
#[test]
fn hit_him_from_the_gnus_back_and_it_bucks_you_off() {
    // The fists are bodies of their own: the first blow starts the throw.
    // (With the giant's arms on it, the third does: the test after this.)
    let mut sim = arena_with_free_fists();
    untouchable_player(&mut sim);
    // His first move, well before the pattern's own buck.
    step_until(&mut sim, 600, "his first move", |sim| scene(sim).performing.is_some());
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    assert_ne!(pattern_move(&mut sim).as_deref(), Some("buck"), "the premise: no buck due yet");

    let stand = ae::Vec2::new(s.scholar.pos.x - 45.0, back.min.y - 40.0);
    assert!(strike_him_from(&mut sim, stand, 1.0), "the premise: a blow from his shoulders lands");
    let (kin, _) = player(&mut sim);
    assert!(
        ((kin.pos.y + kin.size.y * 0.5) - back.min.y).abs() < 2.0,
        "the premise: the blow was struck standing on the gnu's back"
    );
    touchable_player(&mut sim);
    // Its rear-up is 0.35 s (21 frames): thrown within it and a few frames more.
    thrown_off_the_back(&mut sim, back, 28);
}

/// A blow on him from the giant's back, when his book is not up. He moves on
/// the back and guards, so this tries from beside him until a blow lands; the
/// player is put on the side of him that has more of the back.
fn land_a_blow_on_him(sim: &mut Platformer2dSimHarness, back: ae::Aabb) -> f32 {
    for _ in 0..12 {
        let him = scene(sim).scholar;
        let side = if him.pos.x - back.min.x > back.max.x - him.pos.x { -1.0 } else { 1.0 };
        if strike_him_from(sim, ae::Vec2::new(him.pos.x + side * 45.0, back.min.y - 40.0), -side) {
            return side;
        }
    }
    panic!("twelve tries from beside him on the back, and no blow landed");
}

/// With the giant's arms on it (phase 1) a blow from its back LAUNCHES him, as
/// a blow launches a fighter: away from the blow, off his feet, and he comes
/// down on the back, or comes back to it when the blow threw him off it. The
/// player keeps the back for two blows; the third starts the giant's throw.
#[test]
fn with_its_arms_on_a_blow_launches_him_and_the_third_blow_throws() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    step_until(&mut sim, 600, "his first move", |sim| scene(sim).performing.is_some());
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    let level = s.scholar.pos.y;

    for blow in ["first", "second"] {
        let side = land_a_blow_on_him(&mut sim, back);
        let from = scene(&mut sim).scholar.pos;
        step_until(&mut sim, 30, "him in the air, away from the blow", |sim| {
            let him = scene(sim).scholar;
            him.pos.y < level - 20.0 && (him.pos.x - from.x) * -side > 10.0
        });
        step_until(&mut sim, 300, "him on his feet on the back again", |sim| {
            let him = scene(sim).scholar;
            (him.pos.y - level).abs() < 1.0 && (back.min.x..=back.max.x).contains(&him.pos.x)
        });
        let (kin, _) = player(&mut sim);
        assert!(
            ((kin.pos.y + kin.size.y * 0.5) - back.min.y).abs() < 2.0,
            "the {blow} blow threw the player: {:?}",
            kin.pos
        );
    }

    // The third: the giant throws.
    land_a_blow_on_him(&mut sim, back);
    touchable_player(&mut sim);
    thrown_off_the_back(&mut sim, back, 28);
}

/// A blow that throws him off the back does not end him there: he comes back
/// to the back, and a player at its far end is not left alone, he goes to him.
#[test]
fn thrown_off_the_back_he_comes_back_and_he_goes_to_a_player_at_its_far_end() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    step_until(&mut sim, 600, "his first move", |sim| scene(sim).performing.is_some());
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    let (saddle, level) = (s.scholar.pos.x, s.scholar.pos.y);
    // From his left, in his saddle by the right end of the back: off that end.
    assert!(strike_him_from(&mut sim, ae::Vec2::new(saddle - 45.0, back.min.y - 40.0), 1.0), "the premise: a blow lands");
    step_until(&mut sim, 90, "him off the end of the back, under it", |sim| {
        let him = scene(sim).scholar;
        him.pos.x > back.max.x && him.pos.y > level
    });
    step_until(&mut sim, 300, "him on the back again", |sim| {
        let him = scene(sim).scholar;
        (him.pos.y - level).abs() < 1.0 && (back.min.x..=back.max.x).contains(&him.pos.x)
    });
    // The far end of the back: out of the reach of each move he has.
    place_player(&mut sim, ae::Vec2::new(back.min.x + 30.0, back.min.y - 40.0));
    step_until(&mut sim, 300, "him at the player, on his feet", |sim| {
        let him = scene(sim).scholar;
        let (kin, _) = player(sim);
        (him.pos.x - kin.pos.x).abs() < 130.0 && (him.pos.y - level).abs() < 1.0
    });
}

/// His guard and the move on his body, this frame.
fn his_guard_and_move(sim: &mut Platformer2dSimHarness) -> (bool, Option<String>) {
    use ambition_platformer2d::boss_encounter::BossEncounter;
    use ambition_platformer2d::combat::moveset::MovePlayback;
    let world = sim.world_mut();
    let mut bosses = world.query::<(&BossConfig, Option<&MovePlayback>, &BossEncounter)>();
    bosses
        .iter(world)
        .find(|(config, ..)| config.behavior.id == GNU_TON_ID)
        .map(|(_, playback, status)| (status.guarded, playback.map(|pb| pb.spec.id.to_string())))
        .expect("the scholar is in his arena")
}

/// He reads the player. A slow blow (a smash) that he sees come finds his
/// book up, and when a blow of the player is over he does not wait for his
/// turn: he strikes at once.
#[test]
fn he_guards_a_slow_blow_he_reads_and_strikes_when_it_is_over() {
    use ambition_platformer2d::sim::AttackStrengthHint;
    let mut sim = arena();
    untouchable_player(&mut sim);
    step_until(&mut sim, 600, "his first move", |sim| scene(sim).performing.is_some());
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    let hp = s.scholar_hp;
    // Beside him, on the back, face to him, and a smash at once: he has
    // chosen no move yet (the beat before his first is 0.35 s).
    place_player(&mut sim, ae::Vec2::new(s.scholar.pos.x - 45.0, back.min.y - 21.0));
    for _ in 0..10 {
        sim.step(AgentAction { move_x: 1.0, ..Default::default() });
        let (me, _) = player(&mut sim);
        if me.facing == 1.0 && me.vel.y == 0.0 {
            break;
        }
    }
    let (mut live_frames, mut guarded_while_live, mut over_at, mut struck_at) = (0, 0, None, None);
    let mut played: Vec<(usize, String, bool)> = Vec::new();
    for frame in 0..90 {
        let action = match frame {
            0 => AgentAction { move_x: 1.0, attack: true, attack_held: true, attack_strength: AttackStrengthHint::Smash, ..Default::default() },
            _ => AgentAction::default(),
        };
        sim.step(action);
        let live = {
            use ambition_platformer2d::combat::moveset::MovePlayback;
            let world = sim.world_mut();
            let mut players = world.query_filtered::<Option<&MovePlayback>, PrimaryPlayerOnly>();
            players.iter(world).next().flatten().map(|pb| {
                (pb.spec.id.to_string(), pb.spec.windows.iter().any(|w| !w.volumes.is_empty() && (w.start_s..w.end_s).contains(&pb.t)))
            })
        };
        let (guarded, on_body) = his_guard_and_move(&mut sim);
        if let Some((id, _)) = &live {
            played.push((frame, id.clone(), guarded));
        }
        let live = live.map(|(_, live)| live);
        match live {
            Some(true) => {
                live_frames += 1;
                guarded_while_live += usize::from(guarded);
            }
            Some(false) if live_frames > 0 && over_at.is_none() => over_at = Some(frame),
            _ => {}
        }
        if over_at.is_some() && struck_at.is_none() && on_body.is_some_and(|id| id.starts_with("gnu_duel_")) {
            struck_at = Some(frame);
        }
    }
    assert!(live_frames > 0, "the premise: the player's smash was live");
    assert_eq!(guarded_while_live, live_frames, "his book was down for a part of a smash he saw come: {played:?}");
    assert_eq!(scene(&mut sim).scholar_hp, hp, "the smash on his book took health");
    let (over_at, struck_at) = (over_at.expect("the smash ended"), struck_at.expect("after the smash he played no move"));
    assert!(struck_at - over_at <= 6, "he waited {} frames after the player's blow was over", struck_at - over_at);
}

/// With the giant's arms on it and the player on its back, HE fights: he
/// plays moves of the fighter `npc_gnu_ton` on his own body, and they hurt.
/// The giant does not punch then.
#[test]
fn on_the_giants_back_he_fights_with_his_own_moves() {
    use ambition_platformer2d::boss_encounter::BossEncounter;
    use ambition_platformer2d::combat::moveset::MovePlayback;
    let mut sim = arena();
    step_until(&mut sim, 600, "his first move", |sim| scene(sim).performing.is_some());
    let s = scene(&mut sim);
    let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
    // Beside him, at the length of his quill, on the side of the giant's head.
    place_player(&mut sim, ae::Vec2::new(s.scholar.pos.x - 80.0, back.min.y - 40.0));
    let (_, hp_before) = player(&mut sim);
    let mut swung: Vec<String> = Vec::new();
    let mut hurt_at = None;
    // His book: up between his moves, and down while he swings.
    let (mut book_up, mut swung_with_book_up) = (0, 0);
    for frame in 0..240 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut bosses = world.query::<(&BossConfig, Option<&MovePlayback>, &BossEncounter)>();
        let (on_body, guarded) = bosses
            .iter(world)
            .find(|(config, ..)| config.behavior.id == GNU_TON_ID)
            .map(|(_, playback, status)| (playback.map(|pb| pb.spec.id.clone()), status.guarded))
            .expect("the scholar is in his arena");
        let on_body = on_body.filter(|id| id.starts_with("gnu_duel_"));
        book_up += usize::from(guarded);
        swung_with_book_up += usize::from(guarded && on_body.is_some());
        if let Some(id) = on_body {
            if swung.last() != Some(&id) {
                swung.push(id);
            }
        }
        // He is the fight: the giant's punch is not performed.
        if frame > 30 {
            assert_ne!(
                scene(&mut sim).performing.map(|(key, _)| key),
                Some("portal_punch"),
                "frame {frame}: the giant punches while he duels"
            );
        }
        if hurt_at.is_none() && player(&mut sim).1 < hp_before {
            hurt_at = Some(frame);
        }
    }
    assert!(!swung.is_empty(), "in four seconds on the back beside him he played no move of his table");
    assert!(hurt_at.is_some(), "he swung ({swung:?}) at a player who stood beside him, and it never hurt");
    assert!(book_up > 0, "between his moves ({swung:?}) his book was never up");
    assert_eq!(swung_with_book_up, 0, "his book was up while he swung");
}

/// After the apple rain a golden apple knocks him off the gnu: he sits dazed on
/// the floor, then climbs back to the saddle.
#[test]
fn eureka_puts_him_on_the_floor_and_he_climbs_back() {
    let mut sim = arena_with_free_fists();
    untouchable_player(&mut sim);
    // The apple rain comes under a part of his health (`boss_profiles.ron`:
    // 0.52), and the enrage is under 0.28.
    scholar_health_to(&mut sim, 0.45);
    place_player(&mut sim, ae::Vec2::new(300.0, 1200.0));
    step_until(&mut sim, 3000, "him dazed on the floor after the apple rain", |sim| {
        let s = scene(sim);
        s.performing == Some(("eureka", true)) && on_floor(&s.scholar, s.floor)
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
        let mut ids = world.query::<(&FeatureId, &BossConfig)>();
        let scholar =
            ids.iter(world).find(|(_, config)| config.behavior.id == GNU_TON_ID).expect("the scholar").0.clone();
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
        let back = s.giant_back.aabb(&s.giant, s.giant_unmirrored);
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
                // His soles, from the gnu's centre.
                let seat = s.scholar.pos + ae::Vec2::new(0.0, s.scholar.size.y * 0.5) - s.giant.pos;
                assert!((seat - SEAT).length() < 1.0, "frame {frame}: he stands at {seat:?}, the seat is {SEAT:?}");
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

/// Entering the arena, the fists are on the giant's arms and stay there until
/// his first move: the fight starts with the arms on it (phase 1), and a fist
/// that is in the giant's picture has its body inside the giant.
///
/// The conductor eased its fists from a defaulted latch at the world origin, so
/// two frames after the room loaded both jumped ~1300 px to the room's top-left
/// corner and slid back (Jon: "the hands start in the upper left corner, and
/// then very quickly move into the correct location").
#[test]
fn entering_the_arena_the_fists_are_on_the_giants_arms() {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room("gnu_ton_approach");
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the approach to the arena builds headlessly");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let door = {
        let world = sim.world_mut();
        let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)
            .expect("the session has a live room");
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let set = rooms.iter(world).next().expect("a room set");
        set.spec(live_definition).loading_zones
            .iter()
            .find(|zone| zone.id == "gnu_ton_approach_to_arena")
            .cloned()
            .expect("the approach has a door to the GNU-ton arena")
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
            .filter(|(_, limb)| is_fist(limb))
            .map(|(kin, limb)| (kin.pos, limb.home_offset))
            .unzip();
        let giant = world
            .query_filtered::<&ae::BodyKinematics, (With<LimbRig>, Without<Limb>)>()
            .iter(world)
            .next()
            .cloned()
            .expect("the giant is in the arena");
        let mut bosses = world.query::<(Entity, &BossConfig)>();
        let scholar = bosses.iter(world).find(|(_, config)| config.behavior.id == GNU_TON_ID).map(|(e, _)| e);
        let performing = scholar.and_then(|e| conductor_of(world, e)).is_some_and(|view| view.performing.is_some());
        (fists, homes, giant, performing)
    };
    let (built, ..) = fists(&mut sim);
    assert_eq!(built.len(), 2, "the premise: both fists are in the arena from its first frame");
    // A fist is built at its limb's home. The first tick he conducts puts it
    // on the giant's arm, drawn as nothing. That tick is not the first tick
    // of the room (MEASURED 2026-10-11: the second), so for those ticks a
    // fist is at its home. It is a debt of the fight
    // (`docs/planning/game/gnu-ton-fight-debts.md`), and this is its bound.
    let mut waited = 0;
    loop {
        sim.step(AgentAction::default());
        let (now, _, giant, _) = fists(&mut sim);
        if now.iter().all(|at| (*at - giant.pos).length() <= 1.0) {
            break;
        }
        waited += 1;
        assert!(waited < 3, "after {waited} ticks in the arena the fists are not on the giant's arms: {now:?}");
    }
    let (mut last, ..) = fists(&mut sim);
    let mut resting = 0;
    for frame in 0..60 {
        sim.step(AgentAction::default());
        let (now, _homes, giant, performing) = fists(&mut sim);
        // Before his first move each fist's body is in the middle of the giant.
        if !performing {
            resting += 1;
            for at in &now {
                let off = *at - giant.pos;
                assert!(off.length() <= 1.0, "frame {frame}: a fist on its arm is {off:?} from the giant's middle");
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

/// A phase that ends during a windup takes that windup with it on the same
/// tick, so the next phase does not open with one tick of the old phase's move.
///
/// The pattern chooses in `WorldPrep` and the trigger starts the move in
/// `Combat`; the phase changes later in the tick, in `Progression` (a blow
/// crosses a trigger with no tell beat, or a tell beat ends). The conductor
/// reads the scholar's move early in the next tick, before the trigger could
/// cancel it, so the move must be gone when the phase-change tick ends.
#[test]
fn a_phase_that_ends_during_a_windup_takes_the_windup_with_it() {
    use ambition_platformer2d::boss_encounter::BossEncounter;
    fn scholar_state(sim: &mut Platformer2dSimHarness) -> (String, Option<String>) {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &BossAttackState, &BossEncounter)>();
        let (_, state, encounter) = q
            .iter(world)
            .find(|(config, ..)| config.behavior.id == "gnu_ton_rider")
            .expect("GNU-ton is in the arena");
        let phase = format!("{:?}", encounter.encounter_phase());
        let windup = state.telegraph_profile.as_ref().map(|p| match p {
            BossAttackProfile::Strike(key) | BossAttackProfile::Special(key) => key.clone(),
        });
        (phase, windup)
    }
    fn set_scholar_hp_fraction(sim: &mut Platformer2dSimHarness, fraction: f32) {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
        let (_, mut health) = q
            .iter_mut(world)
            .find(|(config, _)| config.behavior.id == "gnu_ton_rider")
            .expect("the scholar");
        health.health.current = ((health.max() as f32) * fraction).ceil() as i32;
    }

    let mut sim = arena();
    untouchable_player(&mut sim);
    // Into Phase 2 (`gnu_ton_rider.ron`: below 0.65, after a 2 s tell beat).
    step_until(&mut sim, 600, "the fight to start", |sim| scholar_state(sim).0 == "Phase1");
    set_scholar_hp_fraction(&mut sim, 0.5);
    step_until(&mut sim, 600, "Phase 2", |sim| scholar_state(sim).0 == "Phase2");
    // The first tick of a Phase 2 windup, so the whole telegraph is still ahead.
    step_until(&mut sim, 900, "a Phase 2 windup", |sim| scholar_state(sim).1.is_some());
    let windup = scholar_state(&mut sim).1.unwrap();

    // A blow lands on the next tick and crosses Enrage's trigger (below 0.28,
    // no tell beat), so Phase 2 ends in that tick's progression phase.
    set_scholar_hp_fraction(&mut sim, 0.2);
    sim.step(AgentAction::default());
    let (now, still) = scholar_state(&mut sim);
    assert_eq!(now, "Enrage", "the fixture did not end Phase 2");
    assert_ne!(
        still.as_deref(),
        Some(windup.as_str()),
        "`{windup}`, chosen in Phase 2, still winds up after the phase became {now}"
    );
}

/// The gnu's two stage facts come from their owners, and each is on the body
/// from the first tick the body exists.
///
/// The art has no mirrored side, so every gnu is `Unmirrored`: the hall's
/// exhibit too, which no scholar rides. The hall stands the whole giant
/// (`npc_giant_gnu_whole`, one stand for each identity), which wears the same
/// sheet. The depth plane is the placement's: the arena stands the gnu behind
/// the fight, and the hall keeps it in the playable plane, where you can walk
/// up to it and inspect it.
#[test]
fn the_gnu_is_built_unmirrored_and_only_its_arena_stands_it_behind() {
    fn first_gnu(sim: &mut Platformer2dSimHarness, room: &str, id: &str) -> (bool, Option<ae::DepthPlane>) {
        for _ in 0..300 {
            let world = sim.world_mut();
            let found = world
                .query::<(&ambition_platformer2d::characters::actor::WornCharacter, Has<ae::Unmirrored>, Option<&ae::DepthPlane>)>()
                .iter(world)
                .find(|(worn, ..)| worn.id() == id)
                .map(|(_, unmirrored, plane)| (unmirrored, plane.copied()));
            if let Some(found) = found {
                return found;
            }
            sim.step(AgentAction::default());
        }
        panic!("{room} builds no body wearing `{id}`, so this test says nothing");
    }
    let mut arena = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ARENA),
    )
    .expect("the gnu-ton arena builds headlessly");
    assert_eq!(first_gnu(&mut arena, ARENA, "npc_giant_gnu"), (true, Some(ae::DepthPlane::BEHIND)), "the arena's gnu");
    let mut hall = crate::common::fixed_60hz_room_sim("hall_of_characters");
    let (unmirrored, plane) = first_gnu(&mut hall, "hall_of_characters", "npc_giant_gnu_whole");
    assert!(unmirrored, "the hall's gnu wears the same art");
    assert!(plane.is_none_or(|plane| plane.is_playable()), "the hall's gnu is an exhibit, got {plane:?}");
}

/// The scholar is built with his drawn row, the one his conductor chooses: the
/// first tick he exists, it is on him. His conductor module writes its record
/// on him on the next tick, and finds his fists then
/// (`entering_the_arena_the_fists_start_where_they_stand`).
#[test]
fn the_scholar_is_built_with_his_conductor() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ARENA),
    )
    .expect("the gnu-ton arena builds headlessly");
    let (scholar, row) = (0..300)
        .find_map(|_| {
            let world = sim.world_mut();
            let found = world
                .query::<(Entity, &BossConfig, Has<ambition_platformer2d::sprite_sheet::character::PinnedRow>)>()
                .iter(world)
                .find(|(_, config, _)| config.behavior.id == GNU_TON_ID)
                .map(|(entity, _, row)| (entity, row));
            if found.is_none() {
                sim.step(AgentAction::default());
            }
            found
        })
        .expect("the arena builds GNU-ton");
    assert!(row, "the scholar has his drawn row on the first tick he exists");
    sim.step(AgentAction::default());
    assert!(
        conductor_of(sim.world_mut(), scholar).is_some_and(|view| view.hall.is_some()),
        "the scholar's conductor has its record on him one tick later, with his hall"
    );
}

/// The fists are built as the scholar's, because the gnu's character says its
/// rider conducts it and the arena seats him on it: on his side, posed by him,
/// combatants, their deaths his to rule, their rows his to choose. The gnu is
/// built with its row his to choose too.
#[test]
fn the_fists_are_built_as_the_scholars() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ARENA),
    )
    .expect("the gnu-ton arena builds headlessly");
    type Row = ambition_platformer2d::sprite_sheet::character::PinnedRow;
    let (fists, gnu_row) = (0..300)
        .find_map(|_| {
            let world = sim.world_mut();
            let fists: Vec<_> = world
                .query_filtered::<(
                    &ambition_platformer2d::actor::ActorFaction,
                    Has<ae::PoseOwnedExternally>,
                    Has<ambition_platformer2d::combat::components::ActiveCombatant>,
                    Has<ambition_platformer2d::combat::components::RulesetOwnsDeath>,
                    Has<Row>,
                ), With<Limb>>()
                .iter(world)
                .map(|(faction, posed, combatant, death, row)| (*faction, posed, combatant, death, row))
                .collect();
            let gnu_row = world.query_filtered::<Has<Row>, (With<LimbRig>, Without<Limb>)>().iter(world).next();
            // Its two fists and its hoof.
            let found = (fists.len() == 3).then_some(()).and(gnu_row).map(|row| (fists, row));
            if found.is_none() {
                sim.step(AgentAction::default());
            }
            found
        })
        .expect("the arena builds the gnu, both fists and its hoof");
    let theirs = (ambition_platformer2d::actor::ActorFaction::Boss, true, true, true, true);
    assert_eq!(fists, vec![theirs; 3], "each limb, on the first tick it exists");
    assert!(gnu_row, "the gnu's row is his to choose");
}


/// Defeated, he lets go: each fist leaves his hold, falls straight down where
/// it is, and comes to rest on what is under it.
///
/// ⛔ A conducted hand is held from construction (`PoseOwnedExternally`), and a
/// held body gets no gravity pull. The release wrote `gravity_scale = 1.0` and
/// left the hold on, so the scale did nothing: after his defeat the fists hung
/// in the air and drifted up at 1 to 4 units per second, and never landed.
#[test]
fn defeated_he_lets_go_and_his_fists_fall() {
    let mut sim = arena();
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    // Left fist first: position, still held, on the ground.
    let fists = |sim: &mut Platformer2dSimHarness| -> Vec<(ae::Vec2, bool, bool)> {
        let world = sim.world_mut();
        let mut fists: Vec<_> = world
            .query::<(&ae::BodyKinematics, Has<ae::PoseOwnedExternally>, &ae::BodyGroundState, &Limb)>()
            .iter(world)
            .filter(|(.., limb)| is_fist(limb))
            .map(|(kin, held, ground, _)| (kin.pos, held, ground.on_ground))
            .collect();
        fists.sort_by(|a, b| a.0.x.total_cmp(&b.0.x));
        fists
    };
    let before = fists(&mut sim);
    assert_eq!(before.len(), 2, "the premise: two fists");
    assert!(before.iter().all(|(_, held, _)| *held), "the premise: he holds both fists: {before:?}");
    {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
        for (config, mut health) in q.iter_mut(world) {
            if config.behavior.id == "gnu_ton_rider" {
                health.health.current = 0;
            }
        }
    }
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }
    let settled = fists(&mut sim);
    sim.step(AgentAction::default());
    let next = fists(&mut sim);
    for (index, (((from, ..), (to, still_held, grounded)), (then, ..))) in
        before.iter().zip(&settled).zip(&next).enumerate()
    {
        assert!(!still_held, "fist {index}: defeated, he lets it go");
        assert!((to.x - from.x).abs() < 1.0, "fist {index} falls straight down: {from:?} -> {to:?}");
        assert!(*grounded, "fist {index} has landed on what is under it: {from:?} -> {to:?}");
        assert!((then.y - to.y).abs() < 0.01, "fist {index} stays where it landed: {to:?} -> {then:?}");
    }
}

/// A defeat that is undone gives him his fists back. A replay can return him to
/// life after he let go; a fist he does not hold falls under gravity between
/// his poses, and a held body does not.
#[test]
fn alive_again_he_takes_his_fists_back() {
    let mut sim = arena();
    let set_health = |sim: &mut Platformer2dSimHarness, full: bool| {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
        for (config, mut health) in q.iter_mut(world) {
            if config.behavior.id == "gnu_ton_rider" {
                health.health.current = if full { health.health.max } else { 0 };
            }
        }
    };
    let held = |sim: &mut Platformer2dSimHarness| -> Vec<bool> {
        let world = sim.world_mut();
        world
            .query::<(Has<ae::PoseOwnedExternally>, &Limb)>()
            .iter(world)
            .filter(|(_, limb)| is_fist(limb))
            .map(|(held, _)| held)
            .collect()
    };
    set_health(&mut sim, false);
    sim.step(AgentAction::default());
    assert_eq!(held(&mut sim), vec![false, false], "the premise: defeated, he let both fists go");
    set_health(&mut sim, true);
    sim.step(AgentAction::default());
    assert_eq!(held(&mut sim), vec![true, true], "alive again, he holds both fists");
}

/// The arena's exit this frame: (ladder carves, floor gate removed), from
/// the overlay the gate writes on the live room's root. The arena is the one
/// live room (its geometry is named "Ambition: gnu ton arena"; its room id is
/// `gnu_ton_arena`).
fn arena_exit(sim: &mut Platformer2dSimHarness) -> (usize, bool) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<
        &ambition_platformer2d::world::FeatureEcsWorldOverlay,
        With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>,
    >();
    let overlay = q.single(world).expect("one live room, the arena, with its overlay");
    (
        overlay.climbable_carves.len(),
        overlay.removed_block_names.iter().any(|name| name == "ladder_floor_gate"),
    )
}

fn gnu_ton_bodies(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &BossConfig)>();
    q.iter(world).filter(|(_, config)| config.behavior.id == GNU_TON_ID).map(|(entity, _)| entity).collect()
}

/// ⭐ DEBT 13, IN THE REAL ARENA: his defeat opens the way out, and the way
/// stays open when his body goes. The gate read a dead body, so the body
/// going closed the arena again.
#[test]
fn defeating_him_opens_the_arena_and_his_body_going_does_not_close_it() {
    let mut sim = arena();
    // The intro ends before he can be beaten: a death in the intro is not a
    // won fight (the phase does not go to Death from Intro).
    let phase = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &ambition_platformer2d::boss_encounter::BossEncounter)>();
        q.iter(world)
            .find(|(config, _)| config.behavior.id == GNU_TON_ID)
            .map(|(_, encounter)| encounter.encounter_phase())
            .expect("GNU-ton")
    };
    let mut ticks = 0;
    while matches!(
        phase(&mut sim),
        ambition_platformer2d::boss_encounter::BossEncounterPhase::Dormant
            | ambition_platformer2d::boss_encounter::BossEncounterPhase::Intro
    ) {
        assert!(ticks < 1200, "the premise: his intro ends");
        sim.step(AgentAction::default());
        ticks += 1;
    }
    let (carves, _) = arena_exit(&mut sim);
    assert!(carves > 0, "the premise: the fight is on and the ladder is hidden");
    assert_eq!(arena_exit(&mut sim).1, false);
    {
        // As a lethal hit kills him (`boss_lifecycle::force_kill_boss`).
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &mut ambition_platformer2d::boss_encounter::BossEncounter, &mut BodyHealth)>();
        for (config, mut status, mut health) in q.iter_mut(world) {
            if config.behavior.id == GNU_TON_ID {
                health.health.current = 0;
                if let Some(phase) = status.encounter.as_mut() {
                    let _ = phase.kill();
                }
            }
        }
    }
    // The boss domain records the defeat when his death settles.
    let recorded = |sim: &mut Platformer2dSimHarness| {
        sim.world().resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().boss("hall.gnu_ton")
            == ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared
    };
    let mut ticks = 0;
    while !recorded(&mut sim) {
        assert!(ticks < 1800, "the premise: his death settles and the boss domain records it ({:?})", phase(&mut sim));
        sim.step(AgentAction::default());
        ticks += 1;
    }
    sim.step(AgentAction::default());
    assert_eq!(arena_exit(&mut sim), (0, true), "beaten (recorded after {ticks} ticks): the ladder and the gap are open");
    let bodies = gnu_ton_bodies(&mut sim);
    assert!(!bodies.is_empty(), "the premise: his body is still there");
    for body in bodies {
        sim.world_mut().despawn(body);
    }
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    assert_eq!(arena_exit(&mut sim), (0, true), "his body went; the recorded defeat keeps the way open");
}

/// ⭐ A won fight leaves the arena open when you come back. A cleared
/// placement is not built again, so the gate that looked for a dead body
/// found nothing and hid the ladder: the way out was shut for good.
#[test]
fn a_won_fight_leaves_the_arena_open_when_you_come_back() {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_boss("hall.gnu_ton", ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared);
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ARENA)
        .with_save(save);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the gnu-ton arena builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let alive: Vec<bool> = {
        let world = sim.world_mut();
        let mut q = world.query::<(&BossConfig, &BodyHealth)>();
        q.iter(world).filter(|(config, _)| config.behavior.id == GNU_TON_ID).map(|(_, health)| health.alive()).collect()
    };
    assert!(!alive.contains(&true), "the premise: no live GNU-ton in a won arena: {alive:?}");
    assert_eq!(arena_exit(&mut sim), (0, true), "a won arena is open");
}

/// The pair of the punch under a GGRS sync-test session. The pair is held by
/// the portal domain's adapter, and which pair each slot holds is rollback
/// state on the scholar (`portal.held_pairs`). A rewind across the tick the
/// pair opened must give it back as it was: if the record were lost, the
/// replay would open a second pair, and the checksum would disagree.
#[test]
fn the_punchs_pair_is_one_pair_across_rewinds() {
    use ambition_platformer2d::portal::extension::HeldPortalOf;
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ARENA)
        .with_sync_test_rollback_settings(4, 10);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the GGRS sync-test arena builds");
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    let mut most = 0;
    let mut opened = false;
    let mut closed_after = false;
    let mut first_error = None;
    for _ in 0..1500 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let open = world.query_filtered::<(), With<HeldPortalOf>>().iter(world).count();
        most = most.max(open);
        opened |= open > 0;
        closed_after |= opened && open == 0;
        if first_error.is_none() {
            first_error = ambition_platformer2d::rollback::session_health(sim.world()).err();
        }
        if closed_after {
            break;
        }
    }
    assert!(opened, "the premise: a punch opened the pair under the sync test");
    assert!(closed_after, "the premise: the pair closed again, so a rewind crossed both edges");
    assert_eq!(most, 2, "one pair, never two, across every rewind");
    assert_eq!(first_error, None, "the sync-test session stayed healthy");
}

/// A fist on its arm has no trail: the arm joins it, and the giant draws the
/// arm, through the punch's pair or not (the fist through the pair is cut with
/// the giant's picture). The start is on the fist (`LimbBondStart`), and the
/// view of the fist carries it (`FeatureView::limb_host`). The control is a
/// loose fist in phase 2, whose trail is from its shoulder's line.
#[test]
fn a_fist_on_its_arm_has_no_trail() {
    let bonds = |sim: &mut Platformer2dSimHarness| -> Vec<(LimbSlot, Option<ae::Vec2>)> {
        let world = sim.world_mut();
        let fists: Vec<(LimbSlot, FeatureId)> = world
            .query::<(&Limb, &FeatureId)>()
            .iter(world)
            .filter(|(limb, _)| is_fist(limb))
            .map(|(limb, id)| (limb.slot, id.clone()))
            .collect();
        let views = world.resource::<ambition_platformer2d::sim_view::FeatureViewIndex>();
        fists.into_iter().map(|(slot, id)| (slot, views.get(id.as_str()).and_then(|view| view.limb_host))).collect()
    };
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    step_until(&mut sim, 900, "the strike of a punch through the pair", |sim| {
        scene(sim).performing == Some(("portal_punch", true))
    });
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let on_arms = bonds(&mut sim);
    assert_eq!(on_arms.len(), 2, "the premise: two fists");
    assert!(on_arms.iter().all(|(_, host)| host.is_none()), "a fist on its arm has a trail: {on_arms:?}");

    let mut sim = arena_with_free_fists();
    step_until(&mut sim, 600, "the trails of the loose fists", |sim| bonds(sim).iter().all(|(_, host)| host.is_some()));
}

/// THE FIST GOES THROUGH THE PAIR (Jon 2026-10-16,
/// `docs/planning/engine/portals-a-body-in-two-places.md`). The fist's body is
/// posed under the NEAR line, where the giant's arm puts it, and it is never
/// under the far line: the portal pair carries it there. A player standing
/// under the far line is struck, and the fist is in the pair while it is.
#[test]
fn the_punch_hurts_under_the_far_line_through_the_pair() {
    use ambition_platformer2d::portal::extension::HeldPortalOf;
    use ambition_platformer2d::portal::{PlacedPortal, PortalTransit};
    let lines = |sim: &mut Platformer2dSimHarness| -> Option<(ae::Vec2, ae::Vec2)> {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&PlacedPortal, With<HeldPortalOf>>();
        let near = q.iter(world).find(|p| p.normal.y < 0.0)?.pos;
        let far = q.iter(world).find(|p| p.normal.y > 0.0)?.pos;
        Some((near, far))
    };
    let right_fist = |sim: &mut Platformer2dSimHarness| -> (ae::Vec2, bool) {
        let world = sim.world_mut();
        world
            .query::<(&Limb, &ae::BodyKinematics, Has<PortalTransit>)>()
            .iter(world)
            .find(|(limb, ..)| limb.slot == LimbSlot::HAND_RIGHT)
            .map(|(_, kin, threaded)| (kin.pos, threaded))
            .expect("the right fist")
    };
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    step_until(&mut sim, 900, "the strike of a punch through the pair", |sim| {
        scene(sim).performing == Some(("portal_punch", true))
    });
    let (near, far) = lines(&mut sim).expect("the pair is open");
    // Under the far line, on the floor, and touchable again.
    place_player(&mut sim, ae::Vec2::new(far.x, far.y + 120.0));
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
        for mut health in q.iter_mut(world) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, false);
        }
    }
    let before = player(&mut sim).1;
    let mut threaded = false;
    for _ in 0..40 {
        sim.step(AgentAction::default());
        let (fist, in_pair) = right_fist(&mut sim);
        assert!((fist.x - near.x).abs() < 1.0, "the fist is posed under the near line, not the far: {fist:?} near {near:?}");
        threaded |= in_pair && fist.y > near.y;
    }
    assert!(threaded, "the fist went into the near line and was in the pair");
    assert!(player(&mut sim).1 < before, "the player under the far line was not struck");
}

/// The blow of this tick, from his conductor: its exit and whether it landed.
fn blow(sim: &mut Platformer2dSimHarness) -> Option<(ambition_content::bosses::gnu_ton::Exit, bool)> {
    let world = sim.world_mut();
    let scholar = world
        .query::<(Entity, &BossConfig)>()
        .iter(world)
        .find(|(_, config)| config.behavior.id == GNU_TON_ID)
        .map(|(entity, _)| entity)?;
    conductor_of(world, scholar)?.blow
}

/// The far end of the open pair of his blow: the end whose line is not fixed
/// to the giant. Its middle and its normal.
fn far_end(sim: &mut Platformer2dSimHarness) -> Option<(ae::Vec2, ae::Vec2)> {
    use ambition_platformer2d::portal::extension::HeldPortalOf;
    use ambition_platformer2d::portal::PlacedPortal;
    let giant = scene(sim).giant.pos;
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&PlacedPortal, With<HeldPortalOf>>();
    q.iter(world).max_by(|a, b| a.pos.distance(giant).total_cmp(&b.pos.distance(giant))).map(|p| (p.pos, p.normal))
}

/// Jon 2026-10-23: "it shouldn't punch so similarly, it should track where
/// the player is". Over the first blows of the fight, with the player on the
/// floor and then on a perch, each blow comes out where the player is, the
/// first is straight down (it shows the way up), no blow is the one before,
/// a far line is at a slant when its blow needs it, and the giant kicks.
#[test]
fn his_blows_are_aimed_at_the_player_and_none_is_the_one_before() {
    use ambition_content::bosses::gnu_ton::Exit;
    let mut sim = arena();
    untouchable_player(&mut sim);
    let floor_spot = ae::Vec2::new(300.0, 1200.0);
    let perch_spot = ae::Vec2::new(1456.0, 520.0);
    place_player(&mut sim, floor_spot);
    let mut exits: Vec<Exit> = Vec::new();
    let mut slanted = false;
    let mut was_open = false;
    for frame in 0..2400 {
        // The first three blows from the floor, the rest from a perch.
        let spot = if exits.len() < 3 { floor_spot } else { perch_spot };
        if frame % 30 == 0 {
            place_player(&mut sim, spot);
        }
        sim.step(AgentAction::default());
        let now = blow(&mut sim);
        if let (Some((exit, _)), false) = (now, was_open) {
            exits.push(exit);
        }
        if let (Some((Exit::Diagonal, _)), Some((_, normal))) = (now, far_end(&mut sim)) {
            slanted |= normal.x.abs() > 0.3 && normal.y.abs() > 0.3;
        }
        was_open = now.is_some();
        if exits.len() >= 6 {
            break;
        }
    }
    assert!(exits.len() >= 5, "the premise: five blows in the time, got {exits:?}");
    assert_eq!(exits[0], Exit::Down, "the first punch is straight down: {exits:?}");
    for pair in exits.windows(2) {
        assert_ne!(pair[0], pair[1], "a blow is the one before it: {exits:?}");
    }
    assert!(exits.iter().any(|e| matches!(e, Exit::Sweep | Exit::Stomp)), "the giant kicks: {exits:?}");
    assert!(exits[3..].iter().any(|e| matches!(e, Exit::Across | Exit::Up | Exit::Diagonal | Exit::Stomp)), "on a perch the blows come at the perch: {exits:?}");
    if exits.contains(&Exit::Diagonal) {
        assert!(slanted, "a blow at a slant came out of a line that is not at a slant");
    }
}

/// The kick is a blow through a real pair as the punch is: the hoof (a limb
/// body of its own, which the giant's picture draws) goes into the near line
/// and hurts where it comes out, and its land is the conductor's to feel.
#[test]
fn a_kick_hurts_where_the_hoof_comes_out_of_the_pair() {
    use ambition_platformer2d::portal::PortalTransit;
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(300.0, 1200.0));
    step_until(&mut sim, 1500, "the tell of a kick", |sim| scene(sim).performing == Some(("portal_kick", false)));
    touchable_player(&mut sim);
    let before = player(&mut sim).1;
    let mut threaded = false;
    let mut landed = false;
    for _ in 0..90 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        threaded |= world
            .query::<(&Limb, Has<PortalTransit>)>()
            .iter(world)
            .any(|(limb, in_pair)| limb.slot.as_str() == "hoof_right" && in_pair);
        landed |= blow(&mut sim).is_some_and(|(_, landed)| landed);
        if scene(&mut sim).performing != Some(("portal_kick", false)) && scene(&mut sim).performing != Some(("portal_kick", true)) {
            break;
        }
    }
    assert!(threaded, "the hoof went into the near line and was in the pair");
    assert!(player(&mut sim).1 < before, "the player the kick was aimed at was not struck");
    assert!(landed, "the conductor did not feel the kick land");
}
