//! Host behaviour in a tiny App: admission refusals, the serial road, state
//! that persists on the body, and the discard of a faulted invocation.

use super::*;
use std::sync::Arc;
use ambition_extension_sdk::{
    ApiVersion, Attachment, CodeIdentity, EntryDescriptor, Fault, FieldDecl, FieldKind, FieldRef,
    Invocation, Limits, ModuleDescriptor, ModuleKey, Phase, PortKey, SaveEligibility, SchemaKey,
    StateSchema, TriggerBinding, Value,
};
use ambition_extension_sdk::{wire, EntryCode, IdlePolicy};
use ambition_time::SimTick;
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const PHASE: Phase = Phase::new("test_phase");

struct Poke;
impl Port for Poke {
    const KEY: PortKey = PortKey::new("test.poke", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = u32;
    fn encode(v: &u32, out: &mut Vec<u8>) {
        wire::put_u32(out, *v);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<u32, wire::WireError> {
        r.u32()
    }
}

struct Height;
impl Port for Height {
    const KEY: PortKey = PortKey::new("test.height", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = f32;
    fn encode(v: &f32, out: &mut Vec<u8>) {
        wire::put_f32(out, *v);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<f32, wire::WireError> {
        r.f32()
    }
}

struct Emit;
impl Port for Emit {
    const KEY: PortKey = PortKey::new("test.emit", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = (u32, u32, f32);
    fn encode(v: &(u32, u32, f32), out: &mut Vec<u8>) {
        wire::put_u32(out, v.0);
        wire::put_u32(out, v.1);
        wire::put_f32(out, v.2);
    }
    fn decode(r: &mut wire::WireReader<'_>) -> Result<(u32, u32, f32), wire::WireError> {
        Ok((r.u32()?, r.u32()?, r.f32()?))
    }
}

#[derive(Component)]
struct Tall(f32);

#[derive(Component)]
struct Poked(u32);

#[derive(Resource, Default)]
struct Lowered(Vec<LoweredEmit>);

/// (body, move use, (poke, count, height)).
type LoweredEmit = (Entity, Option<u32>, (u32, u32, f32));

const COUNTER: SchemaKey = SchemaKey::new("test", "counter", 1);
const COUNT: FieldRef = FieldRef(0);

fn counter_schema() -> StateSchema {
    StateSchema {
        key: COUNTER,
        attachment: Attachment::Body,
        save: SaveEligibility::Transient,
        fields: vec![FieldDecl::new(1, "count", FieldKind::U32)],
    }
}

/// Counts its invocations on the body and emits (poke, count, height).
fn count_and_emit(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let poke = *inv.trigger::<Poke>()?;
    let height = *inv.observe::<Height>()?;
    let record = inv.state(&COUNTER)?;
    let count = record.get(COUNT).ok().and_then(Value::as_u32).unwrap_or(0) + 1;
    record.set(COUNT, Value::U32(count)).ok();
    inv.submit::<Emit>((poke, count, height))?;
    if poke == 13 {
        return Err(Fault::Module("thirteen".into()));
    }
    Ok(())
}

fn entry(key: &'static str, after: Vec<&'static str>) -> EntryDescriptor {
    EntryDescriptor {
        key: key.into(),
        phase: PHASE,
        trigger: TriggerBinding {
            port: Poke::KEY,
            selector: "go".into(),
        },
        reads: vec![Height::KEY],
        writes: vec![COUNTER],
        requests: vec![Emit::KEY],
        after: after.into_iter().map(Into::into).collect(),
        limits: Limits { max_requests: 1 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(count_and_emit),
    }
}

fn module(entries: Vec<EntryDescriptor>) -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new("test", "counter"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: "test".into(),
            version: "0".into(),
        },
        schemas: vec![counter_schema()],
        entries,
    }
}

fn offers() -> Vec<PortOffer> {
    vec![
        PortOffer {
            key: Poke::KEY,
            role: PortRole::Trigger,
            phase: PHASE,
            owner: "test",
        },
        PortOffer {
            key: Height::KEY,
            role: PortRole::Observation,
            phase: PHASE,
            owner: "test",
        },
        PortOffer {
            key: Emit::KEY,
            role: PortRole::Request,
            phase: PHASE,
            owner: "test",
        },
    ]
}

#[test]
fn a_complete_composition_is_admitted() {
    let admitted = admit(API_VERSION, &offers(), &[DeclaredModule::from(module(vec![entry("a", vec![])]))]).unwrap();
    assert_eq!(admitted.entries.len(), 1);
    assert!(admitted.wants(&Poke::KEY, "go"));
    assert!(!admitted.wants(&Poke::KEY, "stop"));
}

#[test]
fn a_port_the_composition_did_not_install_refuses_the_module() {
    // Poison: remove the request offer. The module must be refused, not run
    // with a submit that nothing answers.
    let mut offers = offers();
    offers.retain(|o| o.key != Emit::KEY);
    let refusals = admit(API_VERSION, &offers, &[DeclaredModule::from(module(vec![entry("a", vec![])]))]).unwrap_err();
    assert_eq!(
        refusals,
        vec![Refusal::MissingPort {
            entry: "test::counter/a".into(),
            port: Emit::KEY,
            role: PortRole::Request,
            phase: PHASE,
        }]
    );
}

#[test]
fn a_port_installed_in_another_phase_does_not_count() {
    let mut offers = offers();
    offers[2].phase = Phase::new("later");
    assert!(admit(API_VERSION, &offers, &[DeclaredModule::from(module(vec![entry("a", vec![])]))]).is_err());
}

#[test]
fn an_order_cycle_is_refused_and_an_order_is_obeyed() {
    let cycle = module(vec![
        entry("a", vec!["test::counter/b"]),
        entry("b", vec!["test::counter/a"]),
    ]);
    let refusals = admit(API_VERSION, &offers(), &[DeclaredModule::from(cycle)]).unwrap_err();
    assert!(matches!(refusals[0], Refusal::OrderCycle { .. }));

    let ordered = module(vec![entry("a", vec!["test::counter/b"]), entry("b", vec![])]);
    let admitted = admit(API_VERSION, &offers(), &[DeclaredModule::from(ordered)]).unwrap();
    let paths: Vec<&str> = admitted.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["test::counter/b", "test::counter/a"]);
}

#[test]
fn a_policy_without_a_host_road_is_refused() {
    for (attachment, save) in [
        (Attachment::Session, SaveEligibility::Transient),
        (Attachment::Body, SaveEligibility::Durable),
    ] {
        let mut m = module(vec![entry("a", vec![])]);
        m.schemas[0].attachment = attachment;
        m.schemas[0].save = save;
        let refusals = admit(API_VERSION, &offers(), &[DeclaredModule::from(m)]).unwrap_err();
        assert!(matches!(refusals[0], Refusal::UnsupportedPolicy { .. }));
    }
}

#[test]
fn a_newer_minor_api_and_a_foreign_schema_are_refused() {
    let mut newer = module(vec![entry("a", vec![])]);
    newer.api = ApiVersion {
        major: API_VERSION.major,
        minor: API_VERSION.minor + 1,
    };
    assert!(matches!(
        admit(API_VERSION, &offers(), &[DeclaredModule::from(newer)]).unwrap_err()[0],
        Refusal::ApiVersion { .. }
    ));

    let mut foreign = module(vec![]);
    foreign.schemas[0].key = SchemaKey::new("someone_else", "counter", 1);
    assert!(matches!(
        admit(API_VERSION, &offers(), &[DeclaredModule::from(foreign)]).unwrap_err()[0],
        Refusal::ForeignSchema { .. }
    ));
}

#[test]
fn the_admitted_digest_follows_the_composition() {
    let m = || module(vec![entry("a", vec![])]);
    let one = admit(API_VERSION, &offers(), &[DeclaredModule::from(m())]).unwrap().digest;
    let same = admit(API_VERSION, &offers(), &[DeclaredModule::from(m())]).unwrap().digest;
    assert_eq!(one, same);
    let mut moved = offers();
    moved.push(PortOffer {
        key: PortKey::new("test.other", 1),
        role: PortRole::Request,
        phase: PHASE,
        owner: "test",
    });
    assert_ne!(one, admit(API_VERSION, &moved, &[DeclaredModule::from(m())]).unwrap().digest);
}

fn collect_pokes(mut invocations: ResMut<ExtensionInvocations>, bodies: Query<(Entity, &Poked)>) {
    let mut bodies: Vec<_> = bodies.iter().collect();
    bodies.sort_by_key(|(_, poked)| poked.0);
    for (entity, poked) in bodies {
        invocations.trigger::<Poke>(&PHASE, "go", entity, Some(poked.0 + 100), poked.0 == 0, poked.0);
    }
}

fn height_of(world: &World, scope: Entity) -> Option<f32> {
    world.get::<Tall>(scope).map(|t| t.0)
}

fn lower_emits(mut outbox: ResMut<ExtensionOutbox>, mut lowered: ResMut<Lowered>) {
    for s in outbox.drain::<Emit>() {
        lowered.0.push((s.scope, s.occurrence, s.value));
    }
}

fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<Lowered>()
        .init_resource::<SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<Poke, _>(PHASE, "test", collect_pokes)
        .install_extension_observation::<Height>(PHASE, "test", height_of)
        .install_extension_request::<Emit, _>(PHASE, "test", lower_emits)
        .add_extension_module(module(vec![entry("a", vec![])]));
    app.finish();
    app
}

fn step(app: &mut App) {
    app.world_mut().run_schedule(Sim);
    app.world_mut().resource_mut::<SimTick>().0 += 1;
}

#[test]
fn an_entry_runs_for_each_trigger_and_its_state_stays_on_the_body() {
    let mut app = app();
    let a = app.world_mut().spawn((Poked(1), Tall(2.0))).id();
    let b = app.world_mut().spawn((Poked(2), Tall(5.0))).id();
    step(&mut app);
    step(&mut app);

    let lowered = &app.world().resource::<Lowered>().0;
    assert_eq!(
        lowered,
        &vec![
            (a, Some(101), (1, 1, 2.0)),
            (b, Some(102), (2, 1, 5.0)),
            (a, Some(101), (1, 2, 2.0)),
            (b, Some(102), (2, 2, 5.0)),
        ]
    );
    let records = app.world().get::<BodyRecords>(a).unwrap();
    assert_eq!(records.get(&COUNTER).unwrap().get(COUNT).unwrap(), &Value::U32(2));
    assert!(app.world().resource::<ExtensionOutbox>().is_empty());
    assert!(app.world().resource::<ExtensionInvocations>().is_empty());
}

#[test]
fn a_fault_discards_the_staged_state_and_the_staged_request() {
    let mut app = app();
    // `count_and_emit` stages a count and a request, then faults on 13.
    let body = app.world_mut().spawn((Poked(13), Tall(1.0))).id();
    step(&mut app);
    assert!(app.world().resource::<Lowered>().0.is_empty());
    assert!(app.world().get::<BodyRecords>(body).is_none());
    let faults = app.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 1);
    assert_eq!(faults.recent[0].fault, Fault::Module("thirteen".into()));
}

#[test]
fn a_missing_observation_faults_rather_than_reading_a_default() {
    let mut app = app();
    let body = app.world_mut().spawn(Poked(1)).id();
    step(&mut app);
    assert!(app.world().resource::<Lowered>().0.is_empty());
    let faults = app.world().resource::<ExtensionFaults>();
    assert_eq!(faults.recent[0].scope, body);
    assert_eq!(faults.recent[0].fault, Fault::MissingObservation(Height::KEY));
}

#[test]
#[should_panic(expected = "extension admission refused")]
fn a_refused_module_stops_the_composition() {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .install_extension_trigger::<Poke, _>(PHASE, "test", collect_pokes)
        .add_extension_module(module(vec![entry("a", vec![])]));
    app.finish();
}

#[test]
fn a_hand_driven_app_is_admitted_on_its_first_tick_and_a_second_finish_is_harmless() {
    // Most Apps in this repository are driven by `update` and never finished.
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_extension_module(module(vec![entry("a", vec![])]));
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<Lowered>()
        .init_resource::<SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<Poke, _>(PHASE, "test", collect_pokes)
        .install_extension_observation::<Height>(PHASE, "test", height_of)
        .install_extension_request::<Emit, _>(PHASE, "test", lower_emits);
    let body = app.world_mut().spawn((Poked(1), Tall(2.0))).id();
    step(&mut app);
    assert_eq!(app.world().resource::<Lowered>().0, vec![(body, Some(101), (1, 1, 2.0))]);
    app.finish();
    app.finish();
    step(&mut app);
    assert_eq!(app.world().resource::<Lowered>().0.len(), 2);
}

/// The guest half of `ambition-ext-1`, run in this process: the same bytes a
/// WebAssembly module would receive and return, without an interpreter.
struct InProcess(Vec<ModuleDescriptor>);

impl ModuleBackend for InProcess {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Result<Vec<u8>, String> {
        Ok(ambition_extension_sdk::abi::invoke(&self.0, module, entry, input))
    }
}

/// The modules as a loaded file publishes them: descriptors decoded from the
/// wire, so every entry is `EntryCode::Loaded`.
fn loaded(modules: Vec<ModuleDescriptor>) -> (Arc<dyn ModuleBackend>, Vec<ModuleDescriptor>) {
    let published = ambition_extension_sdk::abi::read_description(
        &ambition_extension_sdk::abi::describe(&modules),
    )
    .expect("a description round-trips");
    (Arc::new(InProcess(modules)), published)
}

fn loaded_app(replaces: bool, also_native: bool) -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<Lowered>()
        .init_resource::<SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<Poke, _>(PHASE, "test", collect_pokes)
        .install_extension_observation::<Height>(PHASE, "test", height_of)
        .install_extension_request::<Emit, _>(PHASE, "test", lower_emits);
    if also_native {
        app.add_extension_module(module(vec![entry("a", vec![])]));
    }
    let (backend, published) = loaded(vec![module(vec![entry("a", vec![])])]);
    app.add_loaded_extension_modules(backend, published, replaces);
    app.finish();
    app
}

