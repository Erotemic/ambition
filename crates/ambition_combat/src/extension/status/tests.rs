//! A body's statuses in a tiny App: a native fixture module holds a status
//! from a special, counts its ticks in a record on the body and lets go of it,
//! through the real host.

use super::*;
use ambition_characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_characters::brain::ActorActionMessage;
use ambition_combat_port::{FighterSpecialPort, HoldStatus, TimeScale};
use ambition_extension_host::ExtensionHostPlugin;
use ambition_extension_sdk::{
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, SchemaKey, TriggerBinding,
};
use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const STATUS: &str = "test.counted";
const HOLD: &str = "test.hold";
const SCALE: &str = "test.scale";

record! {
    /// The fixture's state on a body: the ticks seen and the ticks it lasts.
    struct Ticks = SchemaKey::new("test", "status.ticks", 1);
    1 seen: u32,
    2 lasts: u32,
}

/// Hold the status for the special's `ticks`, starting the count again.
fn hold(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let lasts = inv.trigger::<FighterSpecialPort>()?.number("ticks").unwrap_or(1.0) as u32;
    Ticks { seen: 0, lasts }.store(inv)?;
    inv.submit::<HoldStatusPort>(HoldStatus { kind: STATUS.into(), held: true })
}

/// Count a tick, and let go when the count reaches what it lasts.
fn tick(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let mut ticks = Ticks::load(inv)?;
    ticks.seen += 1;
    ticks.store(inv)?;
    if ticks.seen >= ticks.lasts {
        inv.submit::<HoldStatusPort>(HoldStatus { kind: STATUS.into(), held: false })?;
    }
    Ok(())
}

fn scale(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let scale = inv.trigger::<FighterSpecialPort>()?.number("scale").unwrap_or(1.0);
    inv.submit::<TimeScalePort>(TimeScale { scale })
}

fn app() -> App {
    let mut app = App::new();
    app.init_schedule(Sim);
    app.add_plugins(ExtensionHostPlugin::new(Sim))
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0))
        .add_message::<ActorActionMessage>();
    crate::extension::install_for_specials(&mut app);
    let key = ModuleKey::new("test", "status");
    let entry = |name: &'static str, port, selector: &'static str, requests, after: Vec<String>, run| EntryDescriptor {
        key: name.into(),
        phase: TECHNIQUE_EXECUTION,
        trigger: TriggerBinding { port, selector: selector.into() },
        reads: vec![],
        writes: vec![Ticks::KEY],
        requests,
        after: after.into_iter().map(Into::into).collect(),
        limits: Limits { max_requests: 2 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(run),
    };
    let tick_path = format!("{key}/tick");
    app.add_extension_module(ModuleDescriptor {
        key,
        api: ambition_extension_sdk::API_VERSION,
        code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
        schemas: vec![Ticks::schema()],
        entries: vec![
            entry("tick", StatusTickPort::KEY, STATUS, vec![HoldStatusPort::KEY], vec![], tick),
            // After the tick: a press on a tick the status ends holds it again.
            entry("hold", FighterSpecialPort::KEY, HOLD, vec![HoldStatusPort::KEY], vec![tick_path], hold),
            entry("scale", FighterSpecialPort::KEY, SCALE, vec![TimeScalePort::KEY], vec![], scale),
        ],
    });
    app.finish();
    app
}

fn body(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((ae::BodyKinematics::default(), ambition_time::ProperTimeScale::default()))
        .id()
}

fn press(app: &mut App, who: Entity, key: &str, params: &str) {
    app.world_mut().write_message(ActorActionMessage {
        actor: who,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(key.into()),
            params: ambition_entity_catalog::ParamValue::parse(params).expect("fixture params parse"),
        },
        move_instance: None,
    });
}

fn step(app: &mut App) {
    app.world_mut().run_schedule(Sim);
}

fn holds(app: &App, who: Entity) -> bool {
    app.world().get::<BodyStatuses>(who).is_some_and(|held| held.holds(STATUS))
}

fn seen(app: &mut App, who: Entity) -> u32 {
    app.world()
        .get::<ambition_extension_host::BodyRecords>(who)
        .and_then(|records| records.get(&Ticks::KEY))
        .map(|record| Ticks::from_record(record).expect("the fixture's record reads").seen)
        .unwrap_or(0)
}

/// A held status is ticked once each step from the step after it was held,
/// and goes when its module lets go of it.
#[test]
fn a_held_status_is_ticked_each_step_until_its_module_lets_go() {
    let mut app = app();
    let who = body(&mut app);
    press(&mut app, who, HOLD, "(ticks: 3)");
    step(&mut app);
    assert!(holds(&app, who), "the special did not hold its status");
    assert_eq!(seen(&mut app, who), 0, "the status was ticked on the step that held it");
    step(&mut app);
    step(&mut app);
    assert!(holds(&app, who), "the status went before its module let go of it");
    assert_eq!(seen(&mut app, who), 2, "a status was not ticked once a step");
    step(&mut app);
    assert_eq!(seen(&mut app, who), 3);
    assert!(app.world().get::<BodyStatuses>(who).is_none(), "the status outlived its module letting go");
    step(&mut app);
    assert_eq!(seen(&mut app, who), 3, "a status the body no longer holds was ticked");
}

/// A let-go and a later hold of one kind on one step leave it held: the
/// requests of a step apply in the order they were submitted.
#[test]
fn a_let_go_and_a_hold_on_one_step_leave_it_held() {
    let mut app = app();
    let who = body(&mut app);
    press(&mut app, who, HOLD, "(ticks: 1)");
    step(&mut app);
    // This step's tick lets go; the press, after it, holds again.
    press(&mut app, who, HOLD, "(ticks: 2)");
    step(&mut app);
    assert!(holds(&app, who), "a hold submitted after a let-go on one step was lost");
    assert_eq!(seen(&mut app, who), 0, "the second hold did not start its count again");
}

/// A paused world ticks no status.
#[test]
fn a_paused_world_ticks_no_status() {
    let mut app = app();
    let who = body(&mut app);
    press(&mut app, who, HOLD, "(ticks: 5)");
    step(&mut app);
    app.world_mut().resource_mut::<ambition_time::WorldTime>().set_sim_dt(0.0);
    for _ in 0..3 {
        step(&mut app);
    }
    assert_eq!(seen(&mut app, who), 0, "a paused world ticked a status");
    app.world_mut().resource_mut::<ambition_time::WorldTime>().set_sim_dt(1.0 / 60.0);
    step(&mut app);
    assert_eq!(seen(&mut app, who), 1, "control: a running world ticks it");
}

/// A clock scale of nothing is refused; a slow is applied.
#[test]
fn a_clock_scale_that_stops_the_body_is_refused() {
    let mut app = app();
    let who = body(&mut app);
    press(&mut app, who, SCALE, "(scale: 0.0)");
    step(&mut app);
    let scale = |app: &App| app.world().get::<ambition_time::ProperTimeScale>(who).unwrap().0;
    assert_eq!(scale(&app), 1.0, "a scale of 0 stopped the body's clock");
    press(&mut app, who, SCALE, "(scale: 0.5)");
    step(&mut app);
    assert_eq!(scale(&app), 0.5, "control: a slow is applied");
}
