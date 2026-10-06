//! The hold adapter in a tiny App: a native entry submits one
//! `BodyHold::Seize` for its body, through the real host, and
//! `lower_body_holds` lowers it.

use super::*;
use ambition_extension_host::{ExtensionHostPlugin, ExtensionInvocations};
use ambition_extension_sdk::{
    wire, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, PortKey, PortRole, TriggerBinding,
};
use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

/// The body that asks for the seize.
#[derive(Component)]
struct Seizes;

struct Poke;
impl Port for Poke {
    const KEY: PortKey = PortKey::new("test.seize_poke", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = u32;
    fn encode(v: &u32, out: &mut Vec<u8>) {
        wire::put_u32(out, *v);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<u32, wire::WireError> {
        r.u32()
    }
}

/// The reach of the fixture's seize, on the captor's own axes: 100 ahead, a
/// box 120 long on the captor's floor and 20 tall. It is NOT square, so a
/// reach that turns its centre and not its half extents is seen.
const REACH_OFFSET: [f32; 2] = [100.0, 0.0];
const REACH_HALF: [f32; 2] = [60.0, 10.0];

fn seize(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    inv.submit::<BodyHoldPort>(BodyHold::Seize {
        reach_offset: REACH_OFFSET,
        reach_half: REACH_HALF,
        hold_at: None,
        hold_offset: [20.0, 0.0],
        hold_s: 2.0,
    })
}

fn collect_pokes(mut invocations: ResMut<ExtensionInvocations>, bodies: Query<Entity, With<Seizes>>) {
    for entity in &bodies {
        invocations.trigger::<Poke>(&BOSS_CONDUCT, "go", entity, None, true, 0);
    }
}

fn framed(down: ae::Vec2) -> ResolvedMotionFrame {
    let mut frame = ResolvedMotionFrame::default();
    frame.publish_resolved_frame(ae::MotionFrame::from_direction(down, 900.0));
    frame
}

/// One captor at the origin that faces `facing` under gravity `down`, and one
/// victim at `victim_at`. Returns whether the seize caught the victim.
fn seizes(down: ae::Vec2, facing: f32, victim_at: ae::Vec2) -> bool {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .add_message::<crate::capture::CaptureCarryRequested>()
        .add_message::<crate::capture::CapturePummelRequested>()
        .add_message::<crate::capture::CaptureThrowRequested>()
        .install_extension_trigger::<Poke, _>(BOSS_CONDUCT, "test", collect_pokes)
        .install_extension_request::<BodyHoldPort, _>(BOSS_CONDUCT, "ambition_combat", lower_body_holds)
        .add_extension_module(ModuleDescriptor {
            key: ModuleKey::new("test", "seizer"),
            api: ambition_extension_sdk::API_VERSION,
            code: CodeIdentity::StaticNative {
                crate_name: "test".into(),
                version: "0".into(),
            },
            schemas: vec![],
            entries: vec![EntryDescriptor {
                key: "seize".into(),
                phase: BOSS_CONDUCT,
                trigger: TriggerBinding {
                    port: Poke::KEY,
                    selector: "go".into(),
                },
                reads: vec![],
                writes: vec![],
                requests: vec![BodyHoldPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(seize),
            }],
        });
    app.finish();
    app.world_mut().spawn((
        Seizes,
        ae::BodyKinematics {
            pos: ae::Vec2::ZERO,
            facing,
            size: ae::Vec2::new(16.0, 24.0),
            ..Default::default()
        },
        ActorFaction::Enemy,
        framed(down),
        SimId::placement("captor"),
    ));
    let victim = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: victim_at,
                facing: 1.0,
                size: ae::Vec2::new(16.0, 16.0),
                ..Default::default()
            },
            crate::components::CenteredAabb::new(victim_at, ae::Vec2::new(8.0, 8.0)),
            ActorFaction::Player,
            SimId::placement("victim"),
        ))
        .id();
    app.world_mut().run_schedule(Sim);
    app.world().get::<crate::capture::CapturedBy>(victim).is_some()
}

