//! Held fields use each body's collision box in its own live room.

use super::*;
use ambition_combat_port::{HeldForce, HeldForcePort};
use ambition_extension_host::ExtensionHostPlugin;
use ambition_extension_sdk::{
    wire, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, PortKey, PortRole, TriggerBinding,
};
use ambition_platformer2d_core::{CenteredAabb, PoseOwnedExternally, SweepSample};
use ambition_platformer2d_shared_tangle::lifecycle::{
    InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

#[derive(Component)]
struct Field(HeldForce);

struct FieldTick;
impl Port for FieldTick {
    const KEY: PortKey = PortKey::new("test.field_tick", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = HeldForce;

    fn encode(value: &HeldForce, out: &mut Vec<u8>) {
        HeldForcePort::encode(value, out);
    }

    fn decode(reader: &mut wire::WireReader<'_>) -> Result<HeldForce, wire::WireError> {
        HeldForcePort::decode(reader)
    }
}

fn collect_fields(mut pending: ResMut<ExtensionInvocations>, fields: Query<(Entity, &Field)>) {
    for (entity, field) in &fields {
        pending.trigger::<FieldTick>(&BOSS_CONDUCT, "wind", entity, None, false, field.0.clone());
    }
}

fn hold_field(invocation: &mut Invocation<'_>) -> Result<(), Fault> {
    let field = invocation.trigger::<FieldTick>()?.clone();
    invocation.submit::<HeldForcePort>(field)
}

fn velocities_in(min: [f32; 2], max: [f32; 2]) -> [Vec2; 5] {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0))
        .install_extension_trigger::<FieldTick, _>(BOSS_CONDUCT, "test", collect_fields)
        .install_extension_submitted_request::<HeldForcePort, _>(
            BOSS_CONDUCT,
            "ambition_abilities",
            lower_held_forces::<InBossConduct>,
        )
        .add_extension_module(ModuleDescriptor {
            key: ModuleKey::new("test", "held_force"),
            api: ambition_extension_sdk::API_VERSION,
            code: CodeIdentity::StaticNative {
                crate_name: "test".into(),
                version: "0".into(),
            },
            schemas: vec![],
            entries: vec![EntryDescriptor {
                key: "hold".into(),
                phase: BOSS_CONDUCT,
                trigger: TriggerBinding {
                    port: FieldTick::KEY,
                    selector: "wind".into(),
                },
                reads: vec![],
                writes: vec![],
                requests: vec![HeldForcePort::KEY],
                after: vec![],
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(hold_field),
            }],
        });
    app.finish();

    let room = LiveRoomInstance::ACTIVATION;
    let other_room = room.next();
    for instance in [room, other_room] {
        app.world_mut().spawn((RoomInstanceRoot, instance));
    }
    app.world_mut().spawn((
        Field(HeldForce {
            slot: 0,
            min,
            max,
            force: [60.0, 0.0],
            lift: 0.0,
            well: None,
        }),
        SimId::placement("field"),
        InRoomInstance(room),
    ));
    let bodies = [
        (Some(Vec2::X), room, false),
        (Some(Vec2::Y), room, false),
        (None, room, false),
        (Some(Vec2::X), other_room, false),
        (Some(Vec2::X), room, true),
    ]
    .map(|(down, instance, external)| {
        let kin = BodyKinematics {
            pos: Vec2::ZERO,
            size: Vec2::new(4.0, 20.0),
            ..Default::default()
        };
        let mut body = app.world_mut().spawn((
            kin,
            InRoomInstance(instance),
            // The draw envelope and gravity differ from the wall body's box.
            CenteredAabb::from_center_size(Vec2::ZERO, Vec2::splat(1000.0)),
            ResolvedMotionFrame::default(),
        ));
        if let Some(down) = down {
            body.insert(SweepSample {
                down,
                half: kin.half_oriented(down),
                ..Default::default()
            });
        }
        if external {
            body.insert(PoseOwnedExternally::only(
                ambition_platformer2d_core::PoseHold::Conductor,
            ));
        }
        body.id()
    });
    app.world_mut().run_schedule(Sim);
    bodies.map(|body| app.world().get::<BodyKinematics>(body).unwrap().vel)
}

#[test]
fn a_held_field_uses_the_collision_box_from_the_last_step() {
    let horizontal = velocities_in([7.0, -1.0], [9.0, 1.0]);
    assert_eq!(
        horizontal.map(|velocity| velocity.x > 0.0),
        [true, false, false, false, false]
    );
    let vertical = velocities_in([-1.0, 7.0], [1.0, 9.0]);
    assert_eq!(
        vertical.map(|velocity| velocity.x > 0.0),
        [false, true, true, false, false]
    );
}
