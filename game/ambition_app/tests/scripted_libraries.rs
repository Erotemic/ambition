//! Reusable source behaviors compose over native actions and host-owned instance records.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::{BodyWallet, CharacterForm},
    combat::extension::status::BodyStatuses,
    extension::{
        self, BodyRecords, ExtensionFaults, ExtensionGeneration,
        sdk::{FieldRef, SchemaKey, Value},
    },
    items::{Item, OwnedItems},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

const CLOCK: &str = r#"
const RATE = 1.0;
fn schema(key) {
    #{ key: key, fields: [
        #{ tag: 1, name: "elapsed", kind: "f32" },
        #{ tag: 2, name: "sequence", kind: "u32" },
        #{ tag: 3, name: "applied", kind: "u32" },
        #{ tag: 4, name: "refused", kind: "u32" },
        #{ tag: 5, name: "report", kind: #{ record: [
            #{ tag: 1, name: "message", kind: "text 32" },
            #{ tag: 2, name: "log", kind: #{ seq: 4, of: #{ record: [
                #{ tag: 1, name: "nonce", kind: "u32" },
                #{ tag: 2, name: "verdict", kind: "text 16" },
            ] } } },
        ] } },
    ] }
}
fn pulse(state, dt, period) {
    state.elapsed += dt * global::RATE;
    let ready = state.elapsed >= period;
    if ready {
        state.elapsed -= period;
        state.sequence += 1;
    }
    #{ state: state, ready: ready }
}
fn remember(ctx, cfg) {
    let report = ctx.state[cfg.slot].report;
    report.message = ctx.trigger.code;
    if report.log.len() == 4 { report.log.remove(0); }
    report.log.push(#{nonce: ctx.trigger.ticket.request, verdict: ctx.trigger.stage});
    ctx.state[cfg.slot].report = report;
    ctx
}
"#;

const CRAFT: &str = r#"
import "lib/clock.rhai" as clock;
fn schema(key) { clock::schema(key) }
fn tick(ctx, cfg) {
    let pulse = clock::pulse(ctx.state[cfg.slot], ctx.dt, cfg.period);
    ctx.state[cfg.slot] = pulse.state;
    if pulse.ready {
        ctx.requests.push(request("inventory_exchange", #{
            reply: #{ request: pulse.state.sequence, selector: cfg.selector },
            exchange: #{ grants: [#{ id: cfg.item, count: 1 }], pay_coins: cfg.pay },
        }));
    }
    ctx
}
fn result(ctx, cfg) {
    if ctx.trigger.ticket.domain != "inventory.exchange" || !ctx.trigger.terminal { return ctx; }
    if ctx.trigger.stage == "applied" {
        if ctx.trigger.code != "inventory_exchanged" { throw "a trade did not apply"; }
        ctx.state[cfg.slot].applied += 1;
    } else if ctx.trigger.stage == "rejected" {
        ctx.state[cfg.slot].refused += 1;
    } else { throw "unexpected trade outcome"; }
    clock::remember(ctx, cfg)
}
"#;

const TURRET: &str = r#"
import "lib/clock.rhai" as clock;
fn schema(key) { clock::schema(key) }
fn tick(ctx, cfg) {
    let pulse = clock::pulse(ctx.state[cfg.slot], ctx.dt, cfg.period);
    ctx.state[cfg.slot] = pulse.state;
    if pulse.ready {
        ctx.requests.push(request("projectile", #{
            reply: #{ request: pulse.state.sequence, selector: cfg.selector },
            origin: [ctx.trigger.position[0], ctx.trigger.position[1] - 60.0],
            dir: [0.0, -1.0], speed: 20.0, damage: 1, max_lifetime: 3.0,
            half_extent: [2.0, 2.0], visual_id: "",
        }));
    }
    ctx
}
fn result(ctx, cfg) {
    if ctx.trigger.ticket.domain != "projectiles.spawn" || !ctx.trigger.terminal { return ctx; }
    if ctx.trigger.stage != "applied" || ctx.trigger.code != "projectile_created"
        || ctx.trigger.subjects.len() != 1 { throw "a shot was not created"; }
    ctx.state[cfg.slot].applied += 1;
    clock::remember(ctx, cfg)
}
"#;

