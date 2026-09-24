//! The rate limit is the move. A bolt that snaps to the stick is a cursor, and
//! it removes the move's one cost: a turn spends distance.

use super::*;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::vfx::{Effect, EffectRequest};

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<EffectRequest>();
    // The trail channel. `steer_and_fly_bolts` writes it, and a system that
    // writes an unregistered message fails parameter validation and is
    // dropped without an error.
    app.add_message::<ambition_platformer2d::vfx::vfx::VfxMessage>();
    app.add_message::<ActorActionMessage>();
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    app.add_systems(Update, (fire_authored_bolts, steer_and_fly_bolts).chain());
    app
}

fn fighter(app: &mut App, seat: usize, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::new(x, 0.0),
                facing: 1.0,
                ..Default::default()
            },
            MatchSeat(seat),
            ActorControl(
                ambition_platformer2d::characters::actor::control::ActorControlFrame::neutral(),
            ),
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
    }
}

fn fire(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(STEERED_BOLT.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("bolt params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

/// Hold `stick` on `who` from now on.
fn hold(app: &mut App, who: Entity, stick: ae::Vec2) {
    let mut control = app.world_mut().get_mut::<ActorControl>(who).unwrap();
    // `undamped_locomotion` is what `steer_axis()` returns: the field that
    // survives the damped republish, so a rooted move can be aimed.
    control.0.undamped_locomotion = Some(ae::LocalAxes::new(stick.x, stick.y));
}

fn bolts(app: &mut App) -> Vec<SteeredBolt> {
    app.world_mut()
        .query::<&SteeredBolt>()
        .iter(app.world())
        .cloned()
        .collect()
}

#[test]
fn the_bolt_leaves_forward_and_belongs_to_the_seat_that_fired_it() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    let out = bolts(&mut app);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].owner_seat, 1);
    assert!(out[0].vel.x > 0.0, "it left backwards: {:?}", out[0].vel);
}

/// The turn is rate-limited, which is the cost of steering. At 220°/s one tick
/// turns under 4°; a bolt that snapped would point the other way at once.
#[test]
fn the_stick_turns_the_bolt_gradually_and_never_snaps_it() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    let opening = bolts(&mut app)[0].vel;

    // Ask for a hard reversal, and keep asking.
    hold(&mut app, caster, ae::Vec2::new(-1.0, 0.0));
    app.update();
    let after_one = bolts(&mut app)[0].vel;
    assert!(
        after_one.x > 0.0,
        "one tick of stick reversed the bolt, so it snaps rather than turns: {after_one:?}"
    );
    assert!(
        after_one.y.abs() > 0.0 || after_one.x < opening.x,
        "the bolt did not turn at all"
    );

    // The speed does not change when the bolt turns.
    assert!(
        (after_one.length() - opening.length()).abs() < 0.5,
        "turning changed the speed from {} to {}",
        opening.length(),
        after_one.length()
    );

    // Given enough ticks it does come around.
    for _ in 0..60 {
        app.update();
        if bolts(&mut app).is_empty() {
            break;
        }
    }
}

/// The thunder jacket: the bolt comes home and launches its caster.
#[test]
fn the_bolt_launches_its_caster_and_does_not_damage_him() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    // The bolt must leave him first. This is the move's rule: otherwise every
    // press would be an instant self-launch. Ten ticks at 300px/s is 50px,
    // outside his box.
    for _ in 0..10 {
        app.update();
    }
    assert!(
        bolts(&mut app)[0].clear_of_caster,
        "the bolt never got clear of its caster"
    );
    // Now bring it home.
    {
        let mut bolt = app
            .world_mut()
            .query::<&mut SteeredBolt>()
            .iter_mut(app.world_mut())
            .next()
            .expect("a bolt is out");
        bolt.pos = ae::Vec2::ZERO;
    }
    app.update();
    let kin = app.world().get::<ae::BodyKinematics>(caster).unwrap();
    assert!(
        kin.vel.length() > 300.0,
        "the bolt came home and did not throw him: {:?}",
        kin.vel
    );
    assert!(bolts(&mut app).is_empty(), "the bolt survived coming home");

    let messages = app.world().resource::<Messages<EffectRequest>>();
    let mut cursor = messages.get_cursor();
    assert!(
        !cursor.read(messages).any(|r| matches!(
            &r.effect,
            Effect::DamageBox(b) if b.name == Some("bolt")
        )),
        "the caster took the bolt's damage as well as its launch"
    );
}

