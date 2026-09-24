//! The rule is "4 seconds or a hit with enough velocity, whichever comes first".
//! The tests prove both triggers and both refusals, so a bomb with no impact
//! rule fails.

use super::*;
use ambition_platformer2d::item::{GroundItem, ItemCustody, SettledItem};

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<ambition_platformer2d::vfx::EffectRequest>();
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    app.add_systems(Update, burn_fuses_and_answer_impacts);
    app
}

fn a_bomb(app: &mut App, fuse_s: f32, vel: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            GroundItem::released(
                ambition_platformer2d::characters::brain::held_item_by_id("polygon_bomb")
                    .expect("polygon_bomb is a registered held item"),
                ae::Vec2::ZERO,
                vel,
                ae::Vec2::splat(8.0),
            ),
            LiveBomb {
                fuse_s,
                damage: 12,
                blast_radius: 60.0,
                impact_speed: 300.0,
            },
        ))
        .id()
}

/// Run `ticks` frames, counting blasts as they happen.
///
/// Drained every tick, not at the end. Bevy messages are double-buffered and
/// dropped after two frames, so a check after ten ticks misses a blast on tick
/// six.
fn run(app: &mut App, ticks: usize) -> usize {
    let mut blasts = 0;
    for _ in 0..ticks {
        app.update();
        blasts += app
            .world_mut()
            .resource_mut::<Messages<ambition_platformer2d::vfx::EffectRequest>>()
            .drain()
            .filter(|r| matches!(r.effect, ambition_platformer2d::vfx::Effect::DamageBox(_)))
            .count();
    }
    blasts
}

#[test]
fn the_fuse_runs_out_and_the_bomb_goes_off() {
    let mut app = app();
    // A tenth of a second left, and nothing has hit it.
    let bomb = a_bomb(&mut app, 0.1, ae::Vec2::ZERO);
    assert_eq!(run(&mut app, 5), 0, "it must not go off early");
    assert_eq!(run(&mut app, 5), 1, "the fuse must go off");
    assert!(
        app.world().get_entity(bomb).is_err(),
        "…and take the object with it"
    );
}

#[test]
fn a_hard_impact_goes_off_before_the_fuse_does() {
    let mut app = app();
    // Four whole seconds of fuse left, travelling hard, and it just settled.
    let bomb = a_bomb(&mut app, 4.0, ae::Vec2::ZERO);
    // The speed comes from the settle, published by the step that zeroed the velocity.
    app.world_mut().entity_mut(bomb).insert(SettledItem {
        impact_speed: 520.0,
    });
    assert_eq!(
        run(&mut app, 1),
        1,
        "a bomb that hits something at 520 px/s must not wait out its fuse"
    );
}

/// The paired refusal. A bomb that settles gently keeps its fuse; without this,
/// "detonate on impact" and "detonate on landing" are the same test.
#[test]
fn a_gentle_landing_keeps_its_fuse() {
    let mut app = app();
    let bomb = a_bomb(&mut app, 4.0, ae::Vec2::ZERO);
    app.world_mut()
        .entity_mut(bomb)
        .insert(SettledItem { impact_speed: 40.0 });
    assert_eq!(
        run(&mut app, 1),
        0,
        "40 px/s is a bomb being placed, not a bomb being thrown"
    );
    assert!(app.world().get_entity(bomb).is_ok());
}

/// A fast bomb that has not hit anything keeps flying. Speed alone is not an
/// impact, or every throw would detonate at the thrower's hand.
#[test]
fn speed_without_contact_is_not_an_impact() {
    let mut app = app();
    let bomb = a_bomb(&mut app, 4.0, ae::Vec2::new(900.0, 0.0));
    assert_eq!(run(&mut app, 1), 0);
    assert!(app.world().get_entity(bomb).is_ok());
}

/// A carried bomb still burns, but it must not detonate on impact while held,
/// or every pickup would set it off.
#[test]
fn a_carried_bomb_burns_but_cannot_be_set_off_by_an_impact() {
    let mut app = app();
    let bomb = a_bomb(&mut app, 0.05, ae::Vec2::ZERO);
    let holder = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(bomb).insert((
        SettledItem {
            impact_speed: 900.0,
        },
        ItemCustody::Held { holder },
    ));
    assert_eq!(
        run(&mut app, 1),
        0,
        "picking a bomb up must not detonate it"
    );
    assert_eq!(
        run(&mut app, 5),
        1,
        "…but the fuse keeps burning in your hand"
    );
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
    let mut app = app();
    let picked_up_at = ae::Vec2::new(-400.0, 0.0);
    let bomb = a_bomb(&mut app, 0.05, ae::Vec2::ZERO);
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
    let centers = blast_centers(&mut app, 6);
    assert_eq!(centers.len(), 1, "the fuse did not run out exactly once");
    let at = centers[0];
    assert!(
        at.distance(carried_to) < 60.0,
        "the bomb went off at {at:?}; its holder is at {carried_to:?} and it was \
         picked up at {picked_up_at:?} — a blast at the pickup spot is the world's \
         stale copy of a position the world stopped maintaining"
    );
    assert!(
        at.distance(picked_up_at) > 100.0,
        "poison: the two positions are close enough that this arm would pass \
         reading either one"
    );
}

/// Run `ticks` frames, collecting where each blast happened.
fn blast_centers(app: &mut App, ticks: usize) -> Vec<ae::Vec2> {
    let mut centers = Vec::new();
    for _ in 0..ticks {
        app.update();
        centers.extend(
            app.world_mut()
                .resource_mut::<Messages<ambition_platformer2d::vfx::EffectRequest>>()
                .drain()
                .filter_map(|request| match request.effect {
                    ambition_platformer2d::vfx::Effect::DamageBox(box_) => Some(box_.center),
                    _ => None,
                }),
        );
    }
    centers
}

/// A bomb that reaches a fighter at speed goes off.
///
/// Impact is not only against the collision world: `SettledItem` is published
/// only for a stop against blocks, so the rule also reads body contact.
///
/// One threshold, two surfaces. The bomb sets the speed for "hard" and the
/// collision authority says what was reached. The paired case is the same body
/// contact at a gentle speed, which keeps its fuse like a gentle landing.
#[test]
fn a_bomb_that_reaches_a_fighter_hard_goes_off_and_a_gentle_touch_does_not() {
    use ambition_platformer2d::item::ItemStruckBody;

    let blasts = |speed: f32, ticks: usize| {
        let mut app = app();
        let bomb = a_bomb(&mut app, 4.0, ae::Vec2::ZERO);
        app.world_mut().entity_mut(bomb).insert((
            ItemCustody::InWorld,
            ItemStruckBody {
                impact_speed: speed,
            },
        ));
        run(&mut app, ticks)
    };
    assert_eq!(
        blasts(520.0, 1),
        1,
        "a bomb that reached a fighter at 520px/s waited out its four-second \
         fuse — a body is something, and hitting it is an impact"
    );
    assert_eq!(
        blasts(40.0, 5),
        0,
        "a bomb that drifted into somebody at 40px/s went off — without this \
         arm, 'reaching a fighter' and 'reaching a fighter HARD' are the same \
         rule"
    );
}