#[test]
fn a_loaded_module_runs_the_same_as_its_native_build_across_the_wire() {
    let run = |mut app: App| {
        let a = app.world_mut().spawn((Poked(1), Tall(2.0))).id();
        let b = app.world_mut().spawn((Poked(13), Tall(5.0))).id();
        step(&mut app);
        step(&mut app);
        let lowered = app.world().resource::<Lowered>().0.clone();
        let count = app.world().get::<BodyRecords>(a).cloned();
        let faults = app.world().resource::<ExtensionFaults>().total;
        let b_records = app.world().get::<BodyRecords>(b).is_some();
        (lowered, count, faults, b_records)
    };
    let native = run(app());
    let wire = run(loaded_app(false, false));
    assert_eq!(native.0.len(), 2, "the premise: the native module emitted");
    assert_eq!(wire, native);
}

#[test]
fn a_loaded_module_replaces_a_native_one_only_when_it_says_so() {
    let replaced = loaded_app(true, true);
    let admitted = &replaced.world().resource::<AdmittedExtensions>().0;
    assert_eq!(admitted.entries.len(), 1);
    assert!(matches!(admitted.entries[0].runner, EntryRunner::Loaded { .. }));
    assert_eq!(admitted.replaced.len(), 1);

    let refused = std::panic::catch_unwind(|| loaded_app(false, true));
    assert!(refused.is_err(), "two modules with one key and no replacement refuse");
}

