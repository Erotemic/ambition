//! The ownership claim is the one to guard. A mine answers only its placer; a
//! mine that any press sets off is a shared button, not a mine. The mine
//! comes from its module on the module road.

use bevy::ecs::message::Messages;
use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_demo_smash_modules::mine::MINE;
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_mine::{PlaceMineParams, PLACE_MINE};
use ambition_platformer2d::item::{GroundItem, ItemCustody};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::vfx::{Effect, EffectRequest};

fn app_with(road: Road) -> App {
    app_on(road, ambition_demo_smash_modules::mine::module)
}

fn app() -> App {
    app_with(Road::Linked)
}

/// A fighter who can place mines, at `x`.
fn a_fighter(app: &mut App, slot: u8, x: f32) -> Entity {
    app.world_mut()
        .spawn((ae::BodyKinematics { pos: ae::Vec2::new(x, 0.0), ..Default::default() }, SimId::player_slot(slot)))
        .id()
}

fn params() -> PlaceMineParams {
    PlaceMineParams {
        item_id: "polygon_mine".to_string(),
        arm_s: 1.2,
        damage: 10,
        blast_radius: 52.0,
        half_extents: (8.0, 8.0),
        offset: (-18.0, 14.0),
        // Not the shipped 1.7 and 0.08, so a blast on a Rust constant fails
        // `the_blast_is_authored_as_a_hazard_and_not_as_an_inert_side`.
        knockback: 1.45,
        blast_s: 0.3,
    }
}

/// Press the mine button as `actor`.
fn press(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(PLACE_MINE.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("place-mine params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

/// Every mine on the stage: who placed it, and how long it has lived.
fn mines(app: &mut App) -> Vec<(Option<SimId>, f32)> {
    let mut out: Vec<_> = app
        .world_mut()
        .query::<&ModuleEntity>()
        .iter(app.world())
        .filter(|entity| entity.kind == MINE)
        .map(|entity| (entity.spawner.clone(), entity.age_s))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// A blast as it was asked for.
struct Seen {
    center: ae::Vec2,
    faction: ambition_platformer2d::vfx::HitSide,
    knockback: f32,
    lifetime_s: f32,
}

/// The blasts written this frame.
fn blasts_now(app: &mut App) -> Vec<Seen> {
    let messages = app.world().resource::<Messages<EffectRequest>>();
    let mut cursor = messages.get_cursor();
    cursor
        .read(messages)
        .filter_map(|request| match &request.effect {
            Effect::DamageBox(b) => {
                Some(Seen { center: b.center, faction: b.faction, knockback: b.knockback, lifetime_s: b.lifetime_s })
            }
            _ => None,
        })
        .collect()
}

fn blasts(app: &mut App) -> Vec<ae::Vec2> {
    blasts_now(app).into_iter().map(|b| b.center).collect()
}

/// Spend `secs` of arming without pressing anything.
fn wait(app: &mut App, secs: f32) {
    for _ in 0..((secs * 60.0).ceil() as usize) {
        app.update();
    }
}

/// Assert the faction, not only that a damage box exists. A blast with
/// `HitSide::Neutral` is excluded from the body path by `apply_hitbox_damage`
/// and hurts nobody, but a request-shaped test still passes.
#[test]
fn the_blast_is_authored_as_a_hazard_and_not_as_an_inert_side() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 0, 0.0);
    press(&mut app, fighter);
    wait(&mut app, 1.5);
    press(&mut app, fighter);
    let found = blasts_now(&mut app);
    assert_eq!(found.len(), 1, "the armed mine did not answer at all");
    assert_eq!(found[0].knockback, params().knockback, "the blast throws with {}, not the move's", found[0].knockback);
    assert_eq!(found[0].lifetime_s, params().blast_s, "the blast is live for {} s, not the move's", found[0].lifetime_s);
    assert_eq!(
        found[0].faction,
        ambition_platformer2d::vfx::HitSide::Environment,
        "the mine's blast is authored {:?}, which the damage resolver does not put on the body path — it would \
         explode and hurt nobody",
        found[0].faction
    );
}

#[test]
fn a_press_with_nothing_out_places_a_mine_owned_by_the_presser() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    assert_eq!(mines(&mut app), vec![(Some(SimId::player_slot(1)), 0.0)]);
    assert!(blasts(&mut app).is_empty(), "placing is not detonating");
}

#[test]
fn a_press_while_the_mine_is_still_arming_does_nothing_at_all() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    wait(&mut app, 0.5);
    press(&mut app, fighter);
    // One mine, not two, and no blast. This guards two opposite bugs in the
    // same branch: a second placement and a detonation that ignores the
    // arming delay.
    let out = mines(&mut app);
    assert_eq!(out.len(), 1, "a second press must not plant a second mine");
    assert!(out[0].1 < params().arm_s, "still arming");
    assert!(blasts(&mut app).is_empty(), "an unarmed mine does not answer");
}