/// A SEIZE REACHES ON THE CAPTOR'S OWN AXES.
///
/// The reach is a box in the captor's local frame. Under sideways gravity
/// "ahead" is along the captor's floor, and the long side of the box lies
/// along that floor. The control arm is normal gravity: there the same
/// fixture catches the body at world +x and does not catch the body that is
/// ahead in the turned frame. It proves that the fixture can seize, and that
/// each arm of the turned case can fail.
#[test]
fn a_seize_under_sideways_gravity_reaches_along_the_floor_of_the_captor() {
    let normal = ae::Vec2::new(0.0, 1.0);
    let sideways = ae::Vec2::new(1.0, 0.0);
    // 150 ahead of the captor: inside the long side of the box (100 +/- 60)
    // and 50 from its centre, which is more than its short half (10) plus
    // the victim's half (8). A box whose half extents did not turn does not
    // reach it.
    let ahead = |down: ae::Vec2| ae::AccelerationFrame::new(down).side * 150.0;
    assert_ne!(ahead(normal), ahead(sideways), "the two frames have the same side axis");

    assert!(seizes(normal, 1.0, ahead(normal)), "control: the fixture did not seize a body in its reach");
    assert!(
        !seizes(normal, 1.0, ahead(sideways)),
        "control: under normal gravity the reach caught the body that is ahead only in the turned frame"
    );

    assert!(
        seizes(sideways, 1.0, ahead(sideways)),
        "a captor under sideways gravity did not reach the body ahead of it on its own floor"
    );
    assert!(
        !seizes(sideways, 1.0, ahead(normal)),
        "a captor under sideways gravity reached along world +x, which is its own up or down"
    );
    // The facing mirrors the reach on the captor's side axis, in each frame.
    assert!(
        seizes(sideways, -1.0, -ahead(sideways)),
        "a captor that faces the other way did not reach behind its first reach"
    );
    assert!(!seizes(sideways, -1.0, ahead(sideways)), "the reach did not mirror with the facing");
}

thread_local! {
    /// The hold the jaw fixture's entry asks for: the point it names and the
    /// offset from it. A native entry is a plain function, so the arm of the
    /// test states it here.
    static JAW_HOLD: std::cell::RefCell<(Option<String>, [f32; 2])> =
        const { std::cell::RefCell::new((None, [0.0, 0.0])) };
}

/// A native entry that seizes at the point [`JAW_HOLD`] names. It keeps no
/// number for the point: it states the NAME, and the engine places it.
fn seize_at_a_named_point(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let (hold_at, hold_offset) = JAW_HOLD.with_borrow(Clone::clone);
    inv.submit::<BodyHoldPort>(BodyHold::Seize {
        reach_offset: [0.0, 0.0],
        reach_half: [200.0, 200.0],
        hold_at,
        hold_offset,
        hold_s: 2.0,
    })
}

/// A rig of one `jaw` joint at `(40, -30)` from the feet, with a `jaw`
/// attachment `along` units along it.
fn jaw_rig(along: f32) -> crate::body_rig::BodyRig {
    use ambition_characters::actor::body_rig::{BodyRigDefinition, JointPose, RigAttachment, RigClip, RigJoint};
    let frame = vec![JointPose {
        translation: (40.0, -30.0),
        rotation: 0.0,
        scale: (1.0, 1.0),
    }];
    crate::body_rig::BodyRig(std::sync::Arc::new(
        BodyRigDefinition {
            joints: vec![RigJoint {
                name: "jaw".to_string(),
                parent: None,
            }],
            attachments: vec![RigAttachment {
                name: "jaw".to_string(),
                joint: "jaw".to_string(),
                offset: (along, 0.0),
            }],
            hurt_parts: Vec::new(),
            clips: std::collections::BTreeMap::from([(
                "idle".to_string(),
                RigClip {
                    looping: true,
                    frame_duration_s: 0.1,
                    frames: vec![frame],
                },
            )]),
        }
        .prepare()
        .expect("a valid rig"),
    ))
}

/// How the captor of the jaw fixture is built.
struct JawCaptor {
    rig: Option<crate::body_rig::BodyRig>,
    feet: Option<crate::body_rig::RigFeetOffset>,
    /// Whether its rig's pose is resolved before the seize.
    posed: bool,
    facing: f32,
}

impl JawCaptor {
    fn new(rig: crate::body_rig::BodyRig) -> Self {
        Self {
            rig: Some(rig),
            feet: Some(FEET),
            posed: true,
            facing: 1.0,
        }
    }
}

/// The feet the jaw fixture's captor states: (-4, 20) from its centre.
const FEET: crate::body_rig::RigFeetOffset = crate::body_rig::RigFeetOffset(ae::Vec2::new(-4.0, 20.0));

/// Where the jaw fixture's captor stands. Not the origin, so a hold that is
/// not measured from the captor is seen.
const CAPTOR_AT: ae::Vec2 = ae::Vec2::new(300.0, 100.0);