/// A loaded module's output is checked as strictly as a native one's: a
/// request to a port the entry did not declare faults the invocation.
#[test]
fn a_loaded_module_cannot_submit_to_an_undeclared_port() {
    fn sneaky(inv: &mut Invocation<'_>) -> Result<(), Fault> {
        // The guest has no `requests` check of its own beyond the SDK's, so
        // forge the descriptor the GUEST sees to declare a port the host's
        // copy does not.
        inv.submit::<Emit>((0, 0, 0.0))
    }
    let mut guest = module(vec![entry("a", vec![])]);
    guest.entries[0].run = EntryCode::Native(sneaky);
    let mut host_view = ambition_extension_sdk::abi::read_description(
        &ambition_extension_sdk::abi::describe(std::slice::from_ref(&guest)),
    )
    .unwrap();
    host_view[0].entries[0].requests.clear();
    let backend: Arc<dyn ModuleBackend> = Arc::new(InProcess(vec![guest]));

    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<Lowered>()
        .init_resource::<SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<Poke, _>(PHASE, "test", collect_pokes)
        .install_extension_observation::<Height>(PHASE, "test", height_of)
        .install_extension_request::<Emit, _>(PHASE, "test", lower_emits)
        .add_loaded_extension_modules(backend, host_view, false);
    app.finish();
    app.world_mut().spawn((Poked(1), Tall(2.0)));
    step(&mut app);
    assert!(app.world().resource::<Lowered>().0.is_empty());
    assert_eq!(
        app.world().resource::<ExtensionFaults>().recent[0].fault,
        Fault::UndeclaredRequest(Emit::KEY)
    );
}

