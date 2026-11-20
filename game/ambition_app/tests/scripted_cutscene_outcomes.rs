//! A script asks for a cutscene and hears what playback did with it: queued,
//! started, ended; or refused for an unknown id or one already seen.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}};
use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
use bevy::prelude::*;

const SOURCE: &str = r#"
fn declare() {
    #{provider: "scene_probe", key: "director", schemas: [
        #{key: "scene", fields: [
            #{tag: 1, name: "step", kind: "u32"},
            #{tag: 2, name: "heard", kind: "text 256"},
            #{tag: 3, name: "busy", kind: "text 128"},
        ]},
    ], entries: [
        #{key: "tick", phase: "actor_think", trigger: #{port: "story_tick", selector: "blink_run"},
          writes: ["scene"], requests: ["play_cutscene"], limit: 3},
        #{key: "answer", phase: "actor_think", trigger: #{port: "action_result", selector: "answer"}, writes: ["scene"]},
    ]}
}
fn ask(id, n) {
    request("play_cutscene", #{id: id, reply: #{request: n, selector: "scene_probe::director/answer"}})
}
fn tick() {
    if !this.trigger.flags.contains("scene_probe_begin") { return; }
    let scene = this.state.scene;
    scene.step += 1;
    if scene.step == 3 {
        this.requests.push(ask("probe_scene", 1));
        this.requests.push(ask("no_such_scene", 2));
        this.requests.push(ask("probe_seen", 3));
    }
    if scene.step % 10 == 5 { scene.busy += if this.trigger.busy { "B" } else { "-" }; }
    this.state.scene = scene;
}
fn answer() {
    if this.trigger.stage == "submitted" { return; }
    this.state.scene.heard += `${this.trigger.ticket.request}:${this.trigger.code};`;
}
"#;

/// ⭐ One request is answered `cutscene_queued`, `cutscene_started` and, after
/// its half-second beat, terminal `cutscene_ended`; an unknown id and a
/// cutscene whose seen flag is already set are refused. Sync-test host, so
/// every answer survives the rewinds it rides through.
#[test]
fn a_script_hears_its_cutscene_queued_started_and_ended() {
    use ambition_platformer2d::cutscene::{CutsceneBeat, CutsceneLibrary, CutsceneScript};
    let dir = std::env::temp_dir().join(format!("ambition_cutscene_outcomes_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("director.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(crate::common::fixed_60hz_room_options("blink_run")
        .with_extension_module_files(vec![file])
        .with_sync_test_rollback_settings(4, 12)).unwrap();
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    {
        let mut library = world.resource_mut::<CutsceneLibrary>();
        library.insert(CutsceneScript::new("probe_scene", vec![CutsceneBeat::Wait { seconds: 0.5 }]));
        library.insert(CutsceneScript::new("probe_seen", vec![CutsceneBeat::Wait { seconds: 0.5 }])
            .with_seen_flag("scene_probe_seen"));
    }
    let mut save = world.resource_mut::<ambition_platformer2d::session::AmbitionGameSave>();
    save.data_mut().set_flag("scene_probe_seen", true);
    save.data_mut().set_flag("scene_probe_begin", true);
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 70);
    sim.rollback_health().unwrap();
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let world = sim.world_mut();
    let root = world.query_filtered::<Entity, With<RoomInstanceRoot>>().single(world).unwrap();
    let scene = world.get::<BodyRecords>(root).unwrap().get(&SchemaKey::new("scene_probe", "scene", 1)).cloned().unwrap();
    assert_eq!(scene.get(FieldRef(1)).unwrap(),
        &Value::Text("1:cutscene_queued;2:unknown_cutscene;3:cutscene_already_seen;1:cutscene_started;1:cutscene_ended;".into()),
        "every stage of the played request, and both refusals");
    assert_eq!(scene.get(FieldRef(2)).unwrap(), &Value::Text("BBB----".into()),
        "the room is busy for the half-second the cutscene plays, then free");
    let _ = std::fs::remove_dir_all(dir);
}
