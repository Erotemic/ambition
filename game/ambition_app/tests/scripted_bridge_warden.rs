//! THE BRIDGE WARDEN: the script-first program's capstone mechanism, written
//! in content and Rhai with no Rust adapter.
//!
//! In `blink_run` a script creates a warden and two posts, each a body with
//! its own state. The warden watches the posts: when one falls (a native
//! defeat) it counts it in the world (`add_world_count`) and, once the
//! count it reads back says both are down, lowers the bridge
//! (`set_world_value` with an expected revision), hearing every outcome.
//! The caller, which made the posts, clears each fallen one
//! (`retire_creation`), and the warden hears that too (`object_retired`). In
//! `portal_bridge` a wall waits on the world value (`world.value_is`), so the
//! consequence is in the next room, read back there by its own script
//! (`lock_walls`). Variations change the composition, not the engine: the
//! gate can wait on the count itself, and the warden's methods can be a
//! portable module.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

pub const SOURCE: &str = r#"
fn declare() {
    #{provider: "bridge", key: "warden", schemas: [
        #{key: "caller", fields: [
            #{tag: 1, name: "step", kind: "u32"},
            #{tag: 2, name: "me", kind: "opt object"},
            #{tag: 3, name: "made", kind: "seq 4 object"},
            #{tag: 4, name: "attached", kind: "u32"},
        ]},
        #{key: "post", fields: [
            #{tag: 1, name: "label", kind: "text 16"},
            #{tag: 2, name: "ticks", kind: "u32"},
        ]},
        #{key: "ward", fields: [
            #{tag: 1, name: "downs", kind: "u32"},
            #{tag: 2, name: "gone", kind: "u32"},
            #{tag: 3, name: "asked", kind: "bool"},
            #{tag: 4, name: "log", kind: "text 256"},
        ]},
        #{key: "clearer", fields: [
            #{tag: 1, name: "cleared", kind: "u32"},
            #{tag: 2, name: "log", kind: "text 128"},
        ]},
        #{key: "far", fields: [
            #{tag: 1, name: "trail", kind: "text 64"},
        ]},
    ], entries: [
        #{key: "run", phase: "technique_execution", trigger: #{port: "status_tick", selector: "bridge.run"},
          reads: ["credit"], writes: ["caller", "post"], requests: ["body_query", "create_actors", "blast"], limit: 2},
        #{key: "found", phase: "actor_think", trigger: #{port: "action_result", selector: "found"},
          reads: ["body_query_result"], writes: ["caller"]},
        #{key: "made", phase: "actor_think", trigger: #{port: "action_result", selector: "made"},
          writes: ["caller"], requests: ["attach_behaviors"], limit: 2},
        #{key: "attached", phase: "actor_think", trigger: #{port: "action_result", selector: "attached"}, writes: ["caller"]},
        #{key: "post", phase: "technique_execution", trigger: #{port: "status_tick", selector: "bridge.post"}, writes: ["post"]},
        #{key: "down", phase: "actor_think", trigger: #{port: "object_defeated", selector: "defeated"},
          writes: ["ward"], requests: ["add_world_count"], limit: 1},
        #{key: "counted", phase: "actor_think", trigger: #{port: "action_result", selector: "counted"},
          reads: ["world_values"], writes: ["ward"], requests: ["set_world_value"], limit: 1},
        #{key: "lowered", phase: "actor_think", trigger: #{port: "action_result", selector: "lowered"}, writes: ["ward"]},
        #{key: "gone", phase: "actor_think", trigger: #{port: "object_retired", selector: "retired"}, writes: ["ward"]},
        #{key: "clear", phase: "actor_think", trigger: #{port: "object_defeated", selector: "defeated"},
          writes: ["clearer"], requests: ["retire_creation"], limit: 1},
        #{key: "cleared", phase: "actor_think", trigger: #{port: "action_result", selector: "cleared"}, writes: ["clearer"]},
        #{key: "far", phase: "actor_think", trigger: #{port: "story_tick", selector: "portal_bridge"},
          reads: ["lock_walls"], writes: ["far"]},
    ]}
}
fn actor(key, character, at, allegiance, holds, state) {
    #{key: key, character: character, at: at, allegiance: allegiance, holds: holds, state: state}
}
fn post(label, p, dx) {
    actor(label, "goblin", [p[0] + dx, p[1] - 40.0], "enemy", ["bridge.post"],
        [#{schema: "post", value: #{label: label, ticks: 0}}])
}
fn run() {
    let caller = this.state.caller;
    caller.step += 1;
    let p = this.trigger.position;
    if caller.step == 1 {
        this.requests.push(request("body_query", #{reply: #{request: 1, selector: "bridge::warden/found"},
            origin: p, radius: 64.0, limit: 4, include_self: true}));
    }
    if caller.step == 3 {
        this.requests.push(request("create_actors", #{reply: #{request: 1, selector: "bridge::warden/made"},
            actors: [actor("warden", "practice_sandbag", [p[0] - 200.0, p[1] - 40.0], "neutral", [], []),
                post("north", p, 260.0), post("south", p, 420.0)]}));
    }
    // Strike both posts where they were made (260 and 420 to the right),
    // before they walk off: native blasts, credited to the caller.
    if caller.attached == 2 && caller.step == 25 {
        for dx in [260.0, 420.0] {
            this.requests.push(request("blast", #{center: [p[0] + dx, p[1] - 40.0], half_extent: [72.0, 160.0],
                damage: 999, knockback: 0.0, lifetime_s: 0.05, credit: this.observe.credit}));
        }
    }
    this.state.caller = caller;
}
fn found() {
    if this.trigger.stage != "applied" { return; }
    for target in this.observe.body_query_result.targets {
        if target.driven { this.state.caller.me = target.object; }
    }
}
fn made() {
    if !this.trigger.terminal { return; }
    if this.trigger.code != "actors_created" { throw this.trigger.message; }
    let made = this.trigger.subjects.map(|subject| subject.object);
    this.state.caller.made = made;
    let posts = #{north: made[1], south: made[2]};
    let watch = [#{key: "north", source: "north", method: "down"}, #{key: "south", source: "south", method: "down"},
        #{key: "north_gone", source: "north", method: "gone"}, #{key: "south_gone", source: "south", method: "gone"}];
    this.requests.push(request("attach_behaviors", #{target: made[0], bindings: [#{key: "ward", references: posts,
        methods: [#{name: "down", entry: "bridge::warden/down"}, #{name: "gone", entry: "bridge::warden/gone"}],
        subscriptions: watch}], reply: #{request: 1, selector: "bridge::warden/attached"}}));
    this.requests.push(request("attach_behaviors", #{target: this.state.caller.me, bindings: [#{key: "clearer",
        references: posts, methods: [#{name: "clear", entry: "bridge::warden/clear"}],
        subscriptions: [#{key: "north", source: "north", method: "clear"}, #{key: "south", source: "south", method: "clear"}]}],
        reply: #{request: 2, selector: "bridge::warden/attached"}}));
}
fn attached() {
    if this.trigger.stage == "rejected" { throw this.trigger.message; }
    if this.trigger.terminal { this.state.caller.attached += 1; }
}
fn post() { this.state.post.ticks += 1; }
fn down() {
    this.state.ward.downs += 1;
    this.state.ward.log += "down;";
    this.requests.push(request("add_world_count", #{key: "bridge.posts_down", delta: 1,
        reply: #{request: this.state.ward.downs, selector: "bridge::warden/counted"}}));
}
fn count_of(values, key) {
    for row in values { if row.key == key { return row.value; } }
    0
}
fn counted() {
    if !this.trigger.terminal { return; }
    let ward = this.state.ward;
    ward.log += this.trigger.code + ";";
    // LOWER: the warden reads the count the world holds, not its own tally.
    if !ward.asked && count_of(this.observe.world_values.values, "bridge.posts_down") >= 2 {
        ward.asked = true;
        this.requests.push(request("set_world_value", #{key: "bridge.state", value: "lowered", expect: 0,
            reply: #{request: 1, selector: "bridge::warden/lowered"}}));
    }
    this.state.ward = ward;
}
fn lowered() {
    if this.trigger.terminal { this.state.ward.log += this.trigger.code + ";"; }
}
fn gone() {
    this.state.ward.gone += 1;
    this.state.ward.log += "gone;";
}
fn clear() {
    this.state.clearer.cleared += 1;
    this.requests.push(request("retire_creation", #{object: this.trigger.object,
        reply: #{request: this.state.clearer.cleared, selector: "bridge::warden/cleared"}}));
}
fn cleared() {
    if this.trigger.terminal { this.state.clearer.log += this.trigger.code + ";"; }
}
fn far() {
    for wall in this.observe.lock_walls.walls {
        if wall.id == "warden_gate" && this.state.far.trail.len() < 8 {
            this.state.far.trail += if wall.standing { "S" } else { "o" };
        }
    }
}
"#;

pub const GATE: &str = "world.value_is bridge.state lowered";

/// Run the mechanism in `blink_run`, with the gate in `portal_bridge` waiting
/// on `gate`, then walk across. Returns the harness, the player, and the
/// records it reads.
pub fn play(source: &str, gate: &str, extra: Vec<std::path::PathBuf>, label: &str) -> (Platformer2dSimHarness, Entity, std::time::Instant) {
    use ambition_platformer2d::world::rooms::{EncounterLockWallSpec, RoomSet};
    let path = std::env::temp_dir().join(format!("ambition_bridge_warden_{label}_{}.rhai", std::process::id()));
    std::fs::write(&path, source).unwrap();
    let mut modules = extra;
    modules.push(path);
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options("blink_run")
        .with_extension_module_files(modules)
        .with_sync_test_rollback_settings(4, 12)).unwrap();
    crate::common::doors_cross_at_once(&mut sim);
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    {
        let mut rooms = ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<RoomSet>(world).unwrap();
        let index = rooms.room_index_by_id("portal_bridge").unwrap();
        let room = &mut rooms.rooms[index];
        // In the room's top corner, where no arrival stands.
        let min = ambition_platformer2d::engine_core::Vec2::new(8.0, 8.0);
        room.lock_walls.push(EncounterLockWallSpec {id: "warden_gate".into(), gated_by: Some(gate.into()),
            look: None, min, size: ambition_platformer2d::engine_core::Vec2::new(16.0, 64.0)});
    }
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["bridge.run".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.world_mut().insert_resource(ambition_platformer2d::extension::ExtensionProfile::default());
    let started = std::time::Instant::now();
    sim.step_n(AgentAction::default(), 80);
    (sim, player, started)
}

fn records(sim: &mut Platformer2dSimHarness, schema: &'static str) -> Vec<ambition_platformer2d::extension::sdk::Record> {
    let world = sim.world_mut();
    world.query::<&BodyRecords>().iter(world)
        .filter_map(|records| records.get(&SchemaKey::new("bridge", schema, 1)).cloned()).collect()
}

fn world_value(sim: &Platformer2dSimHarness, key: &str) -> Option<Vec<u8>> {
    sim.world().resource::<ambition_platformer2d::session::AmbitionGameSave>().data().world_value(key)
        .map(|row| row.value.clone())
}

/// The far room's own script reads the gate there after the crossing.
fn far_trail(sim: &mut Platformer2dSimHarness) -> String {
    assert_eq!(crate::common::walk_through_the_door_to(sim, "portal_bridge"), "portal_bridge");
    sim.step_n(AgentAction::default(), 20);
    let far = records(sim, "far");
    assert_eq!(far.len(), 1, "the far room's script ran");
    let Value::Text(trail) = far[0].get(FieldRef(0)).unwrap().clone() else { panic!("trail") };
    trail
}

/// ⭐ THE CAPSTONE. Several stateful bodies, a warden that reacts to native
/// defeats and retirements and handles each outcome, the caller retiring
/// its creations through their owner, and a wall in the next room that the
/// confirmed world value opens. Sync-test host: every outcome survives the
/// rewinds it rides through.
#[test]
fn the_warden_lowers_the_bridge_and_the_next_room_opens() {
    let (mut sim, player, started) = play(SOURCE, GATE, Vec::new(), "rhai");
    cost(&sim, "bridge::warden/", "capstone_rhai", 80, started);
    sim.rollback_health().unwrap();
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let caller = sim.world().get::<BodyRecords>(player).unwrap().get(&SchemaKey::new("bridge", "caller", 1)).cloned().unwrap();
    assert_eq!(caller.get(FieldRef(3)).unwrap(), &Value::U32(2), "the premise: both watches attached");
    let posts = records(&mut sim, "post");
    assert!(posts.is_empty(), "both posts were retired with their state");
    let ward = records(&mut sim, "ward");
    assert_eq!(ward.len(), 1, "one warden");
    assert_eq!(ward[0].get(FieldRef(0)).unwrap(), &Value::U32(2), "both defeats heard");
    assert_eq!(ward[0].get(FieldRef(1)).unwrap(), &Value::U32(2), "both retirements heard");
    let Value::Text(log) = ward[0].get(FieldRef(3)).unwrap().clone() else { panic!("log") };
    assert!(log.contains("world_count_added;") && log.contains("world_value_set;"), "the warden heard its writes: {log}");
    let clearer = records(&mut sim, "clearer");
    assert_eq!(clearer[0].get(FieldRef(1)).unwrap(), &Value::Text("creation_retired;creation_retired;".into()));
    assert!(world_value(&sim, "bridge.state").is_some(), "the bridge's state is in the save");
    assert_eq!(far_trail(&mut sim), "oooooooo", "the next room's wall is open, read by that room's own script");
    sim.rollback_health().unwrap();
}

/// A VARIATION WITH NO ADAPTER: the gate waits on the count itself
/// (`world.count_at_least`), and the warden never lowers anything.
#[test]
fn a_gate_that_waits_on_the_count_opens_without_the_wardens_write() {
    let source = SOURCE.replace("if !ward.asked && count_of", "if false && count_of");
    let (mut sim, _, _) = play(&source, "world.count_at_least bridge.posts_down 2", Vec::new(), "count");
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    assert!(world_value(&sim, "bridge.state").is_none(), "the premise: nothing lowered the bridge");
    assert_eq!(far_trail(&mut sim), "oooooooo", "the count alone opens the gate");
    sim.rollback_health().unwrap();
}

/// ⭐ PORTABLE PARITY: the warden's methods as a WASM module
/// (`fixture::bridge_warden`, built from source here) bound in place of the
/// Rhai ones, with the rest of the composition unchanged. Same defeats, same
/// world writes, same gate in the next room.
#[test]
fn the_wardens_methods_as_a_portable_module_lower_the_same_bridge() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let guest = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture module crate builds");
    let source = SOURCE.replace("#{name: \"down\", entry: \"bridge::warden/down\"}",
        "#{name: \"down\", entry: \"fixture::bridge_warden/down\"}");
    assert_ne!(source, SOURCE, "the premise: the binding names the portable method");
    let (mut sim, _, started) = play(&source, GATE, vec![guest], "wasm");
    cost(&sim, "fixture::bridge_warden/", "capstone_wasm", 80, started);
    cost(&sim, "bridge::warden/", "capstone_wasm_rhai_rest", 80, started);
    sim.rollback_health().unwrap();
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let world = sim.world_mut();
    let ward: Vec<_> = world.query::<&BodyRecords>().iter(world)
        .filter_map(|records| records.get(&SchemaKey::new("fixture", "bridge_warden.ward", 1)).cloned()).collect();
    assert_eq!(ward.len(), 1, "one portable warden");
    assert_eq!(ward[0].get(FieldRef(0)).unwrap(), &Value::U32(2), "both defeats heard by the portable method");
    let Value::Text(log) = ward[0].get(FieldRef(2)).unwrap().clone() else { panic!("log") };
    assert_eq!(log, "down;down;world_count_added;world_count_added;world_value_set;",
        "the same outcomes, in the same order, as the Rhai warden hears");
    let rhai_ward = records(&mut sim, "ward");
    assert_eq!(rhai_ward[0].get(FieldRef(1)).unwrap(), &Value::U32(2), "the Rhai `gone` method still hears both retirements");
    assert!(world_value(&sim, "bridge.state").is_some(), "the bridge's state is in the save");
    assert_eq!(far_trail(&mut sim), "oooooooo", "the next room's wall is open");
    sim.rollback_health().unwrap();
}

/// Print what each entry of `provider` cost: calls, faults, the whole entry
/// and the backend call, beside the ticks and the session's wall time.
fn cost(sim: &Platformer2dSimHarness, provider: &str, label: &str, ticks: u64, started: std::time::Instant) {
    use ambition_platformer2d::extension::ExtensionProfile;
    let profile = sim.world().resource::<ExtensionProfile>();
    let (mut calls, mut total, mut backend) = (0, std::time::Duration::ZERO, std::time::Duration::ZERO);
    for (entry, row) in profile.0.iter().filter(|(entry, _)| entry.starts_with(provider)) {
        println!("BRIDGE_COST {label} entry={entry} calls={} faults={} total_us={} backend_us={}",
            row.calls, row.faults, row.total.as_micros(), row.backend.as_micros());
        calls += row.calls;
        total += row.total;
        backend += row.backend;
    }
    println!("BRIDGE_COST {label} all ticks={ticks} calls={calls} total_us={} backend_us={} session_us={}",
        total.as_micros(), backend.as_micros(), started.elapsed().as_micros());
}

/// The project, built by the host as `ambition_game_bin --project` builds
/// it, with its module files replaced by `modules` when there are any. On a
/// sync-test host when `sync_test`: a reload is refused there (its timeline
/// is the harness's), so the edit loop runs on the game's own session.
fn project(modules: Vec<std::path::PathBuf>, sync_test: bool) -> Platformer2dSimHarness {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = ambition_app::app::project::Project::load(&root.join("examples/projects/bridge_warden"))
        .expect("the example project loads");
    let mut options = crate::common::fixed_60hz_room_options("warden_yard").with_extension_module_files(modules);
    if sync_test {
        options = options.with_sync_test_rollback_settings(4, 12);
    }
    let mut sim = project.sim(options).expect("the project builds");
    assert_eq!(sim.observation().active_room, "warden_yard");
    sim.world_mut().insert_resource(ambition_platformer2d::extension::ExtensionProfile::default());
    sim
}

fn standing_posts(sim: &mut Platformer2dSimHarness) -> Vec<f32> {
    let world = sim.world_mut();
    world.query::<(&ambition_platformer2d::characters::actor::CharacterForm,
        &ambition_platformer2d::engine_core::BodyKinematics, &ambition_platformer2d::characters::actor::BodyHealth)>()
        .iter(world).filter(|(form, _, health)| form.id() == "bridge_post" && health.alive())
        .map(|(_, kin, _)| kin.pos.x).collect()
}

/// Play the yard as a player would: walk to the nearest standing post and
/// swing at it until the bridge is lowered. Returns the frames it took.
fn fight(sim: &mut Platformer2dSimHarness) -> u64 {
    let lowered = |sim: &Platformer2dSimHarness| world_value(sim, "bridge.state").is_some();
    let mut frames = 0;
    while !lowered(sim) && frames < 1200 {
        let at = sim.observation().player_pos.0;
        let nearest = standing_posts(sim).into_iter().reduce(f32::min);
        let action = match nearest {
            Some(post) if at < post - 44.0 => AgentAction {move_x: 1.0, right_pressed: true, ..Default::default()},
            _ => { let swing = frames % 12 == 0; AgentAction {attack: swing, attack_held: swing, ..Default::default()} }
        };
        sim.step(action);
        frames += 1;
    }
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    assert!(lowered(sim), "the runner felled the posts and the warden lowered the bridge ({frames} frames): {:?}",
        records(sim, "ward"));
    // The write's answer reaches the warden on a later tick.
    sim.step_n(AgentAction::default(), 4);
    frames + 4
}

fn ward_log(sim: &mut Platformer2dSimHarness) -> String {
    let ward = records(sim, "ward");
    let Value::Text(log) = ward[0].get(FieldRef(3)).unwrap().clone() else { panic!("log") };
    log
}

/// Walk east through the edge and on past the gate; the bridge room's own
/// script reads the gate there.
fn cross(sim: &mut Platformer2dSimHarness) {
    let mut frames = 0;
    while sim.observation().active_room != "warden_bridge" && frames < 600 {
        sim.step(AgentAction {move_x: 1.0, right_pressed: true, ..Default::default()});
        frames += 1;
    }
    assert_eq!(sim.observation().active_room, "warden_bridge", "the edge leads to the bridge");
    sim.step_n(AgentAction {move_x: 1.0, right_pressed: true, ..Default::default()}, 180);
    let far = records(sim, "far");
    assert_eq!(far[0].get(FieldRef(0)).unwrap(), &Value::Text("o".into()), "the bridge room reads its gate open");
    let x = sim.observation().player_pos.0;
    assert!(x > 500.0, "the runner passed the gate at x 480..496, room pixels (at x {x})");
}

/// ⭐ THE PLAYABLE ROOM: `examples/projects/bridge_warden`, run as a project
/// by the built host. No status is planted and no blast is scripted: the
/// runner walks into the yard swinging, the posts fall to its strikes, the
/// warden lowers the bridge, and the runner walks through the edge and past
/// the gate that opened on the world value. Sync-test host.
#[test]
fn the_bridge_warden_project_is_played_through_by_its_runner() {
    let mut sim = project(Vec::new(), true);
    let started = std::time::Instant::now();
    let frames = fight(&mut sim);
    cost(&sim, "bridge::", "project_sync_test", frames, started);
    assert_eq!(ward_log(&mut sim), "down;world_count_added;down;world_count_added;world_value_set;",
        "each post counted as it fell, and the warden heard its write");
    cross(&mut sim);
    sim.rollback_health().unwrap();
}

/// ⭐ THE EDIT LOOP, IN THE RUNNING HOST: the project's script is edited
/// while the yard runs (one fallen post now lowers the bridge). The host
/// reloads it with no build, the warden keeps its state, and the next post
/// to fall lowers the bridge while the other still stands.
#[test]
fn an_edit_to_the_wardens_script_lowers_the_bridge_on_one_post() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let shipped = std::fs::read_to_string(root.join("examples/projects/bridge_warden/modules/bridge_warden.rhai")).unwrap();
    let dir = root.join(format!("target/extension-modules/bridge_warden_edit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("bridge_warden.rhai");
    std::fs::write(&file, &shipped).unwrap();
    let mut sim = project(vec![file.clone()], false);
    sim.step_n(AgentAction::default(), 30);
    assert_eq!(standing_posts(&mut sim).len(), 2, "the premise: the yard made both posts");
    let edited = shipped.replace("fn posts_to_lower() { 2 }", "fn posts_to_lower() { 1 }");
    assert_ne!(edited, shipped, "the premise: the edit changes the threshold");
    // A file system's clock can be coarse: the edit must look newer.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let edited_at = std::time::Instant::now();
    std::fs::write(&file, edited).unwrap();
    let published = |sim: &Platformer2dSimHarness| {
        sim.world().resource::<ambition_platformer2d::extension::reload::StagedModuleReplacement>().published()
    };
    let mut frames = 0;
    while published(&sim) == 0 {
        sim.step(AgentAction::default());
        frames += 1;
        assert!(frames < 240, "the edited script was not reloaded within 240 frames");
    }
    println!("BRIDGE_EDIT source_edit_to_published_ms={} frames={frames}", edited_at.elapsed().as_millis());
    assert_eq!(standing_posts(&mut sim).len(), 2, "the reload made no second yard");
    sim.world_mut().insert_resource(ambition_platformer2d::extension::ExtensionProfile::default());
    let started = std::time::Instant::now();
    let fought = fight(&mut sim);
    cost(&sim, "bridge::", "project_local_after_edit", fought, started);
    assert_eq!(ward_log(&mut sim), "down;world_count_added;world_value_set;", "one fallen post lowers the bridge now");
    assert_eq!(standing_posts(&mut sim).len(), 1, "the other post still stands");
    println!("BRIDGE_EDIT source_edit_to_lowered_ms={}", edited_at.elapsed().as_millis());
    cross(&mut sim);
    let _ = std::fs::remove_dir_all(dir);
}
