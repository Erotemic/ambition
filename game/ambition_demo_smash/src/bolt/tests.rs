//! The rate limit is the move. A bolt that snaps to the stick is a cursor, and
//! it removes the move's one cost: a turn spends distance. The bolt comes from
//! its module on the module road.

use bevy::ecs::message::Messages;
use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_demo_smash_modules::bolt::BOLT;
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_bolt::{SteeredBoltParams, STEERED_BOLT};
use ambition_platformer2d::platformer::sim_id::{SimId, SimIdCounter};
use ambition_platformer2d::vfx::{Effect, EffectRequest};

const DT: f32 = 1.0 / 60.0;

/// A fighter's half size.
const HALF: ae::Vec2 = ae::Vec2::new(14.0, 26.0);

fn app_with(road: Road) -> App {
    app_on(road, ambition_demo_smash_modules::bolt::module)
}

fn app() -> App {
    app_with(Road::Linked)
}

/// A fighter in `slot` at `x`, facing right, holding nothing: it can fire a
/// bolt (it names what it spawns) and be touched by one.
fn fighter(app: &mut App, slot: u8, x: f32) -> Entity {
    let at = ae::Vec2::new(x, 0.0);
    app.world_mut()
        .spawn((
            ae::BodyKinematics { pos: at, facing: 1.0, size: HALF * 2.0, ..Default::default() },
            ae::CenteredAabb::new(at, HALF),
            ambition_platformer2d::actor::ActorFaction::Player,
            ambition_platformer2d::platformer::lifecycle::FeatureSimEntity,
            SimId::player_slot(slot),
            SimIdCounter::default(),
            ActorControl(ambition_platformer2d::characters::actor::control::ActorControlFrame::neutral()),
        ))
        .id()
}

fn params() -> SteeredBoltParams {
    SteeredBoltParams {
        trail_vfx: "test_trail".to_string(),
        trail_every_s: 0.05,
        speed: 300.0,
        turn_rate_deg: 220.0,
        lifetime_s: 2.0,
        damage: 8,
        radius: 10.0,
        knockback: 90.0,
        self_launch: 640.0,
        offset: (18.0, -10.0),
        // Not the shipped 0.06: see
        // `the_bolt_damages_somebody_else_and_is_spent`.
        hit_s: 0.3,
    }
}

