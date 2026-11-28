//! The final boss (`docs/planning/game/final-boss.md`): the bossofthegame
//! stands on the shoulders of Full Stack, the giant amp-stack robot, on the
//! main stage of the Boss dimension, and the way up to him is the robot's
//! guitar.

use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::mount::BackLedge;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

use crate::common::{base, fixed_60hz_room_sim};

const STAGE: &str = "boss_main_stage";
const STACK: &str = "npc_full_stack";

#[test]
fn the_band_stays_on_its_platforms() {
    let mut sim = fixed_60hz_room_sim("hall_bossofthegame_stage");
    let boss = boss(&mut sim);
    let platform = boss.pos.y + boss.size().y * 0.5;
    sim.step_n(base(), 30);
    let positions = |sim: &mut ambition_app::Platformer2dSimHarness| {
        let world = sim.world_mut();
        let mut bodies = world.query::<(&CharacterForm, &ae::BodyKinematics)>();
        bodies.iter(world).filter(|(form, _)| form.id().ends_with("_band"))
            .map(|(form, body)| (form.id().to_owned(), body.pos + ae::Vec2::Y * body.size().y * 0.5))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let initial = positions(&mut sim);
    assert_eq!(initial.len(), 3);
    for _ in 0..300 {
        sim.step(base());
        let current = positions(&mut sim);
        assert_eq!(current.len(), initial.len());
        for (form, at) in &current {
            assert!((at.y - platform).abs() < 4.0,
                "{form}'s feet must stay on its platform at {platform}: {at:?}");
        }
    }
}

fn stack(sim: &mut ambition_app::Platformer2dSimHarness) -> (ae::BodyKinematics, BackLedge, bool) {
    let world = sim.world_mut();
    let mut q = world.query::<(&CharacterForm, &ae::BodyKinematics, Option<&BackLedge>, Has<ae::Unmirrored>)>();
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
        let mut rows = world.query::<(&CharacterForm, &PinnedRow)>();
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
    assert!(second.contains("row:solo"), "and Full Stack plays its solo while he holds his beat between phases: {second:?}");
}

/// The catalog characters worn by the bodies on the stage now.
fn worn_on_stage(sim: &mut ambition_app::Platformer2dSimHarness) -> std::collections::BTreeSet<String> {
    let world = sim.world_mut();
    let mut q = world.query::<&CharacterForm>();
    q.iter(world).map(|worn| worn.id().to_string()).collect()
}

/// The rows the band's bodies are drawn in now, by the character each wears.
fn band_rows(sim: &mut ambition_app::Platformer2dSimHarness) -> std::collections::BTreeMap<String, String> {
    use ambition_platformer2d::sprite_sheet::character::PinnedRow;
    let world = sim.world_mut();
    let mut q = world.query::<(&CharacterForm, Option<&PinnedRow>)>();
    q.iter(world)
        .filter(|(worn, _)| worn.id().ends_with("_band"))
        .map(|(worn, pin)| (worn.id().to_string(), pin.and_then(|pin| pin.rows.first().cloned()).unwrap_or_default()))
        .collect()
}

/// He calls his named agents in: Geppetto as his second phase opens ("Do I
/// need to get Geppetto to do it?"), Clause and Wren as his encore does
/// ("Spawn subagents."). Until then they play in his band, each at an
/// instrument, and a call takes the called one off it.
#[test]
fn his_named_agents_join_the_fight() {
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
    sim.step_n(base(), 60 * 4);
    let first = worn_on_stage(&mut sim);
    assert!(!first.contains("npc_geppetto"), "his agent plays alone at first: {first:?}");
    let band: Vec<(String, String)> = band_rows(&mut sim).into_iter().collect();
    assert_eq!(
        band,
        [("npc_clause_band", "band_drums"), ("npc_geppetto_band", "band_guitar"), ("npc_wren_band", "band_triangle")]
            .map(|(who, row)| (who.to_string(), row.to_string())),
        "the band plays as the show opens, each at an instrument"
    );
    boss_health_to(&mut sim, 0.5);
    sim.step_n(base(), 60 * 6);
    let second = worn_on_stage(&mut sim);
    assert!(second.contains("npc_geppetto"), "in his second phase Geppetto is on the stage: {second:?}");
    assert!(!second.contains("npc_clause") && !second.contains("npc_wren"), "and not yet the others: {second:?}");
    let band: Vec<String> = band_rows(&mut sim).into_keys().collect();
    assert_eq!(band, ["npc_clause_band", "npc_wren_band"], "Geppetto put the guitar down; the others play on");
    boss_health_to(&mut sim, 0.2);
    sim.step_n(base(), 60 * 6);
    let encore = worn_on_stage(&mut sim);
    assert!(
        encore.contains("npc_clause") && encore.contains("npc_wren"),
        "at his encore Clause and Wren are on the stage: {encore:?}"
    );
    assert!(band_rows(&mut sim).is_empty(), "and the band has left its instruments");
}

#[test]
fn the_stage_defeat_leaves_the_boss_alive_and_the_game_open() {
    use ambition_app::AgentAction;
    use ambition_platformer2d::actors::body::{direction::Posed, navigation::NavigationAdvice, visit::Visit};
    use ambition_platformer2d::boss_encounter::BossEncounter;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    use ambition_platformer2d::cutscene::ActiveCutscene;
    use ambition_platformer2d::persistence::save::AmbitionGameSave;
    use ambition_platformer2d::platformer::schedule::GameMode;
    use crate::common::walk::{Pad, walk_through, walk_to};

    const PLACEMENT: &str = "boss_stage.bossofthegame";
    const GREEN: &str = "boss_green_room";
    let mut sim = fixed_60hz_room_sim(STAGE);
    let boss_entity = {
        let world = sim.world_mut();
        world.query::<(Entity, &BossConfig)>().iter(world)
            .find(|(_, config)| config.id == PLACEMENT).unwrap().0
    };
    let advance = |sim: &mut ambition_app::Platformer2dSimHarness, tick| {
        sim.step(AgentAction { confirm: tick % 6 == 0, ..base() });
    };
    let mut saw_lead = false;
    let mut saw_full = false;
    for tick in 0..600 {
        advance(&mut sim, tick);
        if let Some(title) = sim.world().resource::<ambition_platformer2d::sim_view::BossRenderIndex>()
            .get(PLACEMENT).filter(|view| view.stage.card > 0.0).and_then(|view| view.title.as_deref())
        {
            saw_lead |= title == "Until the day he dies…";
            if title == "Until the day he dies… or retires" {
                assert!(saw_lead, "the start of the title comes before the reveal");
                saw_full = true;
                break;
            }
        }
    }
    assert!(saw_lead && saw_full, "show both parts of the title");

    let can_fight = |sim: &ambition_app::Platformer2dSimHarness| {
        sim.world().resource::<State<GameMode>>().get().allows_gameplay()
            && !sim.world().resource::<ActiveCutscene>().is_playing()
            && sim.world().get::<BossEncounter>(boss_entity).unwrap().encounter.as_ref()
            .is_some_and(|phase| !phase.boss_invulnerable())
    };
    for tick in 0..420 {
        if can_fight(&sim) {
            break;
        }
        advance(&mut sim, tick);
    }
    assert!(can_fight(&sim), "combat opens after the introduction, mode={:?}, scene={:?}",
        sim.world().resource::<State<GameMode>>().get(),
        sim.world().resource::<ActiveCutscene>().iter().map(|scene| scene.line()).collect::<Vec<_>>());
    let at = boss(&mut sim).pos;
    sim.world_mut().write_message(HitEvent {
        target: HitTarget::Feature(boss_entity),
        volume: ae::Aabb::new(at, ae::Vec2::splat(64.0)).into(),
        damage: 9_999,
        source: HitSource::Projectile,
        mode: HitMode::Knockback,
        attacker: None,
        room: None,
        status: None,
        origin: None,
        properties: Default::default(),
        contact: None,
        pieces: Vec::new(),
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
        attacker_move_instance: None,
    });
    sim.step_n(base(), 3);
    assert_eq!(sim.world().get::<BodyHealth>(boss_entity).unwrap().current(), 1,
        "the last hit leaves the boss with enough health to leave");
    let mut left_its_saddle = false;
    let mut walked = false;
    let mut saw_getup = false;
    for tick in 0..1700 {
        advance(&mut sim, tick);
        if let Some(body) = sim.world().get::<ae::BodyKinematics>(boss_entity) {
            assert!(sim.world().get::<BodyHealth>(boss_entity).unwrap().alive(), "the boss yields alive");
            left_its_saddle |= sim.world().get::<ambition_platformer2d::mount::RidingOn>(boss_entity).is_none();
            walked |= saw_getup && body.pos.x > at.x + 450.0;
            saw_getup |= sim.world().get::<Posed>(boss_entity).is_some_and(|pose| pose.row == "getup");
        }
        if sim.world().get_entity(boss_entity).is_err()
            && !sim.world().resource::<ActiveCutscene>().is_playing()
        {
            break;
        }
    }
    assert!(left_its_saddle && saw_getup && walked,
        "the boss gets up and walks to the exit: dismounted={left_its_saddle}, getup={saw_getup}, walked={walked}, start={at:?}, end={:?}, ground={:?}, visit={:?}, route={:?}",
        sim.world().get::<ae::BodyKinematics>(boss_entity).map(|body| body.pos),
        sim.world().get::<ae::BodyGroundState>(boss_entity),
        sim.world().get::<Visit>(boss_entity),
        sim.world().resource::<NavigationAdvice>().of(boss_entity).errand);
    assert!(sim.world().get_entity(boss_entity).is_err(), "the stage body went through the door");
    assert!(crate::boss_lifecycle::boss_cleared(&sim, PLACEMENT));

    let half_height = player(&mut sim).size().y * 0.5;
    place_player(&mut sim, ae::Vec2::new(2470.0, 1088.0 - half_height));
    walk_through(&mut Pad::new(&mut sim), GREEN).expect("the green room is open");
    walk_to(&mut Pad::new(&mut sim), ae::Vec2::new(310.0, 576.0)).expect("reach the green room scene");
    for tick in 0..700 {
        advance(&mut sim, tick);
        if sim.world().resource::<AmbitionGameSave>().data().flag("boss_early_access_seen") {
            break;
        }
    }
    let save = sim.world().resource::<AmbitionGameSave>().data();
    assert!(save.flag("boss_early_access_seen"));
    assert!(!save.flag("bossofthegame_retired"));
    assert!(!sim.world().resource::<ActiveCutscene>().is_playing());
    walk_through(&mut Pad::new(&mut sim), "boss_lobby").expect("keep exploring after the scene");
}