/// A foe takes the hit instead. A bolt that only hit its caster would be a
/// recovery with no offence.
#[test]
fn the_bolt_damages_somebody_else_and_is_spent() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    let rival = fighter(&mut app, 0, 400.0);
    fire(&mut app, caster);
    {
        let mut bolt = app
            .world_mut()
            .query::<&mut SteeredBolt>()
            .iter_mut(app.world_mut())
            .next()
            .expect("a bolt is out");
        bolt.pos = ae::Vec2::new(400.0, 0.0);
    }
    app.update();
    let messages = app.world().resource::<Messages<EffectRequest>>();
    let mut cursor = messages.get_cursor();
    assert_eq!(
        cursor
            .read(messages)
            .filter(|r| matches!(&r.effect, Effect::DamageBox(b) if b.name == Some("bolt")))
            .count(),
        1,
        "the bolt reached a foe and did nothing"
    );
    assert!(bolts(&mut app).is_empty(), "the bolt was not spent");
    assert!(
        app.world()
            .get::<ae::BodyKinematics>(rival)
            .unwrap()
            .vel
            .length()
            < 1.0,
        "the foe was launched directly rather than through the damage box"
    );
}

#[test]
fn the_bolt_fades_on_its_own_clock() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    for _ in 0..(2.0 * 60.0) as usize + 4 {
        app.update();
    }
    assert!(bolts(&mut app).is_empty(), "the bolt outlived its lifetime");
}

#[test]
fn a_caster_with_no_seat_fires_nothing() {
    let mut app = app();
    let unseated = app
        .world_mut()
        .spawn(ae::BodyKinematics::default())
        .id();
    fire(&mut app, unseated);
    assert!(bolts(&mut app).is_empty());
}

/// Caster and rival in one bolt: the result is the offensive hit in either spawn
/// order.
///
/// If the loop stops on the first overlapping body, Bevy's iteration order
/// chooses between the thunder jacket and the hit on the rival. That choice is
/// not authored and is not stable across a rollback resimulation. A rival
/// beats the caster by design: the jacket is what the bolt does when it finds
/// nobody else.
#[test]
fn a_bolt_touching_both_prefers_the_rival_in_either_spawn_order() {
    let meeting = ae::Vec2::new(0.0, 0.0);

    let outcome = |reversed: bool| -> (bool, bool) {
        let mut app = app();
        let seats: [usize; 2] = if reversed { [1, 0] } else { [0, 1] };
        let mut caster = Entity::PLACEHOLDER;
        for &s in &seats {
            let e = fighter(&mut app, s, 0.0);
            if s == 0 {
                caster = e;
            }
        }
        app.world_mut().spawn(SteeredBolt {
            trail_vfx: "test_trail".to_string(),
            trail_every_s: 0.05,
            trail_in_s: 0.0,
            owner_seat: 0,
            pos: meeting,
            vel: ae::Vec2::new(300.0, 0.0),
            remaining_s: 1.0,
            turn_rate: 3.0,
            radius: 10.0,
            damage: 8,
            knockback: 90.0,
            self_launch: 640.0,
            // It has left him, so the jacket is legal this tick and both
            // outcomes are available.
            clear_of_caster: true,
        });
        app.update();
        let caster_launched = app
            .world()
            .get::<ae::BodyKinematics>(caster)
            .is_some_and(|k| k.vel.length() > 1.0);
        let damaged = !app
            .world()
            .resource::<bevy::ecs::message::Messages<EffectRequest>>()
            .is_empty();
        (caster_launched, damaged)
    };

    let forward = outcome(false);
    let backward = outcome(true);
    assert_eq!(
        forward, backward,
        "the bolt did {forward:?} in one spawn order and {backward:?} in the \
         other (caster_launched, damaged) — iteration order is choosing between \
         a recovery and an attack"
    );
    assert_eq!(
        forward,
        (false, true),
        "a bolt touching a rival AND its caster must take the rival: the jacket \
         is what it does when it finds nobody else"
    );
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
    let mut seen =
        bevy::ecs::message::MessageCursor::<ambition_platformer2d::vfx::vfx::VfxMessage>::default();
    let mut marks = 0usize;
    // 30 ticks at 1/60s = 0.5s of flight against a 0.05s interval.
    for _ in 0..30 {
        app.update();
        let messages = app
            .world()
            .resource::<Messages<ambition_platformer2d::vfx::vfx::VfxMessage>>();
        marks += seen
            .read(messages)
            .filter(|m| {
                matches!(
                    m,
                    ambition_platformer2d::vfx::vfx::VfxMessage::Effect { .. }
                )
            })
            .count();
    }

    assert!(
        marks > 0,
        "the bolt flew for half a second and drew nothing — the caster is \
         steering something invisible"
    );
    // 0.5s / 0.05s = 10, with a tick of slack either way.
    assert!(
        (8..=12).contains(&marks),
        "the bolt drew {marks} marks in half a second against an authored 0.05s \
         interval — under eight is a path the player cannot follow, over twelve \
         means the interval is being ignored and one move is flooding the cue \
         channel"
    );
}