/// Press the bolt button as `actor`; the press's tick has run.
fn fire(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(STEERED_BOLT.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("bolt params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

/// Hold `stick` on `who` from now on.
fn hold(app: &mut App, who: Entity, stick: ae::Vec2) {
    let mut control = app.world_mut().get_mut::<ActorControl>(who).unwrap();
    // `undamped_locomotion` is what `steer_axis()` returns: the field that
    // survives the damped republish, so a rooted move can be aimed.
    control.0.undamped_locomotion = Some(ae::LocalAxes::new(stick.x, stick.y));
}

/// Every bolt on the stage: who fired it, and where it is.
fn bolts(app: &mut App) -> Vec<(Option<SimId>, ae::Vec2)> {
    app.world_mut()
        .query::<&ModuleEntity>()
        .iter(app.world())
        .filter(|entity| entity.kind == BOLT)
        .map(|entity| (entity.spawner.clone(), entity.pos))
        .collect()
}

fn the_bolt(app: &mut App) -> ae::Vec2 {
    let out = bolts(app);
    assert_eq!(out.len(), 1, "not one bolt on the stage: {out:?}");
    out[0].1
}

/// Put the one bolt at `at`: its next tick reads who is there.
fn put_bolt(app: &mut App, at: ae::Vec2) {
    let world = app.world_mut();
    let mut q = world.query::<&mut ModuleEntity>();
    let mut bolt = q.iter_mut(world).find(|entity| entity.kind == BOLT).expect("a bolt is out");
    bolt.pos = at;
}

/// The hits of this tick and the last, by their live time.
fn hits(app: &mut App) -> Vec<f32> {
    let messages = app.world().resource::<Messages<EffectRequest>>();
    messages
        .get_cursor()
        .read(messages)
        .filter_map(|request| match &request.effect {
            Effect::DamageBox(hit) => Some(hit.lifetime_s),
            _ => None,
        })
        .collect()
}

/// It leaves the press at the move's offset, and flies forward from the next
/// tick; it is its caster's.
#[test]
fn the_bolt_leaves_forward_and_belongs_to_the_body_that_fired_it() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    let out = bolts(&mut app);
    assert_eq!(out, vec![(Some(SimId::player_slot(1)), ae::Vec2::new(18.0, -10.0))]);
    app.update();
    let step = the_bolt(&mut app) - ae::Vec2::new(18.0, -10.0);
    assert!((step - ae::Vec2::new(300.0 * DT, 0.0)).length() < 1e-3, "it did not leave forward at its speed: {step:?}");
}

/// The turn is rate-limited, which is the cost of steering. At 220°/s one tick
/// turns under 4°; a bolt that snapped would point the other way at once.
#[test]
fn the_stick_turns_the_bolt_gradually_and_never_snaps_it() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    app.update();
    // Ask for a hard reversal, and keep asking.
    hold(&mut app, caster, ae::Vec2::new(-1.0, 0.0));
    let before = the_bolt(&mut app);
    app.update();
    let step = the_bolt(&mut app) - before;
    assert!(step.x > 0.0, "one tick of stick reversed the bolt, so it snaps rather than turns: {step:?}");
    assert!(step.y.abs() > 0.0, "the bolt did not turn at all: {step:?}");
    let turned = step.y.atan2(step.x).abs().to_degrees();
    assert!(turned <= 220.0 * DT + 1e-3, "it turned {turned}° in a tick against 220°/s");
    // The speed does not change when the bolt turns.
    assert!((step.length() - 300.0 * DT).abs() < 1e-3, "turning changed the speed: {step:?}");
    // Given enough ticks it does come around.
    for _ in 0..60 {
        app.update();
    }
    let before = the_bolt(&mut app);
    app.update();
    assert!(the_bolt(&mut app).x < before.x, "a held reversal never brought it around");
}

/// The thunder jacket: the bolt comes home and launches its caster.
#[test]
fn the_bolt_launches_its_caster_and_does_not_damage_him() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    // The bolt must leave him first. This is the move's rule: otherwise every
    // press would be an instant self-launch. It starts inside him and goes
    // nowhere near anyone else, so it is still out after ten ticks.
    for _ in 0..10 {
        app.update();
    }
    assert!(the_bolt(&mut app).x > HALF.x + 10.0, "the bolt never got clear of its caster");
    put_bolt(&mut app, ae::Vec2::ZERO);
    app.update();
    let kin = app.world().get::<ae::BodyKinematics>(caster).unwrap();
    assert!((kin.vel - ae::Vec2::new(640.0, 0.0)).length() < 1e-2, "the bolt came home and did not throw him along it: {:?}", kin.vel);
    assert!(bolts(&mut app).is_empty(), "the bolt survived coming home");
    assert!(hits(&mut app).is_empty(), "the caster took the bolt's damage as well as its launch");
}

/// A bolt still inside its caster neither throws nor hurts him: it has not
/// left him yet.
#[test]
fn a_bolt_inside_its_caster_carries_nobody() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    put_bolt(&mut app, ae::Vec2::ZERO);
    app.update();
    assert_eq!(app.world().get::<ae::BodyKinematics>(caster).unwrap().vel, ae::Vec2::ZERO, "the firing frame threw him");
    assert_eq!(bolts(&mut app).len(), 1, "the firing frame spent the bolt");
}

/// A foe takes the hit instead. A bolt that only hit its caster would be a
/// recovery with no offence.
#[test]
fn the_bolt_damages_somebody_else_and_is_spent() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    let rival = fighter(&mut app, 0, 400.0);
    fire(&mut app, caster);
    put_bolt(&mut app, ae::Vec2::new(400.0, 0.0));
    app.update();
    let hits = hits(&mut app);
    assert_eq!(hits.len(), 1, "the bolt reached a foe and did nothing");
    assert_eq!(hits[0], params().hit_s, "the bolt's hit is live for {} s, not its authored time", hits[0]);
    assert!(bolts(&mut app).is_empty(), "the bolt was not spent");
    assert!(
        app.world().get::<ae::BodyKinematics>(rival).unwrap().vel.length() < 1.0,
        "the foe was launched directly rather than through the damage box"
    );
}

#[test]
fn the_bolt_fades_on_its_own_clock() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    for _ in 0..(2.0 / DT) as usize + 4 {
        app.update();
    }
    assert!(bolts(&mut app).is_empty(), "the bolt outlived its lifetime");
}

/// A body that cannot name what it spawns fires nothing: nobody could steer
/// the bolt or be carried by it.
#[test]
fn a_caster_with_no_identity_fires_nothing() {
    let mut app = app();
    let nameless = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    fire(&mut app, nameless);
    assert!(bolts(&mut app).is_empty());
}

