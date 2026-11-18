//! The contact route in a tiny App: a native fixture module bound to a brain
//! key stomps through the real host, `queue_body_contacts` finds the touches,
//! and the bounce and the defeat are lowered before anything else reads the
//! bodies.

use super::*;
use crate::actor_tuning::ActorConfig;
use ambition_entity_catalog::placements::CharacterBrain;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
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
        let feet = toucher.body.position[1] + toucher.body.size[1] * 0.5;
        if toucher.body.velocity[1] >= 0.0 && feet <= head + 8.0 {
            inv.submit::<ContactBouncePort>(ContactBounce { toucher: index as u32, speed: BOUNCE, pogo_cue: true })?;
            stomped = true;
        }
    }
    if stomped {
        inv.submit::<BodyDefeatPort>(BodyDefeat)?;
    }
    Ok(())
}

fn app() -> App { app_with(stomp) }

fn app_with(run: fn(&mut Invocation<'_>) -> Result<(), Fault>) -> App {
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
        code: CodeIdentity::StaticNative { build: None, crate_name: "test".into(), version: "0".into() },
        schemas: vec![],
        entries: vec![EntryDescriptor {
            key: "contact".into(),
            phase: BODY_CONTACT,
            trigger: TriggerBinding { port: BodyContactPort::KEY, selector: WALKER.into() },
            reads: vec![],
            writes: vec![],
            requests: vec![ContactBouncePort::KEY, BodyDefeatPort::KEY, ambition_combat_port::HoldStatusPort::KEY],
            after: vec![],
            limits: Limits { max_requests: 4 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(run),
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
            ae::BodyKinematics { pos: at, canonical_size: ae::Vec2::new(40.0, 30.0), ..Default::default() },
            BodyHealth::new(Health::new(3)),
            SimId::placement(&format!("walker_{}", at.x)),
        ))
        .id()
}

fn player(app: &mut App, id: &str, at: ae::Vec2, vel: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            DrivingParticipant(PlayerSlot(0)),
            ae::BodyKinematics { pos: at, vel, canonical_size: ae::Vec2::new(24.0, 44.0), ..Default::default() },
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
    let touches = |a: ae::Aabb, b: ae::Aabb| !ae::CombatVolume::aabb(a).surface_contacts(&ae::CombatVolume::aabb(b), 0.0).is_empty();
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

/// Contact uses the physical body envelope, independent of its gravity reference.
#[test]
fn a_touch_is_between_the_boxes_the_bodies_collide_with() {
    for turned in [true, false] {
        let mut app = app();
        let body = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
        // The walker's side is at x = 120: the level box (half 12) stops at
        // 128, the turned box (half 22) at 118.
        let beside = player(&mut app, "p1", ae::Vec2::new(140.0, 900.0), ae::Vec2::ZERO);
        if turned {
            app.world_mut().get_mut::<ae::BodyKinematics>(beside).unwrap().material =
                ae::frame::BodyMaterial::default().turned(std::f64::consts::FRAC_PI_2).unwrap();
        }
        app.world_mut().run_schedule(Sim);
        let table: Vec<(Entity, Vec<Entity>)> =
            app.world().resource::<ContactTouchers>().0.iter().map(|(body, _, touchers)| (*body, touchers.clone())).collect();
        let expected = if turned { vec![(body, vec![beside])] } else { vec![] };
        assert_eq!(table, expected, "turned {turned}: the player touches the walker only with its turned box");
    }
    let kin = ae::BodyKinematics { pos: ae::Vec2::new(140.0, 900.0), canonical_size: ae::Vec2::new(24.0, 44.0), ..Default::default() };
    assert_eq!(contact_body(&kin).size, [24.0, 44.0]);
    let mut turned = kin;
    turned.material = turned.material.turned(std::f64::consts::FRAC_PI_2).unwrap();
    assert_eq!(contact_body(&turned).size, [44.0, 24.0]);
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

#[test]
fn contact_rules_follow_the_body_for_all_controller_pairs_and_status_bindings() {
    for recipient_driven in [false, true] {
        for toucher_driven in [false, true] {
            for brain in [WALKER, "unbound_brain"] {
                let mut app = app();
                let recipient = walker(&mut app, brain, ae::Vec2::new(100.0, 900.0));
                app.world_mut().entity_mut(recipient).insert(crate::extension::status::BodyStatuses(vec![WALKER.into()]));
                if recipient_driven { app.world_mut().entity_mut(recipient).insert(DrivingParticipant(PlayerSlot(1))); }
                let toucher = player(&mut app, "toucher", on_head(100.0), ae::Vec2::new(0.0, 200.0));
                app.world_mut().entity_mut(toucher).remove::<PlayerEntity>();
                if !toucher_driven { app.world_mut().entity_mut(toucher).remove::<DrivingParticipant>(); }
                app.world_mut().run_schedule(Sim);
                assert!(app.world().get_entity(recipient).is_err());
                assert_eq!(app.world().get::<ae::BodyKinematics>(toucher).unwrap().vel.y, -BOUNCE);
                assert_eq!(app.world().resource::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>().len(), 1,
                    "the brain and held status bind the same rule only once");
            }
        }
    }
}

#[test]
fn a_contact_sample_excludes_foreign_sessions_and_held_rooms() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        InRoomInstance, LiveRoomInstance, RoomInstanceRoot, RoomPace, SessionScopeId, SessionScopedEntity,
    };
    for (same_session, held) in [(false, false), (true, true), (true, false)] {
        let mut app = app();
        let room = LiveRoomInstance::ACTIVATION;
        app.world_mut().spawn((RoomInstanceRoot, room, RoomPace { held_until: if held { 10 } else { 0 }, clock_scale: 1.0 }));
        let recipient = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
        app.world_mut().entity_mut(recipient).insert((InRoomInstance(room), SessionScopedEntity(SessionScopeId(1))));
        let toucher = player(&mut app, "toucher", on_head(100.0), ae::Vec2::new(0.0, 200.0));
        app.world_mut().entity_mut(toucher).insert((InRoomInstance(room), SessionScopedEntity(SessionScopeId(if same_session { 1 } else { 2 }))));
        app.world_mut().run_schedule(Sim);
        assert_eq!(app.world().get_entity(recipient).is_err(), same_session && !held);
    }
}

#[test]
fn contact_budgets_keep_identity_order_and_bound_native_responses() {
    use ambition_platformer2d_world::scoped_rules::{DeclareRulesExt, RulesScope};
    let mut app = app();
    app.declare_rules(RulesScope::EveryRoom, ContactFactsLimits { max_touchers: 1, max_surfaces: 1, ..Default::default() });
    walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
    let later = player(&mut app, "b", on_head(100.0), ae::Vec2::new(0.0, 200.0));
    let first = player(&mut app, "a", on_head(100.0), ae::Vec2::new(0.0, 200.0));
    app.world_mut().run_schedule(Sim);
    assert_eq!(app.world().get::<ae::BodyKinematics>(first).unwrap().vel.y, -BOUNCE);
    assert_eq!(app.world().get::<ae::BodyKinematics>(later).unwrap().vel.y, 200.0);
}

#[test]
fn overlapping_envelopes_do_not_report_contact_outside_rotated_material() {
    let mut app = app();
    let recipient = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
    {
        let mut shape = app.world_mut().get_mut::<ae::BodyKinematics>(recipient).unwrap();
        shape.canonical_size = ae::Vec2::new(40.0, 2.0);
        shape.material = shape.material.turned(std::f64::consts::FRAC_PI_4).unwrap();
    }
    let toucher = player(&mut app, "corner", ae::Vec2::new(90.0, 910.0), ae::Vec2::ZERO);
    app.world_mut().get_mut::<ae::BodyKinematics>(toucher).unwrap().canonical_size = ae::Vec2::splat(2.0);
    let first = app.world().get::<ae::BodyKinematics>(recipient).unwrap().aabb();
    let second = app.world().get::<ae::BodyKinematics>(toucher).unwrap().aabb();
    assert!(first.strict_intersects(second));
    app.world_mut().run_schedule(Sim);
    assert!(app.world().resource::<ContactTouchers>().0.is_empty());
    assert!(app.world().get_entity(recipient).is_ok());
}

#[test]
fn a_carried_piece_does_not_leave_a_phantom_contact_behind_its_aperture() {
    use ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry;
    use ae::frame::{PortalAperture, PortalFrame};
    for (y, contacts) in [(-2.5, true), (0.5, false)] {
        let mut app = app();
        let recipient = walker(&mut app, WALKER, ae::Vec2::new(0.0, -1.0));
        app.world_mut().get_mut::<ae::BodyKinematics>(recipient).unwrap().canonical_size = ae::Vec2::splat(4.0);
        let toucher = player(&mut app, "toucher", ae::Vec2::new(0.0, y), ae::Vec2::ZERO);
        app.world_mut().get_mut::<ae::BodyKinematics>(toucher).unwrap().canonical_size = ae::Vec2::splat(0.2);
        let mut geometry = BodyGeometry::default();
        geometry.add_pair(None, (
            PortalAperture { frame: PortalFrame::fixed(ae::Vec2::ZERO, -ae::Vec2::Y), half_length: 10.0 },
            PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(100.0, -50.0), ae::Vec2::Y), half_length: 10.0 },
        ));
        app.insert_resource(geometry);
        app.world_mut().run_schedule(Sim);
        assert_eq!(!app.world().resource::<ContactTouchers>().0.is_empty(), contacts);
    }
}


