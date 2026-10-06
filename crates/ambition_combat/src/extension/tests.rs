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
