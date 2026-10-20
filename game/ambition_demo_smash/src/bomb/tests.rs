//! The rule is "4 seconds or a hit with enough velocity, whichever comes
//! first". The tests prove both triggers and both refusals, so a bomb with no
//! impact rule fails. The bomb comes from its module on the module road.

use bevy::ecs::message::Messages;
use bevy::prelude::*;

use crate::module_road::{app_on, runner_of, Road};
use ambition_demo_smash_modules::bomb::BOMB;
use ambition_platformer2d::abilities::module_entity::ModuleEntity;
use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::smash_bomb::{DropBombParams, DROP_BOMB};
use ambition_platformer2d::item::{GroundItem, ItemCustody, SettledItem};
use ambition_platformer2d::platformer::sim_id::SimId;

fn params(fuse_s: f32) -> DropBombParams {
    DropBombParams {
        item_id: "polygon_bomb".to_string(),
        fuse_s,
        damage: 12,
        blast_radius: 60.0,
        impact_speed: 300.0,
        half_extents: (8.0, 8.0),
        offset: (0.0, 0.0),
        // Not the shipped 2.0 and 0.08: see
        // `the_blast_throws_with_the_bombs_knockback`.
        knockback: 1.35,
        blast_s: 0.3,
    }
}

/// A bomb a fighter dropped through the module on `road`, travelling at
/// `vel`; the tick that drops it has run.
fn a_bomb_on(road: Road, fuse_s: f32, vel: ae::Vec2) -> (App, Entity) {
    let mut app = app_on(road, ambition_demo_smash_modules::bomb::module);
    let dropper = app.world_mut().spawn((ae::BodyKinematics::default(), SimId::player_slot(0))).id();
    app.world_mut().write_message(ActorActionMessage {
        actor: dropper,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(DROP_BOMB.to_string()),
            params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params(fuse_s))
                .expect("bomb params serialize"),
        },
        move_instance: None,
    });
    app.update();
    let bomb = app
        .world_mut()
        .query::<(Entity, &ModuleEntity)>()
        .iter(app.world())
        .find(|(_, entity)| entity.kind == BOMB)
        .map(|(bomb, _)| bomb)
        .expect("the press dropped a bomb");
    app.world_mut().get_mut::<GroundItem>(bomb).expect("the bomb is an item").vel = vel;
    (app, bomb)
}

fn a_bomb(fuse_s: f32, vel: ae::Vec2) -> (App, Entity) {
    a_bomb_on(Road::Linked, fuse_s, vel)
}

/// Run `ticks` frames, collecting each blast as it happens.
///
/// Drained every tick, not at the end. Bevy messages are double-buffered and
/// dropped after two frames, so a check after ten ticks misses a blast on tick
/// six.
fn blasts_of(app: &mut App, ticks: usize) -> Vec<ambition_platformer2d::vfx::DamageBoxEffect> {
    let mut blasts = Vec::new();
    for _ in 0..ticks {
        app.update();
        blasts.extend(
            app.world_mut()
                .resource_mut::<Messages<ambition_platformer2d::vfx::EffectRequest>>()
                .drain()
                .filter_map(|request| match request.effect {
                    ambition_platformer2d::vfx::Effect::DamageBox(blast) => Some(blast),
                    _ => None,
                }),
        );
    }
    blasts
}

fn run(app: &mut App, ticks: usize) -> usize {
    blasts_of(app, ticks).len()
}

/// The blast throws with the knockback its move authored and is live for the
/// time it authored, not ruleset constants, and it takes no side.
#[test]
fn the_blast_throws_with_the_bombs_knockback() {
    let (mut app, _) = a_bomb(0.05, ae::Vec2::ZERO);
    let thrown: Vec<_> = blasts_of(&mut app, 10)
        .into_iter()
        .map(|blast| (blast.knockback, blast.lifetime_s, blast.faction))
        .collect();
    assert_eq!(
        thrown,
        vec![(1.35, 0.3, ambition_platformer2d::vfx::HitSide::Environment)],
        "the blast did not throw with the bomb's authored knockback and live time, as a hazard"
    );
}

#[test]
fn the_fuse_runs_out_and_the_bomb_goes_off() {
    // A tenth of a second, and nothing has hit it.
    let (mut app, bomb) = a_bomb(0.1, ae::Vec2::ZERO);
    assert_eq!(run(&mut app, 5), 0, "it must not go off early");
    assert_eq!(run(&mut app, 5), 1, "the fuse must go off");
    assert!(app.world().get_entity(bomb).is_err(), "…and take the object with it");
}

#[test]
fn a_hard_impact_goes_off_before_the_fuse_does() {
    // Four whole seconds of fuse left, and it just settled hard.
    let (mut app, bomb) = a_bomb(4.0, ae::Vec2::ZERO);
    // The speed comes from the settle, published by the step that zeroed the
    // velocity.
    app.world_mut().entity_mut(bomb).insert(SettledItem { impact_speed: 520.0 });
    assert_eq!(run(&mut app, 1), 1, "a bomb that hits something at 520 px/s must not wait out its fuse");
}

/// The paired refusal. A bomb that settles gently keeps its fuse; without this,
/// "detonate on impact" and "detonate on landing" are the same test.
#[test]
fn a_gentle_landing_keeps_its_fuse() {
    let (mut app, bomb) = a_bomb(4.0, ae::Vec2::ZERO);
    app.world_mut().entity_mut(bomb).insert(SettledItem { impact_speed: 40.0 });
    assert_eq!(run(&mut app, 1), 0, "40 px/s is a bomb being placed, not a bomb being thrown");
    assert!(app.world().get_entity(bomb).is_ok());
}