/// The same counter module, counting by ten: a "rebuilt" module.
fn count_by_ten(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let poke = *inv.trigger::<Poke>()?;
    let height = *inv.observe::<Height>()?;
    let record = inv.state(&COUNTER)?;
    let count = record.get(COUNT).ok().and_then(Value::as_u32).unwrap_or(0) + 10;
    record.set(COUNT, Value::U32(count)).ok();
    inv.submit::<Emit>((poke, count, height))
}

#[test]
fn a_reloaded_module_takes_over_at_publication_and_keeps_its_records() {
    let mut app = loaded_app(true, true);
    let body = app.world_mut().spawn((Poked(1), Tall(2.0))).id();
    step(&mut app);

    let mut rebuilt = module(vec![entry("a", vec![])]);
    rebuilt.entries[0].run = EntryCode::Native(count_by_ten);
    let (backend, published) = loaded(vec![rebuilt]);
    reload::stage_loaded_replacement(app.world_mut(), backend, published).unwrap();
    // Staged is not published: the old code still runs.
    step(&mut app);
    assert!(reload::publish_staged_replacement(app.world_mut()));
    step(&mut app);

    let counts: Vec<u32> = app.world().resource::<Lowered>().0.iter().map(|l| l.2 .1).collect();
    // 1, 2 by the old code; 12 by the new one, from the record the old code left.
    assert_eq!(counts, [1, 2, 12]);
    assert_eq!(
        app.world().get::<BodyRecords>(body).unwrap().get(&COUNTER).unwrap().get(COUNT).unwrap(),
        &Value::U32(12)
    );
    assert!(!reload::publish_staged_replacement(app.world_mut()), "a publication spends the candidate");
}