const ROOT: &str = r#"
import "lib/craft.rhai" as craft;
import "lib/turret.rhai" as turret;
const NAME = "__NAME__";
const CRAFT_PERIOD = __CRAFT_PERIOD__;
const TURRET_PERIOD = __TURRET_PERIOD__;
fn declare() {
    let c = global::NAME + ".craft";
    let t = global::NAME + ".turret";
    #{ provider: "library_probe", key: global::NAME,
       schemas: [craft::schema(c), turret::schema(t)], entries: [
        #{ key: "tick", phase: "technique_execution",
           trigger: #{ port: "status_tick", selector: "library_probe.tick" },
           writes: [c, t], requests: ["inventory_exchange", "projectile"], limit: 2 },
        #{ key: "result", phase: "actor_think",
           trigger: #{ port: "action_result", selector: "result" }, writes: [c, t] },
    ] }
}
fn craft_options() {
    #{ slot: "craft", period: global::CRAFT_PERIOD, item: "manacell", pay: 1,
       selector: "library_probe::" + global::NAME + "/result" }
}
fn turret_options() {
    #{ slot: "turret", period: global::TURRET_PERIOD,
       selector: "library_probe::" + global::NAME + "/result" }
}
fn tick() {
    this = craft::tick(this, craft_options());
    this = turret::tick(this, turret_options());
}
fn result() {
    this = craft::result(this, craft_options());
    this = turret::result(this, turret_options());
}
"#;

fn root(name: &str) -> String {
    ROOT.replace("__NAME__", name)
        .replace(
            "__CRAFT_PERIOD__",
            if name == "alpha" { "0.1" } else { "0.2" },
        )
        .replace(
            "__TURRET_PERIOD__",
            if name == "alpha" { "0.15" } else { "0.3" },
        )
}

struct Sources(std::path::PathBuf);
impl Sources {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("ambition_script_libraries_{}_{serial}", std::process::id()));
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        for (name, text) in [
            ("lib/clock.rhai", CLOCK),
            ("lib/craft.rhai", CRAFT),
            ("lib/turret.rhai", TURRET),
        ] {
            std::fs::write(dir.join(name), text).unwrap();
        }
        for name in ["alpha", "beta"] {
            std::fs::write(dir.join(format!("{name}.rhai")), root(name)).unwrap();
        }
        Self(dir)
    }
    fn edit(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).unwrap();
    }
}
impl Drop for Sources {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn options() -> ambition_app::rl_sim::Platformer2dSimHarnessOptions {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    crate::common::fixed_60hz_room_options("intro_wake_room").with_save(save)
}

fn setup(sim: &mut Platformer2dSimHarness) -> [Entity; 2] {
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let primary = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let creator = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, form)| form.id() == "npc_creator")
        .unwrap()
        .0;
    world.insert_resource(OwnedItems::default());
    for body in [primary, creator] {
        world.entity_mut(body).insert((
            BodyWallet { balance: 1000 },
            BodyStatuses(vec!["library_probe.tick".into()]),
        ));
    }
    sim.rebase_rollback_history().unwrap();
    [primary, creator]
}

fn pause(sim: &mut Platformer2dSimHarness, bodies: [Entity; 2]) {
    for body in bodies {
        sim.world_mut().entity_mut(body).remove::<BodyStatuses>();
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 4);
}

fn resume(sim: &mut Platformer2dSimHarness, bodies: [Entity; 2]) {
    for body in bodies {
        sim.world_mut()
            .entity_mut(body)
            .insert(BodyStatuses(vec!["library_probe.tick".into()]));
    }
    sim.rebase_rollback_history().unwrap();
}

