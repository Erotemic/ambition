//! The final boss (`docs/planning/game/final-boss.md`): the bossofthegame
//! stands on the shoulders of Full Stack, the giant amp-stack robot, on the
//! main stage of the Boss dimension, and the way up to him is the robot's
//! guitar.

use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::mount::BackLedge;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

use crate::common::{base, fixed_60hz_room_sim};

const STAGE: &str = "boss_main_stage";
const STACK: &str = "npc_full_stack";

fn stack(sim: &mut ambition_app::Platformer2dSimHarness) -> (ae::BodyKinematics, BackLedge, bool) {
    let world = sim.world_mut();
    let mut q = world.query::<(&WornCharacter, &ae::BodyKinematics, Option<&BackLedge>, Has<ae::Unmirrored>)>();
    let (_, kin, back, unmirrored) =
        q.iter(world).find(|(worn, ..)| worn.id() == STACK).unwrap_or_else(|| panic!("the stage stands `{STACK}`"));
    (kin.clone(), back.expect("Full Stack's body has its top and its guitar").clone(), unmirrored)
}

fn boss(sim: &mut ambition_app::Platformer2dSimHarness) -> ae::BodyKinematics {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &ae::BodyKinematics)>();
    let (_, kin) = q
        .iter(world)
        .find(|(config, _)| config.behavior.id == "bossofthegame")
        .expect("the stage places the bossofthegame");
    kin.clone()
}

fn player(sim: &mut ambition_app::Platformer2dSimHarness) -> ae::BodyKinematics {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the player").clone()
}

fn place_player(sim: &mut ambition_app::Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

/// He stands on the robot's top, not on the stage's floor.
#[test]
fn the_boss_stands_on_the_top_of_full_stack() {
    let mut sim = fixed_60hz_room_sim(STAGE);
    sim.step_n(base(), 30);
    let (kin, back, unmirrored) = stack(&mut sim);
    let top = back.ground(&kin, unmirrored).map(|ledge| ledge.min.y).reduce(f32::min).expect("its top is ground");
    let boss = boss(&mut sim);
    let feet = boss.pos.y + boss.size().y * 0.5;
    let stack_top = kin.pos.y - kin.size().y * 0.5;
    assert!((top - stack_top).abs() < 2.0, "the ground of its top ({top}) is the top of its body ({stack_top})");
    assert!((feet - top).abs() < 24.0, "the Boss's feet ({feet}) are on the robot's top ({top})");
    assert!(
        boss.pos.x > kin.pos.x - kin.size().x * 0.5 && boss.pos.x < kin.pos.x + kin.size().x * 0.5,
        "and over it: the Boss at x {}, the robot from {} to {}",
        boss.pos.x,
        kin.pos.x - kin.size().x * 0.5,
        kin.pos.x + kin.size().x * 0.5
    );
}

/// The guitar's edge is a way up: a body put on it stands on it.
#[test]
fn a_body_stands_on_the_guitar_of_full_stack() {
    let mut sim = fixed_60hz_room_sim(STAGE);
    sim.step_n(base(), 30);
    let (kin, back, unmirrored) = stack(&mut sim);
    let guitar = back.slope_line(&kin, unmirrored).expect("the guitar is a line a body walks");
    for w in guitar.windows(2) {
        let spot = (w[0] + w[1]) * 0.5 - ae::Vec2::new(0.0, 30.0);
        place_player(&mut sim, spot);
        sim.step_n(base(), 20);
        let body = player(&mut sim);
        let feet = body.pos.y + body.size().y * 0.5;
        let x = body.pos.x;
        let on = guitar.windows(2).any(|s| {
            let (a, b) = (s[0], s[1]);
            (a.x.min(b.x)..=a.x.max(b.x)).contains(&x)
                && (feet - (a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x).max(1e-3))).abs() < 18.0
        });
        assert!(on, "a body put at {spot:?} stands on the guitar: feet at {feet}, at x {x}, moving {:?}", body.vel);
    }
}

/// What the fight throws, by look, and the rows Full Stack is drawn in,
/// over `ticks`.
fn thrown_over(sim: &mut ambition_app::Platformer2dSimHarness, ticks: usize) -> std::collections::BTreeSet<String> {
    use ambition_platformer2d::sprite_sheet::character::PinnedRow;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..ticks {
        sim.step(base());
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::projectiles::ProjectileVisualId>();
        seen.extend(q.iter(world).map(|id| id.as_str().to_string()));
        let mut rows = world.query::<(&WornCharacter, &PinnedRow)>();
        seen.extend(
            rows.iter(world)
                .filter(|(worn, _)| worn.id() == STACK)
                .filter_map(|(_, pin)| pin.rows.first().map(|row| format!("row:{row}"))),
        );
    }
    seen
}

fn boss_health_to(sim: &mut ambition_app::Platformer2dSimHarness, fraction: f32) {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &mut ambition_platformer2d::characters::actor::BodyHealth)>();
    let (_, mut health) =
        q.iter_mut(world).find(|(config, _)| config.behavior.id == "bossofthegame").expect("the bossofthegame");
    health.health.current = ((health.max() as f32) * fraction).ceil() as i32;
}

/// The fight runs through its phases on the hall's practice copy (the main
/// stage opens on its cutscene): his agent plays at first, and once he is
/// hurt he types, and the keys fall.
#[test]
fn the_agent_plays_first_and_then_he_types() {
    let mut sim = fixed_60hz_room_sim("hall_bossofthegame_stage");
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut ambition_platformer2d::characters::actor::BodyHealth, PrimaryPlayerOnly>();
        for mut health in q.iter_mut(world) {
            health.health.invulnerable.set(ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED, true);
        }
    }
    let at = boss(&mut sim).pos;
    sim.teleport_player((at.x - 360.0, at.y + 400.0));
    let first = thrown_over(&mut sim, 60 * 12);
    assert!(first.contains("sound_ring"), "Full Stack plays (chord, riff, feedback) in phase 1: {first:?}");
    assert!(
        first.contains("row:strum") && first.contains("row:idle"),
        "and it is drawn strumming its chord, and its idle strum between: {first:?}"
    );
    assert!(!first.iter().any(|id| id.starts_with("keycap_")), "he does not type yet: {first:?}");
    boss_health_to(&mut sim, 0.5);
    let second = thrown_over(&mut sim, 60 * 12);
    assert!(
        second.iter().any(|id| id.starts_with("keycap_") && id != "keycap_back"),
        "hurt, he types and the keys fall: {second:?}"
    );
}
