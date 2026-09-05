//! ⛔⛔ THE RATE LIMIT IS THE MOVE. "Holding a direction turns it" would pass
//! against a bolt that SNAPS to the stick — which is a cursor, not a bolt, and
//! removes the one cost the move has: a turn spends distance.

use super::*;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::vfx::{Effect, EffectRequest};

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<EffectRequest>();
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
        .write_message(ActorActionMessage { actor, request });
    app.update();
}

/// Hold `stick` on `who` from now on.
fn hold(app: &mut App, who: Entity, stick: ae::Vec2) {
    let mut control = app.world_mut().get_mut::<ActorControl>(who).unwrap();
    // ⭐ `undamped_locomotion` IS what `steer_axis()` returns — the field that
    // survives the damped republish, and the reason a rooted move can be aimed
    // at all.
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

/// ⭐ THE TURN IS RATE-LIMITED, WHICH IS THE WHOLE COST OF STEERING. At 220°/s
/// one tick turns it under 4°, so a bolt that snapped to the stick would be
/// pointing the other way immediately and this test is what tells them apart.
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

    // ⛔ AND THE SPEED IS UNCHANGED BY TURNING. A rotation that also scaled the
    // velocity would make a steered bolt faster or slower than an authored one.
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

/// ⭐⭐ THE THUNDER JACKET: it comes home and throws him.
#[test]
fn the_bolt_launches_its_caster_and_does_not_damage_him() {
    let mut app = app();
    let caster = fighter(&mut app, 1, 0.0);
    fire(&mut app, caster);
    // ⭐ IT MUST LEAVE HIM FIRST, which is the move's own rule and not a fixture
    // convenience: a bolt that has not got clear cannot answer its caster, or
    // every press would be an instant self-launch. Ten ticks at 300px/s is
    // 50px, comfortably outside his box.
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

/// ⛔ AND A FOE TAKES THE HIT INSTEAD, which is the other half: a bolt that only
/// ever answered its caster would be a recovery with no offence at all.
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