fn applied(sim: &Platformer2dSimHarness, body: Entity, module: &str, component: &str) -> u32 {
    let record = sim
        .world()
        .get::<BodyRecords>(body)
        .unwrap()
        .get(&SchemaKey {
            provider: "library_probe".into(),
            key: format!("{module}.{component}").into(),
            revision: 1,
        })
        .unwrap();
    match record.get(FieldRef(2)).unwrap() {
        Value::U32(count) => *count,
        value => panic!("unexpected {value:?}"),
    }
}

fn check_actions(sim: &Platformer2dSimHarness, bodies: [Entity; 2]) -> Vec<u32> {
    let mut counts = Vec::new();
    let mut granted = 0;
    for body in bodies {
        let mut paid = 0;
        for module in ["alpha", "beta"] {
            let craft = applied(sim, body, module, "craft");
            let shots = applied(sim, body, module, "turret");
            assert!(
                craft > 0 && shots > 0,
                "each behavior instance must receive its native outcomes"
            );
            paid += craft;
            granted += craft;
            counts.extend([craft, shots]);
        }
        assert_eq!(
            sim.world().get::<BodyWallet>(body).unwrap().balance,
            1000 - paid as i32
        );
    }
    assert_eq!(
        sim.world().resource::<OwnedItems>().count(Item::ManaCell),
        granted
    );
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    counts
}

fn reports(sim: &Platformer2dSimHarness, bodies: [Entity; 2]) -> Vec<Value> {
    bodies.into_iter().flat_map(|body| ["alpha", "beta"].into_iter().flat_map(move |module| {
        ["craft", "turret"].into_iter().map(move |component| {
            sim.world().get::<BodyRecords>(body).unwrap().get(&SchemaKey {
                provider: "library_probe".into(), key: format!("{module}.{component}").into(), revision: 1,
            }).unwrap().get(FieldRef(4)).unwrap().clone()
        })
    })).collect()
}

#[test]
fn editing_nested_state_preserves_live_instances_and_refuses_narrower_fields() {
    let sources = Sources::new();
    let mut sim = Platformer2dSimHarness::new_with_options(
        options().with_extension_module_files(vec![sources.0.clone()]),
    ).unwrap();
    let bodies = setup(&mut sim);
    sim.step_n(AgentAction::default(), 20);
    pause(&mut sim, bodies);
    check_actions(&sim, bodies);
    let before = reports(&sim, bodies);
    for value in &before {
        let fields = value.as_record().unwrap().values();
        assert!(!fields[0].as_text().unwrap().is_empty());
        assert!(matches!(&fields[1], Value::Seq(log) if !log.is_empty() && log.len() <= 4));
    }
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    let changed = CLOCK.replace(
        "#{ tag: 1, name: \"message\", kind: \"text 32\" },", ""
    ).replace(
        "name: \"log\", kind:", "name: \"history\", kind:"
    ).replace(
        "        ] } },", "            #{ tag: 1, name: \"label\", kind: \"text 64\" },\n            #{ tag: 3, name: \"ready\", kind: \"bool\", initial: true },\n        ] } },"
    ).replace("report.message", "report.label").replace("report.log", "report.history");
    sources.edit("lib/clock.rhai", &changed);
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), generation);
    let expected: Vec<_> = before.into_iter().map(|value| {
        let fields = value.as_record().unwrap().values();
        Value::Record(extension::sdk::Record::from_values(vec![
            fields[1].clone(), fields[0].clone(), Value::Bool(true),
        ]))
    }).collect();
    assert_eq!(reports(&sim, bodies), expected);
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    for invalid in [changed.replace("text 64", "text 31"), changed.replace("\"nonce\", kind: \"u32\"", "\"nonce\", kind: \"i32\"")] {
        sources.edit("lib/clock.rhai", &invalid);
        sim.step_n(AgentAction::default(), 50);
        assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
        assert_eq!(reports(&sim, bodies), expected);
    }
    sources.edit("lib/clock.rhai", &changed);
    sim.step_n(AgentAction::default(), 50);
    resume(&mut sim, bodies);
    sim.step_n(AgentAction::default(), 20);
    pause(&mut sim, bodies);
    check_actions(&sim, bodies);
}

