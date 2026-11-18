//! Scope and retirement rules for accepted native decisions.

use super::*;
use ambition_extension_host::{BodyRecords, ExtensionHostPlugin};
use ambition_extension_sdk::{
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, SchemaKey, TriggerBinding,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    InRoomInstance, LiveRoomInstance, SessionScopeId,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

record! {
    struct Seen = SchemaKey::new("test", "reception", 1);
    1 events: u32,
    2 named: bool,
}
fn observe(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let batch = inv.trigger::<HitReceptionsPort>()?;
    let added = batch.events.len() as u32;
    let named = batch.events.iter().all(|event| event.victim.id.is_some());
    let mut seen = Seen::load(inv)?;
    seen.events += added;
    seen.named = named;
    seen.store(inv)
}
fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, 0.0));
    install(&mut app);
    app.world_mut()
        .get_resource_or_insert_with(
            ambition_platformer2d_world::scoped_rules::DeclaredRules::<HitReceptionRules>::default,
        )
        .declare(
            ambition_platformer2d_world::scoped_rules::RulesScope::EveryRoom,
            HitReceptionRules {
                room_observers: vec!["observe".into()],
                ..Default::default()
            },
        );
    app.add_extension_module(ModuleDescriptor {
        key: ModuleKey::new("test", "reception"),
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: "test".into(),
            version: "0".into(),
        },
        schemas: vec![Seen::schema()],
        entries: vec![EntryDescriptor {
            key: "observe".into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding {
                port: HitReceptionsPort::KEY,
                selector: "observe".into(),
            },
            reads: vec![],
            writes: vec![Seen::KEY],
            requests: vec![],
            after: vec![],
            limits: Limits { max_requests: 0 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(observe),
        }],
    });
    app.finish();
    app
}
fn scope(app: &mut App, name: &str, session: u64, room: u32) -> Entity {
    app.world_mut()
        .spawn((
            SimId::placement(name),
            SessionScopedEntity(SessionScopeId(session)),
            InRoomInstance(LiveRoomInstance::from_ordinal(room)),
            BodyStatuses(vec!["observe".into()]),
        ))
        .id()
}
fn seen(app: &App, scope: Entity) -> Seen {
    app.world()
        .get::<BodyRecords>(scope)
        .and_then(|records| records.get(&Seen::KEY))
        .map(|record| Seen::from_record(record).unwrap())
        .unwrap_or_default()
}

#[test]
fn a_room_observes_a_retired_victim_but_a_replacement_cannot_receive_its_hit() {
    let mut app = app();
    let original = scope(&mut app, "recipient", 1, 0);
    let attacker = scope(&mut app, "source", 1, 0);
    let root = scope(&mut app, "room", 1, 0);
    let other_room = scope(&mut app, "room", 1, 1);
    let other_session = scope(&mut app, "room", 2, 0);
    for entity in [root, other_room, other_session] {
        let room = app.world().get::<InRoomInstance>(entity).unwrap().0;
        app.world_mut()
            .entity_mut(entity)
            .insert((RoomInstanceRoot, room));
    }
    let receiver = HitIdentity {
        id: app.world().get::<SimId>(original).cloned(),
        session: Some(SessionScopeId(1)),
        room: Some(LiveRoomInstance::ACTIVATION),
    };
    let source = HitIdentity {
        id: app.world().get::<SimId>(attacker).cloned(),
        ..receiver.clone()
    };
    app.world_mut().write_message(BodyHitResolved {
            projectile: None,
        body: original,
        attacker: Some(attacker),
        identity: receiver,
        attacker_identity: Some(source),
        observed_at: 0,
        resolution: BodyHitResolution::Damaged {
            damage: 3,
            died: true,
        },
        source: ambition_combat::HitSource::Projectile,
        raw_damage: 3,
        attacker_move_instance: None,
    });
    app.world_mut().despawn(original);
    let replacement = scope(&mut app, "recipient", 1, 0);
    app.world_mut().run_schedule(Sim);
    assert_eq!(seen(&app, root).events, 1);
    assert!(
        seen(&app, root).named,
        "retirement does not erase the captured identity"
    );
    assert_eq!(seen(&app, attacker).events, 1);
    for scope in [replacement, other_room, other_session] {
        assert_eq!(seen(&app, scope).events, 0);
    }
    app.world_mut().run_schedule(Sim);
    assert_eq!(
        seen(&app, root).events,
        1,
        "delivery does not repeat during hitstop"
    );
}

#[test]
fn mechanically_equal_receptions_have_equal_gameplay_payloads_after_different_host_session_histories(
) {
    use ambition_extension_sdk::Port;
    fn payload(session: u64) -> Vec<u8> {
        let body = Entity::PLACEHOLDER;
        let identity = HitIdentity {
            id: Some(SimId::placement("recipient")),
            session: Some(SessionScopeId(session)),
            room: Some(LiveRoomInstance::ACTIVATION),
        };
        let hit = BodyHitResolved {
            projectile: Some(HitIdentity { id: Some(SimId::placement("shot")), room: identity.room, session: identity.session }),
            body,
            attacker: Some(body),
            identity: identity.clone(),
            attacker_identity: Some(identity),
            observed_at: 10,
            resolution: BodyHitResolution::Damaged {
                damage: 2,
                died: false,
            },
            source: ambition_combat::HitSource::Melee,
            raw_damage: 2,
            attacker_move_instance: None,
        };
        let mut bytes = Vec::new();
        HitReceptionsPort::encode(
            &HitReceptions {
                events: vec![fact(&hit, body, Some(SessionScopeId(session)))],
                truncated: false,
            },
            &mut bytes,
        );
        let foreign = fact(&hit, body, Some(SessionScopeId(session + 1)));
        let shot = foreign.projectile.unwrap();
        assert!(shot.id.is_none());
        assert!(shot.room.is_none());
        bytes
    }
    assert_eq!(payload(1), payload(99));
}

#[test]
fn a_foreign_body_is_not_exposed_as_an_identity_in_the_invocations_session() {
    let foreign = HitIdentity {
        id: Some(SimId::placement("same_name")),
        session: Some(SessionScopeId(2)),
        room: Some(LiveRoomInstance::ACTIVATION),
    };
    let value = identity(&foreign, Some(SessionScopeId(1)));
    assert!(value.id.is_none());
    assert!(value.room.is_none());
}