#[test]
fn a_press_once_armed_detonates_the_mine_and_takes_it_off_the_stage() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    wait(&mut app, 1.3);
    press(&mut app, fighter);
    assert_eq!(blasts(&mut app).len(), 1, "the armed mine answered");
    assert!(mines(&mut app).is_empty(), "and it is gone");
}

#[test]
fn a_mine_answers_its_own_placer_and_nobody_elses() {
    let mut app = app();
    let owner = a_fighter(&mut app, 1, 100.0);
    let rival = a_fighter(&mut app, 0, -100.0);
    press(&mut app, owner);
    wait(&mut app, 1.3);
    // The rival presses their own mine button while the owner's mine is armed.
    press(&mut app, rival);
    assert!(blasts(&mut app).is_empty(), "the rival must not be able to set off the owner's mine");
    // The rival got their own mine instead: the press was scoped, not
    // swallowed.
    let placers: Vec<_> = mines(&mut app).into_iter().map(|(placer, _)| placer).collect();
    assert_eq!(placers, vec![Some(SimId::player_slot(0)), Some(SimId::player_slot(1))], "two placers, two mines");
}

#[test]
fn the_blast_lands_where_the_mine_is_now_not_where_it_was_planted() {
    let mut app = app();
    let owner = a_fighter(&mut app, 1, 100.0);
    press(&mut app, owner);
    wait(&mut app, 1.3);
    // Somebody picked it up and ran. `GroundItem::pos` is the world's stale
    // copy; `ItemWorldPos` is the live answer, and the mine must use it.
    let thief = a_fighter(&mut app, 0, -400.0);
    let mine = app
        .world_mut()
        .query::<(Entity, &ModuleEntity)>()
        .iter(app.world())
        .find(|(_, entity)| entity.kind == MINE)
        .map(|(mine, _)| mine)
        .expect("a mine is out");
    app.world_mut().entity_mut(mine).insert(ItemCustody::Held { holder: thief });
    press(&mut app, owner);
    let centres = blasts(&mut app);
    assert_eq!(centres.len(), 1);
    assert!(centres[0].x < -300.0, "the blast followed the carrier to {:?}, not the planting spot", centres[0]);
}

#[test]
fn a_placer_with_no_identity_places_nothing() {
    let mut app = app();
    // No `SimId`. A mine with no nameable owner could never be detonated, so
    // it must not reach the stage.
    let anonymous = app.world_mut().spawn(ae::BodyKinematics::default()).id();
    press(&mut app, anonymous);
    assert!(mines(&mut app).is_empty());
}

#[test]
fn the_placed_mine_is_a_ground_item_somebody_could_pick_up() {
    let mut app = app();
    let fighter = a_fighter(&mut app, 1, 100.0);
    press(&mut app, fighter);
    let mut query = app.world_mut().query::<(&GroundItem, &ItemCustody)>();
    let (item, custody) = query.iter(app.world()).next().expect("the mine is an item");
    assert_eq!(item.spec.id.as_str(), "polygon_mine");
    assert_eq!(*custody, ItemCustody::InWorld);
    // Body-local offset, mirrored by facing: planted behind her, not inside
    // her.
    assert!(item.pos.x < 100.0, "planted behind the placer");
}

/// The loaded file's mine places, waits and goes off as the linked one's. Its
/// runner is the loaded file.
#[test]
fn the_mine_from_its_module_file_answers_as_the_linked_one_does() {
    let mut seen = Vec::new();
    for road in [Road::Linked, Road::Loaded] {
        let mut app = app_with(road);
        let runner = runner_of(&app, PLACE_MINE);
        assert_eq!(runner.starts_with("Loaded"), road == Road::Loaded, "{road:?}: {runner}");
        let fighter = a_fighter(&mut app, 1, 100.0);
        press(&mut app, fighter);
        wait(&mut app, 0.5);
        press(&mut app, fighter);
        let arming = (mines(&mut app).len(), blasts(&mut app));
        wait(&mut app, 1.0);
        press(&mut app, fighter);
        seen.push((arming, mines(&mut app).len(), blasts(&mut app)));
    }
    assert_eq!(seen[1], seen[0], "the loaded mine answered unlike the linked one");
    assert_eq!((seen[0].0 .0, seen[0].1, seen[0].2.len()), (1, 0, 1), "premise: the linked mine waited, then went off");
}