/// One captor and one victim in its reach. The captor's entry seizes at the
/// point `hold_at` names, plus `hold_offset`; then the capture relation poses
/// the victim, as it does at the end of each tick. Returns where the victim
/// is FROM THE CAPTOR (world axes), or `None` when nothing was seized.
fn held_from_the_captor(captor: JawCaptor, hold_at: Option<&str>, hold_offset: [f32; 2]) -> Option<ae::Vec2> {
    use bevy::ecs::system::RunSystemOnce as _;
    JAW_HOLD.set((hold_at.map(str::to_owned), hold_offset));
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .add_message::<crate::capture::CaptureCarryRequested>()
        .add_message::<crate::capture::CapturePummelRequested>()
        .add_message::<crate::capture::CaptureThrowRequested>()
        .install_extension_trigger::<Poke, _>(BOSS_CONDUCT, "test", collect_pokes)
        .install_extension_request::<BodyHoldPort, _>(BOSS_CONDUCT, "ambition_combat", lower_body_holds)
        .add_extension_module(ModuleDescriptor {
            key: ModuleKey::new("test", "jaw_holder"),
            api: ambition_extension_sdk::API_VERSION,
            code: CodeIdentity::StaticNative {
                crate_name: "test".into(),
                version: "0".into(),
            },
            schemas: vec![],
            entries: vec![EntryDescriptor {
                key: "seize".into(),
                phase: BOSS_CONDUCT,
                trigger: TriggerBinding {
                    port: Poke::KEY,
                    selector: "go".into(),
                },
                reads: vec![],
                writes: vec![],
                requests: vec![BodyHoldPort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(seize_at_a_named_point),
            }],
        });
    app.finish();
    let body = app
        .world_mut()
        .spawn((
            Seizes,
            ae::BodyKinematics {
                pos: CAPTOR_AT,
                facing: captor.facing,
                size: ae::Vec2::new(16.0, 24.0),
                ..Default::default()
            },
            ActorFaction::Enemy,
            SimId::placement("captor"),
        ))
        .id();
    if let Some(rig) = captor.rig {
        app.world_mut().entity_mut(body).insert((rig, crate::body_rig::BodyRigPose::default()));
    }
    if let Some(feet) = captor.feet {
        app.world_mut().entity_mut(body).insert(feet);
    }
    let victim_at = CAPTOR_AT + ae::Vec2::new(30.0, 0.0);
    let victim = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: victim_at,
                facing: 1.0,
                size: ae::Vec2::new(16.0, 16.0),
                ..Default::default()
            },
            ae::BodyGroundState::default(),
            crate::components::CenteredAabb::new(victim_at, ae::Vec2::new(8.0, 8.0)),
            ActorFaction::Player,
            SimId::placement("victim"),
        ))
        .id();
    if captor.posed {
        app.world_mut()
            .run_system_once(crate::body_rig::resolve_body_rig_poses)
            .expect("the rig pose resolves");
    }
    app.world_mut().run_schedule(Sim);
    app.world_mut()
        .run_system_once(crate::capture::systems::finalize_new_capture_pose)
        .expect("the capture relation poses its captives");
    let world = app.world();
    world.get::<crate::capture::CapturedBy>(victim)?;
    Some(world.get::<ae::BodyKinematics>(victim).expect("the victim").pos - CAPTOR_AT)
}

/// A MODULE HOLDS A BODY AT A POINT ITS RIG NAMES, AND KEEPS NO NUMBER FOR IT.
///
/// The art package states a named point on a joint of the body's rig. A hold
/// names the point (`BodyHold::Seize::hold_at`); the capture relation places
/// it from the rig's pose when it poses the held body. Moved in the rig, the
/// point moves the held body, and the module is not edited.
#[test]
fn a_module_holds_a_body_at_the_attachment_its_rig_names() {
    let jaw = Some("jaw");
    // The joint is at (40, -30) from the feet, the point 5 along it, and the
    // feet (-4, 20) from the body's centre.
    assert_eq!(
        held_from_the_captor(JawCaptor::new(jaw_rig(5.0)), jaw, [0.0, 0.0]),
        Some(ae::Vec2::new(41.0, -10.0)),
        "the held body is not at the jaw of the rig's pose, placed from the feet the body states"
    );
    // The art moves the point 20 along the jaw: the held body moves 20, with
    // the same entry.
    assert_eq!(
        held_from_the_captor(JawCaptor::new(jaw_rig(25.0)), jaw, [0.0, 0.0]),
        Some(ae::Vec2::new(61.0, -10.0)),
        "the point moved in the rig and the held body did not follow it"
    );
    // The hold's offset is measured from the point, on the captor's axes.
    assert_eq!(
        held_from_the_captor(JawCaptor::new(jaw_rig(5.0)), jaw, [3.0, 2.0]),
        Some(ae::Vec2::new(44.0, -8.0)),
        "the offset of the hold is not measured from the named point"
    );
    // The point and the offset mirror with the captor's facing.
    assert_eq!(
        held_from_the_captor(
            JawCaptor {
                facing: -1.0,
                ..JawCaptor::new(jaw_rig(5.0))
            },
            jaw,
            [3.0, 2.0]
        ),
        Some(ae::Vec2::new(-44.0, -8.0)),
        "a captor that faces the other way does not hold at its mirrored jaw"
    );
    // A body that states no feet has them straight below, half its height.
    assert_eq!(
        held_from_the_captor(
            JawCaptor {
                feet: None,
                ..JawCaptor::new(jaw_rig(5.0))
            },
            jaw,
            [0.0, 0.0]
        ),
        Some(ae::Vec2::new(45.0, -18.0)),
        "a body with no stated feet does not place its rig from the bottom of its box"
    );
    // Control: a hold that names no point is measured from the captor's
    // position, as a fighter's grab is, with the same rig on the body.
    assert_eq!(
        held_from_the_captor(JawCaptor::new(jaw_rig(5.0)), None, [20.0, 0.0]),
        Some(ae::Vec2::new(20.0, 0.0)),
        "a hold that names no point is not measured from the captor's position"
    );
}