#[test]
fn a_reload_that_reshapes_live_state_or_is_refused_leaves_the_running_code() {
    let mut app = loaded_app(true, true);
    let before = app.world().resource::<AdmittedExtensions>().0.digest;

    let mut reshaped = module(vec![entry("a", vec![])]);
    reshaped.schemas[0].fields.push(FieldDecl::new(2, "extra", FieldKind::Bool));
    let (backend, published) = loaded(vec![reshaped]);
    let err = reload::stage_loaded_replacement(app.world_mut(), backend, published).unwrap_err();
    assert!(err.contains("changed shape"), "{err}");

    let mut broken = module(vec![entry("a", vec![])]);
    broken.entries[0].requests.push(PortKey::new("test.nowhere", 1));
    let (backend, published) = loaded(vec![broken]);
    let err = reload::stage_loaded_replacement(app.world_mut(), backend, published).unwrap_err();
    assert!(err.contains("refused"), "{err}");

    assert!(!app.world().resource::<reload::StagedModuleReplacement>().is_staged());
    assert!(!reload::publish_staged_replacement(app.world_mut()));
    assert_eq!(app.world().resource::<AdmittedExtensions>().0.digest, before);
}

#[test]
fn an_idle_trigger_resets_a_reset_state_entry_without_calling_it() {
    let mut app = App::new();
    app.init_schedule(Sim);
    let mut m = module(vec![entry("a", vec![])]);
    m.entries[0].on_idle = IdlePolicy::ResetState;
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<Lowered>()
        .init_resource::<SimTick>()
        .init_resource::<ambition_time::WorldTime>()
        .install_extension_trigger::<Poke, _>(PHASE, "test", collect_pokes)
        .install_extension_observation::<Height>(PHASE, "test", height_of)
        .install_extension_request::<Emit, _>(PHASE, "test", lower_emits)
        .add_extension_module(m);
    app.finish();
    // `collect_pokes` marks a poke of 0 idle.
    let body = app.world_mut().spawn((Poked(3), Tall(1.0))).id();
    step(&mut app);
    step(&mut app);
    let count = |app: &App| {
        app.world().get::<BodyRecords>(body).unwrap().get(&COUNTER).unwrap().get(COUNT).unwrap().clone()
    };
    assert_eq!(count(&app), Value::U32(2));
    app.world_mut().get_mut::<Poked>(body).unwrap().0 = 0;
    step(&mut app);
    assert_eq!(count(&app), Value::U32(0), "an idle tick put the record back");
    assert_eq!(app.world().resource::<Lowered>().0.len(), 2, "and did not call the entry");

    // A body with no record is not given one by an idle tick.
    let fresh = app.world_mut().spawn((Poked(0), Tall(1.0))).id();
    step(&mut app);
    assert!(app.world().get::<BodyRecords>(fresh).is_none());
}