/// Caster and rival in one bolt: the result is the offensive hit in either spawn
/// order. A rival beats the caster by design: the jacket is what the bolt does
/// when it finds nobody else, and the choice must not depend on query order,
/// which a rollback resimulation does not keep.
#[test]
fn a_bolt_touching_both_prefers_the_rival_in_either_spawn_order() {
    let outcome = |reversed: bool| -> (bool, usize) {
        let mut app = app();
        let (caster, _) = if reversed {
            let rival = fighter(&mut app, 0, 0.0);
            (fighter(&mut app, 1, 300.0), rival)
        } else {
            let caster = fighter(&mut app, 1, 300.0);
            (caster, fighter(&mut app, 0, 0.0))
        };
        fire(&mut app, caster);
        // Clear of its caster, then on top of both.
        for _ in 0..10 {
            app.update();
        }
        let world = app.world_mut();
        world.get_mut::<ae::BodyKinematics>(caster).unwrap().pos = ae::Vec2::ZERO;
        *world.get_mut::<ae::CenteredAabb>(caster).unwrap() = ae::CenteredAabb::new(ae::Vec2::ZERO, HALF);
        put_bolt(&mut app, ae::Vec2::ZERO);
        app.update();
        let launched = app.world().get::<ae::BodyKinematics>(caster).unwrap().vel.length() > 1.0;
        (launched, hits(&mut app).len())
    };
    let forward = outcome(false);
    let backward = outcome(true);
    assert_eq!(forward, backward, "the bolt did {forward:?} in one spawn order and {backward:?} in the other");
    assert_eq!(forward, (false, 1), "a bolt touching a rival AND its caster must take the rival");
}

/// The bolt draws a trail while it flies. The caster steers it, so without a
/// visible path the move is unusable.
///
/// The test asserts the interval, not only the presence. A trail every tick is
/// sixty effect requests a second; the authored `trail_every_s` keeps the path
/// readable without flooding the channel.
#[test]
fn the_bolt_marks_its_path_on_the_authored_interval_rather_than_every_tick() {
    let mut app = app();
    let caster = fighter(&mut app, 0, 0.0);
    fire(&mut app, caster);
    // One cursor across the whole flight — a fresh one per tick re-reads the
    // double buffer and counts every mark twice.
    let mut seen = bevy::ecs::message::MessageCursor::<ambition_platformer2d::vfx::vfx::VfxInRoom>::default();
    let mut marks = 0usize;
    // 30 ticks at 1/60s = 0.5s of flight against a 0.05s interval.
    for _ in 0..30 {
        app.update();
        let messages = app.world().resource::<Messages<ambition_platformer2d::vfx::vfx::VfxInRoom>>();
        marks += seen
            .read(messages)
            .filter(|m| matches!(m.vfx, ambition_platformer2d::vfx::vfx::VfxMessage::Effect { .. }))
            .count();
    }
    // 0.5s / 0.05s = 10, with a tick of slack either way.
    assert!(
        (8..=12).contains(&marks),
        "the bolt drew {marks} marks in half a second against an authored 0.05s interval — under eight is a \
         path the player cannot follow, over twelve means the interval is being ignored"
    );
}

/// The bolt built from its module file flies and hits as the linked one does.
#[test]
fn the_bolt_from_its_module_file_flies_as_the_linked_one_does() {
    let run = |road: Road| {
        let mut app = app_with(road);
        let caster = fighter(&mut app, 1, 0.0);
        fighter(&mut app, 0, 200.0);
        fire(&mut app, caster);
        hold(&mut app, caster, ae::Vec2::new(0.0, -1.0));
        let mut path = Vec::new();
        for _ in 0..20 {
            app.update();
            path.push(bolts(&mut app).first().map(|(_, at)| *at));
        }
        put_bolt(&mut app, ae::Vec2::new(200.0, 0.0));
        app.update();
        let entry = runner_of(&app, BOLT);
        (path, hits(&mut app), entry)
    };
    let (linked_path, linked_hits, linked) = run(Road::Linked);
    let (loaded_path, loaded_hits, loaded) = run(Road::Loaded);
    assert!(loaded.starts_with("Loaded") && !linked.starts_with("Loaded"), "{linked} / {loaded}");
    assert_eq!(loaded_path, linked_path);
    assert_eq!((loaded_hits.len(), linked_hits.len()), (1, 1));
}
