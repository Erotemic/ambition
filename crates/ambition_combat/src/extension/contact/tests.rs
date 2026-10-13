//! The contact route in a tiny App: a native fixture module bound to a brain
//! key stomps through the real host, `queue_body_contacts` finds the touches,
//! and the bounce and the defeat are lowered before anything else reads the
//! bodies.

use super::*;
use ae::AabbExt as _;
use ambition_characters::actor::Health;
use ambition_combat_port::{BodyDefeat, ContactBounce};
use ambition_extension_host::ExtensionHostPlugin;
use ambition_extension_sdk::{
    CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey,
    TriggerBinding,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const WALKER: &str = "test_walker";
const BOUNCE: f32 = 460.0;

/// The fixture's rule: a toucher whose feet are on the walker's head and
/// that is not rising stomps it. Each stomper bounces, and the walker is
/// defeated once.
fn stomp(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let contact = inv.trigger::<BodyContactPort>()?.clone();
    let head = contact.body.position[1] - contact.body.size[1] * 0.5;
    let mut stomped = false;
    for (index, toucher) in contact.touchers.iter().enumerate() {
        let feet = toucher.position[1] + toucher.size[1] * 0.5;
        if toucher.velocity[1] >= 0.0 && feet <= head + 8.0 {
            inv.submit::<ContactBouncePort>(ContactBounce { toucher: index as u32, speed: BOUNCE, pogo_cue: true })?;
            stomped = true;
        }
    }
    if stomped {
        inv.submit::<BodyDefeatPort>(BodyDefeat)?;
    }
    Ok(())
}

fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0))
        .add_message::<ambition_sfx::OwnedSfxMessage>();
    install_for_body_contact(&mut app);
    app.add_extension_module(ModuleDescriptor {
        key: ModuleKey::new("test", "stomp"),
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
        schemas: vec![],
        entries: vec![EntryDescriptor {
            key: "contact".into(),
            phase: BODY_CONTACT,
            trigger: TriggerBinding { port: BodyContactPort::KEY, selector: WALKER.into() },
            reads: vec![],
            writes: vec![],
            requests: vec![ContactBouncePort::KEY, BodyDefeatPort::KEY],
            after: vec![],
            limits: Limits { max_requests: 4 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(stomp),
        }],
    });
    app.finish();
    app
}

fn walker(app: &mut App, brain: &str, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ActorConfig {
                tuning: Default::default(),
                brain: CharacterBrain::Custom(brain.into()),
                preserves_mirror_symmetry: false,
            },
            ae::BodyKinematics { pos: at, size: ae::Vec2::new(40.0, 30.0), ..Default::default() },
            BodyHealth::new(Health::new(3)),
            SimId::placement(&format!("walker_{}", at.x)),
        ))
        .id()
}

fn player(app: &mut App, id: &str, at: ae::Vec2, vel: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            ae::BodyKinematics { pos: at, vel, size: ae::Vec2::new(24.0, 44.0), ..Default::default() },
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
            SimId::placement(id),
        ))
        .id()
}

/// Where a player stands on a walker at `x`: feet on its head.
fn on_head(x: f32) -> ae::Vec2 {
    ae::Vec2::new(x, 900.0 - 15.0 - 22.0)
}

/// ⭐ A CONTACT RULE DECIDES WHAT A TOUCH DOES, BEFORE THE CONTACT DAMAGE. A
/// player falling onto a bound walker's head bounces and the walker is
/// defeated (health zero, then gone). A player beside another walker is a
/// toucher too, and the rule leaves it alone. A walker whose brain no entry
/// binds is never asked. A walker nobody touches is never asked.
#[test]
fn a_bound_body_decides_what_a_touch_does() {
    let mut app = app();
    let stomped = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
    let beside = walker(&mut app, WALKER, ae::Vec2::new(400.0, 900.0));
    let unbound = walker(&mut app, "someone_else", ae::Vec2::new(700.0, 900.0));
    let alone = walker(&mut app, WALKER, ae::Vec2::new(1000.0, 900.0));
    let stomper = player(&mut app, "p1", on_head(100.0), ae::Vec2::new(30.0, 200.0));
    let side = player(&mut app, "p2", ae::Vec2::new(400.0 - 30.0, 900.0), ae::Vec2::new(200.0, 0.0));
    let on_unbound = player(&mut app, "p3", on_head(700.0), ae::Vec2::new(0.0, 200.0));
    app.world_mut().run_schedule(Sim);

    assert!(app.world().get_entity(stomped).is_err(), "the stomped walker is gone");
    let vel = app.world().get::<ae::BodyKinematics>(stomper).unwrap().vel;
    assert_eq!(vel, ae::Vec2::new(30.0, -BOUNCE), "the stomper bounces up and keeps its sideways speed");
    let pogo = app.world().resource::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>().len();
    assert_eq!(pogo, 1, "one pogo cue, for the one bounce");

    assert!(app.world().get::<BodyHealth>(beside).is_some_and(|h| h.alive()), "a side touch is left to the contact damage");
    assert_eq!(app.world().get::<ae::BodyKinematics>(side).unwrap().vel, ae::Vec2::new(200.0, 0.0));
    assert!(app.world().get::<BodyHealth>(unbound).is_some_and(|h| h.alive()), "a brain no entry binds is not asked");
    assert_eq!(app.world().get::<ae::BodyKinematics>(on_unbound).unwrap().vel, ae::Vec2::new(0.0, 200.0));
    assert!(app.world().get::<BodyHealth>(alone).is_some_and(|h| h.alive()));
    let table: Vec<(Entity, usize)> =
        app.world().resource::<ContactTouchers>().0.iter().map(|(body, _, touchers)| (*body, touchers.len())).collect();
    assert_eq!(table, vec![(stomped, 1), (beside, 1)], "the touched bound bodies, in identity order, and nobody else");
}

