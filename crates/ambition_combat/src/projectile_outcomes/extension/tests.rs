//! Native retirement remains observable without reaching replacement bodies.

use super::*;
use ambition_extension_host::{BodyRecords, ExtensionHostPlugin};
use ambition_extension_sdk::{
    CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, SchemaKey, TriggerBinding, record,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    InRoomInstance, LiveRoomInstance, SessionScopeId,
};
use ambition_projectiles::{
    LiveProjectile, ProjectileOwner, ProjectileSeq, ownership::ProjectileCredit,
};
use bevy::ecs::schedule::ScheduleLabel;
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;
record! {
    struct Seen = SchemaKey::new("test", "projectile", 1);
    1 events: u32,
    2 named: bool,
    3 truncated: bool,
}
fn observe(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let batch = inv.trigger::<ProjectileOutcomesPort>()?;
    let count = batch.events.len() as u32;
    let named = batch.events.iter().all(|event| {
        event.projectile.id.is_some()
            && event
                .credit
                .as_ref()
                .is_some_and(|credit| credit.id.is_some())
    });
    let truncated = batch.truncated;
    let mut seen = Seen::load(inv)?;
    seen.events += count;
    seen.named = named;
    seen.truncated = truncated;
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
            ambition_platformer2d_world::scoped_rules::DeclaredRules::<ProjectileOutcomeRules>::default,
        )
        .declare(
            ambition_platformer2d_world::scoped_rules::RulesScope::EveryRoom,
            ProjectileOutcomeRules {
                max_events: 2, room_observers: vec!["observe".into()],
                ..Default::default()
            },
        );
    app.add_extension_module(ModuleDescriptor {
        key: ModuleKey::new("test", "projectile"),
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: "test".into(),
            version: "0".into(),
        },
        schemas: vec![Seen::schema()],
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: "observe".into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding {
                port: ProjectileOutcomesPort::KEY,
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
    app.world_mut().run_schedule(Sim);
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
fn native_retirement_keeps_bounded_room_facts_but_does_not_reach_replacement_or_foreign_bodies() {
    let mut app = app();
    let source = scope(&mut app, "source", 1, 0);
    let root = scope(&mut app, "root", 1, 0);
    let other_room = scope(&mut app, "root", 1, 1);
    let other_session = scope(&mut app, "root", 2, 0);
    for entity in [root, other_room, other_session] {
        let room = app.world().get::<InRoomInstance>(entity).unwrap().0;
        app.world_mut()
            .entity_mut(entity)
            .insert((RoomInstanceRoot, room));
    }
    let id = app.world().get::<SimId>(source).unwrap().clone();
    for sequence in 0..3 {
        let shot = app
            .world_mut()
            .spawn((
                LiveProjectile,
                ProjectileOwner(source),
                ProjectileSeq(sequence),
                SimId::spawned(&id, sequence),
                SessionScopedEntity(SessionScopeId(1)),
                InRoomInstance(LiveRoomInstance::ACTIVATION),
                ProjectileCredit {
                    body: Some(id.clone()),
                    room: Some(LiveRoomInstance::ACTIVATION),
                },
                ambition_platformer2d_core::BodyKinematics::default(),
            ))
            .id();
        super::super::retire(
            &mut app.world_mut().commands(),
            shot,
            super::super::ProjectileOutcomeKind::Expired,
            None,
        );
    }
    app.world_mut().flush();
    app.world_mut().despawn(source);
    let replacement = scope(&mut app, "source", 1, 0);
    app.world_mut().run_schedule(Sim);
    assert_eq!(seen(&app, root).events, 2);
    assert!(seen(&app, root).named);
    assert!(seen(&app, root).truncated);
    for entity in [replacement, other_room, other_session] {
        assert_eq!(seen(&app, entity).events, 0);
    }
    app.world_mut().run_schedule(Sim);
    assert_eq!(seen(&app, root).events, 2);
}

#[test]
fn a_live_credited_body_keeps_its_outcome_after_travel() {
    let mut app = app();
    let source = scope(&mut app, "source", 1, 0);
    let id = app.world().get::<SimId>(source).unwrap().clone();
    let shot = app
        .world_mut()
        .spawn((
            LiveProjectile,
            ProjectileOwner(source),
            ProjectileSeq(0),
            SimId::spawned(&id, 0),
            SessionScopedEntity(SessionScopeId(1)),
            InRoomInstance(LiveRoomInstance::ACTIVATION),
            ProjectileCredit {
                body: Some(id),
                room: Some(LiveRoomInstance::ACTIVATION),
            },
            ambition_platformer2d_core::BodyKinematics::default(),
        ))
        .id();
    super::super::retire(
        &mut app.world_mut().commands(),
        shot,
        super::super::ProjectileOutcomeKind::RoomRetired,
        None,
    );
    app.world_mut().flush();
    app.world_mut()
        .entity_mut(source)
        .insert(InRoomInstance(LiveRoomInstance::from_ordinal(1)));
    app.world_mut().run_schedule(Sim);
    assert_eq!(seen(&app, source).events, 1);
    assert!(seen(&app, source).named);
}

#[test]
fn a_hidden_shot_cannot_write_into_its_visible_credited_body() {
    let mut app = app();
    let source = scope(&mut app, "source", 1, 0);
    let id = app.world().get::<SimId>(source).unwrap().clone();
    let shot = app
        .world_mut()
        .spawn((
            LiveProjectile,
            ProjectileOwner(source),
            ProjectileSeq(0),
            SimId::spawned(&id, 0),
            SessionScopedEntity(SessionScopeId(1)),
            InRoomInstance(LiveRoomInstance::ACTIVATION),
            ProjectileCredit {
                body: Some(id),
                room: Some(LiveRoomInstance::ACTIVATION),
            },
            ambition_platformer2d_core::BodyKinematics::default(),
            bevy::ecs::entity_disabling::Disabled,
        ))
        .id();
    super::super::retire(
        &mut app.world_mut().commands(),
        shot,
        super::super::ProjectileOutcomeKind::Expired,
        None,
    );
    app.world_mut().flush();
    app.world_mut().run_schedule(Sim);
    assert_eq!(seen(&app, source).events, 0);
}