#[test]
fn a_geometry_refusal_is_not_published_as_a_proven_empty_contact_sample() {
    fn observe(inv: &mut Invocation<'_>) -> Result<(), Fault> {
        let sample = inv.trigger::<BodyContactPort>()?;
        assert!(sample.incomplete);
        assert!(sample.touchers.is_empty());
        assert_eq!(sample.refusals.len(), 1);
        assert_eq!(sample.refusals[0].id, SimId::placement("broken").as_str());
        assert!(sample.refusals[0].reason.contains("InvalidScale"));
        inv.submit::<ambition_combat_port::HoldStatusPort>(ambition_combat_port::HoldStatus {
            kind: "test.saw_refusal".into(), held: true,
        })
    }
    let mut app = app_with(observe);
    let recipient = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
    let broken = player(&mut app, "broken", on_head(100.0), ae::Vec2::ZERO);
    app.world_mut().get_mut::<ae::BodyKinematics>(broken).unwrap().canonical_size = ae::Vec2::ZERO;
    app.world_mut().run_schedule(Sim);
    assert!(app.world().get::<crate::extension::status::BodyStatuses>(recipient).unwrap().holds("test.saw_refusal"));
}

#[test]
fn world_contacts_keep_solver_order_bounds_and_step_freshness() {
    use ambition_combat_port::world_geometry::{ContactRole, GeometryOrigin};
    use ambition_platformer2d_world::scoped_rules::{DeclareRulesExt, RulesScope};
    fn read(inv: &mut Invocation<'_>) -> Result<(), Fault> {
        let sample = inv.trigger::<BodyContactPort>()?;
        match sample.observed_at {
            0 => {
                assert!(sample.world_available);
                assert!(sample.world_truncated);
                assert_eq!(sample.world.len(), 2);
                assert_eq!(sample.world[0].role, ContactRole::Head);
                let block = sample.world[0].block.as_ref().unwrap();
                assert_eq!(block.geometry.origin, GeometryOrigin::Placement);
                assert_eq!(block.geometry.key.as_deref(), Some("crate:one"));
                assert_eq!(block.geometry.index, 4);
                assert!(sample.world[0].chain.is_none());
                assert_eq!(sample.world[0].impact_speed, 150.0);
                assert!(sample.world[0].involuntary);
                assert_eq!(sample.world[1].chain.as_ref().unwrap().segment, 7);
                assert!(sample.world[1].block.is_none());
            }
            1 => {
                assert!(!sample.world_available, "an old body step is not an empty current step");
                assert!(sample.world.is_empty());
                assert!(!sample.world_truncated);
            }
            2 => {
                assert!(sample.world_available);
                assert!(sample.world.is_empty());
                assert!(!sample.world_truncated);
            }
            tick => panic!("unexpected tick {tick}"),
        }
        inv.submit::<ambition_combat_port::HoldStatusPort>(ambition_combat_port::HoldStatus {
            kind: format!("observed_{}", sample.observed_at), held: true,
        })
    }
    let mut app = app_with(read);
    app.declare_rules(RulesScope::EveryRoom, ContactFactsLimits {
        max_world_contacts: 2, ..Default::default()
    });
    let recipient = walker(&mut app, WALKER, ae::Vec2::new(100.0, 900.0));
    let mut first = ae::collision_semantics::block_face_contact(
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::ONE),
        &ae::Block { id: ae::GeoId::placement(ae::PlacementId::new("crate:one"), 4),
            name: "a display name is not an identity".into(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::ONE),
            kind: ae::BlockKind::Solid, velocity: ae::Vec2::ZERO, art_color: None },
        ae::Vec2::Y, 0.25, ae::collision_semantics::ContactKind::Head, 150.0,
    );
    first.involuntary = true;
    let mut second = first.clone();
    second.source = ae::collision_semantics::ContactSource::Chain { chain: 3, segment: 7 };
    app.world_mut().entity_mut(recipient).insert(ae::BodyFrameOutput {
        observed_at: Some(0), events: ae::FrameEvents {
            contacts: vec![first.clone(), second, first], ..Default::default()
        }, ..Default::default()
    });
    for tick in 0..3 {
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 = tick;
        if tick == 2 {
            let mut frame = app.world_mut().get_mut::<ae::BodyFrameOutput>(recipient).unwrap();
            frame.events = Default::default();
            frame.observed_at = Some(tick);
        }
        app.world_mut().run_schedule(Sim);
        assert!(app.world().get::<crate::extension::status::BodyStatuses>(recipient)
            .unwrap().0.contains(&format!("observed_{tick}")));
    }
}
