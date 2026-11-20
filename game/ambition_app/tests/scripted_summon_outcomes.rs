//! A script summons a ride and hears what the summon executor did with it:
//! the mount it built, by identity, or that nothing was built; an empty label
//! is refused before the executor sees it.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("ride_probe", "rides", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "ride_probe", key: "stable",
       schemas: [#{ key: "rides", fields: [
           #{ tag: 1, name: "step", kind: "u32" },
           #{ tag: 2, name: "heard", kind: "text 256" },
           #{ tag: 3, name: "mount", kind: "text 128" },
       ] }],
       entries: [
           #{ key: "summon", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "ride_probe.call" },
              writes: ["rides"], requests: ["summon_ride"], limit: 3 },
           #{ key: "answer", phase: "actor_think",
              trigger: #{ port: "action_result", selector: "answer" }, writes: ["rides"] },
       ],
    }
}
fn ride(n, label, character) {
    request("summon_ride", #{ label: label, character_id: character, half_size: [48.0, 22.0],
        seconds: 5.0, board_within: 10.0, board_deadline_s: 0.1,
        reply: #{ request: n, selector: "ride_probe::stable/answer" } })
}
fn summon() {
    let step = this.state.rides.step + 1;
    this.state.rides.step = step;
    // The executor builds one tick's summons as a batch, refused whole when
    // any character resolves nothing: ask the unknown one on a later tick.
    if step == 1 {
        this.requests.push(ride(1, "probe_shark", "npc_burning_flying_shark"));
        this.requests.push(ride(3, "", "npc_burning_flying_shark"));
    }
    if step == 4 { this.requests.push(ride(4, "probe_ghost", "no_such_character")); }
}
fn answer() {
    if this.trigger.stage == "submitted" { return; }
    this.state.rides.heard += `${this.trigger.ticket.request}:${this.trigger.code};`;
    if this.trigger.code == "ride_summoned" { this.state.rides.mount = this.trigger.subjects[0].key; }
}
"#;

/// ⭐ The ride the executor builds is answered `ride_summoned` naming the
/// mount that exists; a character that resolves nothing is `ride_not_made`;
/// an empty label is refused at once. Sync-test host.
#[test]
fn a_script_hears_which_ride_was_built() {
    let dir = std::env::temp_dir().join(format!("ambition_summon_outcomes_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("stable.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    let world = sim.world_mut();
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["ride_probe.call".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 12);
    sim.rollback_health().unwrap();
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let world = sim.world_mut();
    let record = world.get::<BodyRecords>(player).unwrap().get(&RECORD).cloned().unwrap();
    assert_eq!(record.get(FieldRef(1)).unwrap(),
        &Value::Text("1:ride_summoning;3:invalid_label;1:ride_summoned;4:ride_summoning;4:ride_not_made;".into()),
        "the empty label is refused at once, and each summon is answered with what the executor built");
    let Value::Text(mount) = record.get(FieldRef(2)).unwrap().clone() else { panic!("mount id") };
    let found = world.query::<(&ambition_platformer2d::platformer::sim_id::SimId,
        &ambition_platformer2d::characters::actor::CharacterForm)>().iter(world)
        .find(|(id, _)| id.as_str() == mount).map(|(_, form)| form.id().to_owned());
    assert_eq!(found.as_deref(), Some("npc_burning_flying_shark"), "the answer names the mount that was built ({mount})");
    let _ = std::fs::remove_dir_all(dir);
}
