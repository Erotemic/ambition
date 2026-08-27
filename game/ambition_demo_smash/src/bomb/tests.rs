//! ⛔⛔ BOTH REASONS, AND BOTH REFUSALS. Jon's rule is *"4 seconds or if it hits
//! something with enough velocity, whichever comes first"* — four claims, and a
//! suite that only proved the fuse would pass against a bomb with no impact rule
//! at all.

use super::*;
use ambition_platformer2d::item::{GroundItem, ItemCustody, SettledItem};

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<ambition_platformer2d::vfx::EffectRequest>();
    let mut time = app.world_mut().resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    app.add_systems(Update, burn_fuses_and_answer_impacts);
    app
}

fn a_bomb(app: &mut App, fuse_s: f32, last_speed: f32, vel: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            GroundItem {
                spec: ambition_platformer2d::characters::brain::held_item_by_id("polygon_bomb")
                    .expect("polygon_bomb is a registered held item"),
                pos: ae::Vec2::ZERO,
                vel,
                half_extent: ae::Vec2::splat(8.0),
            },
            LiveBomb {
                fuse_s,
                damage: 12,
                blast_radius: 60.0,
                impact_speed: 300.0,
                last_speed,
            },
        ))
        .id()
}

/// Run `ticks` frames, counting blasts as they happen.
///
/// ⛔⛔ DRAINED EVERY TICK, NOT AT THE END. Bevy messages are double-buffered
/// and `add_message` installs the cleanup that drops them after two frames — so
/// a test that ran ten ticks and then looked found nothing, and read as "the
/// fuse does not go off" when the fuse had gone off on tick six exactly as
/// authored.
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
    let bomb = a_bomb(&mut app, 0.1, 0.0, ae::Vec2::ZERO);
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
    let bomb = a_bomb(&mut app, 4.0, 520.0, ae::Vec2::ZERO);
    app.world_mut().entity_mut(bomb).insert(SettledItem);
    assert_eq!(
        run(&mut app, 1),
        1,
        "a bomb that hits something at 520 px/s must not wait out its fuse"
    );
}

/// ⛔ THE PAIRED REFUSAL. A bomb that settles GENTLY is a bomb somebody put
/// down, and it keeps its fuse — without this arm, "detonate on impact" and
/// "detonate the moment it lands" are the same test.
#[test]
fn a_gentle_landing_keeps_its_fuse() {
    let mut app = app();
    let bomb = a_bomb(&mut app, 4.0, 40.0, ae::Vec2::ZERO);
    app.world_mut().entity_mut(bomb).insert(SettledItem);
    assert_eq!(
        run(&mut app, 1),
        0,
        "40 px/s is a bomb being placed, not a bomb being thrown"
    );
    assert!(app.world().get_entity(bomb).is_ok());
}

/// ⛔ AND A FAST BOMB THAT HAS NOT HIT ANYTHING KEEPS FLYING. Speed alone is not
/// an impact; without this arm the rule could be "go off once you are fast",
/// which detonates every throw at the thrower's hand.
#[test]
fn speed_without_contact_is_not_an_impact() {
    let mut app = app();
    let bomb = a_bomb(&mut app, 4.0, 900.0, ae::Vec2::new(900.0, 0.0));
    assert_eq!(run(&mut app, 1), 0);
    assert!(app.world().get_entity(bomb).is_ok());
}

/// ⛔⛔ A CARRIED BOMB STILL BURNS, which is the whole tension of holding one —
/// and it must not detonate on IMPACT while it is in a hand, or every pickup
/// would set it off.
#[test]
fn a_carried_bomb_burns_but_cannot_be_set_off_by_an_impact() {
    let mut app = app();
    let bomb = a_bomb(&mut app, 0.05, 900.0, ae::Vec2::ZERO);
    let holder = app.world_mut().spawn_empty().id();
    app.world_mut()
        .entity_mut(bomb)
        .insert((SettledItem, ItemCustody::Held { holder }));
    assert_eq!(run(&mut app, 1), 0, "picking a bomb up must not detonate it");
    assert_eq!(
        run(&mut app, 5),
        1,
        "…but the fuse keeps burning in your hand"
    );
}