/// A fast bomb that has not hit anything keeps flying. Speed alone is not an
/// impact, or every throw would detonate at the thrower's hand.
#[test]
fn speed_without_contact_is_not_an_impact() {
    let (mut app, bomb) = a_bomb(4.0, ae::Vec2::new(900.0, 0.0));
    assert_eq!(run(&mut app, 1), 0);
    assert!(app.world().get_entity(bomb).is_ok());
}

/// A carried bomb still burns, but it must not detonate on impact while held,
/// or every pickup would set it off.
#[test]
fn a_carried_bomb_burns_but_cannot_be_set_off_by_an_impact() {
    let (mut app, bomb) = a_bomb(0.05, ae::Vec2::ZERO);
    let holder = app.world_mut().spawn_empty().id();
    app.world_mut()
        .entity_mut(bomb)
        .insert((SettledItem { impact_speed: 900.0 }, ItemCustody::Held { holder }));
    assert_eq!(run(&mut app, 1), 0, "picking a bomb up must not detonate it");
    assert_eq!(run(&mut app, 5), 1, "…but the fuse keeps burning in your hand");
}

/// A carried bomb goes off where its holder is, not where it was picked up.
///
/// `GroundItem::pos` stops updating when an item is picked up, because
/// `ground_item_physics` does not simulate a held item. The blast must use the
/// live position.
///
/// The holder here is a real body. The test above holds its bomb with an empty
/// entity, so `ItemWorldPos` falls back to the world position and both tests
/// would pass for the wrong reason.
#[test]
fn a_carried_bomb_blasts_where_its_holder_is() {
    let picked_up_at = ae::Vec2::new(-400.0, 0.0);
    let (mut app, bomb) = a_bomb(0.05, ae::Vec2::ZERO);
    let carried_to = ae::Vec2::new(500.0, -120.0);
    let holder = app
        .world_mut()
        .spawn(ae::BodyKinematics {
            pos: carried_to,
            size: ae::Vec2::new(28.0, 64.0),
            facing: 1.0,
            ..Default::default()
        })
        .id();
    {
        let mut entity = app.world_mut().entity_mut(bomb);
        entity.get_mut::<GroundItem>().expect("the bomb").pos = picked_up_at;
        entity.insert(ItemCustody::Held { holder });
    }
    let centers: Vec<_> = blasts_of(&mut app, 6).into_iter().map(|blast| blast.center).collect();
    assert_eq!(centers.len(), 1, "the fuse did not run out exactly once");
    let at = centers[0];
    assert!(
        at.distance(carried_to) < 60.0,
        "the bomb went off at {at:?}; its holder is at {carried_to:?} and it was picked up at {picked_up_at:?} — a \
         blast at the pickup spot is the world's stale copy of a position the world stopped maintaining"
    );
    assert!(
        at.distance(picked_up_at) > 100.0,
        "poison: the two positions are close enough that this arm would pass reading either one"
    );
}

/// A bomb that reaches a fighter at speed goes off.
///
/// Impact is not only against the collision world: `SettledItem` is published
/// only for a stop against blocks, so the rule also reads body contact. The
/// paired case is the same body contact at a gentle speed, which keeps its
/// fuse like a gentle landing.
#[test]
fn a_bomb_that_reaches_a_fighter_hard_goes_off_and_a_gentle_touch_does_not() {
    use ambition_platformer2d::item::ItemStruckBody;

    let blasts = |speed: f32, ticks: usize| {
        let (mut app, bomb) = a_bomb(4.0, ae::Vec2::ZERO);
        app.world_mut().entity_mut(bomb).insert((
            ItemCustody::InWorld,
            ItemStruckBody { impact_speed: speed, bodies: vec![Entity::PLACEHOLDER] },
        ));
        run(&mut app, ticks)
    };
    assert_eq!(
        blasts(520.0, 1),
        1,
        "a bomb that reached a fighter at 520px/s waited out its four-second fuse — a body is something, and \
         hitting it is an impact"
    );
    assert_eq!(
        blasts(40.0, 5),
        0,
        "a bomb that drifted into somebody at 40px/s went off — without this arm, 'reaching a fighter' and \
         'reaching a fighter HARD' are the same rule"
    );
}

/// The loaded file's bomb goes off as the linked one's: on the same tick, as
/// the same blast. Its runner is the loaded file.
#[test]
fn the_bomb_from_its_module_file_goes_off_as_the_linked_one_does() {
    let mut seen = Vec::new();
    for road in [Road::Linked, Road::Loaded] {
        let (mut app, _) = a_bomb_on(road, 0.1, ae::Vec2::ZERO);
        let runner = runner_of(&app, BOMB);
        assert_eq!(runner.starts_with("Loaded"), road == Road::Loaded, "{road:?}: {runner}");
        let per_tick: Vec<Vec<(ae::Vec2, f32, i32)>> = (0..12)
            .map(|_| blasts_of(&mut app, 1).into_iter().map(|b| (b.center, b.half_extent.x, b.damage)).collect())
            .collect();
        seen.push(per_tick);
    }
    assert_eq!(seen[1], seen[0], "the loaded bomb went off unlike the linked one");
    assert_eq!(seen[0].iter().flatten().count(), 1, "premise: the linked bomb went off once");
}
