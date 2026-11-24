//! The seeker locks one foe and follows it: a foe above the line it leaves on
//! is reached, which a straight shot would miss, and a nearer foe behind the
//! caster is not the one it follows. It comes from its module on the module
//! road.

use bevy::ecs::message::Messages;
use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_demo_smash_modules::seeker::SEEKER_SHOT;
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::damage_box::DamageBoxRequest;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_seeker::{SeekerParams, SEEKER};
use ambition_platformer2d::platformer::sim_id::{SimId, SimIdCounter};

/// A fighter's half size.
const HALF: ae::Vec2 = ae::Vec2::new(14.0, 26.0);

/// A body in `slot` at `at`, facing right, on its own match team, as every
/// fighter of a free-for-all is: each is the other's foe.
fn fighter(app: &mut App, slot: u8, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics { pos: at, facing: 1.0, canonical_size: HALF * 2.0, ..Default::default() },
            ae::CenteredAabb::new(at, HALF),
            ambition_platformer2d::actor::ActorFaction::Player,
            ambition_platformer2d::combat::targeting::MatchTeam::new(format!("team_{slot}")),
            ambition_platformer2d::platformer::lifecycle::FeatureSimEntity,
            SimId::player_slot(slot),
            SimIdCounter::default(),
            ActorControl(ambition_platformer2d::characters::actor::control::ActorControlFrame::neutral()),
        ))
        .id()
}

fn params() -> SeekerParams {
    SeekerParams {
        speed: 260.0,
        turn_rate_deg: 150.0,
        lock_range: 560.0,
        lifetime_s: 2.4,
        trail_vfx: "test_trail".into(),
        trail_every_s: 0.07,
        damage: 7,
        radius: 10.0,
        knockback: 1.2,
        offset: (24.0, -12.0),
        hit_s: 0.3,
    }
}

fn fire(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(SEEKER.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params()).unwrap(),
    };
    app.world_mut().write_message(ActorActionMessage { action_interval: None, actor, request, move_instance: None });
    app.update();
}

fn shots(app: &mut App) -> Vec<ae::Vec2> {
    app.world_mut()
        .query::<&ModuleEntity>()
        .iter(app.world())
        .filter(|entity| entity.kind == SEEKER_SHOT)
        .map(|entity| entity.pos)
        .collect()
}

fn damage_this_tick(app: &App) -> Vec<i32> {
    let messages = app.world().resource::<Messages<DamageBoxRequest>>();
    messages.get_cursor().read(messages).map(|request| request.damage_box.damage).collect()
}

/// It follows the foe it locked, above its line and ahead of the caster,
/// past a nearer foe behind the caster, and reaches it: the hit is the
/// move's. On both roads.
#[test]
fn the_seeker_follows_the_foe_it_locked_and_reaches_it() {
    for road in [Road::Linked, Road::Loaded] {
        let mut app = app_on(road, ambition_demo_smash_modules::seeker::module);
        assert_eq!(runner_of(&app, SEEKER).starts_with("Loaded"), road == Road::Loaded);
        let caster = fighter(&mut app, 1, ae::Vec2::ZERO);
        fighter(&mut app, 2, ae::Vec2::new(-80.0, 0.0));
        let mark = ae::Vec2::new(260.0, -170.0);
        fighter(&mut app, 3, mark);
        fire(&mut app, caster);
        assert_eq!(shots(&mut app), vec![ae::Vec2::new(24.0, -12.0)], "{road:?}: the shot is not at the move's offset");
        let mut reached = None;
        for tick in 0..150 {
            app.update();
            let damage = damage_this_tick(&app);
            if !damage.is_empty() {
                reached = Some((tick, damage));
                break;
            }
            let Some(&at) = shots(&mut app).first() else {
                panic!("{road:?}: the shot ended at tick {tick} having reached nobody");
            };
            assert!(at.x > 0.0, "{road:?}: the shot turned back toward the foe behind its caster: {at:?}");
        }
        let (tick, damage) = reached.unwrap_or_else(|| panic!("{road:?}: the shot never reached its foe"));
        assert_eq!(damage, vec![7], "{road:?}: the hit at tick {tick} is not the move's");
        assert!(shots(&mut app).is_empty(), "{road:?}: the shot outlived its hit");
    }
}