/// A HOLD AT A POINT THE RIG DOES NOT STATE IS REFUSED.
///
/// A held body at a wrong point reads as a broken grab. A module that names a
/// point the art does not state, or names one on a body with no rig, catches
/// nothing. The first arm is the control: the same fixture seizes at `jaw`.
#[test]
fn a_hold_at_a_point_the_rig_does_not_state_catches_nothing() {
    assert!(
        held_from_the_captor(JawCaptor::new(jaw_rig(5.0)), Some("jaw"), [0.0, 0.0]).is_some(),
        "control: the fixture did not seize at a point its rig states"
    );
    assert_eq!(
        held_from_the_captor(JawCaptor::new(jaw_rig(5.0)), Some("hand"), [0.0, 0.0]),
        None,
        "the rig states no `hand`, and the seize caught a body"
    );
    assert_eq!(
        held_from_the_captor(
            JawCaptor {
                rig: None,
                ..JawCaptor::new(jaw_rig(5.0))
            },
            Some("jaw"),
            [0.0, 0.0]
        ),
        None,
        "a body with no rig seized at a named point"
    );
}

/// A HOLD AT A NAMED POINT, BEFORE THE RIG HAS A POSE, IS AT THE CENTRE.
///
/// A rig is posed in the combat window of its first tick. A seize before that
/// has a point by name and no place for it yet. The captive is held from the
/// captor's centre for that tick, and from the point on the next.
#[test]
fn a_hold_at_a_point_of_a_rig_with_no_pose_is_measured_from_the_centre() {
    assert_eq!(
        held_from_the_captor(
            JawCaptor {
                posed: false,
                ..JawCaptor::new(jaw_rig(5.0))
            },
            Some("jaw"),
            [3.0, 2.0]
        ),
        Some(ae::Vec2::new(3.0, 2.0)),
    );
}

/// `ambition.body.attachments` gives each named point of a posed rig from the
/// body's POSITION, in its local frame: where a module puts the spark of a
/// bite. It gives nothing for a rig with no pose yet, or a body with no rig.
#[test]
fn the_attachment_observation_gives_each_point_from_the_position_of_the_body() {
    use bevy::ecs::system::RunSystemOnce as _;
    let mut world = World::new();
    let body = world
        .spawn((
            ae::BodyKinematics {
                pos: CAPTOR_AT,
                facing: -1.0,
                size: ae::Vec2::new(16.0, 24.0),
                ..Default::default()
            },
            jaw_rig(5.0),
            crate::body_rig::BodyRigPose::default(),
            FEET,
        ))
        .id();
    let bare = world.spawn(ae::BodyKinematics::default()).id();
    assert_eq!(body_attachments_of(&world, body), None, "a rig with no pose gave a point");
    world
        .run_system_once(crate::body_rig::resolve_body_rig_poses)
        .expect("the rig pose resolves");
    let points = body_attachments_of(&world, body).expect("a posed rig has its points");
    // Local: the facing is not in it. The module mirrors, as for a hold.
    assert_eq!(points.get("jaw"), Some([41.0, -10.0]));
    assert_eq!(points.get("hand"), None);
    assert_eq!(body_attachments_of(&world, bare), None, "a body with no rig gave points");
}