#[test]
fn source_behaviors_compose_for_both_controllers_and_replay_without_files() {
    let prepared: Vec<_> = ["alpha", "beta"]
        .into_iter()
        .map(|name| {
            extension::PreparedScript::with_libraries(
                name,
                &root(name),
                extension::script_ports(),
                200_000,
                Default::default(),
                |id| {
                    let text = match id {
                        "lib/clock.rhai" => CLOCK,
                        "lib/craft.rhai" => CRAFT,
                        "lib/turret.rhai" => TURRET,
                        _ => return Err(format!("missing {id}")),
                    };
                    Ok(extension::LibrarySource {
                        name: id.into(),
                        text: text.into(),
                    })
                },
            )
            .unwrap()
        })
        .collect();
    let mut sim = Platformer2dSimHarness::build(
        options().with_sync_test_rollback_settings(4, 12),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.init_resource::<extension::ExtensionProfile>();
            for (name, source) in ["alpha", "beta"].into_iter().zip(&prepared) {
                extension::add_prepared_script(app, name, source);
            }
            Ok(())
        },
    )
    .unwrap();
    let bodies = setup(&mut sim);
    sim.step_n(AgentAction::default(), 20);
    pause(&mut sim, bodies);
    check_actions(&sim, bodies);
    for (path, row) in &sim.world().resource::<extension::ExtensionProfile>().0 {
        if path.starts_with("library_probe::") && row.calls > 0 {
            println!(
                "SCRIPT_LIBRARY_COST entry={path} calls={} backend_us={} host_us={}",
                row.calls,
                row.backend.as_micros() as f64 / row.calls as f64,
                row.total.as_micros() as f64 / row.calls as f64
            );
        }
    }
}

#[test]
fn editing_a_shared_library_reloads_every_dependent_and_recovers_missing_sources() {
    let sources = Sources::new();
    let mut sim = Platformer2dSimHarness::new_with_options(
        options().with_extension_module_files(vec![sources.0.clone()]),
    )
    .unwrap();
    let bodies = setup(&mut sim);
    sim.step_n(AgentAction::default(), 20);
    pause(&mut sim, bodies);
    let before = check_actions(&sim, bodies);
    let generation = sim.world().resource::<ExtensionGeneration>().clone();
    sources.edit("lib/clock.rhai", "fn pulse() { invalid syntax");
    sim.step_n(AgentAction::default(), 50);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    let with_rate = CLOCK
        .replace("const RATE = 1.0;", "")
        .replace("global::RATE", "rate::value()")
        .replace(
            "name: \"elapsed\", kind: \"f32\"",
            "name: \"elapsed\", kind: rate::elapsed_kind()",
        );
    sources.edit(
        "lib/clock.rhai",
        &format!("import \"lib/rate.rhai\" as rate; {with_rate}"),
    );
    sim.step_n(AgentAction::default(), 50);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    sources.edit(
        "lib/rate.rhai",
        "fn value() { 2.0 } fn elapsed_kind() { \"u32\" }",
    );
    sim.step_n(AgentAction::default(), 50);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), generation);
    sources.edit(
        "lib/rate.rhai",
        "fn value() { 2.0 } fn elapsed_kind() { \"f32\" }",
    );
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(*sim.world().resource::<ExtensionGeneration>(), generation);
    let compiled = sim.world().resource::<ExtensionGeneration>().clone();
    std::fs::remove_file(sources.0.join("lib/rate.rhai")).unwrap();
    sim.step_n(AgentAction::default(), 50);
    assert_eq!(*sim.world().resource::<ExtensionGeneration>(), compiled);
    resume(&mut sim, bodies);
    sim.step_n(AgentAction::default(), 20);
    pause(&mut sim, bodies);
    let after = check_actions(&sim, bodies);
    for (old, new) in before.into_iter().zip(after) {
        assert!(
            new - old > old,
            "both behaviors and both modules must use the faster shared library"
        );
    }
}