/// A body stands on a walker's head with no overlap: it touches it. One
/// pixel above, it does not.
#[test]
fn a_body_that_stands_on_a_head_touches_it_and_one_above_does_not() {
    let body = ae::Aabb::new(ae::Vec2::new(0.0, 100.0), ae::Vec2::new(20.0, 15.0));
    let standing = ae::Aabb::new(ae::Vec2::new(5.0, 100.0 - 15.0 - 22.0), ae::Vec2::new(12.0, 22.0));
    assert!(touches(body, standing));
    assert!(!touches(body, standing.translated(ae::Vec2::new(0.0, -1.0))));
    assert!(!touches(body, standing.translated(ae::Vec2::new(40.0, 0.0))), "beside it, apart");
}

/// ⛔ A TOUCH IS IN ONE LIVE ROOM. A player of another live room at the same
/// place is not a toucher; the control is the same player in the walker's
/// own room, which stomps it.
#[test]
fn a_player_of_another_live_room_does_not_touch() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    for same_room in [true, false] {
        let mut app = app();
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for room in rooms {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        let body = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
        app.world_mut().entity_mut(body).insert(InRoomInstance(rooms[0]));
        let stomper = player(&mut app, "p1", on_head(100.0), ae::Vec2::new(0.0, 200.0));
        app.world_mut().entity_mut(stomper).insert(InRoomInstance(rooms[if same_room { 0 } else { 1 }]));
        app.world_mut().run_schedule(Sim);
        assert_eq!(
            app.world().get_entity(body).is_err(),
            same_room,
            "same room {same_room}: the walker is stomped only by a player of its own room"
        );
    }
}

/// ⛔ A TOUCH IS BETWEEN THE BOXES THE BODIES COLLIDE WITH. A player in
/// sideways gravity (its last step's DOWN is +X) lies along its wall: its
/// 24 x 44 body is a 44 x 24 box. Beside the walker, that box meets the
/// walker's side and the level box does not. The control is the same player
/// with no turned step, which does not touch. The module is given the turned
/// box too.
#[test]
fn a_touch_is_between_the_boxes_the_bodies_collide_with() {
    for turned in [true, false] {
        let mut app = app();
        let body = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
        // The walker's side is at x = 120: the level box (half 12) stops at
        // 128, the turned box (half 22) at 118.
        let beside = player(&mut app, "p1", ae::Vec2::new(140.0, 900.0), ae::Vec2::ZERO);
        if turned {
            let kin = *app.world().get::<ae::BodyKinematics>(beside).unwrap();
            app.world_mut().entity_mut(beside).insert(ae::SweepSample::at_rest(kin, ae::Vec2::X));
        }
        app.world_mut().run_schedule(Sim);
        let table: Vec<(Entity, Vec<Entity>)> =
            app.world().resource::<ContactTouchers>().0.iter().map(|(body, _, touchers)| (*body, touchers.clone())).collect();
        let expected = if turned { vec![(body, vec![beside])] } else { vec![] };
        assert_eq!(table, expected, "turned {turned}: the player touches the walker only with its turned box");
    }
    let kin = ae::BodyKinematics { pos: ae::Vec2::new(140.0, 900.0), size: ae::Vec2::new(24.0, 44.0), ..Default::default() };
    let step = ae::SweepSample::at_rest(kin, ae::Vec2::X);
    assert_eq!(contact_body(&kin, Some(&step)).size, [44.0, 24.0], "the module is given the box the body collides with");
    assert_eq!(contact_body(&kin, None).size, [24.0, 44.0], "with no step, the level box");
}

/// ⛔ A BOUNCE IS AGAINST THE TOUCHER'S OWN GRAVITY. Under reversed gravity
/// (DOWN is -Y), a toucher the rule bounces goes toward +Y, away from its own
/// floor; it went toward -Y, the screen's up (review 2026-10-12). The control
/// is the same stomp under screen gravity. A toucher with no resolved frame
/// is refused, not bounced against the screen, and the defeat still lands.
#[test]
fn a_bounce_is_against_the_touchers_own_gravity() {
    use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
    let falling = ae::Vec2::new(30.0, 200.0);
    for (name, down, expected) in [
        ("reversed", Some(-ae::Vec2::Y), ae::Vec2::new(30.0, BOUNCE)),
        ("screen", Some(ae::Vec2::Y), ae::Vec2::new(30.0, -BOUNCE)),
        ("no frame", None, falling),
    ] {
        let mut app = app();
        let stomped = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
        let toucher = player(&mut app, "p1", on_head(100.0), falling);
        match down {
            Some(down) => {
                let frame = ae::MotionFrame::from_direction(down, 0.0);
                app.world_mut().get_mut::<ResolvedMotionFrame>(toucher).unwrap().publish_resolved_frame(frame);
            }
            None => {
                app.world_mut().entity_mut(toucher).remove::<ResolvedMotionFrame>();
            }
        }
        app.world_mut().run_schedule(Sim);
        assert!(app.world().get_entity(stomped).is_err(), "{name}: premise, the rule stomped the walker");
        let vel = app.world().get::<ae::BodyKinematics>(toucher).unwrap().vel;
        assert_eq!(vel, expected, "{name}: the toucher's velocity after the bounce");
    }
}
